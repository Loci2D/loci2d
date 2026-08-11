use std::net::SocketAddr;
use std::time::Instant;
use loci2d::network::{
    client_intent, ClientIntent, JoinIntent, MoveIntent, ReplayIntentEntry, Vector2,
};
use loci2d::replay::player::ReplayPlayer;
use loci2d::replay::recorder::ReplayRecorder;
use loci2d::world::fixed_point::DeterministicVector2;
use loci2d::world::instance::Instance;

fn main() {
    println!("============================================================");
    println!("      loci2d Phase 4 — Determinism & Performance Benchmark  ");
    println!("============================================================\n");

    // -------------------------------------------------------------
    // Part 1: Performance Benchmark (100 active entities)
    // -------------------------------------------------------------
    println!("1. Running Performance Benchmark (100 active entities, 10,000 ticks)...");
    let mut bench_instance = Instance::new(1, 30, 60);
    for i in 1..=100u64 {
        let addr: SocketAddr = format!("127.0.0.1:{}", 10000 + i).parse().unwrap();
        let name = format!("Entity_{:03}", i);
        bench_instance.handle_join(addr, name);
        if let Some(entity) = bench_instance.entities.get_mut(&i) {
            let angle = (i as f32) * 0.0628;
            entity.velocity = DeterministicVector2::from_f32(angle.cos() * 5.0, angle.sin() * 5.0);
        }
    }

    let iterations = 10_000u64;
    let start_time = Instant::now();
    for tick in 1..=iterations {
        bench_instance.tick(tick);
    }
    let elapsed = start_time.elapsed();
    let total_micros = elapsed.as_micros() as f64;
    let avg_micros_per_tick = total_micros / (iterations as f64);
    let tick_budget_30hz_us = 33_333.33f64; // 33.33 ms
    let budget_pct = (avg_micros_per_tick / tick_budget_30hz_us) * 100.0;

    println!("   - Total Time (10,000 ticks): {:.2} ms", elapsed.as_secs_f64() * 1000.0);
    println!("   - Average Time per Tick: {:.2} µs (Budget Target: < 100 µs)", avg_micros_per_tick);
    println!("   - Tick Budget Consumption: {:.3}% of 33.3ms (Target: < 0.3%)", budget_pct);
    if avg_micros_per_tick < 100.0 {
        println!("   ✅ PASS: Performance satisfies < 100 µs requirement!\n");
    } else {
        println!("   ⚠️  WARN: Average tick time exceeded 100 µs.\n");
    }

    // -------------------------------------------------------------
    // Part 2: Cross-Platform Deterministic Replay Generation
    // -------------------------------------------------------------
    println!("2. Generating 3,000-Tick 20-Player Deterministic Match Recording ('benchmark.loci')...");
    let total_ticks = 3000u64;
    let checkpoint_interval = 60u64;
    let seed = 42u64;
    let mut recorder = ReplayRecorder::new(1, 30, seed, "default_arena".to_string(), checkpoint_interval);
    let mut sim_instance = Instance::new(1, 30, 60);

    let player_count = 20u64;

    for tick in 1..=total_ticks {
        let mut tick_entries = Vec::new();

        // 20 players join across the first 20 ticks
        if tick <= player_count {
            let pid = tick;
            let addr: SocketAddr = format!("127.0.0.1:{}", 20000 + pid).parse().unwrap();
            let name = format!("Player_{:02}", pid);
            sim_instance.handle_join(addr, name.clone());
            tick_entries.push(ReplayIntentEntry {
                entity_id: pid,
                player_name: name.clone(),
                intent: Some(ClientIntent {
                    intent: Some(client_intent::Intent::Join(JoinIntent {
                        player_name: name,
                    })),
                }),
            });
        }

        // Change player movement directions periodically
        if tick % 15 == 0 {
            for pid in 1..=player_count {
                if let Some(entity) = sim_instance.entities.get_mut(&pid) {
                    let angle = ((tick * pid * 13) % 360) as f32;
                    let dx = (angle.to_radians()).cos() * 3.5;
                    let dy = (angle.to_radians()).sin() * 3.5;
                    entity.velocity = DeterministicVector2::from_f32(dx, dy);

                    tick_entries.push(ReplayIntentEntry {
                        entity_id: pid,
                        player_name: String::new(),
                        intent: Some(ClientIntent {
                            intent: Some(client_intent::Intent::Move(MoveIntent {
                                direction: Some(Vector2 { x: dx, y: dy }),
                            })),
                        }),
                    });
                }
            }
        }

        recorder.record_tick(tick, tick_entries);
        sim_instance.tick(tick);
        recorder.maybe_record_checkpoint(tick, &sim_instance);
    }

    let file_path = "benchmark.loci";
    match recorder.save_to_file(file_path) {
        Ok(()) => {
            println!("   - Saved '{}' ({} frames, {} checkpoints)", 
                file_path, recorder.frame_count(), recorder.checkpoint_count());
        }
        Err(e) => {
            eprintln!("   ❌ Failed to save benchmark replay file: {}", e);
            std::process::exit(1);
        }
    }

    // -------------------------------------------------------------
    // Part 3: Immediate Determinism Verification of Generated File
    // -------------------------------------------------------------
    println!("\n3. Verifying Determinism of Generated Replay File...");
    let mut player = ReplayPlayer::load_from_file(file_path).expect("Failed to reload replay");
    match player.verify_determinism() {
        Ok(report) => {
            println!("   ✅ {}", report);
        }
        Err(desync) => {
            eprintln!("   ❌ Determinism verification failed:\n{}", desync);
            std::process::exit(1);
        }
    }

    println!("\n============================================================");
    println!("  You can copy 'benchmark.loci' to any macOS or Windows machine");
    println!("  and run: cargo run --bin loci2d -- --replay benchmark.loci --verify");
    println!("============================================================\n");
}
