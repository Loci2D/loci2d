/// Configuration loaded once at startup.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: String,
    pub tick_rate: u32,
}

impl ServerConfig {
    pub fn from_env() -> Self {
        // dotenvy::dotenv() is a no-op if .env does not exist — safe in production
        let _ = dotenvy::dotenv();

        Self {
            bind_addr: std::env::var("BIND_ADDR")
                .unwrap_or_else(|_| "127.0.0.1:8080".to_string()),
            tick_rate: std::env::var("TICK_RATE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
        }
    }
}
