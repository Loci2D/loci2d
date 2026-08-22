use loci2d::scripting::{ScriptEngine, Command, CommandBuffer};
use loci2d::scripting::api::with_scoped_api;
use loci2d::world::instance::{Instance, DeterministicVector2};

#[test]
fn test_deterministic_vector2_userdata() {
    let engine = ScriptEngine::new(42).unwrap();
    let script = r#"
        local v1 = Loci.Vector2(1.5, 2.0)
        local v2 = Loci.Vector2(0.5, -1.0)
        local sum = v1 + v2
        local diff = v1 - v2

        assert(sum:x_float() == 2.0)
        assert(sum:y_float() == 1.0)
        assert(diff:x_float() == 1.0)
        assert(diff:y_float() == 3.0)
    "#;
    engine.load_script(script).unwrap();
}

#[test]
fn test_command_buffer_and_entity_api() {
    let engine = ScriptEngine::new(42).unwrap();
    let mut instance = Instance::new(1, 30, 10, 42);
    
    // Add an entity
    instance.handle_join("127.0.0.1:12345".parse().unwrap(), "Alice".to_string());
    
    // Position it
    if let Some(entity) = instance.entities.values_mut().next() {
        entity.position = DeterministicVector2::from_f32(10.0, 20.0);
    }
    
    let mut command_buffer = CommandBuffer::new();

    let script = r#"
        local alice_id = Loci.get_entity_by_name("Alice")
        assert(alice_id ~= nil)
        
        local pos = Loci.get_entity_position(alice_id)
        assert(pos ~= nil)
        assert(pos:x_float() == 10.0)
        assert(pos:y_float() == 20.0)

        Loci.Commands.spawn_entity({
            blueprint = "box",
            position = pos + Loci.Vector2(5.0, 0.0)
        })
    "#;

    // We must compile/load the function, then run it inside with_scoped_api
    let chunk = engine.lua().load(script);
    let func = chunk.into_function().unwrap();

    with_scoped_api(engine.lua(), &instance, &mut command_buffer, || {
        func.call::<()>(())
    }).unwrap();

    assert_eq!(command_buffer.commands.len(), 1);
    
    match &command_buffer.commands[0] {
        Command::SpawnEntity { blueprint, position } => {
            assert_eq!(blueprint, "box");
            assert_eq!(position.to_f32(), (15.0, 20.0));
        }
    }
}
