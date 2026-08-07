use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc;
use chrono::Local;
use prost::Message;
use super::packets::{GamePacket, ClientIntent};

pub fn run_server(intent_tx: mpsc::Sender<(SocketAddr, ClientIntent)>, bind_addr: &str) {
    // Bind to UDP socket
    let socket = UdpSocket::bind(bind_addr).expect("Failed to bind to address");
    println!("[{}] UDP Server listening on {}", Local::now().format("%Y-%m-%d %H:%M:%S"), bind_addr);

    let mut buf = [0u8; 1024];

    loop {
        // Receive data from any client
        match socket.recv_from(&mut buf) {
            Ok((num_bytes, src_addr)) => {
                let received_data = &buf[..num_bytes];
                
                // Deserialize Protobuf GamePacket
                match GamePacket::decode(received_data) {
                    Ok(packet) => {
                        println!("[{}] Received {} bytes from {}: sequence_id={}, intent={:?}", 
                            Local::now().format("%Y-%m-%d %H:%M:%S"),
                            num_bytes,
                            src_addr,
                            packet.sequence_id,
                            packet.intent
                        );

                        if let Some(intent) = packet.intent {
                            let _ = intent_tx.send((src_addr, intent));
                        }
                    }
                    Err(e) => {
                        println!("[{}] Failed to deserialize Protobuf packet from {}: {}", 
                            Local::now().format("%Y-%m-%d %H:%M:%S"),
                            src_addr,
                            e
                        );
                    }
                }
            }
            Err(e) => {
                println!("[{}] Error receiving data: {}", Local::now().format("%Y-%m-%d %H:%M:%S"), e);
            }
        }
    }
}

