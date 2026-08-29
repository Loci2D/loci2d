use crate::scripting::command::{Command, CommandBuffer};
use crate::world::instance::{DeterministicVector2, Instance};
use mlua::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

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
            println!("[Lua] INFO: {}", msg);
            Ok(())
        })?,
    )?;
    log_table.set(
        "warn",
        lua.create_function(|_, msg: String| {
            eprintln!("[Lua] WARN: {}", msg);
            Ok(())
        })?,
    )?;
    log_table.set(
        "error",
        lua.create_function(|_, msg: String| {
            eprintln!("[Lua] ERROR: {}", msg);
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

        // Loci.get_entity_property(id, key)
        let get_entity_property = scope.create_function(|_, (id, key): (u64, String)| {
            if let Some(entity) = instance.get_entity(id) {
                Ok(entity.properties.get(&key).cloned())
            } else {
                Ok(None)
            }
        })?;
        loci_table.set("get_entity_property", get_entity_property)?;

        // Loci.get_global(key)
        let get_global =
            scope.create_function(|_, key: String| Ok(instance.globals.get(&key).cloned()))?;
        loci_table.set("get_global", get_global)?;

        // Loci.Commands
        let commands_table = lua.create_table()?;

        let cmd_buf_spawn = Rc::clone(&cmd_buffer_rc);
        let spawn_entity = scope.create_function(move |_, args: mlua::Table| {
            let blueprint: String = args.get("blueprint")?;
            let position_ud: mlua::AnyUserData = args.get("position")?;
            let position = *position_ud.borrow::<DeterministicVector2>()?;

            cmd_buf_spawn.borrow_mut().push(Command::SpawnEntity {
                blueprint,
                position,
            });
            Ok(())
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
            scope.create_function(move |_, (id, position_ud): (u64, mlua::AnyUserData)| {
                let position = *position_ud.borrow::<DeterministicVector2>()?;
                cmd_buf_set_pos.borrow_mut().push(Command::SetPosition {
                    entity_id: id,
                    position,
                });
                Ok(())
            })?;
        commands_table.set("set_position", set_position)?;

        let cmd_buf_set_vel = Rc::clone(&cmd_buffer_rc);
        let set_velocity =
            scope.create_function(move |_, (id, velocity_ud): (u64, mlua::AnyUserData)| {
                let velocity = *velocity_ud.borrow::<DeterministicVector2>()?;
                cmd_buf_set_vel.borrow_mut().push(Command::SetVelocity {
                    entity_id: id,
                    velocity,
                });
                Ok(())
            })?;
        commands_table.set("set_velocity", set_velocity)?;

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

        loci_table.set("Commands", commands_table)?;

        // Execute the user's closure
        f()
    })
}
