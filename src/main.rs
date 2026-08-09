use std::net::UdpSocket;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use loci2d::config::ServerConfig;
use loci2d::game_loop::tick::GameLoop;
use loci2d::network::server::run_server;
use loci2d::world::instance::Instance;

fn main() {
    let cfg = ServerConfig::from_env();
    println!("[Config] bind_addr={} tick_rate={} Hz client_timeout={}s", 
        cfg.bind_addr, cfg.tick_rate, cfg.client_timeout_secs);

    let socket = UdpSocket::bind(&cfg.bind_addr).expect("Failed to bind UDP socket");
    let socket = Arc::new(socket);
    println!("[Server] Bound to {}", socket.local_addr().unwrap());

    let (intent_tx, intent_rx) = mpsc::channel();
    let tick_rate = cfg.tick_rate;
    let client_timeout_secs = cfg.client_timeout_secs;

    // Network thread: produces intents
    let net_socket = Arc::clone(&socket);
    let net_thread = thread::spawn(move || {
        run_server(net_socket, intent_tx);
    });

    // Game loop thread: consumes intents, advances simulation, and broadcasts snapshots
    let loop_socket = Arc::clone(&socket);
    let loop_thread = thread::spawn(move || {
        let instance = Instance::new(1, tick_rate, client_timeout_secs);
        let mut game_loop = GameLoop::new(tick_rate);
        game_loop.start(instance, intent_rx, loop_socket);
    });

    // Wait for both (they run indefinitely in production)
    let _ = net_thread.join();
    let _ = loop_thread.join();
}

