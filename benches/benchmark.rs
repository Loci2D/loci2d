use criterion::{criterion_group, criterion_main, Criterion};
use loci2d::world::instance::Instance;

fn lua_stress_benchmark(c: &mut Criterion) {
    c.bench_function("lua_stress_actions", |b| {
        let script = r#"
            function on_tick(tick)
                for i = 1, 10 do
                    local id = Loci.Commands.spawn_entity({
                        blueprint = "box",
                        position = Loci.Vector2(i * 1.0, 0.0)
                    })
                    Loci.Commands.set_move_speed(id, 5.0)
                    Loci.Commands.set_property(id, "health", "100")
                end
            end
        "#;
        b.iter_batched(
            || {
                let mut inst = Instance::new(1, 30, 10, 42);
                inst.load_script(script).unwrap();
                inst
            },
            |mut inst| {
                let _ = inst.tick(1);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, lua_stress_benchmark);
criterion_main!(benches);
