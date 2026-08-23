#![allow(unused_must_use)]
use loci2d::network::{
    ClientIntent, DisconnectIntent, GamePacket, JoinIntent, MoveIntent, PingIntent,
    ReplayIntentEntry, ServerPacket, Vector2, client_intent, run_server, server_packet,
};
use loci2d::replay::{ReplayPlayer, ReplayRecorder};
use loci2d::world::fixed_point::DeterministicVector2;
use loci2d::world::instance::Instance;
use prost::Message;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[test]
fn test_live_spectator_replay_broadcasting() {
    let tick_rate = 30u32;
    let total_ticks = 40u64;

    // 1. Record a deterministic match with 2 players
    let mut recorder = ReplayRecorder::new(1, tick_rate, 42, "spectator_arena".to_string(), 10, "".to_string());
    let mut instance = Instance::new(1, tick_rate, 60, 42);

    // Tick 1: Player 1 (Alice) & Player 2 (Bob) Join
    instance.handle_join("127.0.0.1:1001".parse().unwrap(), "Alice".to_string());
    instance.handle_join("127.0.0.1:1002".parse().unwrap(), "Bob".to_string());

    recorder.record_tick(
        1,
        vec![
            ReplayIntentEntry {
                entity_id: 1,
                player_name: "Alice".to_string(),
                intent: Some(ClientIntent {
                    intent: Some(client_intent::Intent::Join(JoinIntent {
                        player_name: "Alice".to_string(),
                    })),
                }),
            },
            ReplayIntentEntry {
                entity_id: 2,
                player_name: "Bob".to_string(),
                intent: Some(ClientIntent {
                    intent: Some(client_intent::Intent::Join(JoinIntent {
                        player_name: "Bob".to_string(),
                    })),
                }),
            },
        ],
    );
    instance.tick(1);

    // Tick 2: Movement
    instance.entities.get_mut(&1).unwrap().velocity = DeterministicVector2::from_f32(1.0, 0.0);
    instance.entities.get_mut(&2).unwrap().velocity = DeterministicVector2::from_f32(0.0, 1.0);

    recorder.record_tick(
        2,
        vec![
            ReplayIntentEntry {
                entity_id: 1,
                player_name: String::new(),
                intent: Some(ClientIntent {
                    intent: Some(client_intent::Intent::Move(MoveIntent {
                        direction: Some(Vector2 {
                            x_bits: (1.0f32 * 65536.0) as i32,
                            y_bits: 0,
                        }),
                    })),
                }),
            },
            ReplayIntentEntry {
                entity_id: 2,
                player_name: String::new(),
                intent: Some(ClientIntent {
                    intent: Some(client_intent::Intent::Move(MoveIntent {
                        direction: Some(Vector2 {
                            x_bits: 0,
                            y_bits: (1.0f32 * 65536.0) as i32,
                        }),
                    })),
                }),
            },
        ],
    );

    for t in 2..=total_ticks {
        instance.tick(t);
        recorder.maybe_record_checkpoint(t, &instance);
    }

    let bytes = recorder.to_bytes().expect("Failed to encode ReplayFile");
    let temp_dir = tempfile::tempdir().unwrap();
    let replay_path = temp_dir.path().join("spectator_test.loci");
    std::fs::write(&replay_path, &bytes).unwrap();

    // 2. Start Spectator Replay Broadcast Server
    let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").expect("Failed to bind UDP socket"));
    let broadcast_addr = socket.local_addr().unwrap();

    let (intent_tx, intent_rx) = mpsc::channel();
    let net_socket = Arc::clone(&socket);
    thread::spawn(move || {
        run_server(net_socket, intent_tx);
    });

    let running = Arc::new(AtomicBool::new(true));
    let running_clone = Arc::clone(&running);
    let mut player = ReplayPlayer::load_from_file(&replay_path).expect("Failed to load replay");

    const TEST_REPLAY_SPEED: f32 = 2.0;
    let player_handle = thread::spawn(move || {
        player.broadcast_live(socket, intent_rx, TEST_REPLAY_SPEED, 128, running_clone);
    });

    // 3. Connect Spectator 1 (Godot-like) sending JoinIntent
    let spectator1_sock = UdpSocket::bind("127.0.0.1:0").unwrap();
    spectator1_sock
        .set_read_timeout(Some(Duration::from_millis(1000)))
        .unwrap();

    let join_packet = GamePacket {
        sequence_id: 1,
        timestamp: 0,
        intent: Some(ClientIntent {
            intent: Some(client_intent::Intent::Join(JoinIntent {
                player_name: "GodotSpectator".to_string(),
            })),
        }),
    };
    let mut join_buf = Vec::new();
    join_packet.encode(&mut join_buf).unwrap();
    spectator1_sock.send_to(&join_buf, broadcast_addr).unwrap();

    // 4. Connect Spectator 2 (Love2D-like) sending PingIntent
    let spectator2_sock = UdpSocket::bind("127.0.0.1:0").unwrap();
    spectator2_sock
        .set_read_timeout(Some(Duration::from_millis(1000)))
        .unwrap();

    let ping_packet = GamePacket {
        sequence_id: 1,
        timestamp: 0,
        intent: Some(ClientIntent {
            intent: Some(client_intent::Intent::Ping(PingIntent {})),
        }),
    };
    let mut ping_buf = Vec::new();
    ping_packet.encode(&mut ping_buf).unwrap();
    spectator2_sock.send_to(&ping_buf, broadcast_addr).unwrap();

    // 5. Verify Spectator 1 receives broadcasted WorldState frames
    let mut recv_buf = [0u8; 2048];
    let mut spectator1_saw_entities = false;
    for _ in 0..20 {
        if let Ok((bytes_read, _)) = spectator1_sock.recv_from(&mut recv_buf)
            && let Ok(packet) = ServerPacket::decode(&recv_buf[..bytes_read])
            && let Some(server_packet::Payload::WorldState(state)) = packet.payload
            && state.tick >= 1
        {
            assert_eq!(state.entities.len(), 2);
            let names: Vec<String> = state.entities.iter().map(|e| e.name.clone()).collect();
            assert!(names.contains(&"Alice".to_string()));
            assert!(names.contains(&"Bob".to_string()));
            assert!(!names.contains(&"GodotSpectator".to_string()));
            spectator1_saw_entities = true;
            break;
        }
    }
    assert!(
        spectator1_saw_entities,
        "Spectator 1 must receive WorldState frame with 2 entities"
    );

    // 6. Verify Spectator 2 receives broadcasted WorldState frames
    let mut spectator2_saw_entities = false;
    for _ in 0..20 {
        if let Ok((bytes_read, _)) = spectator2_sock.recv_from(&mut recv_buf)
            && let Ok(packet) = ServerPacket::decode(&recv_buf[..bytes_read])
            && let Some(server_packet::Payload::WorldState(state)) = packet.payload
            && state.tick >= 1
        {
            assert_eq!(state.entities.len(), 2);
            spectator2_saw_entities = true;
            break;
        }
    }
    assert!(
        spectator2_saw_entities,
        "Spectator 2 must receive WorldState frame with 2 entities"
    );

    // 7. Spectator 1 Disconnects
    let disc_packet = GamePacket {
        sequence_id: 2,
        timestamp: 0,
        intent: Some(ClientIntent {
            intent: Some(client_intent::Intent::Disconnect(DisconnectIntent {
                reason: "Spectator closed window".to_string(),
            })),
        }),
    };
    let mut disc_buf = Vec::new();
    disc_packet.encode(&mut disc_buf).unwrap();
    spectator1_sock.send_to(&disc_buf, broadcast_addr).unwrap();

    // Allow broadcast to finish
    player_handle.join().unwrap();
}
