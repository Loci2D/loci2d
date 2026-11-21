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
    instance.apply_intent(addr, action_intent);

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
            player_name: "Bob".to_string(), schema_version: 1, session_token: String::new() }))
    };
    instance.apply_intent(addr, join_intent);
    
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
        velocity: None,
        entity_type: "Prop".to_string(),
        move_speed: I16F16::from_num(1.0),
        radius: I16F16::from_num(2.0),
        is_sensor: None,
        lifetime_ticks: None,
        collision_layer: None,
        collision_mask: None,
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
    instance.apply_intent(addr, disconnect_intent);
    
    // The entity should be destroyed now
    assert!(instance.get_entity(entity_id).is_none());
    
    // But the script should have captured the position before destruction
    let globals = instance.script_engine.lua().globals();
    let last_x: f64 = globals.get("LAST_X").unwrap();
    assert_eq!(last_x, 12.0);
}

#[test]
fn test_intent_rejection_feedback() {
    let mut instance = Instance::new(1, 30, 10, 42);
    let addr = "127.0.0.1:12345".parse().unwrap();
    let _alice_id = instance.handle_join(addr, "Alice".to_string());

    let script = r#"
        function on_move_intent(entity_id, dir_x, dir_y)
            return false, "Stunned"
        end
    "#;
    instance.script_engine.load_script(script).unwrap();

    let move_intent = ClientIntent {
        intent: Some(client_intent::Intent::Move(loci2d::network::packets::MoveIntent {
            direction: Some(Vector2 { x_bits: 65536, y_bits: 0 }),
        })),
    };

    let result = instance.apply_intent(addr, move_intent);
    
    match result {
        loci2d::world::instance::ApplyIntentResult::Rejected(reason) => {
            assert_eq!(reason, "Stunned");
        }
        _ => panic!("Expected Intent to be rejected"),
    }
}

#[test]
fn test_lifecycle_rejection_rollback() {
    let mut instance = Instance::new(1, 30, 10, 42);
    let addr = "127.0.0.1:12345".parse().unwrap();
    
    let script = r#"
        function on_player_join(entity_id)
            return false, "Server Full"
        end
    "#;
    instance.script_engine.load_script(script).unwrap();

    let join_intent = ClientIntent {
        intent: Some(client_intent::Intent::Join(loci2d::network::JoinIntent {
            player_name: "Eve".to_string(), schema_version: 1, session_token: String::new() }))
    };

    let result = instance.apply_intent(addr, join_intent);
    
    match result {
        loci2d::world::instance::ApplyIntentResult::Rejected(reason) => {
            assert_eq!(reason, "Server Full");
        }
        _ => panic!("Expected Join to be rejected"),
    }

    // Verify rollback: session and entity should not exist
    assert!(instance.sessions.get(&addr).is_none());
    assert_eq!(instance.entities.len(), 0);
}

#[test]
fn test_dash_ability_with_raycast_and_collision_filter() {
    use loci2d::world::physics::map::StaticObstacle;
    use loci2d::world::physics::primitives::{DeterministicAABB, ColliderShape};
    let mut instance = Instance::new(1, 30, 10, 42);

    // Add solid wall from x=10 to x=15, y=-10 to y=10
    let wall = StaticObstacle::solid_wall(
        100,
        ColliderShape::AABB(DeterministicAABB::new(
            DeterministicVector2::new(I16F16::from_num(10), I16F16::from_num(-10)),
            DeterministicVector2::new(I16F16::from_num(15), I16F16::from_num(10)),
        )),
    );
    instance.add_static_obstacle(wall);

    let script = r#"
        function on_dash(entity_id, dir_x, dir_y, dash_dist)
            local pos = Loci.get_entity_position(entity_id)
            if not pos then return end
            
            -- Temporarily become intangible to enemies (e.g. layer 1, mask 0 to ignore others)
            Loci.Commands.set_collision_filter(entity_id, 1, 0)
            
            local origin = pos
            local dir = Loci.Vector2(dir_x, dir_y)
            
            local hit = Loci.Physics.raycast(origin, dir, dash_dist)
            local final_dist = dash_dist
            
            if hit then
                -- Wall in the way, dash only up to the wall, minus a small margin
                -- The fraction is between 0 and 1
                final_dist = dash_dist * hit.fraction - 0.1
                if final_dist < 0 then final_dist = 0 end
            end
            
            local final_pos = origin + Loci.Vector2(dir_x * final_dist, dir_y * final_dist)
            Loci.Commands.set_position(entity_id, final_pos)
        end
    "#;
    instance.script_engine.load_script(script).unwrap();

    let addr = "127.0.0.1:12345".parse().unwrap();
    let entity_id = instance.handle_join(addr, "Dasher".to_string());
    
    if let Some(entity) = instance.entities.get_mut(&entity_id) {
        entity.position = DeterministicVector2::ZERO;
        entity.collision_filter = loci2d::world::physics::map::CollisionFilter::new(1, 1);
    }

    let globals = instance.script_engine.lua().globals();
    let on_dash_fn: mlua::Function = globals.get("on_dash").unwrap();

    // Dash towards the wall: distance 20, dir (1, 0)
    // Wall is at x=10, distance to wall is 10.
    let mut cmd_buffer = CommandBuffer::new();
    with_scoped_api(instance.script_engine.lua(), &instance, &mut cmd_buffer, || {
        on_dash_fn.call::<()>((entity_id, 1.0, 0.0, 20.0))
    }).unwrap();

    cmd_buffer.flush_and_apply(&mut instance);

    let entity = instance.get_entity(entity_id).unwrap();
    
    // Check position: It should be near the wall (e.g. 10 - 0.1 = 9.9)
    assert!(entity.position.x > I16F16::from_num(9.0) && entity.position.x < I16F16::from_num(10.0));
    
    // Check collision filter: Layer 1, Mask 0
    assert_eq!(entity.collision_filter.layer, 1);
    assert_eq!(entity.collision_filter.mask, 0);
}

