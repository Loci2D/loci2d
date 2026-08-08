use std::sync::mpsc;
use std::thread;
use loci2d::config::ServerConfig;
use loci2d::game_loop::tick::GameLoop;
use loci2d::network::server::run_server;
use loci2d::world::instance::Instance;

fn main() {
    let cfg = ServerConfig::from_env();
    println!("[Config] bind_addr={} tick_rate={} Hz", cfg.bind_addr, cfg.tick_rate);

    let (intent_tx, intent_rx) = mpsc::channel();
    let bind_addr = cfg.bind_addr.clone();
    let tick_rate = cfg.tick_rate;

    // Network thread: produces intents
    let net_thread = thread::spawn(move || {
        run_server(intent_tx, &bind_addr);
    });

    // Game loop thread: consumes intents and advances simulation
    let loop_thread = thread::spawn(move || {
        let instance = Instance::new(1, tick_rate);
        let mut game_loop = GameLoop::new(tick_rate);
        game_loop.start(instance, intent_rx);
    });

    // Wait for both (they run indefinitely in production)
    let _ = net_thread.join();
    let _ = loop_thread.join();
}

