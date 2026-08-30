use loci2d::game_loop::tick::GameLoop;
use loci2d::network::{
    ClientIntent, DisconnectIntent, GamePacket, JoinIntent, MoveIntent, Vector2, client_intent,
    run_server,
};
use loci2d::world::instance::Instance;
use prost::Message;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[test]
fn test_end_to_end_session_lifecycle() {
    let test_bind = "127.0.0.1:18080";
    let socket = UdpSocket::bind(test_bind).expect("Failed to bind UDP socket");
    let socket = Arc::new(socket);

    let (intent_tx, intent_rx) = mpsc::channel();

    // Spawn server thread
    let net_socket = Arc::clone(&socket);
    thread::spawn(move || {
        run_server(net_socket, intent_tx);
    });

    // Spawn game loop thread
    let loop_socket = Arc::clone(&socket);
    thread::spawn(move || {
        let instance = Instance::new(1, 60, 2, 42);
        let mut game_loop = GameLoop::new(60);
        game_loop.start(instance, intent_rx, loop_socket);
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
    client_sock
        .send_to(&buf, test_bind)
        .expect("Failed to send join");

    // 2. Send MoveIntent
    thread::sleep(Duration::from_millis(50));
    let move_packet = GamePacket {
        sequence_id: 2,
        timestamp: 0,
        intent: Some(ClientIntent {
            intent: Some(client_intent::Intent::Move(MoveIntent {
                direction: Some(Vector2 {
                    x_bits: (3.0f32 * 65536.0) as i32,
                    y_bits: (4.0f32 * 65536.0) as i32,
                }),
            })),
        }),
    };
    buf.clear();
    move_packet.encode(&mut buf).unwrap();
    client_sock
        .send_to(&buf, test_bind)
        .expect("Failed to send move");

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
    client_sock
        .send_to(&buf, test_bind)
        .expect("Failed to send disconnect");

    thread::sleep(Duration::from_millis(50));
}