#[test]
fn test_vfs_multi_file_require_and_caching() {
    let mut instance = Instance::new(1, 30, 10, 42);

    let mut vfs = std::collections::BTreeMap::new();
    vfs.insert("main.lua".to_string(), r#"
        local utils = require("utils.math")
        local greetings = require("greetings")
        RESULT = utils.add(10, 20)
        GREETING = greetings.message
        CACHED = (package.loaded["greetings"] == greetings)
    "#.to_string());
    vfs.insert("utils/math.lua".to_string(), r#"
        local M = {}
        function M.add(a, b)
            return a + b
        end
        return M
    "#.to_string());
    vfs.insert("greetings.lua".to_string(), r#"
        return { message = "hello vfs" }
    "#.to_string());

    let payload = serde_json::to_string(&vfs).unwrap();
    let res = instance.load_script(&payload);
    assert!(res.is_ok(), "Failed to load VFS bundle: {:?}", res.err());

    let lua = instance.script_engine.lua();
    let result: i32 = lua.globals().get("RESULT").unwrap();
    let greeting: String = lua.globals().get("GREETING").unwrap();
    let cached: bool = lua.globals().get("CACHED").unwrap();

    assert_eq!(result, 30);
    assert_eq!(greeting, "hello vfs");
    assert!(cached);
}

#[test]
fn test_vfs_require_missing_module_error() {
    let instance = Instance::new(1, 30, 10, 42);

    let mut vfs = std::collections::BTreeMap::new();
    vfs.insert("main.lua".to_string(), r#"
        require("unknown_mod")
    "#.to_string());

    let payload = serde_json::to_string(&vfs).unwrap();
    let res = instance.script_engine.load_script(&payload);
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(err.contains("module 'unknown_mod' not found in VFS"));
}

#[test]
fn test_vfs_load_script_from_file_directory_bundle() {
    use tempfile::tempdir;
    let dir = tempdir().unwrap();
    let map_dir = dir.path();

    std::fs::create_dir_all(map_dir.join("skills")).unwrap();
    std::fs::write(
        map_dir.join("main.lua"),
        r#"
        local skill = require("skills.attack")
        DAMAGE = skill.power
        "#,
    ).unwrap();
    std::fs::write(
        map_dir.join("skills/attack.lua"),
        r#"
        return { power = 9000 }
        "#,
    ).unwrap();

    let mut instance = Instance::new(1, 30, 10, 42);
    let res = instance.load_script_from_file(map_dir.join("main.lua"));
    assert!(res.is_ok(), "Failed load_script_from_file: {:?}", res.err());

    let damage: i32 = instance.script_engine.lua().globals().get("DAMAGE").unwrap();
    assert_eq!(damage, 9000);
    assert!(!instance.script_hash.is_empty());
    assert!(!instance.script_payload.is_empty());

    // Verify replay reload round-trip with stored payload
    let mut replay_instance = Instance::new(2, 30, 10, 42);
    let replay_res = replay_instance.load_script(&instance.script_payload);
    assert!(replay_res.is_ok());
    let replay_damage: i32 = replay_instance.script_engine.lua().globals().get("DAMAGE").unwrap();
    assert_eq!(replay_damage, 9000);
    assert_eq!(replay_instance.script_hash, instance.script_hash);
}

#[test]
fn test_vfs_circular_require_detection() {
    let instance = Instance::new(1, 30, 10, 42);

    let mut vfs = std::collections::BTreeMap::new();
    vfs.insert("main.lua".to_string(), r#"
        require("module_a")
    "#.to_string());
    vfs.insert("module_a.lua".to_string(), r#"
        local b = require("module_b")
        return { name = "a" }
    "#.to_string());
    vfs.insert("module_b.lua".to_string(), r#"
        local a = require("module_a")
        return { name = "b" }
    "#.to_string());

    let payload = serde_json::to_string(&vfs).unwrap();
    let res = instance.script_engine.load_script(&payload);
    assert!(res.is_err(), "Circular require should have failed");
    let err = res.unwrap_err();
    assert!(
        err.contains("circular dependency detected while requiring module 'module_a'"),
        "Unexpected error: {err}"
    );
}

#[test]
fn test_vfs_rejects_bytecode() {
    let instance = Instance::new(1, 30, 10, 42);

    let mut vfs = std::collections::BTreeMap::new();
    // \x1bLua is the standard Lua bytecode header
    vfs.insert("main.lua".to_string(), "\x1bLua\x51\x00\x01\x04\x08...".to_string());

    let payload = serde_json::to_string(&vfs).unwrap();
    let res = instance.script_engine.load_script(&payload);
    assert!(res.is_err(), "Bytecode should be rejected");
    let err = res.unwrap_err();
    assert!(
        err.contains("attempt to load a binary chunk") || err.contains("precompiled chunk"),
        "Expected binary chunk rejection error, got: {err}"
    );
}

#[test]
fn test_vfs_crlf_normalization_and_symlink_skip() {
    use tempfile::tempdir;
    let dir = tempdir().unwrap();
    let map_dir = dir.path();

    // Create main.lua with CRLF line endings
    std::fs::write(
        map_dir.join("main.lua"),
        "x = 1\r\ny = 2\r\n",
    ).unwrap();

    // Create a symlink that points to main.lua (or a cyclic symlink)
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let _ = symlink(map_dir.join("main.lua"), map_dir.join("link.lua"));
        let _ = symlink(map_dir, map_dir.join("recursive_link"));
    }

    let mut instance_crlf = Instance::new(1, 30, 10, 42);
    let res = instance_crlf.load_script_from_file(map_dir.join("main.lua"));
    assert!(res.is_ok(), "Failed to load: {:?}", res.err());

    // Compare with an instance loaded with pure LF
    let dir_lf = tempdir().unwrap();
    std::fs::write(
        dir_lf.path().join("main.lua"),
        "x = 1\ny = 2\n",
    ).unwrap();
    let mut instance_lf = Instance::new(2, 30, 10, 42);
    instance_lf.load_script_from_file(dir_lf.path().join("main.lua")).unwrap();

    // Hashes must match despite CRLF vs LF on disk
    assert_eq!(instance_crlf.script_hash, instance_lf.script_hash);
}

#[test]
fn test_spawn_static_obstacle_aabb_collision_raycast_and_snapshot() {
    use loci2d::network::{MoveIntent, ObstacleShape};

    let mut instance = Instance::new(1, 30, 10, 42);
    let script = r#"
        WALL_ID = nil
        function on_init()
            WALL_ID = Loci.Commands.spawn_static_obstacle({
                blueprint = "wall",
                position = { x = 10, y = 0 },
                width = 4,
                height = 20,
                collision_type = "solid",
                properties = { material = "stone" }
            })
        end

        function on_move_intent(entity_id, dx, dy)
            Loci.Commands.set_velocity(entity_id, { x = dx, y = dy })
        end
    "#;
    instance.load_script(script).unwrap();

    let wall_id: u64 = instance.script_engine.lua().globals().get("WALL_ID").unwrap();
    assert_eq!(wall_id, 1);
    assert_eq!(instance.static_obstacles.len(), 1);

    // Inspect via snapshot
    let snapshot = instance.create_snapshot(0);
    assert_eq!(snapshot.obstacles.len(), 1);
    let obs_proto = &snapshot.obstacles[0];
    assert_eq!(obs_proto.id, wall_id);
    assert_eq!(obs_proto.blueprint, "wall");
    assert_eq!(obs_proto.shape, ObstacleShape::ObstacleAabb as i32);
    assert_eq!(obs_proto.position.as_ref().unwrap().x_bits, 10 << 16);
    assert_eq!(obs_proto.position.as_ref().unwrap().y_bits, 0);
    assert_eq!(obs_proto.width_bits, 4 << 16);
    assert_eq!(obs_proto.height_bits, 20 << 16);
    assert!(obs_proto.is_solid);
    assert_eq!(obs_proto.collision_type, "solid");
    assert_eq!(obs_proto.properties.len(), 1);
    assert_eq!(obs_proto.properties[0].key, "material");
    assert_eq!(obs_proto.properties[0].value, "stone");

    // Verify raycast hits Lua-spawned wall and Lua getters work
    let mut cmd_buf = CommandBuffer::new();
    let check_lua = instance
        .script_engine
        .lua()
        .load(
            r#"
            local obs = Loci.get_obstacle(WALL_ID)
            assert(obs ~= nil)
            assert(obs.blueprint == "wall")
            assert(obs.shape == "aabb")
            assert(obs.width == 4.0)
            assert(obs.height == 20.0)
            assert(obs.is_solid == true)
            assert(obs.collision_type == "solid")
            assert(obs.properties.material == "stone")
            assert(Loci.get_obstacle_property(WALL_ID, "material") == "stone")
            local pos = Loci.get_obstacle_position(WALL_ID)
            assert(pos:x_float() == 10.0 and pos:y_float() == 0.0)

            local hit = Loci.Physics.raycast({ x = 0, y = 0 }, { x = 1, y = 0 }, 20.0)
            assert(hit ~= nil, "Raycast must hit Lua-spawned wall")
            assert(hit.obstacle_id == WALL_ID)
            assert(math.abs(hit.point_x - 8.0) < 0.01)
            "#,
        )
        .into_function()
        .unwrap();
    with_scoped_api(instance.script_engine.lua(), &instance, &mut cmd_buf, || {
        check_lua.call::<()>(())
    })
    .unwrap();

    // Join player at (0, 0) with radius 2 and move East (+1, 0) toward wall [8..12]
    let addr = "127.0.0.1:12345".parse().unwrap();
    let player_id = instance.handle_join(addr, "Runner".to_string());
    let move_intent = ClientIntent {
        intent: Some(client_intent::Intent::Move(MoveIntent {
            direction: Some(Vector2 {
                x_bits: (2.0f32 * 65536.0) as i32,
                y_bits: 0,
            }),
        })),
    };
    instance.apply_intent(addr, move_intent);

    for tick in 1..=10 {
        instance.tick(tick).unwrap();
    }

    let player = instance.get_entity(player_id).unwrap();
    // Wall left edge is 8.0, player radius is 2.0 => clamped at x <= 6.0
    assert!(
        player.position.x <= I16F16::from_num(6.0),
        "Player should be blocked by wall at x<=6.0, got {}",
        player.position.x
    );
}

#[test]
fn test_destructible_crate_lifecycle_and_snapshot_removal() {
    use loci2d::network::MoveIntent;

    let mut instance = Instance::new(1, 30, 10, 42);
    let script = r#"
        CRATE_ID = nil
        function on_init()
            CRATE_ID = Loci.Commands.spawn_static_obstacle({
                blueprint = "crate",
                position = { x = 6, y = 0 },
                width = 4,
                height = 4,
                collision_type = "destructible",
                properties = { hp = 2, state = "intact" }
            })
        end

        function on_move_intent(entity_id, dx, dy)
            Loci.Commands.set_velocity(entity_id, { x = dx, y = dy })
        end

        function on_action(entity_id, ability_id, dir_x, dir_y)
            local pos = Loci.get_entity_position(entity_id)
            local hits = Loci.Physics.get_obstacles_in_radius(pos, 6.0)
            for _, obs_id in ipairs(hits) do
                local obs = Loci.get_obstacle(obs_id)
                if obs and obs.collision_type == "destructible" then
                    local hp = tonumber(Loci.get_obstacle_property(obs_id, "hp")) or 0
                    hp = hp - 1
                    if hp <= 0 then
                        Loci.Commands.destroy_static_obstacle(obs_id)
                    else
                        Loci.Commands.set_obstacle_property(obs_id, "hp", tostring(hp))
                        Loci.Commands.set_obstacle_property(obs_id, "state", "damaged")
                    end
                end
            end
        end
    "#;
    instance.load_script(script).unwrap();

    let addr = "127.0.0.1:12345".parse().unwrap();
    let player_id = instance.handle_join(addr, "Breaker".to_string());

    // Hit 1: crate goes from hp=2 to hp=1, remains solid & present in snapshot
    instance.apply_intent(
        addr,
        ClientIntent {
            intent: Some(client_intent::Intent::Action(ActionIntent {
                ability_id: 1,
                target_direction: None,
            })),
        },
    );
    let snap_after_hit1 = instance.create_snapshot(1);
    assert_eq!(snap_after_hit1.obstacles.len(), 1);
    let hp_prop = snap_after_hit1.obstacles[0]
        .properties
        .iter()
        .find(|p| p.key == "hp")
        .unwrap();
    assert_eq!(hp_prop.value, "1");

    // Walk East: blocked by crate left edge (4.0) - player radius (2.0) = 2.0
    instance.apply_intent(
        addr,
        ClientIntent {
            intent: Some(client_intent::Intent::Move(MoveIntent {
                direction: Some(Vector2 {
                    x_bits: (2.0f32 * 65536.0) as i32,
                    y_bits: 0,
                }),
            })),
        },
    );
    for t in 1..=5 {
        instance.tick(t).unwrap();
    }
    assert!(instance.get_entity(player_id).unwrap().position.x <= I16F16::from_num(2.0));

    // Hit 2: crate hp drops to 0 -> destroyed
    instance.apply_intent(
        addr,
        ClientIntent {
            intent: Some(client_intent::Intent::Action(ActionIntent {
                ability_id: 1,
                target_direction: None,
            })),
        },
    );
    assert!(instance.static_obstacles.is_empty());
    assert!(instance.create_snapshot(6).obstacles.is_empty());

    // Continue walking East: player now passes cleanly past x=6
    instance.apply_intent(
        addr,
        ClientIntent {
            intent: Some(client_intent::Intent::Move(MoveIntent {
                direction: Some(Vector2 {
                    x_bits: (2.0f32 * 65536.0) as i32,
                    y_bits: 0,
                }),
            })),
        },
    );
    for t in 6..=12 {
        instance.tick(t).unwrap();
    }
    assert!(
        instance.get_entity(player_id).unwrap().position.x > I16F16::from_num(8.0),
        "Player should pass freely after crate destruction"
    );
}

#[test]
fn test_lua_circle_trigger_zone_and_min_max_aabb() {
    use loci2d::network::ObstacleShape;

    let mut instance = Instance::new(1, 30, 10, 42);
    let script = r#"
        TRIGGER_LOG = ""
        TRIGGER_ID = nil
        WALL_ID = nil
        function on_init()
            TRIGGER_ID = Loci.Commands.spawn_static_obstacle({
                blueprint = "heal_zone",
                shape = "circle",
                position = { x = 10, y = 0 },
                radius = 4,
                collision_type = "trigger"
            })
            WALL_ID = Loci.Commands.spawn_static_obstacle({
                blueprint = "barrier",
                min = { x = -30, y = -10 },
                max = { x = -20, y = 10 },
                collision_type = "solid"
            })
        end

        function on_trigger_enter(entity_id, trigger_id)
            TRIGGER_LOG = TRIGGER_LOG .. "enter:" .. tostring(trigger_id) .. ";"
        end
        function on_trigger_stay(entity_id, trigger_id)
            TRIGGER_LOG = TRIGGER_LOG .. "stay:" .. tostring(trigger_id) .. ";"
        end
        function on_trigger_exit(entity_id, trigger_id)
            TRIGGER_LOG = TRIGGER_LOG .. "exit:" .. tostring(trigger_id) .. ";"
        end
    "#;
    instance.load_script(script).unwrap();

    let trigger_id: u64 = instance.script_engine.lua().globals().get("TRIGGER_ID").unwrap();
    let snap = instance.create_snapshot(0);
    assert_eq!(snap.obstacles.len(), 2);
    let trig_proto = snap.obstacles.iter().find(|o| o.id == trigger_id).unwrap();
    assert_eq!(trig_proto.shape, ObstacleShape::ObstacleCircle as i32);
    assert_eq!(trig_proto.radius_bits, 4 << 16);
    assert!(!trig_proto.is_solid);
    assert_eq!(trig_proto.collision_type, "trigger");

    // Check get_obstacles_by_blueprint
    let mut cmd_buf = CommandBuffer::new();
    let check_bp = instance
        .script_engine
        .lua()
        .load(
            r#"
            local list = Loci.get_obstacles_by_blueprint("heal_zone")
            assert(#list == 1 and list[1] == TRIGGER_ID)
            "#,
        )
        .into_function()
        .unwrap();
    with_scoped_api(instance.script_engine.lua(), &instance, &mut cmd_buf, || {
        check_bp.call::<()>(())
    })
    .unwrap();

    // Move player into trigger (x=10) and then out (x=25)
    let addr = "127.0.0.1:12345".parse().unwrap();
    let player_id = instance.handle_join(addr, "Tester".to_string());
    instance.entities.get_mut(&player_id).unwrap().position = DeterministicVector2::from_f32(10.0, 0.0);
    instance.tick(1).unwrap(); // enter
    instance.tick(2).unwrap(); // stay
    instance.entities.get_mut(&player_id).unwrap().position = DeterministicVector2::from_f32(25.0, 0.0);
    instance.tick(3).unwrap(); // exit

    let log: String = instance.script_engine.lua().globals().get("TRIGGER_LOG").unwrap();
    assert_eq!(log, "enter:1;stay:1;exit:1;");
}

#[test]
fn test_set_map_bounds_and_obstacle_validation_errors() {
    let mut instance = Instance::new(1, 30, 10, 42);
    let script = r#"
        function on_init()
            Loci.Commands.set_map_bounds({ x = -50, y = -50 }, { x = 50, y = 50 })
        end
    "#;
    instance.load_script(script).unwrap();

    assert_eq!(
        instance.map_bounds.min,
        DeterministicVector2::from_i32(-50, -50)
    );
    assert_eq!(
        instance.map_bounds.max,
        DeterministicVector2::from_i32(50, 50)
    );

    // Validation errors on invalid obstacle inputs
    let mut cmd_buf = CommandBuffer::new();
    for bad_script in [
        r#"Loci.Commands.spawn_static_obstacle({ position = { x = 0, y = 0 }, width = -5, height = 10 })"#,
        r#"Loci.Commands.spawn_static_obstacle({ shape = "circle", position = { x = 0, y = 0 }, radius = 0 })"#,
        r#"Loci.Commands.spawn_static_obstacle({ position = { x = 0, y = 0 }, width = 10, height = 10, collision_type = "bogus" })"#,
        r#"Loci.Commands.set_map_bounds({ x = 10, y = 0 }, { x = -10, y = 5 })"#,
    ] {
        let f = instance.script_engine.lua().load(bad_script).into_function().unwrap();
        let res = with_scoped_api(instance.script_engine.lua(), &instance, &mut cmd_buf, || {
            f.call::<()>(())
        });
        assert!(res.is_err(), "Expected error for script: {}", bad_script);
    }
}

#[test]
fn test_spawn_projectile_defaults_and_snapshot() {
    use loci2d::network::{EntityType as ProtoEntityType};
    use loci2d::world::entity::EntityType;
    use loci2d::world::physics::CollisionFilter;

    let mut instance = Instance::new(1, 30, 10, 42);
    let script = r#"
        PROJ_ID = nil
        ENTITY_INFO_OK = false
        LAYER_CONSTANTS_OK = false

        function on_init()
            PROJ_ID = Loci.Commands.spawn_entity({
                blueprint = "fireball",
                entity_type = "Projectile",
                position = { x = 10, y = -5 },
                velocity = { x = 6, y = 0 },
                radius = 1.5,
                lifetime_ticks = 10,
                properties = { damage = 50, owner = "1" }
            })
        end

        function on_tick(tick)
            local ent = Loci.get_entity(PROJ_ID)
            if ent and ent.entity_type == "Projectile" and ent.is_sensor == true
               and ent.lifetime_ticks == 10 and ent.velocity.x == 6
               and Loci.get_entity_type(PROJ_ID) == "Projectile" then
                ENTITY_INFO_OK = true
            end
            if Loci.Physics.Layer.PROJECTILE == 8 and Loci.Physics.Layer.SOLID_WALL == 1
               and Loci.Physics.Layer.PLAYER == 2 then
                LAYER_CONSTANTS_OK = true
            end
        end
    "#;
    instance.load_script(script).unwrap();

    let proj_id: u64 = instance.script_engine.lua().globals().get("PROJ_ID").unwrap();
    let proj = instance.get_entity(proj_id).unwrap();
    assert_eq!(proj.entity_type, EntityType::Projectile);
    assert_eq!(proj.collision_filter, CollisionFilter::default_projectile());
    assert!(proj.is_sensor);
    assert!(proj.navigation.is_none());
    assert_eq!(proj.velocity, DeterministicVector2::from_f32(6.0, 0.0));
    assert_eq!(proj.remaining_lifetime_ticks, Some(10));

    let snap = instance.create_snapshot(0);
    let snap_ent = snap.entities.iter().find(|e| e.id == proj_id).unwrap();
    assert_eq!(snap_ent.entity_type, ProtoEntityType::Projectile as i32);

    instance.tick(1).unwrap();
    let globals = instance.script_engine.lua().globals();
    assert!(globals.get::<bool>("ENTITY_INFO_OK").unwrap());
    assert!(globals.get::<bool>("LAYER_CONSTANTS_OK").unwrap());

    // Mutate sensor and lifetime via Lua commands
    let mut cmd_buf = CommandBuffer::new();
    let mutate_fn = instance
        .script_engine
        .lua()
        .load(
            r#"
            Loci.Commands.set_sensor(PROJ_ID, false)
            Loci.Commands.set_lifetime(PROJ_ID, 25)
            "#,
        )
        .into_function()
        .unwrap();
    with_scoped_api(instance.script_engine.lua(), &instance, &mut cmd_buf, || {
        mutate_fn.call::<()>(())
    })
    .unwrap();
    cmd_buf.flush_and_apply(&mut instance);

    let updated = instance.get_entity(proj_id).unwrap();
    assert!(!updated.is_sensor);
    assert_eq!(updated.remaining_lifetime_ticks, Some(25));
}

#[test]
fn test_projectile_sensor_collision_no_player_pushback() {
    let mut instance = Instance::new(1, 30, 10, 42);
    let addr = "127.0.0.1:12345".parse().unwrap();
    let player_id = instance.handle_join(addr, "TargetPlayer".to_string());
    instance.entities.get_mut(&player_id).unwrap().position =
        DeterministicVector2::from_f32(10.0, 0.0);

    let script = r#"
        HIT_LOG = ""
        PROJ_ID = nil

        function on_init()
            PROJ_ID = Loci.Commands.spawn_entity({
                blueprint = "arrow",
                entity_type = "Projectile",
                position = { x = 5, y = 0 },
                velocity = { x = 4, y = 0 },
                radius = 1.5
            })
        end

        function on_collision(id_a, id_b)
            HIT_LOG = HIT_LOG .. tostring(id_a) .. "," .. tostring(id_b) .. ";"
            if id_a == PROJ_ID or id_b == PROJ_ID then
                Loci.Commands.destroy_entity(PROJ_ID)
            end
        end
    "#;
    instance.load_script(script).unwrap();
    let proj_id: u64 = instance.script_engine.lua().globals().get("PROJ_ID").unwrap();

    instance.tick(1).unwrap();

    // Collision fired and projectile destroyed, while player position & velocity stayed untouched
    let hit_log: String = instance.script_engine.lua().globals().get("HIT_LOG").unwrap();
    let expected = format!("{},{};", player_id.min(proj_id), player_id.max(proj_id));
    assert_eq!(hit_log, expected);
    assert!(instance.get_entity(proj_id).is_none());

    let player_after = instance.get_entity(player_id).unwrap();
    assert_eq!(player_after.position, DeterministicVector2::from_f32(10.0, 0.0));
    assert_eq!(player_after.velocity, DeterministicVector2::ZERO);
}

#[test]
fn test_projectile_static_and_boundary_collision_callbacks() {
    let mut instance = Instance::new(1, 30, 10, 42);
    let script = r#"
        STATIC_LOG = ""
        BOUNDARY_LOG = ""
        DESTRUCT_PROJ = nil
        BOUNCE_PROJ = nil

        function on_init()
            Loci.Commands.set_map_bounds({ x = -20, y = -20 }, { x = 20, y = 20 })
            Loci.Commands.spawn_static_obstacle({
                blueprint = "wall",
                position = { x = 10, y = 0 },
                width = 4,
                height = 10,
                collision_type = "solid"
            })
            DESTRUCT_PROJ = Loci.Commands.spawn_entity({
                blueprint = "fireball",
                entity_type = "Projectile",
                position = { x = 4, y = 0 },
                velocity = { x = 6, y = 0 },
                radius = 1.0
            })
            BOUNCE_PROJ = Loci.Commands.spawn_entity({
                blueprint = "orb",
                entity_type = "Projectile",
                position = { x = 0, y = 17 },
                velocity = { x = 0, y = 5 },
                radius = 1.0
            })
        end

        function on_static_collision(entity_id, obstacle_id, nx, ny)
            STATIC_LOG = STATIC_LOG .. string.format("%d:%d:%.1f,%.1f;", entity_id, obstacle_id, nx, ny)
            if entity_id == DESTRUCT_PROJ then
                Loci.Commands.destroy_entity(entity_id)
            end
        end

        function on_boundary_collision(entity_id, nx, ny)
            BOUNDARY_LOG = BOUNDARY_LOG .. string.format("%d:%.1f,%.1f;", entity_id, nx, ny)
            if entity_id == BOUNCE_PROJ then
                local v = Loci.get_velocity(entity_id)
                local dot = v.x * nx + v.y * ny
                local rvx = v.x - 2 * dot * nx
                local rvy = v.y - 2 * dot * ny
                Loci.Commands.set_velocity(entity_id, { x = rvx, y = rvy })
            end
        end
    "#;
    instance.load_script(script).unwrap();

    let destruct_id: u64 = instance.script_engine.lua().globals().get("DESTRUCT_PROJ").unwrap();
    let bounce_id: u64 = instance.script_engine.lua().globals().get("BOUNCE_PROJ").unwrap();

    instance.tick(1).unwrap();

    let static_log: String = instance.script_engine.lua().globals().get("STATIC_LOG").unwrap();
    let boundary_log: String = instance.script_engine.lua().globals().get("BOUNDARY_LOG").unwrap();
    assert_eq!(static_log, format!("{}:1:-1.0,0.0;", destruct_id));
    assert_eq!(boundary_log, format!("{}:0.0,-1.0;", bounce_id));
    assert!(instance.get_entity(destruct_id).is_none());

    // Bounce projectile reflected velocity to (0, -5) and sits clamped at y = 19
    let bounce_after_t1 = instance.get_entity(bounce_id).unwrap();
    assert_eq!(bounce_after_t1.position, DeterministicVector2::from_f32(0.0, 19.0));
    assert_eq!(bounce_after_t1.velocity, DeterministicVector2::from_f32(0.0, -5.0));

    instance.tick(2).unwrap();
    let bounce_after_t2 = instance.get_entity(bounce_id).unwrap();
    assert_eq!(bounce_after_t2.position, DeterministicVector2::from_f32(0.0, 14.0));
}

#[test]
fn test_fast_projectile_swept_ccd_no_tunneling() {
    // 1. Fast projectile vs thin static wall
    let mut wall_inst = Instance::new(1, 30, 10, 42);
    let wall_script = r#"
        WALL_HIT = false
        HIT_POS_X = nil
        PROJ_ID = nil

        function on_init()
            Loci.Commands.spawn_static_obstacle({
                blueprint = "thin_wall",
                position = { x = 15, y = 0 },
                width = 2.0,
                height = 20.0,
                collision_type = "solid"
            })
            PROJ_ID = Loci.Commands.spawn_entity({
                blueprint = "rail_bolt",
                entity_type = "Projectile",
                position = { x = 0, y = 0 },
                velocity = { x = 40, y = 0 },
                radius = 1.0
            })
        end

        function on_static_collision(entity_id, obstacle_id, nx, ny)
            if entity_id == PROJ_ID and nx == -1.0 and ny == 0.0 then
                WALL_HIT = true
                local pos = Loci.get_entity_position(entity_id)
                HIT_POS_X = pos:x_float()
                Loci.Commands.destroy_entity(entity_id)
            end
        end
    "#;
    wall_inst.load_script(wall_script).unwrap();
    wall_inst.tick(1).unwrap();

    let globals = wall_inst.script_engine.lua().globals();
    assert!(globals.get::<bool>("WALL_HIT").unwrap());
    let hit_x: f64 = globals.get("HIT_POS_X").unwrap();
    assert!(
        (hit_x - 13.0).abs() < 0.05,
        "Expected swept stop near x=13.0 (wall min.x=14 - radius=1), got {}",
        hit_x
    );

    // 2. Fast projectile vs player entity (without tunneling and without pushback)
    let mut player_inst = Instance::new(2, 30, 10, 42);
    let addr = "127.0.0.1:12346".parse().unwrap();
    let player_id = player_inst.handle_join(addr, "Target".to_string());
    player_inst.entities.get_mut(&player_id).unwrap().position =
        DeterministicVector2::from_f32(15.0, 0.0);

    let player_script = r#"
        PLAYER_HIT = false
        PROJ_ID = nil

        function on_init()
            PROJ_ID = Loci.Commands.spawn_entity({
                blueprint = "rail_bolt",
                entity_type = "Projectile",
                position = { x = 0, y = 0 },
                velocity = { x = 40, y = 0 },
                radius = 1.0
            })
        end

        function on_collision(id_a, id_b)
            if id_a == PROJ_ID or id_b == PROJ_ID then
                PLAYER_HIT = true
                Loci.Commands.destroy_entity(PROJ_ID)
            end
        end
    "#;
    player_inst.load_script(player_script).unwrap();
    player_inst.tick(1).unwrap();

    let p_globals = player_inst.script_engine.lua().globals();
    assert!(p_globals.get::<bool>("PLAYER_HIT").unwrap());
    let target = player_inst.get_entity(player_id).unwrap();
    assert_eq!(target.position, DeterministicVector2::from_f32(15.0, 0.0));
}

#[test]
fn test_projectile_lifetime_auto_despawn() {
    let mut instance = Instance::new(1, 30, 10, 42);
    let script = r#"
        PROJ_ID = nil
        function on_init()
            PROJ_ID = Loci.Commands.spawn_entity({
                blueprint = "spark",
                entity_type = "Projectile",
                position = { x = 0, y = 0 },
                velocity = { x = 2, y = 0 },
                radius = 0.5,
                lifetime_ticks = 3
            })
        end
    "#;
    instance.load_script(script).unwrap();
    let proj_id: u64 = instance.script_engine.lua().globals().get("PROJ_ID").unwrap();

    instance.tick(1).unwrap();
    let p1 = instance.get_entity(proj_id).unwrap();
    assert_eq!(p1.position, DeterministicVector2::from_f32(2.0, 0.0));
    assert_eq!(p1.remaining_lifetime_ticks, Some(2));

    instance.tick(2).unwrap();
    let p2 = instance.get_entity(proj_id).unwrap();
    assert_eq!(p2.position, DeterministicVector2::from_f32(4.0, 0.0));
    assert_eq!(p2.remaining_lifetime_ticks, Some(1));

    instance.tick(3).unwrap();
    assert!(
        instance.get_entity(proj_id).is_none(),
        "Projectile should auto-despawn on tick 3"
    );
}

#[test]
fn test_projectile_wall_ricochet_moves_away_next_tick() {
    let mut instance = Instance::new(1, 30, 10, 42);
    let script = r#"
        WALL_HITS = 0
        PROJ_ID = nil

        function on_init()
            Loci.Commands.spawn_static_obstacle({
                blueprint = "wall",
                position = { x = 15, y = 0 },
                width = 2.0,
                height = 20.0,
                collision_type = "solid"
            })
            PROJ_ID = Loci.Commands.spawn_entity({
                blueprint = "ricochet_orb",
                entity_type = "Projectile",
                position = { x = 0, y = 0 },
                velocity = { x = 40, y = 0 },
                radius = 1.0
            })
        end

        function on_static_collision(entity_id, obstacle_id, nx, ny)
            if entity_id == PROJ_ID then
                WALL_HITS = WALL_HITS + 1
                local v = Loci.get_velocity(entity_id)
                local dot = v.x * nx + v.y * ny
                Loci.Commands.set_velocity(entity_id, {
                    x = v.x - 2 * dot * nx,
                    y = v.y - 2 * dot * ny
                })
            end
        end
    "#;
    instance.load_script(script).unwrap();
    let proj_id: u64 = instance.script_engine.lua().globals().get("PROJ_ID").unwrap();

    // Tick 1: swept CCD stops at x = 13.0 and reflects velocity to (-40, 0)
    instance.tick(1).unwrap();
    assert_eq!(instance.script_engine.lua().globals().get::<i32>("WALL_HITS").unwrap(), 1);
    let p1 = instance.get_entity(proj_id).unwrap();
    assert!((p1.position.x.to_num::<f64>() - 13.0).abs() < 0.05);
    assert_eq!(p1.velocity, DeterministicVector2::from_f32(-40.0, 0.0));

    // Tick 2: must move cleanly away to x ≈ -27.0 without sticking at t == 0 or re-colliding
    instance.tick(2).unwrap();
    assert_eq!(instance.script_engine.lua().globals().get::<i32>("WALL_HITS").unwrap(), 1);
    let p2 = instance.get_entity(proj_id).unwrap();
    assert!(
        (p2.position.x.to_num::<f64>() - (-27.0)).abs() < 0.05,
        "Expected ricochet to move away to x≈-27.0, got {}",
        p2.position.x.to_num::<f64>()
    );
}

#[test]
fn test_fast_projectile_wall_occlusion_and_piercing_no_snag() {
    let mut instance = Instance::new(1, 30, 10, 42);
    let addr_ally = "127.0.0.1:12350".parse().unwrap();
    let ally_id = instance.handle_join(addr_ally, "Ally".to_string());
    instance.entities.get_mut(&ally_id).unwrap().position =
        DeterministicVector2::from_f32(10.0, 0.0);

    let addr_enemy = "127.0.0.1:12351".parse().unwrap();
    let enemy_id = instance.handle_join(addr_enemy, "Enemy".to_string());
    instance.entities.get_mut(&enemy_id).unwrap().position =
        DeterministicVector2::from_f32(22.0, 0.0);

    let addr_behind = "127.0.0.1:12352".parse().unwrap();
    let behind_id = instance.handle_join(addr_behind, "BehindWall".to_string());
    instance.entities.get_mut(&behind_id).unwrap().position =
        DeterministicVector2::from_f32(36.0, 0.0);

    let script = r#"
        COLLISIONS = ""
        WALL_HIT = false
        PROJ_ID = nil

        function on_init()
            Loci.Commands.spawn_static_obstacle({
                blueprint = "wall",
                position = { x = 30, y = 0 },
                width = 2.0,
                height = 20.0,
                collision_type = "solid"
            })
            PROJ_ID = Loci.Commands.spawn_entity({
                blueprint = "piercing_bolt",
                entity_type = "Projectile",
                position = { x = 0, y = 0 },
                velocity = { x = 40, y = 0 },
                radius = 1.0
            })
        end

        function on_collision(id_a, id_b)
            local other = (id_a == PROJ_ID) and id_b or id_a
            COLLISIONS = COLLISIONS .. tostring(other) .. ";"
        end

        function on_static_collision(entity_id, obstacle_id, nx, ny)
            if entity_id == PROJ_ID then
                WALL_HIT = true
            end
        end
    "#;
    instance.load_script(script).unwrap();
    let proj_id: u64 = instance.script_engine.lua().globals().get("PROJ_ID").unwrap();

    instance.tick(1).unwrap();

    let globals = instance.script_engine.lua().globals();
    let collisions: String = globals.get("COLLISIONS").unwrap();
    let wall_hit: bool = globals.get("WALL_HIT").unwrap();

    assert!(wall_hit, "Projectile should hit solid wall at x=30");
    assert_eq!(
        collisions,
        format!("{};{};", ally_id, enemy_id),
        "Should detect ally and enemy along path without snagging, and remain occluded from behind_id"
    );

    let proj_after = instance.get_entity(proj_id).unwrap();
    assert!(
        (proj_after.position.x.to_num::<f64>() - 28.0).abs() < 0.05,
        "Projectile should travel past ally/enemy and stop at wall surface x≈28.0, got {}",
        proj_after.position.x.to_num::<f64>()
    );
}
