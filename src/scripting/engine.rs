use mlua::prelude::*;
use mlua::VmState;
use std::fmt;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use crate::scripting::api::{setup_base_api, with_scoped_api};
use crate::scripting::command::CommandBuffer;
use crate::world::instance::Instance;

/// Default maximum Lua instructions per callback before interrupting/aborting (DoS protection).
// TODO(Phase 6.5): Calibrate against real tick budget measurements (tick_rate × budget_µs).
pub const DEFAULT_MAX_LUA_INSTRUCTIONS: u64 = 100_000;

pub struct ScriptEngine {
    lua: Lua,
    max_instructions: u64,
    instruction_counter: Arc<AtomicU64>,
}

impl fmt::Debug for ScriptEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScriptEngine")
            .field("max_instructions", &self.max_instructions)
            .finish_non_exhaustive()
    }
}



impl ScriptEngine {
    /// Creates a new sandboxed `ScriptEngine`.
    /// Excludes dangerous standard libraries (`io`, `os`, `package`, `debug`).
    /// Configures execution instruction limits for DoS protection.
    pub fn new(seed: u64) -> LuaResult<Self> {
        Self::new_with_instruction_limit(DEFAULT_MAX_LUA_INSTRUCTIONS, seed)
    }

    /// Creates a sandboxed `ScriptEngine` with a custom instruction limit for DoS protection.
    pub fn new_with_instruction_limit(max_instructions: u64, seed: u64) -> LuaResult<Self> {
        // Load only safe standard libraries. EXCLUDE `io`, `os`, `package`, and `debug`.
        let std_libs = mlua::StdLib::TABLE | mlua::StdLib::STRING | mlua::StdLib::MATH;
        let lua = Lua::new_with(std_libs, mlua::LuaOptions::default())?;

        let engine = Self {
            lua,
            max_instructions,
            instruction_counter: Arc::new(AtomicU64::new(0)),
        };

        engine.setup_sandbox_hooks();
        engine.setup_determinism(seed)?;
        setup_base_api(&engine.lua)?;
        Ok(engine)
    }

    /// Overrides non-deterministic elements (PRNG, pairs) to enforce reproducibility.
    fn setup_determinism(&self, seed: u64) -> LuaResult<()> {
        let globals = self.lua.globals();

        // 1. Remove `pairs` to force users to use `ipairs` or deterministic iteration
        globals.set("pairs", mlua::Value::Nil)?;

        // 2. Overwrite `math.random` with a deterministic LCG PRNG
        let math_table: mlua::Table = globals.get("math")?;
        
        let prng_state = Arc::new(AtomicU64::new(seed));
        let random_fn = self.lua.create_function(move |_, (min, max): (Option<i32>, Option<i32>)| {
            // Knuth multiplicative LCG: fast, portable, sufficient quality for game scripting.
            // Not cryptographically secure, but deterministic cross-platform by construction.
            let mut state = prng_state.load(Ordering::Relaxed);
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            prng_state.store(state, Ordering::Relaxed);
            
            let rand_val = (state >> 32) as u32;

            match (min, max) {
                (None, None) => {
                    Ok(mlua::Value::Number((rand_val as f64) / (std::u32::MAX as f64 + 1.0)))
                }
                (Some(m), None) => {
                    if m < 1 {
                        return Err(mlua::Error::RuntimeError("bad argument #1 to 'random' (interval is empty)".to_string()));
                    }
                    // Note: modulo bias exists for non-power-of-2 ranges. Acceptable for game scripting.
                    let res = 1 + (rand_val % (m as u32)) as i32;
                    Ok(mlua::Value::Integer(res as i64))
                }
                (Some(m), Some(n)) => {
                    if m > n {
                        return Err(mlua::Error::RuntimeError("bad argument #2 to 'random' (interval is empty)".to_string()));
                    }
                    let range = (n as u32).wrapping_sub(m as u32).wrapping_add(1);
                    let res = m.wrapping_add((rand_val % range) as i32);
                    Ok(mlua::Value::Integer(res as i64))
                }
                _ => Err(mlua::Error::RuntimeError("invalid arguments to 'random'".to_string())),
            }
        })?;

        math_table.set("random", random_fn)?;
        
        // Neutralize `math.randomseed` so users don't get confused
        math_table.set("randomseed", self.lua.create_function(|_, _: mlua::Value| {
            // Deterministic PRNG is seeded by the Instance at initialization.
            // Calling math.randomseed() from Lua has no effect.
            Ok(())
        })?)?;

        Ok(())
    }

    /// Sets up the instruction count hook for DoS / infinite loop protection.
    fn setup_sandbox_hooks(&self) {
        if self.max_instructions > 0 {
            let max_inst = self.max_instructions;
            let counter = Arc::clone(&self.instruction_counter);

            let mut triggers = mlua::HookTriggers::default();
            triggers.every_nth_instruction = Some(100);

            self.lua.set_hook(triggers, move |_, _| {
                // Single-threaded VM; Relaxed ordering is sufficient.
                let current = counter.fetch_add(100, Ordering::Relaxed);
                if current >= max_inst {
                    Err(LuaError::RuntimeError(format!(
                        "Execution limit exceeded: script exceeded maximum allowed instruction limit ({max_inst})"
                    )))
                } else {
                    Ok(VmState::Continue)
                }
            });
        }
    }

