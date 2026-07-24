use std::net::UdpSocket;
use std::str;
use chrono::Local;

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
                let message = str::from_utf8(received_data).unwrap_or("<invalid utf-8>");
                
                println!("[{}] Received {} bytes from {}: {}", 
                    Local::now().format("%Y-%m-%d %H:%M:%S"),
                    num_bytes,
                    src_addr,
                    message
                );

                // Echo the message back to the client
                let response = format!("ACK: {}", message);
                socket.send_to(response.as_bytes(), src_addr).expect("Failed to send response");
                println!("[{}] Sent response to {}: {}", 
                    Local::now().format("%Y-%m-%d %H:%M:%S"),
                    src_addr,
                    response
                );
            }
            Err(e) => {
                println!("[{}] Error receiving data: {}", Local::now().format("%Y-%m-%d %H:%M:%S"), e);
            }
        }
    }
}
