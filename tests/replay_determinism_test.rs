#![allow(unused_must_use)]
use loci2d::game_loop::tick::GameLoop;
use loci2d::network::{
    ClientIntent, DisconnectIntent, GamePacket, JoinIntent, MoveIntent, MoveToPositionIntent,
    ReplayIntentEntry, Vector2, client_intent, run_server,
};
use loci2d::replay::{ReplayPlayer, ReplayRecorder};
use loci2d::world::fixed_point::DeterministicVector2;
use loci2d::world::instance::Instance;
use prost::Message;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[test]
fn test_1000_tick_multi_player_replay_determinism() {
    let tick_rate = 30u32;
    let total_ticks = 1000u64;
    let checkpoint_interval = 50u64;
    let seed = 99999u64;

    let mut recorder = ReplayRecorder::new(
        1,
        tick_rate,
        seed,
        "determinism_arena".to_string(),
        checkpoint_interval,
        "".to_string(),
    );
    let mut author_instance = Instance::new(1, tick_rate, 60, 42);

    // Schedule 10 players joining and moving over 1000 ticks
    let player_count = 10;
    for tick in 1..=total_ticks {
        let mut tick_entries = Vec::new();

        // Staggered joins
        if tick <= player_count * 10 && tick % 10 == 0 {
            let pid = tick / 10;
            let name = format!("Player_{}", pid);
            author_instance.handle_join(
                format!("127.0.0.1:{}", 10000 + pid).parse().unwrap(),
                name.clone(),
            );

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
                                direction: Some(DeterministicVector2::from_f32(dx, dy).to_proto()),
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
    assert_eq!(
        recorder.checkpoint_count(),
        (total_ticks / checkpoint_interval) as usize
    );

    let bytes = recorder.to_bytes().expect("Failed to encode ReplayFile");

    // Save to tempfile and verify via ReplayPlayer
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("match_1000_ticks.loci");
    std::fs::write(&file_path, &bytes).unwrap();

    let mut player = ReplayPlayer::load_from_file(&file_path).expect("Failed to load ReplayPlayer");
    let report = player
        .verify_determinism()
        .expect("1000-tick replay must verify with 0 desyncs");

    assert_eq!(report.total_ticks, total_ticks);
    assert_eq!(
        report.verified_checkpoints,
        (total_ticks / checkpoint_interval) as usize
    );
    assert!(!report.final_hash.is_empty());

    // Explicit Cross-Architecture Mathematical Proof:
    // This hash must be identical on both x86_64 and ARM64.
    // If floating-point non-determinism leaks in, this will fail on one of the runners.
    assert_eq!(
        report.final_hash,
        "56a0962085ffbce7af4d1a91a11f208ff77e98cace527bad14afc1abaacb7802",
        "Cross-architecture determinism compromised!"
    );
}

#[test]
fn test_rejoin_entity_state_preservation_determinism() {
    let mut recorder =
        ReplayRecorder::new(1, 30, 42, "rejoin_arena".to_string(), 10, "".to_string());
    let mut live_instance = Instance::new(1, 30, 60, 42);
    let addr = "127.0.0.1:20000".parse().unwrap();

    // 1. Initial Join as "Alice"
    live_instance.handle_join(addr, "Alice".to_string());
    recorder.record_tick(
        1,
        vec![ReplayIntentEntry {
            entity_id: 1,
            player_name: "Alice".to_string(),
            intent: Some(ClientIntent {
                intent: Some(client_intent::Intent::Join(JoinIntent {
                    player_name: "Alice".to_string(),
                })),
            }),
        }],
    );
    live_instance.tick(1);

    // 2. Set velocity and tick 2..=5
    live_instance.entities.get_mut(&1).unwrap().velocity = DeterministicVector2::from_f32(2.0, 1.0);
    recorder.record_tick(
        2,
        vec![ReplayIntentEntry {
            entity_id: 1,
            player_name: String::new(),
            intent: Some(ClientIntent {
                intent: Some(client_intent::Intent::Move(MoveIntent {
                    direction: Some(Vector2 {
                        x_bits: (2.0f32 * 65536.0) as i32,
                        y_bits: (1.0f32 * 65536.0) as i32,
                    }),
                })),
            }),
        }],
    );

    for t in 2..=5 {
        live_instance.tick(t);
    }

    // 3. Rejoin with new name "Alice_Updated" at tick 6
    live_instance.handle_join(addr, "Alice_Updated".to_string());
    recorder.record_tick(
        6,
        vec![ReplayIntentEntry {
            entity_id: 1,
            player_name: "Alice_Updated".to_string(),
            intent: Some(ClientIntent {
                intent: Some(client_intent::Intent::Join(JoinIntent {
                    player_name: "Alice_Updated".to_string(),
                })),
            }),
        }],
    );

    for t in 6..=10 {
        live_instance.tick(t);
    }
    recorder.maybe_record_checkpoint(10, &live_instance);

    let bytes = recorder.to_bytes().unwrap();
    let mut player = ReplayPlayer::from_bytes(&bytes).unwrap();
    let report = player
        .verify_determinism()
        .expect("Rejoin replay must match live state deterministically");

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
    game_loop.enable_recording(
        1,
        42,
        "timeout_arena".to_string(),
        5,
        replay_path_str,
        "".to_string(),
    );
    let running: Arc<AtomicBool> = game_loop.running_handle();

    let loop_handle = thread::spawn(move || {
        let instance = Instance::new(1, 60, 0, 42); // 0s timeout
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
    let report = player
        .verify_determinism()
        .expect("Timeout disconnect replay must verify with 0 desyncs");
    assert!(report.total_ticks > 0);
}

#[test]
fn test_desync_diagnostic_report_on_tampered_frame() {
    let mut recorder = ReplayRecorder::new(1, 30, 42, "test_arena".to_string(), 10, "".to_string());
    let mut instance = Instance::new(1, 30, 60, 42);

    instance.handle_join("127.0.0.1:5000".parse().unwrap(), "Alice".to_string());
    recorder.record_tick(
        1,
        vec![ReplayIntentEntry {
            entity_id: 1,
            player_name: "Alice".to_string(),
            intent: Some(ClientIntent {
                intent: Some(client_intent::Intent::Join(JoinIntent {
                    player_name: "Alice".to_string(),
                })),
            }),
        }],
    );
    instance.tick(1);

    for t in 2..=10 {
        instance.tick(t);
    }

    // Tamper with checkpoint state hash
    recorder.record_checkpoint(10, [0xEE; 32], 1);

    let bytes = recorder.to_bytes().unwrap();
    let mut player = ReplayPlayer::from_bytes(&bytes).unwrap();

    let desync = player
        .verify_determinism()
        .expect_err("Desync must be detected");
    assert_eq!(desync.tick, 10);
    assert_eq!(
        desync.expected_hash,
        loci2d::replay::hex_encode(&[0xEE; 32])
    );
    assert_ne!(desync.actual_hash, desync.expected_hash);
    assert!(!desync.entity_summary.is_empty());
    assert!(desync.entity_summary[0].contains("Alice"));
}

#[test]
fn test_corrupted_header_magic_fails_gracefully() {
    let recorder = ReplayRecorder::new(1, 30, 42, "test_arena".to_string(), 10, "".to_string());
    let bytes = recorder.to_bytes().unwrap();

    // Tamper with bytes to alter magic string
    let mut tampered = bytes;
    if let Some(pos) = tampered.windows(11).position(|w| w == b"LOCI_REPLAY") {
        tampered[pos] = b'X';
    }

    let result = ReplayPlayer::from_bytes(&tampered);
    assert!(result.is_err(), "Corrupted magic must return error");
}

#[test]
fn test_click_to_move_replay_determinism() {
    use fixed::types::I16F16;

    let tick_rate = 30u32;
    let total_ticks = 500u64;
    let checkpoint_interval = 25u64;
    let seed = 123456789u64;

    let mut recorder = ReplayRecorder::new(
        1,
        tick_rate,
        seed,
        "nav_replay_arena".to_string(),
        checkpoint_interval,
        "".to_string(),
    );
    let mut author_instance = Instance::new(1, tick_rate, 60, 42);

    let player_count = 5;

    // Join 5 players at tick 1
    let mut join_entries = Vec::new();
    for pid in 1..=player_count {
        let name = format!("NavPlayer_{}", pid);
        let addr = format!("127.0.0.1:{}", 15000 + pid).parse().unwrap();
        author_instance.handle_join(addr, name.clone());

        join_entries.push(ReplayIntentEntry {
            entity_id: pid,
            player_name: name.clone(),
            intent: Some(ClientIntent {
                intent: Some(client_intent::Intent::Join(JoinIntent {
                    player_name: name,
                })),
            }),
        });
    }
    recorder.record_tick(1, join_entries);
    author_instance.tick(1);

    for tick in 2..=total_ticks {
        let mut tick_entries = Vec::new();

        // Every 50 ticks, issue new destination MoveToPos commands to players
        if tick % 50 == 0 {
            for pid in 1..=player_count {
                let target_x = I16F16::from_num(((tick * pid) % 160) as i16 - 80);
                let target_y = I16F16::from_num(((tick * (pid + 3)) % 160) as i16 - 80);

                let move_to_pos_intent = ClientIntent {
                    intent: Some(client_intent::Intent::MoveToPos(MoveToPositionIntent {
                        target_position: Some(Vector2 {
                            x_bits: target_x.to_bits(),
                            y_bits: target_y.to_bits(),
                        }),
                    })),
                };

                let addr = format!("127.0.0.1:{}", 15000 + pid).parse().unwrap();
                author_instance.apply_intent(addr, move_to_pos_intent.clone());

                tick_entries.push(ReplayIntentEntry {
                    entity_id: pid,
                    player_name: String::new(),
                    intent: Some(move_to_pos_intent),
                });
            }
        }

        // At tick 175, player 2 issues a manual WASD preemption
        if tick == 175 {
            let wasd_intent = ClientIntent {
                intent: Some(client_intent::Intent::Move(MoveIntent {
                    direction: Some(
                        DeterministicVector2::new(I16F16::from_num(1), I16F16::from_num(-1))
                            .to_proto(),
                    ),
                })),
            };
            let addr = "127.0.0.1:15002".parse().unwrap();
            author_instance.apply_intent(addr, wasd_intent.clone());
            tick_entries.push(ReplayIntentEntry {
                entity_id: 2,
                player_name: String::new(),
                intent: Some(wasd_intent),
            });
        }

        recorder.record_tick(tick, tick_entries);
        author_instance.tick(tick);

        if tick % checkpoint_interval == 0 {
            let hash = loci2d::replay::compute_canonical_state_hash(&author_instance, tick);
            recorder.record_checkpoint(tick, hash, author_instance.entities.len() as u32);
        }
    }

    let bytes = recorder.to_bytes().unwrap();
    let mut player = ReplayPlayer::from_bytes(&bytes).unwrap();
    let report = player
        .verify_determinism()
        .expect("Click-to-move replay playback must match author checksums exactly");

    assert_eq!(report.total_ticks, total_ticks);
    assert_eq!(
        report.verified_checkpoints,
        (total_ticks / checkpoint_interval) as usize
    );
}
