/// Configuration loaded once at startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerConfig {
    pub bind_addr: String,
    pub tick_rate: u32,
    pub client_timeout_secs: u64,
    pub max_spectators: usize,
    pub scripts_dir: String,
    pub script_path: Option<String>,
}

impl ServerConfig {
    pub fn from_env() -> Self {
        // dotenvy::dotenv() is a no-op if .env does not exist — safe in production
        let _ = dotenvy::dotenv();

        Self {
            bind_addr: std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string()),
            tick_rate: std::env::var("TICK_RATE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
            client_timeout_secs: std::env::var("CLIENT_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            max_spectators: std::env::var("MAX_SPECTATORS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(128),
            scripts_dir: std::env::var("SCRIPTS_DIR").unwrap_or_else(|_| "scripts".to_string()),
            script_path: std::env::var("SCRIPT_PATH")
                .ok()
                .filter(|s| !s.trim().is_empty()),
        }
    }

    /// Resolves the authoritative Lua script path for a given map name.
    ///
    /// If `script_path` is explicitly configured, it is returned directly.
    /// Otherwise, it resolves to `{scripts_dir}/{map_name}/main.lua` (defaulting to `scripts/{map_name}/main.lua`).
    pub fn resolve_script_path(&self, map_name: &str) -> String {
        if let Some(ref path) = self.script_path {
            path.clone()
        } else {
            format!("{}/{}/main.lua", self.scripts_dir, map_name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_script_path_default() {
        let cfg = ServerConfig {
            bind_addr: "127.0.0.1:8080".to_string(),
            tick_rate: 30,
            client_timeout_secs: 10,
            max_spectators: 128,
            scripts_dir: "scripts".to_string(),
            script_path: None,
        };

        assert_eq!(
            cfg.resolve_script_path("default_arena"),
            "scripts/default_arena/main.lua"
        );
        assert_eq!(
            cfg.resolve_script_path("custom_map"),
            "scripts/custom_map/main.lua"
        );
    }

    #[test]
    fn test_resolve_script_path_custom_dir() {
        let cfg = ServerConfig {
            bind_addr: "127.0.0.1:8080".to_string(),
            tick_rate: 30,
            client_timeout_secs: 10,
            max_spectators: 128,
            scripts_dir: "server/scripts".to_string(),
            script_path: None,
        };

        assert_eq!(
            cfg.resolve_script_path("default_arena"),
            "server/scripts/default_arena/main.lua"
        );
    }

    #[test]
    fn test_resolve_script_path_explicit_override() {
        let cfg = ServerConfig {
            bind_addr: "127.0.0.1:8080".to_string(),
            tick_rate: 30,
            client_timeout_secs: 10,
            max_spectators: 128,
            scripts_dir: "scripts".to_string(),
            script_path: Some("/custom/path/rules.lua".to_string()),
        };

        assert_eq!(
            cfg.resolve_script_path("default_arena"),
            "/custom/path/rules.lua"
        );
    }
}

