use loci2d::game_loop::tick::GameLoop;
use loci2d::network::{
    ClientIntent, GamePacket, JoinIntent, MoveIntent, ReplayIntentEntry, Vector2, client_intent,
    run_server,
};
use loci2d::replay::{ReplayPlayer, ReplayRecorder};
use loci2d::world::fixed_point::DeterministicVector2;
use loci2d::world::instance::{ApplyIntentResult, Instance};
use loci2d::world::physics::{ColliderShape, DeterministicAABB, StaticObstacle};
use prost::Message;
use std::net::UdpSocket;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn build_recorded_match(
    script: &str,
    wall: Option<StaticObstacle>,
) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("create tempdir");
    let replay_path = dir.path().join("match.loci");

    let mut instance = Instance::new(1, 30, 60, 42);
    if let Some(obs) = wall {
        instance.add_static_obstacle(obs);
    }
    if !script.is_empty() {
        instance.load_script(script).expect("load script");
    }

    let mut recorder = ReplayRecorder::new(
        1,
        30,
        42,
        "arena".to_string(),
        5,
        instance.script_hash.clone(),
        instance.script_payload.clone(),
    );

    let addr_a: std::net::SocketAddr = "127.0.0.1:10001".parse().unwrap();
    let join_a = ClientIntent {
        intent: Some(client_intent::Intent::Join(JoinIntent {
            player_name: "Alice, Jr".to_string(),
            schema_version: 1,
        })),
    };
    let move_a = ClientIntent {
        intent: Some(client_intent::Intent::Move(MoveIntent {
            direction: Some(Vector2 {
                x_bits: (1.5f32 * 65536.0) as i32,
                y_bits: 0,
            }),
        })),
    };

    for tick in 0..=8u64 {
        let mut entries: Vec<ReplayIntentEntry> = Vec::new();
        if tick == 0
            && let ApplyIntentResult::Ok(Some(e)) = instance.apply_intent(addr_a, join_a.clone())
        {
            entries.push(e);
        }
        if tick >= 1
            && let ApplyIntentResult::Ok(Some(e)) = instance.apply_intent(addr_a, move_a.clone())
        {
            entries.push(e);
        }
        recorder.record_tick(tick, entries);
        instance.tick(tick).expect("tick");
        recorder.maybe_record_checkpoint(tick, &instance);
    }

    recorder.save_to_file(&replay_path).expect("save replay");
    (dir, replay_path)
}

#[test]
fn test_replay_export_csv_and_escaping() {
    let script = r#"
        function on_move_intent(entity_id, dx, dy)
            Loci.Commands.set_velocity(entity_id, { x = dx, y = dy })
        end
    "#;
    let (dir, replay_path) = build_recorded_match(script, None);
    let csv_path = dir.path().join("telemetry.csv");

    let player = ReplayPlayer::load_from_file(&replay_path).expect("load replay");
    let report = player.export_csv(&csv_path).expect("export csv");
    assert_eq!(report.total_ticks, 8);
    assert_eq!(report.rows_written, 9); // 1 entity across ticks 0..=8

    let csv_text = std::fs::read_to_string(&csv_path).expect("read csv");
    let lines: Vec<&str> = csv_text.lines().collect();
    assert_eq!(
        lines[0],
        "tick,entity_id,name,entity_type,pos_x,pos_y,vel_x,vel_y"
    );
    assert_eq!(lines.len(), 10);
    // Name with comma must be CSV-quoted
    assert!(lines[1].starts_with("0,1,\"Alice, Jr\",Player,0.0000,0.0000,"));
    // Tick 8 position = 8 ticks * 1.5 = 12.0000
    assert!(lines[9].starts_with("8,1,\"Alice, Jr\",Player,12.0000,0.0000,"));
}

