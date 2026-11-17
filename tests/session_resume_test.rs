use fixed::types::I16F16;
use loci2d::game_loop::tick::GameLoop;
use loci2d::network::{
    ActionIntent, ClientIntent, DisconnectIntent, GamePacket, JoinIntent, MoveIntent,
    ServerPacket, Vector2, WorldState, client_intent, run_server, server_packet,
};
use loci2d::world::instance::{Instance, JoinResult};
use prost::Message;
use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

fn recv_packet(sock: &UdpSocket, timeout: Duration) -> Option<ServerPacket> {
    let orig = sock.read_timeout().ok().flatten();
    sock.set_read_timeout(Some(timeout)).ok()?;
    let mut buf = [0u8; 4096];
    let res = match sock.recv_from(&mut buf) {
        Ok((n, _)) => ServerPacket::decode(&buf[..n]).ok(),
        Err(_) => None,
    };
    let _ = sock.set_read_timeout(orig);
    res
}

fn wait_for_response(sock: &UdpSocket, timeout: Duration) -> Option<(String, String)> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if let Some(ServerPacket {
            payload: Some(server_packet::Payload::Response(resp)),
            ..
        }) = recv_packet(sock, Duration::from_millis(50))
        {
            return Some((resp.status, resp.session_token));
        }
    }
    None
}

fn wait_for_world_state(sock: &UdpSocket, timeout: Duration) -> Option<WorldState> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if let Some(ServerPacket {
            payload: Some(server_packet::Payload::WorldState(ws)),
            ..
        }) = recv_packet(sock, Duration::from_millis(50))
        {
            return Some(ws);
        }
    }
    None
}

