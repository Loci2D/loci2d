use mlua::prelude::*;
use mlua::VmState;
use std::fmt;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Default maximum Lua instructions per callback before interrupting/aborting (DoS protection).
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

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new().expect("Failed to initialize ScriptEngine")
    }
}

impl ScriptEngine {
    /// Creates a new sandboxed `ScriptEngine`.
    /// Excludes dangerous standard libraries (`io`, `os`, `package`, `debug`).
    /// Configures execution instruction limits for DoS protection.
    pub fn new() -> LuaResult<Self> {
        Self::new_with_instruction_limit(DEFAULT_MAX_LUA_INSTRUCTIONS)
    }

    /// Creates a sandboxed `ScriptEngine` with a custom instruction limit for DoS protection.
    pub fn new_with_instruction_limit(max_instructions: u64) -> LuaResult<Self> {
        // Load only safe standard libraries. EXCLUDE `io`, `os`, `package`, and `debug`.
        let std_libs = mlua::StdLib::TABLE | mlua::StdLib::STRING | mlua::StdLib::MATH;
        let lua = Lua::new_with(std_libs, mlua::LuaOptions::default())?;

        let engine = Self {
            lua,
            max_instructions,
            instruction_counter: Arc::new(AtomicU64::new(0)),
        };

        engine.setup_sandbox_hooks();
        Ok(engine)
    }

    /// Sets up the instruction count hook for DoS / infinite loop protection.
    fn setup_sandbox_hooks(&self) {
        if self.max_instructions > 0 {
            let max_inst = self.max_instructions;
            let counter = Arc::clone(&self.instruction_counter);

            let mut triggers = mlua::HookTriggers::default();
            triggers.every_nth_instruction = Some(100);

            self.lua.set_hook(triggers, move |_, _| {
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
}
