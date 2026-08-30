#![allow(unused_must_use)]
use loci2d::scripting::api::with_scoped_api;
use loci2d::scripting::{Command, CommandBuffer, ScriptEngine};
use loci2d::world::instance::{DeterministicVector2, Instance};

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
    })
    .unwrap();

    assert_eq!(command_buffer.commands.len(), 1);

    match &command_buffer.commands[0] {
        Command::SpawnEntity {
            blueprint,
            position,
        } => {
            assert_eq!(blueprint, "box");
            assert_eq!(position.to_f32(), (15.0, 20.0));
        }
        _ => panic!("Expected SpawnEntity command"),
    }
}

use loci2d::network::{ClientIntent, client_intent, ActionIntent, Vector2};
use fixed::types::I16F16;

#[test]
fn test_on_action_direction_passthrough() {
    let mut instance = Instance::new(1, 30, 10, 42);
    let addr = "127.0.0.1:12345".parse().unwrap();
    let alice_id = instance.handle_join(addr, "Alice".to_string());

    let script = r#"
        ACTION_DIR_X = 0
        ACTION_DIR_Y = 0
        function on_action(entity_id, ability_id, dir_x, dir_y)
            ACTION_DIR_X = dir_x
            ACTION_DIR_Y = dir_y
        end
    "#;
    instance.script_engine.load_script(script).unwrap();

    let action_intent = ClientIntent {
        intent: Some(client_intent::Intent::Action(ActionIntent {
            ability_id: 1,
            target_direction: Some(Vector2 {
                x_bits: (1.5f32 * 65536.0) as i32,
                y_bits: (-0.5f32 * 65536.0) as i32,
            }),
        })),
    };
    instance.apply_intent(addr, action_intent).unwrap();

    let globals = instance.script_engine.lua().globals();
    let dir_x: f64 = globals.get("ACTION_DIR_X").unwrap();
    let dir_y: f64 = globals.get("ACTION_DIR_Y").unwrap();
    
    assert_eq!(dir_x, 1.5);
    assert_eq!(dir_y, -0.5);
}

#[test]
fn test_set_move_speed_command() {
    let mut instance = Instance::new(1, 30, 10, 42);
    
    let script = r#"
        function on_player_join(entity_id)
            Loci.Commands.set_move_speed(entity_id, 3.0)
        end
    "#;
    instance.script_engine.load_script(script).unwrap();

    let addr = "127.0.0.1:12345".parse().unwrap();
    
    let join_intent = ClientIntent {
        intent: Some(client_intent::Intent::Join(loci2d::network::JoinIntent {
            player_name: "Bob".to_string(),
        }))
    };
    instance.apply_intent(addr, join_intent).unwrap();
    
    let entity_id = instance.sessions.get(&addr).unwrap().entity_id;
    let entity = instance.get_entity(entity_id).unwrap();
    
    assert_eq!(entity.navigation.as_ref().unwrap().move_speed, I16F16::from_num(3.0));
}

#[test]
fn test_spawn_entity_round_trip() {
    let mut instance = Instance::new(1, 30, 10, 42);
    let initial_count = instance.entities.len();
    
    let mut command_buffer = CommandBuffer::new();
    command_buffer.push(Command::SpawnEntity {
        blueprint: "magic_missile".to_string(),
        position: DeterministicVector2::from_f64(10.0, 10.0),
    });
    
    command_buffer.flush_and_apply(&mut instance);
    
    assert_eq!(instance.entities.len(), initial_count + 1);
    
    let spawned_entity = instance.entities.values().find(|e| e.name == "magic_missile").unwrap();
    assert_eq!(spawned_entity.position.to_f32(), (10.0, 10.0));
    assert_eq!(spawned_entity.entity_type, loci2d::world::entity::EntityType::Prop);
}
