use std::net::UdpSocket;
use chrono::Local;
use bincode;
use teste::types::{GamePacket, ServerResponse};

fn main() {
    // Bind to UDP socket on localhost:8080
    let socket = UdpSocket::bind("127.0.0.1:8080").expect("Failed to bind to address");
    println!("[{}] UDP Server listening on 127.0.0.1:8080", Local::now().format("%Y-%m-%d %H:%M:%S"));

    let mut buf = [0u8; 1024];

    loop {
        // Receive data from any client
        match socket.recv_from(&mut buf) {
            Ok((num_bytes, src_addr)) => {
                let received_data = &buf[..num_bytes];
                
                // Deserialize binary GamePacket
                match bincode::deserialize::<GamePacket>(received_data) {
                    Ok(packet) => {
                        println!("[{}] Received {} bytes from {}: sequence_id={}, intent={:?}", 
                            Local::now().format("%Y-%m-%d %H:%M:%S"),
                            num_bytes,
                            src_addr,
                            packet.sequence_id,
                            packet.intent
                        );

                        // Create and send binary response
                        let response = ServerResponse {
                            sequence_id: packet.sequence_id,
                            status: format!("ACK: {:?}", packet.intent),
                        };
                        
                        let serialized_response = bincode::serialize(&response).expect("Failed to serialize response");
                        socket.send_to(&serialized_response, src_addr).expect("Failed to send response");
                        println!("[{}] Sent {} bytes response to {}: sequence_id={}", 
                            Local::now().format("%Y-%m-%d %H:%M:%S"),
                            serialized_response.len(),
                            src_addr,
                            response.sequence_id
                        );
                    }
                    Err(e) => {
                        println!("[{}] Failed to deserialize packet from {}: {}", 
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
