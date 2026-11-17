use loci2d::config::ServerConfig;
use loci2d::game_loop::tick::GameLoop;
use loci2d::logging::{self, LogLevel};
use loci2d::network::server::run_server;
use loci2d::replay::player::ReplayPlayer;
use loci2d::world::instance::Instance;
use loci2d::{log_error, log_info, log_warn};
use std::net::UdpSocket;
use std::process;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;

fn print_help() {
    println!(
        "loci2d - High-Performance Authoritative 2D Game Server & Deterministic Replay Engine\n"
    );
    println!("USAGE:");
    println!("  loci2d [OPTIONS]\n");
    println!("OPTIONS:");
    println!("  --record <FILE>              Enable live match recording to a .loci file");
    println!("  --record-csv <FILE>          Capture per-tick/per-entity CSV telemetry during live match");
    println!("  --replay <FILE>              Load and execute a .loci replay file");
    println!("  --verify                     Run headless deterministic replay verification");
    println!("  --export-csv <FILE>          Export per-tick/per-entity CSV telemetry from a --replay file");
    println!("  --checkpoint-interval <N>    Checkpoint frequency in ticks (default: 60)");
    println!("  --seed <N>                   Random PRNG seed for match instance (default: 42)");
    println!("  --map <NAME>                 Map arena identifier (default: default_arena)");
    println!("  --speed <FLOAT>              Replay playback speed multiplier (default: 1.0)");
    println!(
        "  --broadcast <ADDR>           Spectator UDP broadcast destination (e.g. 127.0.0.1:4000)"
    );
    println!("  --scripts-dir <DIR>          Base directory containing map scripts (default: scripts)");
    println!("  --script-path <FILE>         Explicit path to authoritative Lua script (overrides --scripts-dir and --map)");
    println!("  --log-level <LEVEL>          Set log verbosity: off, error, warn, info, debug, trace (or LOCI_LOG env)");
    println!("  --verbose, -v                Shortcut for --log-level debug");
    println!("  --quiet, -q                  Shortcut for --log-level warn");
    println!("  --log-file <PATH>            Write structured server logs to a file on disk");
    println!("  --help, -h                   Show this help message");
}

fn next_arg<'a>(args: &'a [String], index: usize, flag: &str) -> &'a str {
    if index + 1 < args.len() {
        &args[index + 1]
    } else {
        eprintln!("Error: {} requires an argument value", flag);
        process::exit(1);
    }
}

fn build_replay_instance(
    player: &ReplayPlayer,
    custom_scripts_dir: &Option<String>,
    custom_script_path: &Option<String>,
) -> Result<Instance, String> {
    let header = player.header();
    let mut instance = Instance::new(header.instance_id, header.tick_rate, 60, header.random_seed);
    instance.logging_enabled = true;

    let mut cfg = ServerConfig::from_env();
    if let Some(dir) = custom_scripts_dir {
        cfg.scripts_dir = dir.clone();
    }
    if let Some(path) = custom_script_path {
        cfg.script_path = Some(path.clone());
    }

    let script_path = cfg.resolve_script_path(&header.map_name);
    if custom_script_path.is_some() || std::path::Path::new(&script_path).exists() {
        log_info!(
            "Replay",
            "Loading local script bundle from '{}'",
            script_path
        );
        instance
            .load_script_from_file(&script_path)
            .map_err(|e| format!("Failed to load script bundle '{}': {}", script_path, e))?;
    } else if !header.script_payload.is_empty() {
        log_info!(
            "Replay",
            "Loading embedded script payload from replay header (hash: {})",
            header.script_hash
        );
        instance
            .load_script(&header.script_payload)
            .map_err(|e| format!("Failed to load embedded replay script: {}", e))?;
    }

    Ok(instance)
}

fn spawn_console_listener(running: Arc<AtomicBool>) {
    thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut line = String::new();
        while running.load(Ordering::Relaxed) {
            line.clear();
            if stdin.read_line(&mut line).is_ok() {
                let trimmed = line.trim();
                if trimmed.eq_ignore_ascii_case("quit")
                    || trimmed.eq_ignore_ascii_case("stop")
                    || trimmed.eq_ignore_ascii_case("exit")
                    || trimmed.eq_ignore_ascii_case("q")
                {
                    log_info!(
                        "Server",
                        "Shutdown command received ('{}'); stopping gracefully...",
                        trimmed
                    );
                    running.store(false, Ordering::Relaxed);
                    break;
                }
            } else {
                // EOF on stdin (e.g. piped input or Ctrl+D)
                running.store(false, Ordering::Relaxed);
                break;
            }
        }
    });
}

