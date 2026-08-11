use std::net::UdpSocket;
use std::process;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use loci2d::config::ServerConfig;
use loci2d::game_loop::tick::GameLoop;
use loci2d::network::server::run_server;
use loci2d::replay::player::ReplayPlayer;
use loci2d::world::instance::Instance;

fn print_help() {
    println!("loci2d - High-Performance Authoritative 2D Game Server & Deterministic Replay Engine\n");
    println!("USAGE:");
    println!("  loci2d [OPTIONS]\n");
    println!("OPTIONS:");
    println!("  --record <FILE>              Enable live match recording to a .loci file");
    println!("  --replay <FILE>              Load and execute a .loci replay file");
    println!("  --verify                     Run headless deterministic replay verification");
    println!("  --checkpoint-interval <N>    Checkpoint frequency in ticks (default: 60)");
    println!("  --seed <N>                   Random PRNG seed for match instance (default: 42)");
    println!("  --map <NAME>                 Map arena identifier (default: default_arena)");
    println!("  --speed <FLOAT>              Replay playback speed multiplier (default: 1.0)");
    println!("  --broadcast <ADDR>           Spectator UDP broadcast destination (e.g. 127.0.0.1:4000)");
    println!("  --help, -h                   Show this help message");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut record_file: Option<String> = None;
    let mut replay_file: Option<String> = None;
    let mut verify_mode = false;
    let mut checkpoint_interval: u64 = 60;
    let mut seed: u64 = 42;
    let mut map_name = "default_arena".to_string();
    let mut replay_speed = 1.0f32;
    let mut broadcast_addr: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--record" => {
                if i + 1 < args.len() {
                    record_file = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --record requires a file path argument");
                    process::exit(1);
                }
            }
            "--replay" => {
                if i + 1 < args.len() {
                    replay_file = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --replay requires a file path argument");
                    process::exit(1);
                }
            }
            "--verify" => {
                verify_mode = true;
                i += 1;
            }
            "--checkpoint-interval" => {
                if i + 1 < args.len() {
                    match args[i + 1].parse::<u64>() {
                        Ok(v) => checkpoint_interval = v,
                        Err(_) => {
                            eprintln!("Error: --checkpoint-interval requires a valid integer value (e.g. 60)");
                            process::exit(1);
                        }
                    }
                    i += 2;
                } else {
                    eprintln!("Error: --checkpoint-interval requires an integer value");
                    process::exit(1);
                }
            }
            "--seed" => {
                if i + 1 < args.len() {
                    match args[i + 1].parse::<u64>() {
                        Ok(v) => seed = v,
                        Err(_) => {
                            eprintln!("Error: --seed requires a valid integer value (e.g. 42)");
                            process::exit(1);
                        }
                    }
                    i += 2;
                } else {
                    eprintln!("Error: --seed requires an integer value");
                    process::exit(1);
                }
            }
            "--map" => {
                if i + 1 < args.len() {
                    map_name = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("Error: --map requires a string value");
                    process::exit(1);
                }
            }
            "--speed" => {
                if i + 1 < args.len() {
                    match args[i + 1].parse::<f32>() {
                        Ok(v) => replay_speed = v,
                        Err(_) => {
                            eprintln!("Error: --speed requires a valid float value (e.g. 1.0, 2.0)");
                            process::exit(1);
                        }
                    }
                    i += 2;
                } else {
                    eprintln!("Error: --speed requires a float value");
                    process::exit(1);
                }
            }
            "--broadcast" => {
                if i + 1 < args.len() {
                    broadcast_addr = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --broadcast requires an IP:PORT address");
                    process::exit(1);
                }
            }
            "--help" | "-h" => {
                print_help();
                return;
            }
            _ => {
                eprintln!("Unknown argument: '{}'. Use --help for usage.", args[i]);
                process::exit(1);
            }
        }
    }

    // 1. Validation for --verify
    if verify_mode && replay_file.is_none() {
        eprintln!("Error: --verify requires a replay file specified via --replay <FILE>");
        process::exit(1);
    }

    // 2. Replay Modes (Headless Verification vs Live Spectator Broadcast)
    if let Some(ref path) = replay_file {
        if verify_mode {
            // 2.1 Headless Replay Verification Mode
            println!("[Replay] Loading replay file '{}' for headless verification...", path);
            let mut player = match ReplayPlayer::load_from_file(path) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("[Replay] Failed to load replay file: {}", e);
                    process::exit(1);
                }
            };

            let header = player.header();
            println!("[Replay] Header: magic={} version={} tick_rate={}Hz seed={} map='{}'",
                header.magic, header.version, header.tick_rate, header.random_seed, header.map_name);
            println!("[Replay] Loaded {} frames and {} checkpoints.", player.frames().len(), player.checkpoints().len());

            match player.verify_determinism() {
                Ok(report) => {
                    println!("\n✅ {}", report);
                    process::exit(0);
                }
                Err(desync) => {
                    eprintln!("\n❌ {}", desync);
                    process::exit(1);
                }
            }
        } else {
            // 2.2 Live Spectator Replay Broadcast Mode
            let cfg = ServerConfig::from_env();
            let bind_target = broadcast_addr.unwrap_or(cfg.bind_addr);

            println!("[Spectator] Starting Replay Broadcast Server for '{}' at {} ({:.1}x speed)",
                path, bind_target, replay_speed);

            let mut player = match ReplayPlayer::load_from_file(path) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("[Spectator] Failed to load replay file: {}", e);
                    process::exit(1);
                }
            };

            let socket = match UdpSocket::bind(&bind_target) {
                Ok(s) => Arc::new(s),
                Err(e) => {
                    eprintln!("[Spectator] Failed to bind UDP socket to '{}': {}", bind_target, e);
                    process::exit(1);
                }
            };

            let (intent_tx, intent_rx) = mpsc::channel();
            let net_socket = Arc::clone(&socket);
            thread::spawn(move || {
                run_server(net_socket, intent_tx);
            });

            let running = Arc::new(AtomicBool::new(true));
            player.broadcast_live(socket, intent_rx, replay_speed, running);
            println!("[Spectator] Replay broadcast completed.");
            process::exit(0);
        }
    }

    // 3. Standard Authoritative Server Mode (with optional live match recording)
    let cfg = ServerConfig::from_env();
    println!("[Config] bind_addr={} tick_rate={} Hz client_timeout={}s", 
        cfg.bind_addr, cfg.tick_rate, cfg.client_timeout_secs);

    let socket = UdpSocket::bind(&cfg.bind_addr).expect("Failed to bind UDP socket");
    let socket = Arc::new(socket);
    println!("[Server] Bound to {}", socket.local_addr().unwrap());

    let (intent_tx, intent_rx) = mpsc::channel();
    let tick_rate = cfg.tick_rate;
    let client_timeout_secs = cfg.client_timeout_secs;

    let net_socket = Arc::clone(&socket);
    let net_thread = thread::spawn(move || {
        run_server(net_socket, intent_tx);
    });

    let loop_socket = Arc::clone(&socket);
    let loop_thread = thread::spawn(move || {
        let instance = Instance::new(1, tick_rate, client_timeout_secs);
        let mut game_loop = GameLoop::new(tick_rate);

        if let Some(record_path) = record_file {
            println!("[Replay] Live match recording enabled -> '{}' (checkpoint interval: {} ticks)", 
                record_path, checkpoint_interval);
            game_loop.enable_recording(1, seed, map_name, checkpoint_interval, record_path);
        }

        game_loop.start(instance, intent_rx, loop_socket);
    });

    let _ = net_thread.join();
    let _ = loop_thread.join();
}
