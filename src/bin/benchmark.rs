use fixed::types::I16F16;
use loci2d::network::{
    ClientIntent, JoinIntent, MoveIntent, MoveToPositionIntent, client_intent,
};
use loci2d::replay::player::ReplayPlayer;
use loci2d::replay::recorder::ReplayRecorder;
use loci2d::world::fixed_point::DeterministicVector2;
use loci2d::world::instance::Instance;
use loci2d::world::physics::{
    ColliderShape, DeterministicAABB, DeterministicCircle, MapBounds, StaticObstacle,
};
use std::net::SocketAddr;
use std::time::Instant;

fn main() {
    println!("============================================================");
    println!("      loci2d Phase 4 — Determinism & Performance Benchmark  ");
    println!("============================================================\n");

    // -------------------------------------------------------------
    // Part 1: Performance Benchmark (100 active entities)
    // -------------------------------------------------------------
    println!("1. Running Performance Benchmark (100 active entities, 10,000 ticks)...");
    let mut bench_instance = Instance::new(1, 30, 60, 42);
    bench_instance.set_map_bounds(MapBounds::default_arena());

    bench_instance.add_static_obstacle(StaticObstacle::solid_wall(
        1000,
        ColliderShape::AABB(DeterministicAABB::from_center_half_extents(
            DeterministicVector2::ZERO,
            DeterministicVector2::new(I16F16::from_num(50), I16F16::from_num(50)),
        )),
    ));

    for i in 1..=100u64 {
        let addr: SocketAddr = format!("127.0.0.1:{}", 10000 + i).parse().unwrap();
        let name = format!("Entity_{:03}", i);
        bench_instance.handle_join(addr, name);
        if let Some(entity) = bench_instance.entities.get_mut(&i) {
            let angle = (i as f32) * 0.0628;
            entity.velocity = DeterministicVector2::from_f32(angle.cos() * 5.0, angle.sin() * 5.0);
            entity.collider = Some(ColliderShape::Circle(DeterministicCircle::new(
                DeterministicVector2::ZERO,
                I16F16::from_num(2),
            )));
        }
    }

    let iterations = 10_000u64;
    let start_time = Instant::now();
    for tick in 1..=iterations {
        bench_instance.tick(tick).unwrap();
    }
    let elapsed = start_time.elapsed();
    let total_micros = elapsed.as_micros() as f64;
    let avg_micros_per_tick = total_micros / (iterations as f64);
    let tick_budget_30hz_us = 33_333.33f64; // 33.33 ms
    let budget_pct = (avg_micros_per_tick / tick_budget_30hz_us) * 100.0;

    println!(
        "   - Total Time (10,000 ticks): {:.2} ms",
        elapsed.as_secs_f64() * 1000.0
    );
    println!(
        "   - Average Time per Tick: {:.2} µs (Budget Target: < 100 µs)",
        avg_micros_per_tick
    );
    println!(
        "   - Tick Budget Consumption: {:.3}% of 33.3ms (Target: < 0.3%)",
        budget_pct
    );
    if avg_micros_per_tick < 100.0 {
        println!("   ✅ PASS: Performance satisfies < 100 µs requirement!\n");
    } else {
        println!("   ⚠️  WARN: Average tick time exceeded 100 µs.\n");
    }

    // -------------------------------------------------------------
    // Part 2: Cross-Platform Deterministic Replay Generation
    // -------------------------------------------------------------
    println!(
        "2. Generating 3,000-Tick 20-Player Deterministic Match Recording ('benchmark.loci')..."
    );
    let total_ticks = 3000u64;
    let checkpoint_interval = 60u64;
    let seed = 42u64;
    let mut recorder = ReplayRecorder::new(
        1,
        30,
        seed,
        "default_arena".to_string(),
        checkpoint_interval,
        "".to_string(),
    );
    let mut sim_instance = Instance::new(1, 30, 60, 42);
    sim_instance.set_map_bounds(MapBounds::default_arena());
    sim_instance.add_static_obstacle(StaticObstacle::solid_wall(
        1000,
        ColliderShape::AABB(DeterministicAABB::from_center_half_extents(
            DeterministicVector2::ZERO,
            DeterministicVector2::new(I16F16::from_num(50), I16F16::from_num(50)),
        )),
    ));

    let player_count = 20u64;

    for tick in 1..=total_ticks {
        let mut tick_entries = Vec::new();

        // 20 players join across the first 20 ticks
        if tick <= player_count {
            let pid = tick;
            let addr: SocketAddr = format!("127.0.0.1:{}", 20000 + pid).parse().unwrap();
            let name = format!("Player_{:02}", pid);
            let join_intent = ClientIntent {
                intent: Some(client_intent::Intent::Join(JoinIntent {
                    player_name: name.clone(),
                })),
            };
            if let Ok(Some(entry)) = sim_instance.apply_intent(addr, join_intent) {
                tick_entries.push(entry);
            }
        }

        // Change player movement directions periodically
        if tick % 15 == 0 {
            for pid in 1..=player_count {
                if let Some(_entity) = sim_instance.entities.get_mut(&pid) {
                    let angle = ((tick * pid * 13) % 360) as f32;
                    let dx = (angle.to_radians()).cos() * 3.5;
                    let dy = (angle.to_radians()).sin() * 3.5;
                    let vec_f32 = DeterministicVector2::from_f32(dx, dy);

                    let inner_intent = if pid % 2 == 0 {
                        // Even players use click-to-move
                        client_intent::Intent::MoveToPos(MoveToPositionIntent {
                            target_position: Some(
                                DeterministicVector2::from_f32(dx * 50.0, dy * 50.0).to_proto(),
                            ),
                        })
                    } else {
                        // Odd players use direct velocity
                        client_intent::Intent::Move(MoveIntent {
                            direction: Some(vec_f32.to_proto()),
                        })
                    };

                    let client_intent_payload = ClientIntent {
                        intent: Some(inner_intent),
                    };

                    let addr: SocketAddr = format!("127.0.0.1:{}", 20000 + pid).parse().unwrap();
                    if let Ok(Some(entry)) = sim_instance.apply_intent(addr, client_intent_payload)
                    {
                        tick_entries.push(entry);
                    }
                }
            }
        }

        recorder.record_tick(tick, tick_entries);
        sim_instance.tick(tick).unwrap();
        recorder.maybe_record_checkpoint(tick, &sim_instance);
    }

    let file_path = "benchmark.loci";
    match recorder.save_to_file(file_path) {
        Ok(()) => {
            println!(
                "   - Saved '{}' ({} frames, {} checkpoints)",
                file_path,
                recorder.frame_count(),
                recorder.checkpoint_count()
            );
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
    let mut verify_instance = Instance::new(1, 30, 60, 42);
    verify_instance.set_map_bounds(MapBounds::default_arena());
    verify_instance.add_static_obstacle(StaticObstacle::solid_wall(
        1000,
        ColliderShape::AABB(DeterministicAABB::from_center_half_extents(
            DeterministicVector2::ZERO,
            DeterministicVector2::new(I16F16::from_num(50), I16F16::from_num(50)),
        )),
    ));

    match player.verify_determinism_with_instance(&mut verify_instance) {
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
