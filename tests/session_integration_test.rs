use std::net::UdpSocket;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use prost::Message;
use loci2d::network::{
    run_server, GamePacket, ClientIntent, Vector2, MoveIntent, JoinIntent, DisconnectIntent, client_intent,
};
use loci2d::world::instance::Instance;
use loci2d::game_loop::tick::GameLoop;

#[test]
fn test_end_to_end_session_lifecycle() {
    let test_bind = "127.0.0.1:18080";
    let (intent_tx, intent_rx) = mpsc::channel();

    // Spawn server thread
    let server_bind = test_bind.to_string();
    thread::spawn(move || {
        run_server(intent_tx, &server_bind);
    });

    // Spawn game loop thread
    thread::spawn(move || {
        let instance = Instance::new(1, 60, 2);
        let mut game_loop = GameLoop::new(60);
        game_loop.start(instance, intent_rx);
    });

    // Give the server a moment to bind
    thread::sleep(Duration::from_millis(100));

    let client_sock = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind client socket");

    // 1. Send JoinIntent
    let join_packet = GamePacket {
        sequence_id: 1,
        timestamp: 0,
        intent: Some(ClientIntent {
            intent: Some(client_intent::Intent::Join(JoinIntent {
                player_name: "Arthur".to_string(),
            })),
        }),
    };
    let mut buf = Vec::new();
    join_packet.encode(&mut buf).unwrap();
    client_sock.send_to(&buf, test_bind).expect("Failed to send join");

    // 2. Send MoveIntent
    thread::sleep(Duration::from_millis(50));
    let move_packet = GamePacket {
        sequence_id: 2,
        timestamp: 0,
        intent: Some(ClientIntent {
            intent: Some(client_intent::Intent::Move(MoveIntent {
                direction: Some(Vector2 { x: 3.0, y: 4.0 }),
            })),
        }),
    };
    buf.clear();
    move_packet.encode(&mut buf).unwrap();
    client_sock.send_to(&buf, test_bind).expect("Failed to send move");

    // 3. Send DisconnectIntent
    thread::sleep(Duration::from_millis(50));
    let dc_packet = GamePacket {
        sequence_id: 3,
        timestamp: 0,
        intent: Some(ClientIntent {
            intent: Some(client_intent::Intent::Disconnect(DisconnectIntent {
                reason: "Match finished".to_string(),
            })),
        }),
    };
    buf.clear();
    dc_packet.encode(&mut buf).unwrap();
    client_sock.send_to(&buf, test_bind).expect("Failed to send disconnect");

    thread::sleep(Duration::from_millis(50));
}
