use crate::scripting::command::{Command, CommandBuffer};
use crate::world::instance::{DeterministicVector2, Instance};
use crate::world::physics::{
    ColliderShape, CollisionFilter, DeterministicAABB, DeterministicCircle, MapBounds,
    intersect_shapes,
};
use fixed::types::I16F16;
use mlua::prelude::*;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

fn extract_vector2(value: mlua::Value) -> LuaResult<DeterministicVector2> {
    match value {
        mlua::Value::UserData(ud) => {
            if let Ok(vec) = ud.borrow::<DeterministicVector2>() {
                Ok(*vec)
            } else {
                Err(mlua::Error::RuntimeError("Expected Loci.Vector2 userdata".to_string()))
            }
        }
        mlua::Value::Table(t) => {
            let x: f64 = t.get("x").map_err(|_| mlua::Error::RuntimeError("Table missing 'x' field or it is not a number".to_string()))?;
            let y: f64 = t.get("y").map_err(|_| mlua::Error::RuntimeError("Table missing 'y' field or it is not a number".to_string()))?;
            Ok(DeterministicVector2::from_f64(x, y))
        }
        _ => Err(mlua::Error::RuntimeError("Expected Loci.Vector2 or table with x, y".to_string())),
    }
}

fn extract_properties(props_table: mlua::Table) -> LuaResult<BTreeMap<String, String>> {
    let mut properties = BTreeMap::new();
    for pair in props_table.pairs::<String, mlua::Value>() {
        let (k, v) = pair?;
        let str_val = match v {
            mlua::Value::String(s) => s.to_string_lossy().to_string(),
            mlua::Value::Integer(n) => n.to_string(),
            mlua::Value::Number(n) => n.to_string(),
            mlua::Value::Boolean(b) => b.to_string(),
            _ => {
                return Err(mlua::Error::RuntimeError(format!(
                    "property '{}' value must be string, number, or boolean",
                    k
                )));
            }
        };
        properties.insert(k, str_val);
    }
    Ok(properties)
}

struct ParsedObstacleArgs {
    blueprint: String,
    shape: ColliderShape,
    filter: CollisionFilter,
    is_solid: bool,
    collision_type: String,
    properties: BTreeMap<String, String>,
}

