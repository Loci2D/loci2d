use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use prost::Message;
use loci2d::game_loop::tick::GameLoop;
use loci2d::network::{
    run_server, GamePacket,
    client_intent, ClientIntent, DisconnectIntent, JoinIntent, MoveIntent, ReplayIntentEntry,
    Vector2,
};
use loci2d::replay::{ReplayPlayer, ReplayRecorder};
use loci2d::world::fixed_point::DeterministicVector2;
use loci2d::world::instance::Instance;

#[test]
fn test_1000_tick_multi_player_replay_determinism() {
    let tick_rate = 30u32;
    let total_ticks = 1000u64;
    let checkpoint_interval = 50u64;
    let seed = 99999u64;

    let mut recorder = ReplayRecorder::new(1, tick_rate, seed, "determinism_arena".to_string(), checkpoint_interval);
    let mut author_instance = Instance::new(1, tick_rate, 60);

    // Schedule 10 players joining and moving over 1000 ticks
    let player_count = 10;
    for tick in 1..=total_ticks {
        let mut tick_entries = Vec::new();

        // Staggered joins
        if tick <= player_count * 10 && tick % 10 == 0 {
            let pid = tick / 10;
            let name = format!("Player_{}", pid);
            author_instance.handle_join(format!("127.0.0.1:{}", 10000 + pid).parse().unwrap(), name.clone());

            tick_entries.push(ReplayIntentEntry {
                entity_id: pid,
                player_name: name,
                intent: Some(ClientIntent {
                    intent: Some(client_intent::Intent::Join(JoinIntent {
                        player_name: format!("Player_{}", pid),
                    })),
                }),
            });
        }

        // Direction changes based on tick
        if tick % 25 == 0 {
            for pid in 1..=player_count {
                if let Some(entity) = author_instance.entities.get_mut(&pid) {
                    let angle = ((tick * pid) % 360) as f32;
                    let dx = (angle.to_radians()).cos() * 2.0;
                    let dy = (angle.to_radians()).sin() * 2.0;
                    entity.velocity = DeterministicVector2::from_f32(dx, dy);

                    tick_entries.push(ReplayIntentEntry {
                        entity_id: pid,
                        player_name: String::new(),
                        intent: Some(ClientIntent {
                            intent: Some(client_intent::Intent::Move(MoveIntent {
                                direction: Some(Vector2 { x: dx, y: dy }),
                            })),
                        }),
                    });
                }
            }
        }

        // Staggered disconnects in the last 200 ticks
        if tick > 800 && tick % 40 == 0 {
            let pid = (tick - 800) / 40;
            if pid <= player_count {
                author_instance.entities.remove(&pid);
                tick_entries.push(ReplayIntentEntry {
                    entity_id: pid,
                    player_name: String::new(),
                    intent: Some(ClientIntent {
                        intent: Some(client_intent::Intent::Disconnect(DisconnectIntent {
                            reason: "Test complete".to_string(),
                        })),
                    }),
                });
            }
        }

        // Record tick entries
        recorder.record_tick(tick, tick_entries);

        // Advance author simulation
        author_instance.tick(tick);

        // Record checkpoints
        recorder.maybe_record_checkpoint(tick, &author_instance);
    }

    // Assert checkpoints were captured
    assert_eq!(recorder.checkpoint_count(), (total_ticks / checkpoint_interval) as usize);

    let bytes = recorder.to_bytes().expect("Failed to encode ReplayFile");
    
    // Save to tempfile and verify via ReplayPlayer
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("match_1000_ticks.loci");
    std::fs::write(&file_path, &bytes).unwrap();

    let mut player = ReplayPlayer::load_from_file(&file_path).expect("Failed to load ReplayPlayer");
    let report = player.verify_determinism().expect("1000-tick replay must verify with 0 desyncs");

    assert_eq!(report.total_ticks, total_ticks);
    assert_eq!(report.verified_checkpoints, (total_ticks / checkpoint_interval) as usize);
    assert!(!report.final_hash.is_empty());
}

#[test]
fn test_rejoin_entity_state_preservation_determinism() {
    let mut recorder = ReplayRecorder::new(1, 30, 42, "rejoin_arena".to_string(), 10);
    let mut live_instance = Instance::new(1, 30, 60);
    let addr = "127.0.0.1:20000".parse().unwrap();

    // 1. Initial Join as "Alice"
    live_instance.handle_join(addr, "Alice".to_string());
    recorder.record_tick(1, vec![ReplayIntentEntry {
        entity_id: 1,
        player_name: "Alice".to_string(),
        intent: Some(ClientIntent {
            intent: Some(client_intent::Intent::Join(JoinIntent {
                player_name: "Alice".to_string(),
            })),
        }),
    }]);
    live_instance.tick(1);

    // 2. Set velocity and tick 2..=5
    live_instance.entities.get_mut(&1).unwrap().velocity = DeterministicVector2::from_f32(2.0, 1.0);
    recorder.record_tick(2, vec![ReplayIntentEntry {
        entity_id: 1,
        player_name: String::new(),
        intent: Some(ClientIntent {
            intent: Some(client_intent::Intent::Move(MoveIntent {
                direction: Some(Vector2 { x: 2.0, y: 1.0 }),
            })),
        }),
    }]);

    for t in 2..=5 {
        live_instance.tick(t);
    }

    // 3. Rejoin with new name "Alice_Updated" at tick 6
    live_instance.handle_join(addr, "Alice_Updated".to_string());
    recorder.record_tick(6, vec![ReplayIntentEntry {
        entity_id: 1,
        player_name: "Alice_Updated".to_string(),
        intent: Some(ClientIntent {
            intent: Some(client_intent::Intent::Join(JoinIntent {
                player_name: "Alice_Updated".to_string(),
            })),
        }),
    }]);

    for t in 6..=10 {
        live_instance.tick(t);
    }
    recorder.maybe_record_checkpoint(10, &live_instance);

    let bytes = recorder.to_bytes().unwrap();
    let mut player = ReplayPlayer::from_bytes(&bytes).unwrap();
    let report = player.verify_determinism().expect("Rejoin replay must match live state deterministically");

    assert_eq!(report.total_ticks, 10);
    assert_eq!(report.verified_checkpoints, 1);
}