    /// Resets the instruction counter for the DoS hook to zero before executing a script callback.
    pub fn reset_instruction_counter(&self) {
        self.instruction_counter.store(0, Ordering::Relaxed);
    }

    /// Loads and evaluates a script from a raw string.
    /// NOTE: This does not reset Lua globals. Should be called only once at Instance init.
    /// Full VM resetting for hot-reloading is deferred to Phase 6.4/9.
    pub fn load_script(&self, script: &str) -> Result<(), String> {
        self.reset_instruction_counter();
        self.lua
            .load(script)
            .exec()
            .map_err(|e| format!("Lua script execution error: {e}"))
    }

    /// Reads a file from local host path and evaluates its contents.
    pub fn load_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let path_ref = path.as_ref();
        let content = std::fs::read_to_string(path_ref)
            .map_err(|e| format!("Failed to read script file '{}': {e}", path_ref.display()))?;
        self.load_script(&content)
    }

    /// Accessor for underlying Lua VM instance.
    pub fn lua(&self) -> &Lua {
        &self.lua
    }

    // --- Lifecycle Event Hooks ---

    pub fn on_init(&self, instance: &Instance, cmd_buffer: &mut CommandBuffer) -> LuaResult<()> {
        self.reset_instruction_counter();
        with_scoped_api(&self.lua, instance, cmd_buffer, || {
            let globals = self.lua.globals();
            if let Ok(on_init_fn) = globals.get::<mlua::Function>("on_init") {
                on_init_fn.call::<()>(())?;
            }
            Ok(())
        })
    }

    pub fn on_tick(&self, instance: &Instance, current_tick: u64, cmd_buffer: &mut CommandBuffer) -> LuaResult<()> {
        self.reset_instruction_counter();
        with_scoped_api(&self.lua, instance, cmd_buffer, || {
            let globals = self.lua.globals();
            if let Ok(on_tick_fn) = globals.get::<mlua::Function>("on_tick") {
                on_tick_fn.call::<()>(current_tick)?;
            }
            Ok(())
        })
    }

    pub fn on_player_join(&self, instance: &Instance, entity_id: u64, cmd_buffer: &mut CommandBuffer) -> LuaResult<()> {
        self.reset_instruction_counter();
        with_scoped_api(&self.lua, instance, cmd_buffer, || {
            let globals = self.lua.globals();
            if let Ok(on_join_fn) = globals.get::<mlua::Function>("on_player_join") {
                on_join_fn.call::<()>(entity_id)?;
            }
            Ok(())
        })
    }

    pub fn on_player_leave(&self, instance: &Instance, entity_id: u64, cmd_buffer: &mut CommandBuffer) -> LuaResult<()> {
        self.reset_instruction_counter();
        with_scoped_api(&self.lua, instance, cmd_buffer, || {
            let globals = self.lua.globals();
            if let Ok(on_leave_fn) = globals.get::<mlua::Function>("on_player_leave") {
                on_leave_fn.call::<()>(entity_id)?;
            }
            Ok(())
        })
    }

    pub fn on_collision(&self, instance: &Instance, entity_a: u64, entity_b: u64, cmd_buffer: &mut CommandBuffer) -> LuaResult<()> {
        self.reset_instruction_counter();
        with_scoped_api(&self.lua, instance, cmd_buffer, || {
            let globals = self.lua.globals();
            if let Ok(on_col_fn) = globals.get::<mlua::Function>("on_collision") {
                on_col_fn.call::<()>((entity_a, entity_b))?;
            }
            Ok(())
        })
    }

    pub fn on_trigger_enter(&self, instance: &Instance, entity_id: u64, trigger_id: u64, cmd_buffer: &mut CommandBuffer) -> LuaResult<()> {
        self.reset_instruction_counter();
        with_scoped_api(&self.lua, instance, cmd_buffer, || {
            let globals = self.lua.globals();
            if let Ok(on_enter_fn) = globals.get::<mlua::Function>("on_trigger_enter") {
                on_enter_fn.call::<()>((entity_id, trigger_id))?;
            }
            Ok(())
        })
    }

    pub fn on_trigger_stay(&self, instance: &Instance, entity_id: u64, trigger_id: u64, cmd_buffer: &mut CommandBuffer) -> LuaResult<()> {
        self.reset_instruction_counter();
        with_scoped_api(&self.lua, instance, cmd_buffer, || {
            let globals = self.lua.globals();
            if let Ok(on_stay_fn) = globals.get::<mlua::Function>("on_trigger_stay") {
                on_stay_fn.call::<()>((entity_id, trigger_id))?;
            }
            Ok(())
        })
    }

    pub fn on_trigger_exit(&self, instance: &Instance, entity_id: u64, trigger_id: u64, cmd_buffer: &mut CommandBuffer) -> LuaResult<()> {
        self.reset_instruction_counter();
        with_scoped_api(&self.lua, instance, cmd_buffer, || {
            let globals = self.lua.globals();
            if let Ok(on_exit_fn) = globals.get::<mlua::Function>("on_trigger_exit") {
                on_exit_fn.call::<()>((entity_id, trigger_id))?;
            }
            Ok(())
        })
    }
}
