use loci2d::scripting::CommandBuffer;
use loci2d::world::entity::{Entity, EntityType};
use loci2d::world::instance::Instance;
use mlua::Function;

/// Validates command-buffer deferral semantics during entity iteration.
///
/// Safety note: This test cannot panic due to iterator invalidation by design.
/// `with_scoped_api` takes `&Instance` (shared ref), so while the for-loop holds
/// `&instance.entities`, Rust statically prevents any `&mut` access that could
/// invalidate the iterator — the borrow checker enforces this at compile time, not
/// at runtime. What this test *does* prove is that:
///   1. Commands issued inside Lua callbacks are correctly deferred to the buffer.
///   2. The entity map is unchanged until `flush_and_apply` is called.
///   3. The correct entity IDs are queued for destruction.
#[test]
fn test_commands_deferred_during_entity_iteration() {
    let mut instance = Instance::new(1, 30, 10, 42);

    
    let e1 = Entity::new(1, "Player1".to_string(), EntityType::Player);
    let e2 = Entity::new(2, "Player2".to_string(), EntityType::Player);
    
    instance.add_entity(e1);
    instance.add_entity(e2);
    
    let script = r#"
        function on_collision(entity_id, other_id)
            Loci.Commands.destroy_entity(other_id)
        end
    "#;
    instance.script_engine.load_script(script).unwrap();
    
    let mut cmd_buffer = CommandBuffer::new();
    
    // Simulate iterating through entities during physics resolution
    // We want to ensure calling Lua doesn't invalidate the iterator
    // or borrow checker.
    for id in instance.entities.keys() {
        let globals = instance.script_engine.lua().globals();
        if let Ok(on_col) = globals.get::<Function>("on_collision") {
            let other_id = if *id == 1 { 2 } else { 1 };
            
            loci2d::scripting::api::with_scoped_api(
                instance.script_engine.lua(),
                &instance,
                &mut cmd_buffer,
                || on_col.call::<()>((*id, other_id)),
            )
            .unwrap();
        }
    }
    
    // Map remains unchanged during iteration — commands were deferred, not applied inline
    assert_eq!(instance.entities.len(), 2);

    // Both collisions produced exactly one DestroyEntity command each
    assert_eq!(cmd_buffer.commands.len(), 2);

    // Verify the correct entity IDs were queued (not just *any* two commands)
    use loci2d::scripting::Command;
    let destroyed_ids: Vec<u64> = cmd_buffer
        .commands
        .iter()
        .filter_map(|c| match c {
            Command::DestroyEntity { entity_id } => Some(*entity_id),
            _ => None,
        })
        .collect();
    assert!(destroyed_ids.contains(&1), "entity 1 should be queued for destruction");
    assert!(destroyed_ids.contains(&2), "entity 2 should be queued for destruction");

    cmd_buffer.flush_and_apply(&mut instance);
    
    // Both got destroyed
    assert_eq!(instance.entities.len(), 0);
}
