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
            entity_id: _,
            blueprint,
            position,
            ..
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
    let _alice_id = instance.handle_join(addr, "Alice".to_string());

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
        entity_id: 2,
        blueprint: "magic_missile".to_string(),
        position: DeterministicVector2::from_f64(10.0, 10.0),
        entity_type: "Prop".to_string(),
        move_speed: I16F16::from_num(1.0),
        radius: I16F16::from_num(2.0),
        properties: std::collections::BTreeMap::new(),
    });
    
    command_buffer.flush_and_apply(&mut instance);
    
    assert_eq!(instance.entities.len(), initial_count + 1);
    
    let spawned_entity = instance.entities.values().find(|e| e.name == "magic_missile").unwrap();
    assert_eq!(spawned_entity.position.to_f32(), (10.0, 10.0));
    assert_eq!(spawned_entity.entity_type, loci2d::world::entity::EntityType::Prop);
}

#[test]
fn test_timer_lifecycle() {
    let mut instance = Instance::new(1, 30, 10, 42);
    let script = r#"
        TIMER_COMPLETED = false
        function on_init()
            Loci.Commands.start_timer("my_timer", 2)
        end
        function on_timer_complete(timer_id)
            if timer_id == "my_timer" then
                TIMER_COMPLETED = true
            end
        end
    "#;
    instance.load_script(script).unwrap();
    assert_eq!(instance.active_timers.len(), 1);
    assert_eq!(instance.active_timers.get("my_timer").unwrap().remaining_ticks, 2);
    
    instance.tick(1).unwrap();
    assert_eq!(instance.active_timers.get("my_timer").unwrap().remaining_ticks, 1);
    
    let globals = instance.script_engine.lua().globals();
    assert!(!globals.get::<bool>("TIMER_COMPLETED").unwrap());
    
    instance.tick(2).unwrap();
    assert_eq!(instance.active_timers.len(), 0);
    assert!(globals.get::<bool>("TIMER_COMPLETED").unwrap());
}

#[test]
fn test_match_state_transitions() {
    use loci2d::world::instance::MatchState;
    let mut instance = Instance::new(1, 30, 10, 42);
    let script = r#"
        TICK_COUNT = 0
        function on_tick(tick)
            TICK_COUNT = TICK_COUNT + 1
        end
    "#;
    instance.load_script(script).unwrap();
    
    instance.tick(1).unwrap();
    let globals = instance.script_engine.lua().globals();
    assert_eq!(globals.get::<i32>("TICK_COUNT").unwrap(), 1);
    
    // Pause via command buffer
    let mut cmd_buffer = CommandBuffer::new();
    cmd_buffer.push(Command::PauseMatch);
    cmd_buffer.flush_and_apply(&mut instance);
    assert_eq!(instance.state, MatchState::Paused);
    
    instance.tick(2).unwrap(); // Should skip on_tick
    assert_eq!(globals.get::<i32>("TICK_COUNT").unwrap(), 1);
    
    // End via command buffer
    cmd_buffer.push(Command::EndMatch { winner_data: "red_team".to_string() });
    cmd_buffer.flush_and_apply(&mut instance);
    assert_eq!(instance.state, MatchState::Ended { winner_data: "red_team".to_string() });
    
    instance.tick(3).unwrap(); // Should skip on_tick
    assert_eq!(globals.get::<i32>("TICK_COUNT").unwrap(), 1);
}

#[test]
fn test_properties_api() {
    let mut instance = Instance::new(1, 30, 10, 42);
    let addr = "127.0.0.1:12345".parse().unwrap();
    let entity_id = instance.handle_join(addr, "Charlie".to_string());
    
    let script = r#"
        PROP_VALUE = nil
        function set_initial_prop(id)
            Loci.Commands.set_property(id, "health", "100")
        end
        function read_prop(id)
            PROP_VALUE = Loci.get_entity_property(id, "health")
        end
    "#;
    instance.script_engine.load_script(script).unwrap();
    
    let globals = instance.script_engine.lua().globals();
    let set_fn: mlua::Function = globals.get("set_initial_prop").unwrap();
    let read_fn: mlua::Function = globals.get("read_prop").unwrap();
    
    let mut cmd_buffer = CommandBuffer::new();
    loci2d::scripting::api::with_scoped_api(instance.script_engine.lua(), &instance, &mut cmd_buffer, || {
        set_fn.call::<()>(entity_id)
    }).unwrap();
    cmd_buffer.flush_and_apply(&mut instance);
    
    // Validate Rust side
    assert_eq!(instance.get_entity(entity_id).unwrap().properties.get("health").unwrap(), "100");
    
    // Validate Lua side read
    loci2d::scripting::api::with_scoped_api(instance.script_engine.lua(), &instance, &mut cmd_buffer, || {
        read_fn.call::<()>(entity_id)
    }).unwrap();
    assert_eq!(globals.get::<String>("PROP_VALUE").unwrap(), "100");
    
    // Validate Snapshot
    let snapshot = instance.create_snapshot(1);
    let entity_state = snapshot.entities.iter().find(|e| e.id == entity_id).unwrap();
    let prop = entity_state.properties.iter().find(|p| p.key == "health").unwrap();
    assert_eq!(prop.value, "100");
}

#[test]
fn test_on_player_leave_can_access_entity() {
    let mut instance = Instance::new(1, 30, 10, 42);
    let addr = "127.0.0.1:12345".parse().unwrap();
    let entity_id = instance.handle_join(addr, "Dave".to_string());
    
    let script = r#"
        LAST_X = 0
        function on_player_leave(id)
            local pos = Loci.get_entity_position(id)
            if pos then
                LAST_X = pos:x_float()
            end
        end
    "#;
    instance.script_engine.load_script(script).unwrap();
    
    if let Some(entity) = instance.entities.get_mut(&entity_id) {
        entity.position = DeterministicVector2::from_f64(12.0, 0.0);
    }
    
    // Trigger disconnect via intent
    let disconnect_intent = loci2d::network::ClientIntent {
        intent: Some(loci2d::network::client_intent::Intent::Disconnect(loci2d::network::DisconnectIntent {
            reason: "test".to_string(),
        })),
    };
    instance.apply_intent(addr, disconnect_intent).unwrap();
    
    // The entity should be destroyed now
    assert!(instance.get_entity(entity_id).is_none());
    
    // But the script should have captured the position before destruction
    let globals = instance.script_engine.lua().globals();
    let last_x: f64 = globals.get("LAST_X").unwrap();
    assert_eq!(last_x, 12.0);
}