#[test]
fn test_cli_export_csv_verify_and_log_levels() {
    let script = r#"
        function on_init()
            Loci.Commands.spawn_entity({
                blueprint = "Dummy",
                position = { x = 5.0, y = 0.0 },
                radius = 2.0
            })
        end

        function on_move_intent(entity_id, dx, dy)
            Loci.Commands.set_velocity(entity_id, { x = dx, y = dy })
        end
    "#;
    let (dir, replay_path) = build_recorded_match(script, None);
    let bin = env!("CARGO_BIN_EXE_loci2d");

    // 1. Default INFO log level: verify + export-csv produces zero DEBUG/TRACE physics spam
    let csv_info = dir.path().join("out_info.csv");
    let log_info = dir.path().join("info.log");
    let status = Command::new(bin)
        .args([
            "--replay",
            replay_path.to_str().unwrap(),
            "--verify",
            "--export-csv",
            csv_info.to_str().unwrap(),
            "--log-file",
            log_info.to_str().unwrap(),
        ])
        .status()
        .expect("run loci2d binary");
    assert!(status.success(), "--verify --export-csv must succeed");
    assert!(csv_info.exists(), "CSV output file must be created");

    let info_log_text = std::fs::read_to_string(&log_info).expect("read info log");
    assert!(info_log_text.contains("[INFO ][Verify]"));
    assert!(info_log_text.contains("[INFO ][CSV]"));
    assert!(
        !info_log_text.contains("[Physics]"),
        "Default INFO logs must not include per-tick physics output"
    );

    // 2. --verbose (DEBUG): logs collision enter events without per-tick TRACE movement spam
    let csv_debug = dir.path().join("out_debug.csv");
    let log_debug = dir.path().join("debug.log");
    let status = Command::new(bin)
        .args([
            "--replay",
            replay_path.to_str().unwrap(),
            "--export-csv",
            csv_debug.to_str().unwrap(),
            "--verbose",
            "--log-file",
            log_debug.to_str().unwrap(),
        ])
        .status()
        .expect("run loci2d binary in verbose mode");
    assert!(status.success());

    let debug_log_text = std::fs::read_to_string(&log_debug).expect("read debug log");
    let enter_count = debug_log_text.matches("entered dynamic collision").count();
    assert_eq!(
        enter_count, 1,
        "DEBUG mode must log dynamic collision enter transition cleanly once, got:\n{}",
        debug_log_text
    );
    assert!(
        !debug_log_text.contains("[TRACE]"),
        "--verbose (DEBUG) must not emit TRACE per-tick movement lines"
    );

    // 3. --log-level trace: includes TRACE movement and sustained collision telemetry
    let csv_trace = dir.path().join("out_trace.csv");
    let log_trace = dir.path().join("trace.log");
    let status = Command::new(bin)
        .args([
            "--replay",
            replay_path.to_str().unwrap(),
            "--export-csv",
            csv_trace.to_str().unwrap(),
            "--log-level",
            "trace",
            "--log-file",
            log_trace.to_str().unwrap(),
        ])
        .status()
        .expect("run loci2d binary in trace mode");
    assert!(status.success());

    let trace_log_text = std::fs::read_to_string(&log_trace).expect("read trace log");
    assert!(trace_log_text.contains("[TRACE][Physics]"));
    assert!(trace_log_text.contains("moved to"));
    assert!(trace_log_text.contains("collided with Entity"));
}

#[test]
fn test_cli_export_csv_requires_replay_flag() {
    let bin = env!("CARGO_BIN_EXE_loci2d");
    let output = Command::new(bin)
        .args(["--export-csv", "/tmp/unused_loci_test.csv"])
        .output()
        .expect("run loci2d binary");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--export-csv requires a replay file"));
}

#[test]
fn test_live_game_loop_record_csv() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let csv_path = dir.path().join("live.csv");

    let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").expect("bind server socket"));
    let server_addr = socket.local_addr().unwrap();
    let (intent_tx, intent_rx) = mpsc::channel();

    let net_socket = Arc::clone(&socket);
    thread::spawn(move || {
        run_server(net_socket, intent_tx);
    });

    let mut game_loop = GameLoop::new(60);
    game_loop
        .enable_csv_recording(&csv_path)
        .expect("enable csv recording");
    let running = game_loop.running_handle();

    let wall = StaticObstacle::solid_wall(
        1,
        ColliderShape::AABB(DeterministicAABB::new(
            DeterministicVector2::from_i32(8, -10),
            DeterministicVector2::from_i32(12, 10),
        )),
    );

    let loop_socket = Arc::clone(&socket);
    let loop_handle = thread::spawn(move || {
        let mut instance = Instance::new(1, 60, 5, 42);
        instance.add_static_obstacle(wall);
        game_loop.start(instance, intent_rx, loop_socket);
    });

    let client_sock = UdpSocket::bind("127.0.0.1:0").expect("bind client socket");
    let join_packet = GamePacket {
        sequence_id: 1,
        timestamp: 0,
        intent: Some(ClientIntent {
            intent: Some(client_intent::Intent::Join(JoinIntent {
                player_name: "Runner".to_string(),
                schema_version: 1,
            })),
        }),
    };
    let buf = join_packet.encode_to_vec();
    client_sock.send_to(&buf, server_addr).unwrap();
    thread::sleep(Duration::from_millis(80));

    running.store(false, Ordering::Relaxed);
    loop_handle.join().expect("join game loop");

    let csv_text = std::fs::read_to_string(&csv_path).expect("read live csv");
    let lines: Vec<&str> = csv_text.lines().collect();
    assert_eq!(
        lines[0],
        "tick,entity_id,name,entity_type,pos_x,pos_y,vel_x,vel_y"
    );
    assert!(
        lines.len() >= 2,
        "Expected live CSV telemetry to capture joined player rows, got {} lines",
        lines.len()
    );
    assert!(lines.iter().skip(1).any(|l| l.contains(",Runner,Player,")));
}