fn parse_static_obstacle_args(args: &mlua::Table) -> LuaResult<ParsedObstacleArgs> {
    let explicit_is_solid: Option<bool> = args.get::<Option<bool>>("is_solid")?;
    let raw_collision_type: Option<String> = args.get::<Option<String>>("collision_type")?;

    let collision_type = match raw_collision_type {
        Some(ct) => {
            let normalized = ct.trim().to_ascii_lowercase();
            match normalized.as_str() {
                "solid" | "destructible" | "trigger" => normalized,
                _ => {
                    return Err(mlua::Error::RuntimeError(format!(
                        "spawn_static_obstacle: invalid collision_type '{}' (expected 'solid', 'destructible', or 'trigger')",
                        ct
                    )));
                }
            }
        }
        None => match explicit_is_solid {
            Some(false) => "trigger".to_string(),
            _ => "solid".to_string(),
        },
    };

    let is_solid = explicit_is_solid.unwrap_or_else(|| collision_type != "trigger");

    let blueprint = match args.get::<String>("blueprint") {
        Ok(bp) => {
            if bp.trim().is_empty() {
                return Err(mlua::Error::RuntimeError(
                    "spawn_static_obstacle: blueprint cannot be empty".to_string(),
                ));
            }
            bp
        }
        Err(_) => {
            if is_solid {
                "wall".to_string()
            } else {
                "trigger".to_string()
            }
        }
    };

    let default_filter = if is_solid {
        CollisionFilter::default_solid_wall()
    } else {
        CollisionFilter::default_trigger_zone()
    };
    let layer: u16 = args.get("layer").unwrap_or(default_filter.layer);
    let mask: u16 = args.get("mask").unwrap_or(default_filter.mask);
    let filter = CollisionFilter::new(layer, mask);

    let has_radius = args.contains_key("radius")?;
    let has_dims = args.contains_key("width")? || args.contains_key("height")?;
    let shape_kind = match args.get::<String>("shape") {
        Ok(s) => s.trim().to_ascii_lowercase(),
        Err(_) => {
            if has_radius && !has_dims {
                "circle".to_string()
            } else {
                "aabb".to_string()
            }
        }
    };

    let shape = match shape_kind.as_str() {
        "circle" => {
            let pos_val: mlua::Value = args.get("position").map_err(|_| {
                mlua::Error::RuntimeError("spawn_static_obstacle: circle obstacle requires 'position'".to_string())
            })?;
            let center = extract_vector2(pos_val)?;
            let radius: f64 = args.get("radius").map_err(|_| {
                mlua::Error::RuntimeError("spawn_static_obstacle: circle obstacle requires numeric 'radius'".to_string())
            })?;
            if !radius.is_finite() || radius <= 0.0 {
                return Err(mlua::Error::RuntimeError(
                    "spawn_static_obstacle: 'radius' must be finite and > 0".to_string(),
                ));
            }
            ColliderShape::Circle(DeterministicCircle::new(center, I16F16::from_num(radius)))
        }
        "aabb" => {
            if args.contains_key("min")? || args.contains_key("max")? {
                let min_val: mlua::Value = args.get("min").map_err(|_| {
                    mlua::Error::RuntimeError("spawn_static_obstacle: 'min' required when 'max' is provided".to_string())
                })?;
                let max_val: mlua::Value = args.get("max").map_err(|_| {
                    mlua::Error::RuntimeError("spawn_static_obstacle: 'max' required when 'min' is provided".to_string())
                })?;
                let min = extract_vector2(min_val)?;
                let max = extract_vector2(max_val)?;
                if min.x >= max.x || min.y >= max.y {
                    return Err(mlua::Error::RuntimeError(
                        "spawn_static_obstacle: 'min' must be strictly less than 'max' on both axes".to_string(),
                    ));
                }
                ColliderShape::AABB(DeterministicAABB::new(min, max))
            } else {
                let pos_val: mlua::Value = args.get("position").map_err(|_| {
                    mlua::Error::RuntimeError("spawn_static_obstacle: AABB obstacle requires 'position' (or 'min'/'max')".to_string())
                })?;
                let center = extract_vector2(pos_val)?;
                let width: f64 = args.get("width").map_err(|_| {
                    mlua::Error::RuntimeError("spawn_static_obstacle: AABB obstacle requires numeric 'width'".to_string())
                })?;
                let height: f64 = args.get("height").map_err(|_| {
                    mlua::Error::RuntimeError("spawn_static_obstacle: AABB obstacle requires numeric 'height'".to_string())
                })?;
                if !width.is_finite() || width <= 0.0 || !height.is_finite() || height <= 0.0 {
                    return Err(mlua::Error::RuntimeError(
                        "spawn_static_obstacle: 'width' and 'height' must be finite and > 0".to_string(),
                    ));
                }
                let half_extents = DeterministicVector2::from_f64(width / 2.0, height / 2.0);
                ColliderShape::AABB(DeterministicAABB::from_center_half_extents(center, half_extents))
            }
        }
        other => {
            return Err(mlua::Error::RuntimeError(format!(
                "spawn_static_obstacle: invalid shape '{}' (expected 'aabb' or 'circle')",
                other
            )));
        }
    };

    let properties = match args.get::<mlua::Table>("properties") {
        Ok(tbl) => extract_properties(tbl)?,
        Err(_) => BTreeMap::new(),
    };

    Ok(ParsedObstacleArgs {
        blueprint,
        shape,
        filter,
        is_solid,
        collision_type,
        properties,
    })
}

