use chrono::Local;
use loci2d::network::{
    ActionIntent, ClientIntent, DisconnectIntent, GamePacket, JoinIntent, MoveIntent, PingIntent,
    ServerPacket, Vector2, WorldState, client_intent, server_packet,
};
use prost::Message;
use std::io::{self, Write};
use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

fn send_packet(
    socket: &UdpSocket,
    server_addr: &str,
    sequence_id: &mut u64,
    intent_inner: client_intent::Intent,
) {
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
        println!(
            "[{}] Failed to serialize packet: {}",
            Local::now().format("%Y-%m-%d %H:%M:%S"),
            e
        );
        return;
    }

    match socket.send_to(&serialized, server_addr) {
        Ok(num_bytes) => {
            println!(
                "[{}] Sent {} bytes to server: sequence_id={}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                num_bytes,
                packet.sequence_id,
            );
        }
        Err(e) => {
            println!(
                "[{}] Failed to send message: {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                e
            );
        }
    }
}

fn print_world_state(ws: &WorldState) {
    println!(
        "\n--- [World State Snapshot | Tick {} | Timestamp: {}] ---",
        ws.tick, ws.timestamp
    );
    if ws.entities.is_empty() {
        println!("  (No active entities in instance)");
    } else {
        println!("  Active Entities ({}):", ws.entities.len());
        for entity in &ws.entities {
            let type_str = match entity.entity_type {
                0 => "Player",
                1 => "NPC",
                2 => "Prop",
                _ => "Unknown",
            };
            let pos = entity
                .position
                .as_ref()
                .map(|p| (p.x_bits as f32 / 65536.0, p.y_bits as f32 / 65536.0))
                .unwrap_or((0.0, 0.0));
            let vel = entity
                .velocity
                .as_ref()
                .map(|v| (v.x_bits as f32 / 65536.0, v.y_bits as f32 / 65536.0))
                .unwrap_or((0.0, 0.0));
            println!(
                "    - Entity {} (\"{}\", {}) @ ({:.1}, {:.1}), vel=({:.1}, {:.1})",
                entity.id, entity.name, type_str, pos.0, pos.1, vel.0, vel.1
            );
        }
    }
    println!("---------------------------------------------------------");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let is_spectator_init = args
        .iter()
        .any(|a| a == "--spectate" || a == "--replay" || a == "-s");

    // Bind to any available port for the client
    let socket = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind client socket");
    println!(
        "[{}] UDP Client started on {}",
        Local::now().format("%Y-%m-%d %H:%M:%S"),
        socket.local_addr().unwrap()
    );

    let server_addr = "127.0.0.1:8080";
    println!(
        "[{}] Connecting to server at {}",
        Local::now().format("%Y-%m-%d %H:%M:%S"),
        server_addr
    );

    let latest_state: Arc<Mutex<Option<WorldState>>> = Arc::new(Mutex::new(None));
    let stream_enabled = Arc::new(AtomicBool::new(is_spectator_init));
    let is_spectator = Arc::new(AtomicBool::new(is_spectator_init));

    if is_spectator_init {
        println!("[Spectator] Mode: SPECTATOR (Replay Viewer) -> Streaming snapshots enabled.");
    }

    println!(
        "Commands: join <name> | spectate | status | move <x> <y> | stream <on|off> | leave [reason] | action <id> | ping | quit"
    );

    // Spawn background listener thread to receive snapshots and keep latest state
    let recv_socket = socket
        .try_clone()
        .expect("Failed to clone socket for receiver");
    let state_clone = Arc::clone(&latest_state);
    let stream_clone = Arc::clone(&stream_enabled);

    thread::spawn(move || {
        let mut buf = [0u8; 2048];
        let mut last_stream_print = Instant::now();

        while let Ok((num_bytes, _src_addr)) = recv_socket.recv_from(&mut buf) {
            if let Ok(server_packet) = ServerPacket::decode(&buf[..num_bytes]) {
                match server_packet.payload {
                    Some(server_packet::Payload::WorldState(ws)) => {
                        // If streaming is enabled, throttle logging to ~1 second
                        if stream_clone.load(Ordering::Relaxed)
                            && last_stream_print.elapsed() >= Duration::from_millis(1000)
                        {
                            print_world_state(&ws);
                            print!(
                                "[{}] Enter command: ",
                                Local::now().format("%Y-%m-%d %H:%M:%S")
                            );
                            let _ = io::stdout().flush();
                            last_stream_print = Instant::now();
                        }
                        *state_clone.lock().unwrap() = Some(ws);
                    }
                    Some(server_packet::Payload::Response(resp)) => {
                        println!(
                            "\n[{}] [Server Response] sequence_id={}, status={}",
                            Local::now().format("%Y-%m-%d %H:%M:%S"),
                            resp.sequence_id,
                            resp.status
                        );
                        print!(
                            "[{}] Enter command: ",
                            Local::now().format("%Y-%m-%d %H:%M:%S")
                        );
                        let _ = io::stdout().flush();
                    }
                    None => {}
                }
            }
        }
    });

    // Spawn heartbeat thread for spectator mode
    let heartbeat_socket = socket
        .try_clone()
        .expect("Failed to clone socket for heartbeat");
    let is_spec_heartbeat = Arc::clone(&is_spectator);
    thread::spawn(move || {
        let mut seq: u64 = 50000;
        loop {
            if is_spec_heartbeat.load(Ordering::Relaxed) {
                let packet = GamePacket {
                    sequence_id: seq,
                    timestamp: 0,
                    intent: Some(ClientIntent {
                        intent: Some(client_intent::Intent::Ping(PingIntent {})),
                    }),
                };
                seq += 1;
                let mut buf = Vec::new();
                if packet.encode(&mut buf).is_ok() {
                    let _ = heartbeat_socket.send_to(&buf, server_addr);
                }
            }
            thread::sleep(Duration::from_millis(1000));
        }
    });

    let mut sequence_id: u64 = 0;

    loop {
        // Read user input
        print!(
            "[{}] Enter command: ",
            Local::now().format("%Y-%m-%d %H:%M:%S")
        );
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
            if !is_spectator.load(Ordering::Relaxed) {
                println!(
                    "[{}] Sending disconnect and shutting down...",
                    Local::now().format("%Y-%m-%d %H:%M:%S")
                );
                send_packet(
                    &socket,
                    server_addr,
                    &mut sequence_id,
                    client_intent::Intent::Disconnect(DisconnectIntent {
                        reason: "normal quit".to_string(),
                    }),
                );
            }
            break;
        }

        if input == "spectate" {
            is_spectator.store(true, Ordering::Relaxed);
            stream_enabled.store(true, Ordering::Relaxed);
            println!(
                "[Spectator] Mode: SPECTATOR (Replay Viewer) -> Periodic heartbeat pings & live snapshot streaming ENABLED."
            );
            continue;
        }

        if input == "status" || input == "state" || input == "entities" {
            let guard = latest_state.lock().unwrap();
            match &*guard {
                Some(ws) => print_world_state(ws),
                None => println!("[Status] No world state snapshot received yet from server."),
            }
            continue;
        }

        if input == "stream on" {
            stream_enabled.store(true, Ordering::Relaxed);
            println!("[Stream] Live snapshot logging ENABLED (throttled to 1s).");
            continue;
        }

        if input == "stream off" {
            stream_enabled.store(false, Ordering::Relaxed);
            println!(
                "[Stream] Live snapshot logging DISABLED. Use 'status' to inspect world state."
            );
            continue;
        }

        let intent_inner = match input {
            cmd if cmd.starts_with("join") => {
                is_spectator.store(false, Ordering::Relaxed);
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
                    "leaving".to_string()
                };
                client_intent::Intent::Disconnect(DisconnectIntent { reason })
            }
            "ping" => client_intent::Intent::Ping(PingIntent {}),
            cmd if cmd.starts_with("move") => {
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if parts.len() >= 3 {
                    let x: f32 = parts[1].parse().unwrap_or(0.0);
                    let y: f32 = parts[2].parse().unwrap_or(0.0);
                    client_intent::Intent::Move(MoveIntent {
                        direction: Some(Vector2 {
                            x_bits: (x * 65536.0) as i32,
                            y_bits: (y * 65536.0) as i32,
                        }),
                    })
                } else {
                    println!("[Usage] move <x> <y> (e.g. move 1.0 0.0)");
                    continue;
                }
            }
            cmd if cmd.starts_with("action") => {
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                let ability_id: u32 = if parts.len() >= 2 {
                    parts[1].parse().unwrap_or(1)
                } else {
                    1
                };
                client_intent::Intent::Action(ActionIntent { ability_id })
            }
            _ => {
                println!(
                    "Unknown command: '{}'. Available: join <name>, spectate, status, move <x> <y>, stream <on|off>, leave [reason], action <id>, ping, quit",
                    input
                );
                continue;
            }
        };

        send_packet(&socket, server_addr, &mut sequence_id, intent_inner);
    }
}
