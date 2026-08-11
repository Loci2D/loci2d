use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use prost::Message;
use loci2d::network::{
    run_server, GamePacket, ClientIntent, Vector2, MoveIntent, JoinIntent, DisconnectIntent,
    client_intent, ReplayFile,
};
use loci2d::world::instance::Instance;
use loci2d::game_loop::tick::GameLoop;

#[test]
fn test_live_match_recording_flow() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let replay_path = temp_dir.path().join("test_match.loci");
    let replay_path_str = replay_path.to_str().unwrap().to_string();

    let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").expect("Failed to bind test server socket"));
    let server_addr = socket.local_addr().expect("Failed to get local addr");

    let (intent_tx, intent_rx) = mpsc::channel();

    let net_socket = Arc::clone(&socket);
    thread::spawn(move || {
        run_server(net_socket, intent_tx);
    });

    let loop_socket = Arc::clone(&socket);
    let loop_path_str = replay_path_str.clone();
    
    let mut game_loop = GameLoop::new(60);
    game_loop.enable_recording(1, 42, "arena_test".to_string(), 5, loop_path_str);
    let running: Arc<AtomicBool> = game_loop.running_handle();

    let loop_handle = thread::spawn(move || {
        let instance = Instance::new(1, 60, 5);
        game_loop.start(instance, intent_rx, loop_socket);
    });

    // Client connects and performs actions
    let client_sock = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind client socket");

    // 1. Join
    let join_packet = GamePacket {
        sequence_id: 1,
        timestamp: 0,
        intent: Some(ClientIntent {
            intent: Some(client_intent::Intent::Join(JoinIntent {
                player_name: "Alice".to_string(),
            })),
        }),
    };
    let mut buf = Vec::new();
    join_packet.encode(&mut buf).unwrap();
    client_sock.send_to(&buf, server_addr).unwrap();
    thread::sleep(Duration::from_millis(50));

    // 2. Move
    let move_packet = GamePacket {
        sequence_id: 2,
        timestamp: 0,
        intent: Some(ClientIntent {
            intent: Some(client_intent::Intent::Move(MoveIntent {
                direction: Some(Vector2 { x: 3.0, y: -1.5 }),
            })),
        }),
    };
    buf.clear();
    move_packet.encode(&mut buf).unwrap();
    client_sock.send_to(&buf, server_addr).unwrap();
    thread::sleep(Duration::from_millis(50));

    // 3. Disconnect
    let disc_packet = GamePacket {
        sequence_id: 3,
        timestamp: 0,
        intent: Some(ClientIntent {
            intent: Some(client_intent::Intent::Disconnect(DisconnectIntent {
                reason: "Match finished".to_string(),
            })),
        }),
    };
    buf.clear();
    disc_packet.encode(&mut buf).unwrap();
    client_sock.send_to(&buf, server_addr).unwrap();
    thread::sleep(Duration::from_millis(150));

    // Stop game loop gracefully and wait for thread to join and flush replay file
    running.store(false, Ordering::Relaxed);
    loop_handle.join().expect("Game loop thread failed to join");

    // Assert that the replay file was generated directly by GameLoop upon shutdown
    assert!(replay_path.exists(), "Replay file must exist after game loop shutdown");
    let bytes = std::fs::read(&replay_path).expect("Failed to read replay file");
    let replay = ReplayFile::decode(&bytes[..]).expect("Failed to decode ReplayFile");

    let header = replay.header.expect("Missing replay header");
    assert_eq!(header.magic, "LOCI_REPLAY");
    assert_eq!(header.version, 1);
    assert_eq!(header.tick_rate, 60);
    assert_eq!(header.random_seed, 42);
    assert_eq!(header.map_name, "arena_test");

    assert!(!replay.frames.is_empty(), "Replay should contain recorded frames from GameLoop");

    // Verify recorded intents were stamped with entity_id == 1
    let join_frame = replay.frames.iter().find(|f| {
        f.entries.iter().any(|e| matches!(&e.intent, Some(ClientIntent { intent: Some(client_intent::Intent::Join(_)) })))
    }).expect("Join frame should be recorded in replay");
    
    let join_entry = &join_frame.entries[0];
    assert_eq!(join_entry.entity_id, 1);
    assert_eq!(join_entry.player_name, "Alice");

    // Verify checkpoints were captured on intervals
    assert!(!replay.checkpoints.is_empty(), "Checkpoints should be captured on interval");
    for cp in &replay.checkpoints {
        assert_eq!(cp.state_sha256.len(), 32, "Checkpoint state hash must be 32 bytes");
    }
}