/// Sets up the base Loci global API which doesn't require an active Instance context.
/// This includes the Loci.Vector2 constructor and basic logging.
pub fn setup_base_api(lua: &Lua) -> LuaResult<()> {
    let globals = lua.globals();

    let loci_table = lua.create_table()?;

    // Loci.Vector2(x, y)
    let vector2_fn =
        lua.create_function(|_, (x, y): (f64, f64)| Ok(DeterministicVector2::from_f64(x, y)))?;
    loci_table.set("Vector2", vector2_fn)?;

    // Setup Loci.Log for scripts
    let log_table = lua.create_table()?;
    log_table.set(
        "info",
        lua.create_function(|_, msg: String| {
            crate::log_info!("Lua", "{}", msg);
            Ok(())
        })?,
    )?;
    log_table.set(
        "warn",
        lua.create_function(|_, msg: String| {
            crate::log_warn!("Lua", "{}", msg);
            Ok(())
        })?,
    )?;
    log_table.set(
        "error",
        lua.create_function(|_, msg: String| {
            crate::log_error!("Lua", "{}", msg);
            Ok(())
        })?,
    )?;
    loci_table.set("Log", log_table)?;

    globals.set("Loci", loci_table)?;

    Ok(())
}

/// Helper function to execute a closure with the scoped Loci API bound.
/// This safely bridges the Rust Instance and CommandBuffer into Lua for a single callback.
pub fn with_scoped_api<F, R>(
    lua: &Lua,
    instance: &Instance,
    command_buffer: &mut CommandBuffer,
    f: F,
) -> LuaResult<R>
where
    F: FnOnce() -> LuaResult<R>,
{
    // We use a RefCell to allow mutating the command buffer from multiple Lua closures
    let cmd_buffer_rc = Rc::new(RefCell::new(command_buffer));

    lua.scope(|scope| {
        let globals = lua.globals();
        let loci_table: mlua::Table = globals.get("Loci")?;

        // Loci.get_entity_by_name(name)
        let get_entity_by_name = scope.create_function(|_, name: String| {
            let entity_id = instance
                .entities
                .values()
                .find(|e| e.name == name)
                .map(|e| e.id);
            Ok(entity_id)
        })?;
        loci_table.set("get_entity_by_name", get_entity_by_name)?;

        // Loci.get_entity_position(id)
        let get_entity_position = scope.create_function(|_, id: u64| {
            if let Some(entity) = instance.get_entity(id) {
                Ok(Some(entity.position))
            } else {
                Ok(None)
            }
        })?;
        loci_table.set("get_entity_position", get_entity_position)?;

        // Loci.get_entities_in_radius(position, radius)
        let get_entities_in_radius = scope.create_function(|lua, (pos_val, radius): (mlua::Value, f64)| {
            let position = extract_vector2(pos_val)?;
            let radius_fp = fixed::types::I16F16::from_num(radius);
            
            let mut ids = Vec::new();
            for entity in instance.entities.values() {
                if entity.position.distance(position) <= radius_fp {
                    ids.push(entity.id);
                }
            }
            
            let table = lua.create_table()?;
            for (i, id) in ids.into_iter().enumerate() {
                table.set(i + 1, id)?;
            }
            
            Ok(table)
        })?;
        loci_table.set("get_entities_in_radius", get_entities_in_radius)?;

        // Loci.get_velocity(id)
        let get_velocity = scope.create_function(|lua, id: u64| {
            if let Some(entity) = instance.get_entity(id) {
                let vel_table = lua.create_table()?;
                vel_table.set("x", entity.velocity.x.to_num::<f64>())?;
                vel_table.set("y", entity.velocity.y.to_num::<f64>())?;
                Ok(Some(vel_table))
            } else {
                Ok(None)
            }
        })?;
        loci_table.set("get_velocity", get_velocity)?;

        // Loci.get_move_speed(id)
        let get_move_speed = scope.create_function(|_, id: u64| {
            if let Some(entity) = instance.get_entity(id) {
                if let Some(nav) = &entity.navigation {
                    Ok(Some(nav.move_speed.to_num::<f64>()))
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        })?;
        loci_table.set("get_move_speed", get_move_speed)?;

        // Loci.get_entity_name(id)
        let get_entity_name = scope.create_function(|_, id: u64| {
            if let Some(entity) = instance.get_entity(id) {
                Ok(Some(entity.name.clone()))
            } else {
                Ok(None)
            }
        })?;
        loci_table.set("get_entity_name", get_entity_name)?;

        // Loci.get_entity_property(id, key)
        let get_entity_property = scope.create_function(|_, (id, key): (u64, String)| {
            if let Some(entity) = instance.get_entity(id) {
                Ok(entity.properties.get(&key).cloned())
            } else {
                Ok(None)
            }
        })?;
        loci_table.set("get_entity_property", get_entity_property)?;

        // Loci.get_entity(id)
        let get_entity = scope.create_function(|lua, id: u64| {
            let Some(entity) = instance.get_entity(id) else {
                return Ok(None);
            };
            let tbl = lua.create_table()?;
            tbl.set("id", entity.id)?;
            tbl.set("name", entity.name.clone())?;
            tbl.set("blueprint", entity.name.clone())?;
            let type_str = match entity.entity_type {
                crate::world::entity::EntityType::Player => "Player",
                crate::world::entity::EntityType::NPC => "NPC",
                crate::world::entity::EntityType::Prop => "Prop",
                crate::world::entity::EntityType::Projectile => "Projectile",
            };
            tbl.set("entity_type", type_str)?;
            tbl.set("position", entity.position)?;
            let vel_tbl = lua.create_table()?;
            vel_tbl.set("x", entity.velocity.x.to_num::<f64>())?;
            vel_tbl.set("y", entity.velocity.y.to_num::<f64>())?;
            tbl.set("velocity", vel_tbl)?;
            tbl.set("is_sensor", entity.is_sensor)?;
            if let Some(ticks) = entity.remaining_lifetime_ticks {
                tbl.set("lifetime_ticks", ticks)?;
            }
            let props_tbl = lua.create_table()?;
            for (k, v) in &entity.properties {
                props_tbl.set(k.clone(), v.clone())?;
            }
            tbl.set("properties", props_tbl)?;
            Ok(Some(tbl))
        })?;
        loci_table.set("get_entity", get_entity)?;

        // Loci.get_entity_type(id)
        let get_entity_type = scope.create_function(|_, id: u64| {
            Ok(instance.get_entity(id).map(|entity| match entity.entity_type {
                crate::world::entity::EntityType::Player => "Player",
                crate::world::entity::EntityType::NPC => "NPC",
                crate::world::entity::EntityType::Prop => "Prop",
                crate::world::entity::EntityType::Projectile => "Projectile",
            }))
        })?;
        loci_table.set("get_entity_type", get_entity_type)?;

        // Loci.get_global(key)
        let get_global =
            scope.create_function(|_, key: String| Ok(instance.globals.get(&key).cloned()))?;
        loci_table.set("get_global", get_global)?;

        // Loci.get_obstacle(id)
        let get_obstacle = scope.create_function(|lua, id: u64| {
            let Some(obs) = instance.get_static_obstacle(id) else {
                return Ok(None);
            };
            let tbl = lua.create_table()?;
            tbl.set("id", obs.id)?;
            tbl.set("blueprint", obs.blueprint.clone())?;
            tbl.set("is_solid", obs.is_solid)?;
            tbl.set("collision_type", obs.collision_type.clone())?;
            tbl.set("position", obs.shape.center())?;
            match &obs.shape {
                ColliderShape::AABB(aabb) => {
                    tbl.set("shape", "aabb")?;
                    tbl.set("width", aabb.width().to_num::<f64>())?;
                    tbl.set("height", aabb.height().to_num::<f64>())?;
                }
                ColliderShape::Circle(circle) => {
                    tbl.set("shape", "circle")?;
                    tbl.set("radius", circle.radius.to_num::<f64>())?;
                }
            }
            let props_tbl = lua.create_table()?;
            for (k, v) in &obs.properties {
                props_tbl.set(k.clone(), v.clone())?;
            }
            tbl.set("properties", props_tbl)?;
            Ok(Some(tbl))
        })?;
        loci_table.set("get_obstacle", get_obstacle)?;

        // Loci.get_obstacle_position(id)
        let get_obstacle_position = scope.create_function(|_, id: u64| {
            Ok(instance
                .get_static_obstacle(id)
                .map(|obs| obs.shape.center()))
        })?;
        loci_table.set("get_obstacle_position", get_obstacle_position)?;

        // Loci.get_obstacle_property(id, key)
        let get_obstacle_property = scope.create_function(|_, (id, key): (u64, String)| {
            if let Some(obs) = instance.get_static_obstacle(id) {
                Ok(obs.properties.get(&key).cloned())
            } else {
                Ok(None)
            }
        })?;
        loci_table.set("get_obstacle_property", get_obstacle_property)?;

        // Loci.get_obstacles_by_blueprint(blueprint)
        let get_obstacles_by_blueprint = scope.create_function(|lua, blueprint: String| {
            let table = lua.create_table()?;
            let mut idx = 1usize;
            for (&id, obs) in &instance.static_obstacles {
                if obs.blueprint == blueprint {
                    table.set(idx, id)?;
                    idx += 1;
                }
            }
            Ok(table)
        })?;
        loci_table.set("get_obstacles_by_blueprint", get_obstacles_by_blueprint)?;

        // Loci.Commands
        let commands_table = lua.create_table()?;

        let cmd_buf_spawn = Rc::clone(&cmd_buffer_rc);
        let spawn_entity = scope.create_function(move |_, args: mlua::Table| {
            let blueprint: String = args.get("blueprint")?;
            if blueprint.trim().is_empty() {
                return Err(mlua::Error::RuntimeError("spawn_entity: blueprint cannot be empty".to_string()));
            }

            let position_val: mlua::Value = args.get("position")?;
            let position = extract_vector2(position_val)?;

            let velocity = match args.get::<mlua::Value>("velocity")? {
                mlua::Value::Nil => None,
                v => Some(extract_vector2(v)?),
            };

            let entity_type: String = args.get("entity_type").unwrap_or_else(|_| "Prop".to_string());
            let move_speed: f64 = args.get("move_speed").unwrap_or(1.0);
            let radius: f64 = args.get("radius").unwrap_or(2.0);
            let is_sensor: Option<bool> = args.get("is_sensor")?;
            let raw_lifetime: Option<i64> = args.get("lifetime_ticks")?;
            let lifetime_ticks = raw_lifetime.and_then(|t| if t > 0 { u32::try_from(t).ok() } else { None });
            let collision_layer: Option<u16> = args.get("collision_layer")?;
            let collision_mask: Option<u16> = args.get("collision_mask")?;

            let properties = match args.get::<mlua::Table>("properties") {
                Ok(props_table) => extract_properties(props_table)?,
                Err(_) => BTreeMap::new(),
            };

            let entity_id = instance.allocate_entity_id();

            cmd_buf_spawn.borrow_mut().push(Command::SpawnEntity {
                entity_id,
                blueprint,
                position,
                velocity,
                entity_type,
                move_speed: fixed::types::I16F16::from_num(move_speed),
                radius: fixed::types::I16F16::from_num(radius),
                is_sensor,
                lifetime_ticks,
                collision_layer,
                collision_mask,
                properties,
            });
            Ok(entity_id)
        })?;
        commands_table.set("spawn_entity", spawn_entity)?;

        let cmd_buf_destroy = Rc::clone(&cmd_buffer_rc);
        let destroy_entity = scope.create_function(move |_, id: u64| {
            cmd_buf_destroy
                .borrow_mut()
                .push(Command::DestroyEntity { entity_id: id });
            Ok(())
        })?;
        commands_table.set("destroy_entity", destroy_entity)?;

        let cmd_buf_set_pos = Rc::clone(&cmd_buffer_rc);
        let set_position =
            scope.create_function(move |_, (id, position_val): (u64, mlua::Value)| {
                let position = extract_vector2(position_val)?;
                cmd_buf_set_pos.borrow_mut().push(Command::SetPosition {
                    entity_id: id,
                    position,
                });
                Ok(())
            })?;
        commands_table.set("set_position", set_position)?;

        let cmd_buf_set_vel = Rc::clone(&cmd_buffer_rc);
        let set_velocity =
            scope.create_function(move |_, (id, velocity_val): (u64, mlua::Value)| {
                let velocity = extract_vector2(velocity_val)?;
                cmd_buf_set_vel.borrow_mut().push(Command::SetVelocity {
                    entity_id: id,
                    velocity,
                });
                Ok(())
            })?;
        commands_table.set("set_velocity", set_velocity)?;

        let cmd_buf_set_nav = Rc::clone(&cmd_buffer_rc);
        let set_navigation_target =
            scope.create_function(move |_, (id, target_val): (u64, mlua::Value)| {
                let target = extract_vector2(target_val)?;
                cmd_buf_set_nav.borrow_mut().push(Command::SetNavigationTarget {
                    entity_id: id,
                    target,
                });
                Ok(())
            })?;
        commands_table.set("set_navigation_target", set_navigation_target)?;

        let cmd_buf_set_speed = Rc::clone(&cmd_buffer_rc);
        let set_move_speed =
            scope.create_function(move |_, (id, speed): (u64, f64)| {
                cmd_buf_set_speed.borrow_mut().push(Command::SetMoveSpeed {
                    entity_id: id,
                    speed: fixed::types::I16F16::from_num(speed),
                });
                Ok(())
            })?;
        commands_table.set("set_move_speed", set_move_speed)?;

        let cmd_buf_set_prop = Rc::clone(&cmd_buffer_rc);
        let set_property =
            scope.create_function(move |_, (id, key, value): (u64, String, String)| {
                cmd_buf_set_prop
                    .borrow_mut()
                    .push(Command::SetEntityProperty {
                        entity_id: id,
                        key,
                        value,
                    });
                Ok(())
            })?;
        commands_table.set("set_property", set_property)?;

        let cmd_buf_set_global = Rc::clone(&cmd_buffer_rc);
        let set_global = scope.create_function(move |_, (key, value): (String, String)| {
            cmd_buf_set_global
                .borrow_mut()
                .push(Command::SetGlobalProperty { key, value });
            Ok(())
        })?;
        commands_table.set("set_global", set_global)?;

        let cmd_buf_event = Rc::clone(&cmd_buffer_rc);
        let send_event =
            scope.create_function(move |_, (event_name, data): (String, String)| {
                cmd_buf_event
                    .borrow_mut()
                    .push(Command::SendEvent { event_name, data });
                Ok(())
            })?;
        commands_table.set("send_event", send_event)?;

        let cmd_buf_start = Rc::clone(&cmd_buffer_rc);
        let start_match = scope.create_function(move |_, ()| {
            cmd_buf_start.borrow_mut().push(Command::StartMatch);
            Ok(())
        })?;
        commands_table.set("start_match", start_match)?;

        let cmd_buf_pause = Rc::clone(&cmd_buffer_rc);
        let pause_match = scope.create_function(move |_, ()| {
            cmd_buf_pause.borrow_mut().push(Command::PauseMatch);
            Ok(())
        })?;
        commands_table.set("pause_match", pause_match)?;

        let cmd_buf_end = Rc::clone(&cmd_buffer_rc);
        let end_match = scope.create_function(move |_, winner_data: String| {
            cmd_buf_end
                .borrow_mut()
                .push(Command::EndMatch { winner_data });
            Ok(())
        })?;
        commands_table.set("end_match", end_match)?;

        let cmd_buf_timer = Rc::clone(&cmd_buffer_rc);
        let start_timer = scope.create_function(move |_, (timer_id, ticks): (String, u32)| {
            if ticks == 0 {
                return Err(mlua::Error::RuntimeError(
                    "Timer remaining_ticks must be strictly greater than 0".to_string(),
                ));
            }
            // Note: If a timer with the same timer_id already exists, it is silently overwritten.
            // This allows scripts to easily reset or restart active timers.
            cmd_buf_timer.borrow_mut().push(Command::StartTimer {
                timer_id,
                remaining_ticks: ticks,
            });
            Ok(())
        })?;
        commands_table.set("start_timer", start_timer)?;

        let cmd_buf_col_filter = Rc::clone(&cmd_buffer_rc);
        let set_collision_filter = scope.create_function(move |_, (id, layer, mask): (u64, u16, u16)| {
            cmd_buf_col_filter.borrow_mut().push(Command::SetCollisionFilter {
                entity_id: id,
                layer,
                mask,
            });
            Ok(())
        })?;
        commands_table.set("set_collision_filter", set_collision_filter)?;

        let cmd_buf_set_sensor = Rc::clone(&cmd_buffer_rc);
        let set_sensor = scope.create_function(move |_, (id, is_sensor): (u64, bool)| {
            cmd_buf_set_sensor.borrow_mut().push(Command::SetSensor {
                entity_id: id,
                is_sensor,
            });
            Ok(())
        })?;
        commands_table.set("set_sensor", set_sensor)?;

        let cmd_buf_set_lifetime = Rc::clone(&cmd_buffer_rc);
        let set_lifetime = scope.create_function(move |_, (id, ticks): (u64, Option<i64>)| {
            let lifetime_ticks = ticks.and_then(|t| if t > 0 { u32::try_from(t).ok() } else { None });
            cmd_buf_set_lifetime.borrow_mut().push(Command::SetLifetime {
                entity_id: id,
                lifetime_ticks,
            });
            Ok(())
        })?;
        commands_table.set("set_lifetime", set_lifetime)?;

        let cmd_buf_spawn_obs = Rc::clone(&cmd_buffer_rc);
        let spawn_static_obstacle = scope.create_function(move |_, args: mlua::Table| {
            let parsed = parse_static_obstacle_args(&args)?;
            let obstacle_id = instance.allocate_obstacle_id();
            cmd_buf_spawn_obs
                .borrow_mut()
                .push(Command::SpawnStaticObstacle {
                    obstacle_id,
                    blueprint: parsed.blueprint,
                    shape: parsed.shape,
                    filter: parsed.filter,
                    is_solid: parsed.is_solid,
                    collision_type: parsed.collision_type,
                    properties: parsed.properties,
                });
            Ok(obstacle_id)
        })?;
        commands_table.set("spawn_static_obstacle", spawn_static_obstacle)?;

        let cmd_buf_destroy_obs = Rc::clone(&cmd_buffer_rc);
        let destroy_static_obstacle = scope.create_function(move |_, obstacle_id: u64| {
            cmd_buf_destroy_obs
                .borrow_mut()
                .push(Command::DestroyStaticObstacle { obstacle_id });
            Ok(())
        })?;
        commands_table.set("destroy_static_obstacle", destroy_static_obstacle)?;

        let cmd_buf_set_obs_prop = Rc::clone(&cmd_buffer_rc);
        let set_obstacle_property =
            scope.create_function(move |_, (obstacle_id, key, value): (u64, String, String)| {
                cmd_buf_set_obs_prop
                    .borrow_mut()
                    .push(Command::SetObstacleProperty {
                        obstacle_id,
                        key,
                        value,
                    });
                Ok(())
            })?;
        commands_table.set("set_obstacle_property", set_obstacle_property)?;

        let cmd_buf_map_bounds = Rc::clone(&cmd_buffer_rc);
        let set_map_bounds =
            scope.create_function(move |_, (min_val, max_val): (mlua::Value, mlua::Value)| {
                let min = extract_vector2(min_val)?;
                let max = extract_vector2(max_val)?;
                if min.x > max.x || min.y > max.y {
                    return Err(mlua::Error::RuntimeError(
                        "set_map_bounds: min must be <= max on both axes".to_string(),
                    ));
                }
                cmd_buf_map_bounds
                    .borrow_mut()
                    .push(Command::SetMapBounds {
                        bounds: MapBounds::new(min, max),
                    });
                Ok(())
            })?;
        commands_table.set("set_map_bounds", set_map_bounds)?;

        loci_table.set("Commands", commands_table)?;

        let physics_table = lua.create_table()?;

        let raycast_fn = scope.create_function(|lua, (origin_val, dir_val, max_dist_val): (mlua::Value, mlua::Value, f64)| {
            let origin = extract_vector2(origin_val)?;
            let direction = extract_vector2(dir_val)?;
            let max_distance = fixed::types::I16F16::from_num(max_dist_val);
            
            // Note: Loci.Physics.raycast queries only static solid obstacles (walls).
            // It does not detect dynamic entities. Use get_entities_in_radius for entity queries.
            // Also note: the `direction` vector is normalized internally by the engine before casting.

            // Call the deterministic raycast from the physics engine
            if let Some(hit) = crate::world::physics::collision::fixed_raycast(
                origin, 
                direction, 
                max_distance, 
                &instance.static_obstacles
            ) {
                let hit_table = lua.create_table()?;
                hit_table.set("fraction", hit.fraction.to_num::<f64>())?;
                hit_table.set("obstacle_id", hit.obstacle_id)?;
                hit_table.set("point_x", hit.point.x.to_num::<f64>())?;
                hit_table.set("point_y", hit.point.y.to_num::<f64>())?;
                hit_table.set("normal_x", hit.normal.x.to_num::<f64>())?;
                hit_table.set("normal_y", hit.normal.y.to_num::<f64>())?;
                Ok(Some(hit_table))
            } else {
                Ok(None)
            }
        })?;
        physics_table.set("raycast", raycast_fn)?;

        let get_obstacles_in_radius = scope.create_function(|lua, (pos_val, radius): (mlua::Value, f64)| {
            if !radius.is_finite() || radius < 0.0 {
                return Err(mlua::Error::RuntimeError(
                    "get_obstacles_in_radius: radius must be finite and >= 0".to_string(),
                ));
            }
            let center = extract_vector2(pos_val)?;
            let query_shape = ColliderShape::Circle(DeterministicCircle::new(
                center,
                I16F16::from_num(radius),
            ));

            let table = lua.create_table()?;
            let mut idx = 1usize;
            for (&id, obs) in &instance.static_obstacles {
                if intersect_shapes(&query_shape, &obs.shape).is_colliding {
                    table.set(idx, id)?;
                    idx += 1;
                }
            }
            Ok(table)
        })?;
        physics_table.set("get_obstacles_in_radius", get_obstacles_in_radius)?;

        let layer_table = lua.create_table()?;
        layer_table.set("NONE", CollisionFilter::NONE)?;
        layer_table.set("SOLID_WALL", CollisionFilter::SOLID_WALL)?;
        layer_table.set("PLAYER", CollisionFilter::PLAYER)?;
        layer_table.set("TRIGGER_ZONE", CollisionFilter::TRIGGER_ZONE)?;
        layer_table.set("PROJECTILE", CollisionFilter::PROJECTILE)?;
        physics_table.set("Layer", layer_table)?;

        loci_table.set("Physics", physics_table)?;

        // Execute the user's closure
        f()
    })
}