#[test]
fn test_session_resume_across_new_udp_socket_and_grace_period() {
    let script = r#"
        function on_init()
            Loci.Commands.set_global("join_count", "0")
            Loci.Commands.set_global("leave_count", "0")
        end

        function on_player_join(entity_id)
            local joins = tonumber(Loci.get_global("join_count")) or 0
            Loci.Commands.set_global("join_count", tostring(joins + 1))
            Loci.Commands.set_property(entity_id, "team", "Blue")
            Loci.Commands.set_property(entity_id, "score", "0")
        end

        function on_player_leave(entity_id)
            local leaves = tonumber(Loci.get_global("leave_count")) or 0
            Loci.Commands.set_global("leave_count", tostring(leaves + 1))
        end

        function on_move_intent(entity_id, dx, dy)
            Loci.Commands.set_velocity(entity_id, { x = dx, y = dy })
        end

        function on_action(entity_id, ability_id, dir_x, dir_y)
            local score = tonumber(Loci.get_entity_property(entity_id, "score")) or 0
            Loci.Commands.set_property(entity_id, "score", tostring(score + 10))
        end
    "#;

    let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").expect("bind server"));
    let server_addr = socket.local_addr().unwrap();
    let (intent_tx, intent_rx) = mpsc::channel();

    let net_socket = Arc::clone(&socket);
    thread::spawn(move || run_server(net_socket, intent_tx));

    let mut game_loop = GameLoop::new(60);
    let running = game_loop.running_handle();

    let loop_socket = Arc::clone(&socket);
    let loop_handle = thread::spawn(move || {
        let mut instance = Instance::new(1, 60, 1, 42).with_grace_period_secs(30);
        instance.load_script(script).expect("load script");
        game_loop.start(instance, intent_rx, loop_socket);
    });

    // 1. Client joins over socket 1
    let sock1 = UdpSocket::bind("127.0.0.1:0").unwrap();
    let join_pkt = GamePacket {
        sequence_id: 1,
        timestamp: 0,
        intent: Some(ClientIntent {
            intent: Some(client_intent::Intent::Join(JoinIntent {
                player_name: "Arthur".to_string(),
                schema_version: 1,
                session_token: String::new(),
            })),
        }),
    };
    sock1
        .send_to(&join_pkt.encode_to_vec(), server_addr)
        .unwrap();

    let (status, token) =
        wait_for_response(&sock1, Duration::from_secs(2)).expect("must receive join ServerResponse");
    assert_eq!(status, "joined");
    assert!(!token.is_empty(), "server must issue non-empty session_token");

    // Cast action (+10 score) and move right
    let action_pkt = GamePacket {
        sequence_id: 2,
        timestamp: 0,
        intent: Some(ClientIntent {
            intent: Some(client_intent::Intent::Action(ActionIntent {
                ability_id: 1,
                target_direction: None,
            })),
        }),
    };
    sock1
        .send_to(&action_pkt.encode_to_vec(), server_addr)
        .unwrap();

    let move_pkt = GamePacket {
        sequence_id: 3,
        timestamp: 0,
        intent: Some(ClientIntent {
            intent: Some(client_intent::Intent::Move(MoveIntent {
                direction: Some(Vector2 {
                    x_bits: 65536,
                    y_bits: 0,
                }),
            })),
        }),
    };
    sock1
        .send_to(&move_pkt.encode_to_vec(), server_addr)
        .unwrap();
    thread::sleep(Duration::from_millis(80));
    drop(sock1); // Abrupt drop (no DisconnectIntent)

    // 2. Wait past 1s inactivity timeout so session enters GracePeriod (30s)
    thread::sleep(Duration::from_millis(1200));

    // 3. Client reconnects from brand-new UDP socket on different ephemeral port with saved token
    let sock2 = UdpSocket::bind("127.0.0.1:0").unwrap();
    let resume_pkt = GamePacket {
        sequence_id: 4,
        timestamp: 0,
        intent: Some(ClientIntent {
            intent: Some(client_intent::Intent::Join(JoinIntent {
                player_name: "Arthur".to_string(),
                schema_version: 1,
                session_token: token.clone(),
            })),
        }),
    };
    sock2
        .send_to(&resume_pkt.encode_to_vec(), server_addr)
        .unwrap();

    let (resume_status, returned_token) = wait_for_response(&sock2, Duration::from_secs(2))
        .expect("must receive resume ServerResponse on new socket");
    assert_eq!(resume_status, "resumed");
    assert_eq!(returned_token, token);

    let ws = wait_for_world_state(&sock2, Duration::from_secs(2))
        .expect("new socket must receive WorldState snapshots");
    assert_eq!(ws.entities.len(), 1, "must NOT spawn a duplicate clone");
    let ent = &ws.entities[0];
    assert_eq!(ent.id, 1);
    assert_eq!(ent.name, "Arthur");
    assert!(
        ent.position.as_ref().unwrap().x_bits > 0,
        "previous avatar position must be preserved"
    );

    let team = ent.properties.iter().find(|p| p.key == "team").unwrap();
    let score = ent.properties.iter().find(|p| p.key == "score").unwrap();
    assert_eq!(team.value, "Blue");
    assert_eq!(score.value, "10");

    let join_count = ws.globals.iter().find(|g| g.key == "join_count").unwrap();
    let leave_count = ws.globals.iter().find(|g| g.key == "leave_count").unwrap();
    assert_eq!(
        join_count.value, "1",
        "on_player_join must NOT re-fire on Session Resume"
    );
    assert_eq!(
        leave_count.value, "0",
        "on_player_leave must NOT fire while inside GracePeriod"
    );

    // 4. Control resumed avatar on sock2
    sock2
        .send_to(&move_pkt.encode_to_vec(), server_addr)
        .unwrap();
    thread::sleep(Duration::from_millis(80));
    let ws2 = wait_for_world_state(&sock2, Duration::from_secs(1)).unwrap();
    assert!(
        ws2.entities[0].position.as_ref().unwrap().x_bits > ent.position.as_ref().unwrap().x_bits
    );

    running.store(false, Ordering::Relaxed);
    loop_handle.join().unwrap();
}

#[test]
fn test_session_resume_rebinds_active_session_without_timeout() {
    let mut instance = Instance::new(1, 30, 10, 42).with_grace_period_secs(30);
    let addr1: SocketAddr = "127.0.0.1:41001".parse().unwrap();
    let addr2: SocketAddr = "127.0.0.1:41002".parse().unwrap();

    let res1 = instance.handle_join_with_token(addr1, "WiFiPlayer".to_string(), None);
    let (id1, tok1) = match res1 {
        JoinResult::Joined {
            entity_id,
            session_token,
        } => (entity_id, session_token),
        _ => panic!("Expected Joined"),
    };
    assert_eq!(id1, 1);
    assert_eq!(instance.sessions.len(), 1);

    // Simulate network switch (Wi-Fi -> 4G) sending token immediately from addr2
    let res2 =
        instance.handle_join_with_token(addr2, "WiFiPlayer".to_string(), Some(tok1.as_str()));
    assert_eq!(
        res2,
        JoinResult::Resumed {
            entity_id: 1,
            session_token: tok1.clone(),
        }
    );
    assert_eq!(instance.entities.len(), 1);
    assert_eq!(instance.sessions.len(), 1);
    assert!(!instance.sessions.contains_key(&addr1));
    assert!(instance.sessions.contains_key(&addr2));
    assert_eq!(instance.entity_to_addr.get(&1), Some(&addr2));
}

