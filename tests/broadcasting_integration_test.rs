use loci2d::game_loop::tick::GameLoop;
use loci2d::network::{
    ClientIntent, DisconnectIntent, GamePacket, JoinIntent, MoveIntent, ServerPacket, Vector2,
    WorldState, client_intent, run_server, server_packet,
};
use loci2d::world::instance::Instance;
use prost::Message;
use std::net::UdpSocket;
use std::sync::Arc;
mod common;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

fn start_test_server(tick_rate: u32, timeout_secs: u64) -> (std::net::SocketAddr, Arc<UdpSocket>) {
    let socket =
        Arc::new(UdpSocket::bind("127.0.0.1:0").expect("Failed to bind test server socket"));
    let local_addr = socket.local_addr().expect("Failed to get local addr");

    let (intent_tx, intent_rx) = mpsc::channel();

    let net_socket = Arc::clone(&socket);
    thread::spawn(move || {
        run_server(net_socket, intent_tx);
    });

    let loop_socket = Arc::clone(&socket);
    thread::spawn(move || {
        let mut instance = Instance::new(1, tick_rate, timeout_secs, 42);
        let script = common::PASSTHROUGH_MOVEMENT_SCRIPT;
        instance.load_script(script).unwrap();
        let mut game_loop = GameLoop::new(tick_rate);
        game_loop.start(instance, intent_rx, loop_socket);
    });

    (local_addr, socket)
}

fn recv_server_packet(client_sock: &UdpSocket, timeout: Duration) -> Option<ServerPacket> {
    let original_timeout = client_sock.read_timeout().ok().flatten();
    client_sock.set_read_timeout(Some(timeout)).ok()?;

    let mut buf = [0u8; 2048];
    let result = match client_sock.recv_from(&mut buf) {
        Ok((num_bytes, _)) => ServerPacket::decode(&buf[..num_bytes]).ok(),
        Err(_) => None,
    };

    client_sock.set_read_timeout(original_timeout).ok()?;
    result
}

fn wait_for_snapshot(client_sock: &UdpSocket, timeout: Duration) -> Option<WorldState> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if let Some(ServerPacket {
            payload: Some(server_packet::Payload::WorldState(ws)),
            ..
        }) = recv_server_packet(client_sock, Duration::from_millis(50))
        {
            return Some(ws);
        }
    }
    None
}

fn wait_for_entities_matching<F>(client_sock: &UdpSocket, timeout: Duration, predicate: F) -> bool
where
    F: Fn(&WorldState) -> bool,
{
    let start = Instant::now();
    while start.elapsed() < timeout {
        let Some(ws) = wait_for_snapshot(client_sock, Duration::from_millis(50)) else {
            continue;
        };
        if predicate(&ws) {
            return true;
        }
    }
    false
}

#[test]
fn test_client_receives_world_state_on_join() {
    let (server_addr, _server_sock) = start_test_server(60, 5);
    let client_sock = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind client socket");

    // Send JoinIntent
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
    client_sock
        .send_to(&buf, server_addr)
        .expect("Failed to send join");

    // Wait for snapshot broadcast
    let ws = wait_for_snapshot(&client_sock, Duration::from_secs(2))
        .expect("Did not receive WorldState snapshot");
    assert_eq!(ws.entities.len(), 1);
    let entity = &ws.entities[0];
    assert_eq!(entity.name, "Alice");
    assert_eq!(entity.entity_type, 0); // Player
    let pos = entity.position.as_ref().unwrap();
    assert_eq!(pos.x_bits, 0);
    assert_eq!(pos.y_bits, 0);
}

#[test]
fn test_client_receives_position_updates_after_move() {
    let (server_addr, _server_sock) = start_test_server(60, 5);
    let client_sock = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind client socket");

    // 1. Join
    let join_packet = GamePacket {
        sequence_id: 1,
        timestamp: 0,
        intent: Some(ClientIntent {
            intent: Some(client_intent::Intent::Join(JoinIntent {
                player_name: "Bob".to_string(),
            })),
        }),
    };
    let mut buf = Vec::new();
    join_packet.encode(&mut buf).unwrap();
    client_sock.send_to(&buf, server_addr).unwrap();

    let _ = wait_for_snapshot(&client_sock, Duration::from_secs(1))
        .expect("Did not receive initial snapshot");

    // 2. Send MoveIntent
    let move_packet = GamePacket {
        sequence_id: 2,
        timestamp: 0,
        intent: Some(ClientIntent {
            intent: Some(client_intent::Intent::Move(MoveIntent {
                direction: Some(Vector2 {
                    x_bits: (2.0f32 * 65536.0) as i32,
                    y_bits: (1.0f32 * 65536.0) as i32,
                }),
            })),
        }),
    };
    buf.clear();
    move_packet.encode(&mut buf).unwrap();
    client_sock.send_to(&buf, server_addr).unwrap();

    // 3. Wait for subsequent snapshot reflecting movement
    thread::sleep(Duration::from_millis(100));
    let ws = wait_for_snapshot(&client_sock, Duration::from_secs(1))
        .expect("Did not receive updated snapshot");
    assert_eq!(ws.entities.len(), 1);
    let entity = &ws.entities[0];
    let pos = entity.position.as_ref().unwrap();
    let vel = entity.velocity.as_ref().unwrap();

    assert_eq!(vel.x_bits, (2.0f32 * 65536.0) as i32);
    assert_eq!(vel.y_bits, (1.0f32 * 65536.0) as i32);
    assert!(
        pos.x_bits > 0,
        "Expected pos.x_bits > 0, got {}",
        pos.x_bits
    );
    assert!(
        pos.y_bits > 0,
        "Expected pos.y_bits > 0, got {}",
        pos.y_bits
    );
}

