use std::net::UdpSocket;
use std::io;
use chrono::Local;
use prost::Message;
use loci2d::network::{
    GamePacket, ClientIntent, Vector2, MoveIntent, ActionIntent, PingIntent,
    client_intent,
};

fn main() {
    // Bind to any available port for the client
    let socket = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind client socket");
    println!("[{}] UDP Client started", Local::now().format("%Y-%m-%d %H:%M:%S"));

    let server_addr = "127.0.0.1:8080";
    println!("[{}] Connecting to server at {}", Local::now().format("%Y-%m-%d %H:%M:%S"), server_addr);

    let mut sequence_id: u64 = 0;


    loop {
        // Read user input
        print!("[{}] Enter command (move/action/ping/quit): ", Local::now().format("%Y-%m-%d %H:%M:%S"));
        io::Write::flush(&mut io::stdout()).expect("Failed to flush stdout");

        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read input");
        let input = input.trim();

        if input == "quit" {
            println!("[{}] Client shutting down", Local::now().format("%Y-%m-%d %H:%M:%S"));
            break;
        }

        if input.is_empty() {
            continue;
        }

        // Create GamePacket based on input
        let intent_inner = match input {
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
            _ => client_intent::Intent::Ping(PingIntent {}),
        };

        let packet = GamePacket {
            sequence_id,
            timestamp: 0,
            intent: Some(ClientIntent {
                intent: Some(intent_inner),
            }),
        };
        sequence_id += 1;

        // Serialize and send packet to server
        let mut serialized = Vec::new();
        if let Err(e) = packet.encode(&mut serialized) {
            println!("[{}] Failed to serialize packet: {}", Local::now().format("%Y-%m-%d %H:%M:%S"), e);
            continue;
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
                continue;
            }
        }
    }
}

