use super::packets::{ClientIntent, GamePacket};
use chrono::Local;
use prost::Message;
use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::sync::mpsc;

pub fn run_server(socket: Arc<UdpSocket>, intent_tx: mpsc::Sender<(SocketAddr, ClientIntent)>) {
    let mut buf = [0u8; 2048];

    loop {
        // Receive data from any client
        match socket.recv_from(&mut buf) {
            Ok((num_bytes, src_addr)) => {
                let received_data = &buf[..num_bytes];

                // Deserialize Protobuf GamePacket
                match GamePacket::decode(received_data) {
                    Ok(packet) => {
                        if let Some(intent) = packet.intent {
                            let _ = intent_tx.send((src_addr, intent));
                        }
                    }
                    Err(e) => {
                        println!(
                            "[{}] Failed to deserialize GamePacket from {}: {}",
                            Local::now().format("%Y-%m-%d %H:%M:%S"),
                            src_addr,
                            e
                        );
                    }
                }
            }
            Err(e) => {
                println!(
                    "[{}] Error receiving data: {}",
                    Local::now().format("%Y-%m-%d %H:%M:%S"),
                    e
                );
            }
        }
    }
}
