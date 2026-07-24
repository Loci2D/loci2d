use std::net::UdpSocket;
use std::io;
use std::str;
use chrono::Local;

fn main() {
    // Bind to any available port for the client
    let socket = UdpSocket::bind("127.0.0.1:0").expect("Failed to bind client socket");
    println!("[{}] UDP Client started", Local::now().format("%Y-%m-%d %H:%M:%S"));

    let server_addr = "127.0.0.1:8080";
    println!("[{}] Connecting to server at {}", Local::now().format("%Y-%m-%d %H:%M:%S"), server_addr);

    let mut buf = [0u8; 1024];

    loop {
        // Read user input
        print!("[{}] Enter message to send (or 'quit' to exit): ", Local::now().format("%Y-%m-%d %H:%M:%S"));
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

        // Send message to server
        match socket.send_to(input.as_bytes(), server_addr) {
            Ok(num_bytes) => {
                println!("[{}] Sent {} bytes to server: {}", 
                    Local::now().format("%Y-%m-%d %H:%M:%S"),
                    num_bytes,
                    input
                );
            }
            Err(e) => {
                println!("[{}] Failed to send message: {}", Local::now().format("%Y-%m-%d %H:%M:%S"), e);
                continue;
            }
        }

        // Wait for response from server (with timeout)
        socket.set_read_timeout(Some(std::time::Duration::from_secs(5))).expect("Failed to set timeout");

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
            }
            Err(e) => {
                println!("[{}] No response from server (timeout or error): {}", Local::now().format("%Y-%m-%d %H:%M:%S"), e);
            }
        }
    }
}
