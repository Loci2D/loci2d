use crate::world::instance::{DeterministicVector2, Instance};
use crate::scripting::command::{Command, CommandBuffer};
use mlua::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// Sets up the base Loci global API which doesn't require an active Instance context.
/// This includes the Loci.Vector2 constructor and basic logging.
pub fn setup_base_api(lua: &Lua) -> LuaResult<()> {
    let globals = lua.globals();

    let loci_table = lua.create_table()?;

    // Loci.Vector2(x, y)
    let vector2_fn = lua.create_function(|_, (x, y): (f64, f64)| {
        Ok(DeterministicVector2::from_f32(x as f32, y as f32))
    })?;
    loci_table.set("Vector2", vector2_fn)?;

    // Setup Loci.Log for scripts
    let log_table = lua.create_table()?;
    log_table.set("info", lua.create_function(|_, msg: String| {
        println!("[Lua] INFO: {}", msg);
        Ok(())
    })?)?;
    log_table.set("warn", lua.create_function(|_, msg: String| {
        eprintln!("[Lua] WARN: {}", msg);
        Ok(())
    })?)?;
    log_table.set("error", lua.create_function(|_, msg: String| {
        eprintln!("[Lua] ERROR: {}", msg);
        Ok(())
    })?)?;
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
            let entity_id = instance.entities.values().find(|e| e.name == name).map(|e| e.id);
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

        loci_table.set("Commands", commands_table)?;

        // Execute the user's closure
        let result = f();

        // Clean up scoped functions so they can't be used outside this execution
        loci_table.set("get_entity_by_name", mlua::Value::Nil)?;
        loci_table.set("get_entity_position", mlua::Value::Nil)?;
        loci_table.set("Commands", mlua::Value::Nil)?;

        result
    })
}