#[test]
fn test_inactivity_timeout_disconnect_synchronization() {
    let temp_dir = tempfile::tempdir().unwrap();
    let replay_path = temp_dir.path().join("timeout_match.loci");
    let replay_path_str = replay_path.to_str().unwrap().to_string();

    let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").unwrap());
    let server_addr = socket.local_addr().unwrap();

    let (intent_tx, intent_rx) = mpsc::channel();
    let net_socket = Arc::clone(&socket);
    thread::spawn(move || {
        run_server(net_socket, intent_tx);
    });

    let loop_socket = Arc::clone(&socket);
    let mut game_loop = GameLoop::new(60);
    // 0-second timeout so it times out immediately on the next sweep
    game_loop.enable_recording(1, 42, "timeout_arena".to_string(), 5, replay_path_str);
    let running: Arc<AtomicBool> = game_loop.running_handle();

    let loop_handle = thread::spawn(move || {
        let instance = Instance::new(1, 60, 0); // 0s timeout
        game_loop.start(instance, intent_rx, loop_socket);
    });

    let client_sock = UdpSocket::bind("127.0.0.1:0").unwrap();

    // Client joins
    let join_packet = GamePacket {
        sequence_id: 1,
        timestamp: 0,
        intent: Some(ClientIntent {
            intent: Some(client_intent::Intent::Join(JoinIntent {
                player_name: "TimeoutPlayer".to_string(),
            })),
        }),
    };
    let mut buf = Vec::new();
    join_packet.encode(&mut buf).unwrap();
    client_sock.send_to(&buf, server_addr).unwrap();

    // Wait for server to process join and time out the client on subsequent ticks
    thread::sleep(Duration::from_millis(150));

    // Shutdown loop
    running.store(false, Ordering::Relaxed);
    loop_handle.join().unwrap();

    assert!(replay_path.exists());
    let mut player = ReplayPlayer::load_from_file(&replay_path).unwrap();
    let report = player.verify_determinism().expect("Timeout disconnect replay must verify with 0 desyncs");
    assert!(report.total_ticks > 0);
}

#[test]
fn test_desync_diagnostic_report_on_tampered_frame() {
    let mut recorder = ReplayRecorder::new(1, 30, 42, "test_arena".to_string(), 10);
    let mut instance = Instance::new(1, 30, 60);

    instance.handle_join("127.0.0.1:5000".parse().unwrap(), "Alice".to_string());
    recorder.record_tick(1, vec![ReplayIntentEntry {
        entity_id: 1,
        player_name: "Alice".to_string(),
        intent: Some(ClientIntent {
            intent: Some(client_intent::Intent::Join(JoinIntent {
                player_name: "Alice".to_string(),
            })),
        }),
    }]);
    instance.tick(1);

    for t in 2..=10 {
        instance.tick(t);
    }

    // Tamper with checkpoint state hash
    recorder.record_checkpoint(10, [0xEE; 32], 1);

    let bytes = recorder.to_bytes().unwrap();
    let mut player = ReplayPlayer::from_bytes(&bytes).unwrap();

    let desync = player.verify_determinism().expect_err("Desync must be detected");
    assert_eq!(desync.tick, 10);
    assert_eq!(desync.expected_hash, loci2d::replay::hex_encode(&[0xEE; 32]));
    assert_ne!(desync.actual_hash, desync.expected_hash);
    assert!(!desync.entity_summary.is_empty());
    assert!(desync.entity_summary[0].contains("Alice"));
}

#[test]
fn test_corrupted_header_magic_fails_gracefully() {
    let recorder = ReplayRecorder::new(1, 30, 42, "test_arena".to_string(), 10);
    let bytes = recorder.to_bytes().unwrap();

    // Tamper with bytes to alter magic string
    let mut tampered = bytes;
    if let Some(pos) = tampered.windows(11).position(|w| w == b"LOCI_REPLAY") {
        tampered[pos] = b'X';
    }

    let result = ReplayPlayer::from_bytes(&tampered);
    assert!(result.is_err(), "Corrupted magic must return error");
}