fn main() {
    logging::init_from_env();

    let args: Vec<String> = std::env::args().collect();

    let mut record_file: Option<String> = None;
    let mut record_csv_file: Option<String> = None;
    let mut replay_file: Option<String> = None;
    let mut verify_mode = false;
    let mut export_csv_file: Option<String> = None;
    let mut checkpoint_interval: u64 = 60;
    let mut seed: u64 = 42;
    let mut map_name = "default_arena".to_string();
    let mut replay_speed = 1.0f32;
    let mut broadcast_addr: Option<String> = None;
    let mut custom_scripts_dir: Option<String> = None;
    let mut custom_script_path: Option<String> = None;
    let mut log_file_path: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--record" => {
                record_file = Some(next_arg(&args, i, "--record").to_string());
                i += 2;
            }
            "--record-csv" => {
                record_csv_file = Some(next_arg(&args, i, "--record-csv").to_string());
                i += 2;
            }
            "--replay" => {
                replay_file = Some(next_arg(&args, i, "--replay").to_string());
                i += 2;
            }
            "--verify" => {
                verify_mode = true;
                i += 1;
            }
            "--export-csv" => {
                export_csv_file = Some(next_arg(&args, i, "--export-csv").to_string());
                i += 2;
            }
            "--checkpoint-interval" => {
                let raw = next_arg(&args, i, "--checkpoint-interval");
                checkpoint_interval = raw.parse::<u64>().unwrap_or_else(|_| {
                    eprintln!(
                        "Error: --checkpoint-interval requires a valid integer value (e.g. 60)"
                    );
                    process::exit(1);
                });
                i += 2;
            }
            "--seed" => {
                let raw = next_arg(&args, i, "--seed");
                seed = raw.parse::<u64>().unwrap_or_else(|_| {
                    eprintln!("Error: --seed requires a valid integer value (e.g. 42)");
                    process::exit(1);
                });
                i += 2;
            }
            "--map" => {
                map_name = next_arg(&args, i, "--map").to_string();
                i += 2;
            }
            "--speed" => {
                let raw = next_arg(&args, i, "--speed");
                replay_speed = raw.parse::<f32>().unwrap_or_else(|_| {
                    eprintln!("Error: --speed requires a valid float value (e.g. 1.0, 2.0)");
                    process::exit(1);
                });
                i += 2;
            }
            "--broadcast" => {
                broadcast_addr = Some(next_arg(&args, i, "--broadcast").to_string());
                i += 2;
            }
            "--scripts-dir" => {
                custom_scripts_dir = Some(next_arg(&args, i, "--scripts-dir").to_string());
                i += 2;
            }
            "--script-path" => {
                custom_script_path = Some(next_arg(&args, i, "--script-path").to_string());
                i += 2;
            }
            "--log-level" => {
                let raw = next_arg(&args, i, "--log-level");
                match LogLevel::parse(raw) {
                    Ok(lvl) => logging::set_level(lvl),
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        process::exit(1);
                    }
                }
                i += 2;
            }
            "--verbose" | "-v" => {
                logging::set_level(LogLevel::Debug);
                i += 1;
            }
            "--quiet" | "-q" => {
                logging::set_level(LogLevel::Warn);
                i += 1;
            }
            "--log-file" => {
                log_file_path = Some(next_arg(&args, i, "--log-file").to_string());
                i += 2;
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

    if let Some(ref path) = log_file_path
        && let Err(e) = logging::set_log_file(path)
    {
        eprintln!("Error: Failed to open log file '{}': {}", path, e);
        process::exit(1);
    }

    // 1. Validation for --verify / --export-csv
    if verify_mode && replay_file.is_none() {
        log_error!("CLI", "--verify requires a replay file specified via --replay <FILE>");
        process::exit(1);
    }
    if export_csv_file.is_some() && replay_file.is_none() {
        log_error!(
            "CLI",
            "--export-csv requires a replay file specified via --replay <FILE>"
        );
        process::exit(1);
    }

    // 2. Replay Modes (Headless Verification / CSV Export vs Live Spectator Broadcast)
    if let Some(ref path) = replay_file {
        if verify_mode || export_csv_file.is_some() {
            log_info!("Replay", "Loading replay file '{}'...", path);
            let mut player = match ReplayPlayer::load_from_file(path) {
                Ok(p) => p,
                Err(e) => {
                    log_error!("Replay", "Failed to load replay file: {}", e);
                    process::exit(1);
                }
            };

            let header = player.header();
            log_info!(
                "Replay",
                "Header: magic={} version={} tick_rate={}Hz seed={} map='{}'",
                header.magic,
                header.version,
                header.tick_rate,
                header.random_seed,
                header.map_name
            );
            log_info!(
                "Replay",
                "Loaded {} frames and {} checkpoints",
                player.frames().len(),
                player.checkpoints().len()
            );

            if verify_mode {
                let mut instance =
                    match build_replay_instance(&player, &custom_scripts_dir, &custom_script_path)
                    {
                        Ok(inst) => inst,
                        Err(e) => {
                            log_error!("Verify", "{}", e);
                            process::exit(1);
                        }
                    };
                match player.verify_determinism_with_instance(&mut instance) {
                    Ok(report) => {
                        log_info!("Verify", "✅ {}", report);
                    }
                    Err(desync) => {
                        log_error!("Verify", "❌ {}", desync);
                        logging::flush();
                        process::exit(1);
                    }
                }
            }

            if let Some(ref csv_path) = export_csv_file {
                let mut instance =
                    match build_replay_instance(&player, &custom_scripts_dir, &custom_script_path)
                    {
                        Ok(inst) => inst,
                        Err(e) => {
                            log_error!("CSV", "{}", e);
                            logging::flush();
                            process::exit(1);
                        }
                    };
                match player.export_csv_with_instance(&mut instance, csv_path) {
                    Ok(report) => {
                        log_info!("CSV", "✅ {}", report);
                    }
                    Err(e) => {
                        log_error!("CSV", "Failed to export CSV '{}': {}", csv_path, e);
                        logging::flush();
                        process::exit(1);
                    }
                }
            }

            logging::flush();
            process::exit(0);
        } else {
            // 2.2 Live Spectator Replay Broadcast Mode
            let cfg = ServerConfig::from_env();
            let bind_target = if let Some(ref addr) = broadcast_addr {
                addr.clone()
            } else {
                log_info!(
                    "Spectator",
                    "No --broadcast specified, using server default: {}",
                    cfg.bind_addr
                );
                cfg.bind_addr
            };

            log_info!(
                "Spectator",
                "Starting Replay Broadcast Server for '{}' at {} ({:.1}x speed)",
                path,
                bind_target,
                replay_speed
            );

            let mut player = match ReplayPlayer::load_from_file(path) {
                Ok(p) => p,
                Err(e) => {
                    log_error!("Spectator", "Failed to load replay file: {}", e);
                    logging::flush();
                    process::exit(1);
                }
            };

            let socket = match UdpSocket::bind(&bind_target) {
                Ok(s) => Arc::new(s),
                Err(e) => {
                    log_error!(
                        "Spectator",
                        "Failed to bind UDP socket to '{}': {}",
                        bind_target,
                        e
                    );
                    logging::flush();
                    process::exit(1);
                }
            };

            let (intent_tx, intent_rx) = mpsc::channel();
            let net_socket = Arc::clone(&socket);
            thread::spawn(move || {
                run_server(net_socket, intent_tx);
            });

            let running = Arc::new(AtomicBool::new(true));

            let r_ctrlc = Arc::clone(&running);
            let _ = ctrlc::set_handler(move || {
                log_info!(
                    "Spectator",
                    "Shutdown signal received (Ctrl+C); stopping broadcast..."
                );
                r_ctrlc.store(false, Ordering::Relaxed);
            });
            spawn_console_listener(Arc::clone(&running));

            log_info!(
                "Spectator",
                "Type 'stop' or 'quit' (or press Ctrl+C) to shut down the spectator server"
            );
            player.broadcast_live(socket, intent_rx, replay_speed, cfg.max_spectators, running);
            log_info!("Spectator", "Replay broadcast completed");
            logging::flush();
            process::exit(0);
        }
    }

    // 3. Standard Authoritative Server Mode (with optional live match recording / CSV capture)
    let mut cfg = ServerConfig::from_env();
    if let Some(dir) = custom_scripts_dir {
        cfg.scripts_dir = dir;
    }
    if let Some(path) = custom_script_path {
        cfg.script_path = Some(path);
    }
    log_info!(
        "Config",
        "bind_addr={} tick_rate={} Hz client_timeout={}s grace_period={}s scripts_dir='{}' log_level={}",
        cfg.bind_addr,
        cfg.tick_rate,
        cfg.client_timeout_secs,
        cfg.grace_period_secs,
        cfg.scripts_dir,
        logging::level().as_str().trim()
    );

    let socket = UdpSocket::bind(&cfg.bind_addr).expect("Failed to bind UDP socket");
    let socket = Arc::new(socket);
    log_info!("Server", "Bound to {}", socket.local_addr().unwrap());

    let (intent_tx, intent_rx) = mpsc::channel();
    let tick_rate = cfg.tick_rate;
    let client_timeout_secs = cfg.client_timeout_secs;
    let grace_period_secs = cfg.grace_period_secs;

    let net_socket = Arc::clone(&socket);
    let _net_thread = thread::spawn(move || {
        run_server(net_socket, intent_tx);
    });

    let (script_hash, script_payload) = {
        let mut temp_instance = Instance::new(1, tick_rate, client_timeout_secs, seed);
        let script_path = cfg.resolve_script_path(&map_name);
        if std::path::Path::new(&script_path).exists() {
            log_info!(
                "Script",
                "Loading script from '{}' (and bundling directory)",
                script_path
            );
            if let Err(e) = temp_instance.load_script_from_file(&script_path) {
                log_error!("Script", "Failed to load script bundle: {}", e);
                logging::flush();
                process::exit(1);
            }
            log_info!(
                "Script",
                "Script loaded successfully (hash: {})",
                temp_instance.script_hash
            );
            (temp_instance.script_hash.clone(), temp_instance.script_payload.clone())
        } else {
            log_warn!(
                "Script",
                "No script found at '{}', running without game logic",
                script_path
            );
            ("".to_string(), "".to_string())
        }
    };

    let mut game_loop = GameLoop::new(tick_rate);
    let running = game_loop.running_handle();

    if let Some(record_path) = record_file {
        log_info!(
            "Replay",
            "Live match recording enabled -> '{}' (checkpoint interval: {} ticks)",
            record_path,
            checkpoint_interval
        );
        game_loop.enable_recording(
            1,
            seed,
            map_name.clone(),
            checkpoint_interval,
            record_path,
            script_hash,
            script_payload.clone(),
        );
    }

    if let Some(csv_path) = record_csv_file {
        if let Err(e) = game_loop.enable_csv_recording(&csv_path) {
            log_error!("CSV", "Failed to initialize CSV capture '{}': {}", csv_path, e);
            logging::flush();
            process::exit(1);
        }
        log_info!("CSV", "Live CSV telemetry enabled -> '{}'", csv_path);
    }

    // Ctrl+C handler for graceful match saving
    let r_ctrlc = Arc::clone(&running);
    let _ = ctrlc::set_handler(move || {
        log_info!(
            "Server",
            "Shutdown signal received (Ctrl+C); saving recording and stopping..."
        );
        r_ctrlc.store(false, Ordering::Relaxed);
    });

    // Console stdin listener for 'stop' / 'quit' command
    spawn_console_listener(Arc::clone(&running));
    log_info!(
        "Server",
        "Server running; type 'stop' or 'quit' (or press Ctrl+C) to shut down cleanly"
    );

    let loop_socket = Arc::clone(&socket);
    let loop_payload = script_payload.clone();
    let loop_thread = thread::spawn(move || {
        let mut instance = Instance::new(1, tick_rate, client_timeout_secs, seed);
        instance.set_grace_period_secs(grace_period_secs);
        instance.logging_enabled = true;
        if !loop_payload.is_empty()
            && let Err(e) = instance.load_script(&loop_payload)
        {
            log_error!("Script", "Failed to load script payload in game loop: {}", e);
        }
        game_loop.start(instance, intent_rx, loop_socket);
    });

    let _ = loop_thread.join();
    log_info!("Server", "Shutdown complete");
    logging::flush();
    process::exit(0);
}