#[test]
fn test_multi_client_world_state_broadcasting() {
    let (server_addr, _server_sock) = start_test_server(60, 5);
    let client1_sock = UdpSocket::bind("127.0.0.1:0").unwrap();
    let client2_sock = UdpSocket::bind("127.0.0.1:0").unwrap();

    // Client 1 joins as Alice
    let join1 = GamePacket {
        sequence_id: 1,
        timestamp: 0,
        intent: Some(ClientIntent {
            intent: Some(client_intent::Intent::Join(JoinIntent {
                player_name: "Alice".to_string(),
            })),
        }),
    };
    let mut buf = Vec::new();
    join1.encode(&mut buf).unwrap();
    client1_sock.send_to(&buf, server_addr).unwrap();

    // Client 2 joins as Charlie
    let join2 = GamePacket {
        sequence_id: 1,
        timestamp: 0,
        intent: Some(ClientIntent {
            intent: Some(client_intent::Intent::Join(JoinIntent {
                player_name: "Charlie".to_string(),
            })),
        }),
    };
    buf.clear();
    join2.encode(&mut buf).unwrap();
    client2_sock.send_to(&buf, server_addr).unwrap();

    let check_both_entities = |ws: &WorldState| {
        if ws.entities.len() != 2 {
            return false;
        }
        let names: Vec<String> = ws.entities.iter().map(|e| e.name.clone()).collect();
        names.contains(&"Alice".to_string()) && names.contains(&"Charlie".to_string())
    };

    let c1_saw_both =
        wait_for_entities_matching(&client1_sock, Duration::from_secs(2), check_both_entities);
    let c2_saw_both =
        wait_for_entities_matching(&client2_sock, Duration::from_secs(2), check_both_entities);

    assert!(
        c1_saw_both,
        "Client 1 should receive snapshot with both Alice and Charlie"
    );
    assert!(
        c2_saw_both,
        "Client 2 should receive snapshot with both Alice and Charlie"
    );
}

#[test]
fn test_despawn_synchronization_on_disconnect() {
    let (server_addr, _server_sock) = start_test_server(60, 5);
    let client1_sock = UdpSocket::bind("127.0.0.1:0").unwrap();
    let client2_sock = UdpSocket::bind("127.0.0.1:0").unwrap();

    // 1. Join Alice
    let join1 = GamePacket {
        sequence_id: 1,
        timestamp: 0,
        intent: Some(ClientIntent {
            intent: Some(client_intent::Intent::Join(JoinIntent {
                player_name: "Alice".to_string(),
            })),
        }),
    };
    let mut buf = Vec::new();
    join1.encode(&mut buf).unwrap();
    client1_sock.send_to(&buf, server_addr).unwrap();

    // 2. Join Dave
    let join2 = GamePacket {
        sequence_id: 1,
        timestamp: 0,
        intent: Some(ClientIntent {
            intent: Some(client_intent::Intent::Join(JoinIntent {
                player_name: "Dave".to_string(),
            })),
        }),
    };
    buf.clear();
    join2.encode(&mut buf).unwrap();
    client2_sock.send_to(&buf, server_addr).unwrap();

    // Wait until Alice sees both Alice and Dave
    let saw_both = wait_for_entities_matching(&client1_sock, Duration::from_secs(2), |ws| {
        ws.entities.len() == 2
    });
    assert!(saw_both, "Alice should have seen both players initially");

    // 3. Dave disconnects
    let dc = GamePacket {
        sequence_id: 2,
        timestamp: 0,
        intent: Some(ClientIntent {
            intent: Some(client_intent::Intent::Disconnect(DisconnectIntent {
                reason: "Dave is leaving".to_string(),
            })),
        }),
    };
    buf.clear();
    dc.encode(&mut buf).unwrap();
    client2_sock.send_to(&buf, server_addr).unwrap();

    // 4. Verify Alice sees only Alice now
    let saw_only_alice = wait_for_entities_matching(&client1_sock, Duration::from_secs(2), |ws| {
        ws.entities.len() == 1 && ws.entities[0].name == "Alice"
    });
    assert!(
        saw_only_alice,
        "Alice should receive snapshot with Dave omitted after disconnect"
    );
}