#[test]
fn test_grace_period_expiry_destroys_entity_and_invalidates_token() {
    let script = r#"
        LEAVE_COUNT = 0
        function on_player_leave(entity_id)
            LEAVE_COUNT = LEAVE_COUNT + 1
        end
    "#;
    let mut instance = Instance::new(1, 30, 0, 42).with_grace_period_secs(1);
    instance.load_script(script).unwrap();
    let addr1: SocketAddr = "127.0.0.1:42001".parse().unwrap();
    let addr2: SocketAddr = "127.0.0.1:42002".parse().unwrap();

    let res = instance.handle_join_with_token(addr1, "Drifter".to_string(), None);
    let token = res.session_token().to_string();
    instance.entities.get_mut(&1).unwrap().velocity =
        loci2d::world::fixed_point::DeterministicVector2::new(I16F16::from_num(3), I16F16::ZERO);

    // Tick 1: 0s inactivity timeout -> enters 1s GracePeriod (entity preserved, velocity cleared)
    instance.tick(1).unwrap();
    assert_eq!(instance.sessions.len(), 0);
    assert_eq!(instance.disconnected_sessions.len(), 1);
    assert_eq!(instance.entities.len(), 1);
    assert_eq!(
        instance.get_entity(1).unwrap().velocity,
        loci2d::world::fixed_point::DeterministicVector2::ZERO
    );

    // Wait past 1s grace period and tick again -> expires & cleans up
    thread::sleep(Duration::from_millis(1100));
    instance.tick(2).unwrap();
    assert_eq!(instance.disconnected_sessions.len(), 0);
    assert_eq!(instance.entities.len(), 0);
    assert!(!instance.tokens.contains_key(&token));
    let leaves: i64 = instance
        .script_engine
        .lua()
        .globals()
        .get("LEAVE_COUNT")
        .unwrap();
    assert_eq!(leaves, 1);

    // Attempting to resume with expired token must allocate a fresh entity
    let fresh = instance.handle_join_with_token(addr2, "Drifter".to_string(), Some(&token));
    assert!(matches!(fresh, JoinResult::Joined { entity_id: 2, .. }));
}

#[test]
fn test_explicit_disconnect_invalidates_session_token() {
    let mut instance = Instance::new(1, 30, 10, 42).with_grace_period_secs(30);
    let addr1: SocketAddr = "127.0.0.1:43001".parse().unwrap();
    let addr2: SocketAddr = "127.0.0.1:43002".parse().unwrap();

    let join_pkt = ClientIntent {
        intent: Some(client_intent::Intent::Join(JoinIntent {
            player_name: "Quitter".to_string(),
            schema_version: 1,
            session_token: String::new(),
        })),
    };
    instance.apply_intent(addr1, join_pkt);
    let token = instance
        .sessions
        .get(&addr1)
        .unwrap()
        .session_token
        .clone();
    assert!(instance.tokens.contains_key(&token));

    // Explicit disconnect invalidates resume token immediately
    let dc_pkt = ClientIntent {
        intent: Some(client_intent::Intent::Disconnect(DisconnectIntent {
            reason: "bye".to_string(),
        })),
    };
    instance.apply_intent(addr1, dc_pkt);
    assert!(!instance.tokens.contains_key(&token));
    assert_eq!(instance.disconnected_sessions.len(), 0);

    // Rejoining with old token starts a fresh session
    let fresh = instance.handle_join_with_token(addr2, "Quitter".to_string(), Some(&token));
    assert!(matches!(fresh, JoinResult::Joined { entity_id: 2, .. }));
}
