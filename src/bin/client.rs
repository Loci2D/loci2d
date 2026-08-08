use std::net::UdpSocket;
use std::io::{self, Write};
use chrono::Local;
use prost::Message;
use loci2d::network::{
    GamePacket, ClientIntent, Vector2, MoveIntent, ActionIntent, PingIntent,
    JoinIntent, DisconnectIntent, client_intent,
};

fn send_packet(socket: &UdpSocket, server_addr: &str, sequence_id: &mut u64, intent_inner: client_intent::Intent) {
    let packet = GamePacket {
        sequence_id: *sequence_id,
        timestamp: 0,
        intent: Some(ClientIntent {
            intent: Some(intent_inner),
        }),
    };
    *sequence_id += 1;

    let mut serialized = Vec::new();
    if let Err(e) = packet.encode(&mut serialized) {
        println!("[{}] Failed to serialize packet: {}", Local::now().format("%Y-%m-%d %H:%M:%S"), e);
        return;
    }

    match socket.send_to(&serialized, server_addr) {
        Ok(num_bytes) => {
            println!("[{}] Sent {} bytes to server: sequence_id={}", 
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                num_bytes,
                packet.sequence_id,
            );
        }
        Err(e) => {
            println!("[{}] Failed to send message: {}", Local::now().format("%Y-%m-%d %H:%M:%S"), e);
        }
    }
}

fn main() {
    // Bind to any available port for the client
    let socket = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind client socket");
    println!("[{}] UDP Client started on {}", Local::now().format("%Y-%m-%d %H:%M:%S"), socket.local_addr().unwrap());

    let server_addr = "127.0.0.1:8080";
    println!("[{}] Connecting to server at {}", Local::now().format("%Y-%m-%d %H:%M:%S"), server_addr);
    println!("Commands: join <name> | leave [reason] | move <x> <y> | action <id> | ping | quit");

    let mut sequence_id: u64 = 0;

    loop {
        // Read user input
        print!("[{}] Enter command: ", Local::now().format("%Y-%m-%d %H:%M:%S"));
        io::stdout().flush().expect("Failed to flush stdout");

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        if input == "quit" {
            println!("[{}] Sending disconnect and shutting down...", Local::now().format("%Y-%m-%d %H:%M:%S"));
            send_packet(&socket, server_addr, &mut sequence_id, client_intent::Intent::Disconnect(DisconnectIntent {
                reason: "normal quit".to_string(),
            }));
            break;
        }

        let intent_inner = match input {
            cmd if cmd.starts_with("join") => {
                let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
                let player_name = if parts.len() > 1 && !parts[1].trim().is_empty() {
                    parts[1].trim().to_string()
                } else {
                    "Player".to_string()
                };
                client_intent::Intent::Join(JoinIntent { player_name })
            }
            cmd if cmd.starts_with("leave") => {
                let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
                let reason = if parts.len() > 1 {
                    parts[1].trim().to_string()
                } else {
                    "leaving session".to_string()
                };
                client_intent::Intent::Disconnect(DisconnectIntent { reason })
            }
            "ping" => client_intent::Intent::Ping(PingIntent {}),
            cmd if cmd.starts_with("move") => {
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                let (x, y) = if parts.len() >= 3 {
                    (parts[1].parse::<f32>().unwrap_or(0.0), parts[2].parse::<f32>().unwrap_or(0.0))
                } else {
                    (1.0, 0.0)
                };
                client_intent::Intent::Move(MoveIntent {
                    direction: Some(Vector2 { x, y }),
                })
            }
            cmd if cmd.starts_with("action") => {
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                let ability_id = if parts.len() >= 2 {
                    parts[1].parse::<u32>().unwrap_or(0)
                } else {
                    0
                };
                client_intent::Intent::Action(ActionIntent { ability_id })
            }
            _ => {
                println!("Unknown command. Available: join <name>, leave [reason], move <x> <y>, action <id>, ping, quit");
                continue;
            }
        };

        send_packet(&socket, server_addr, &mut sequence_id, intent_inner);
    }
}
