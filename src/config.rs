/// Configuration loaded once at startup.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: String,
    pub tick_rate: u32,
    pub client_timeout_secs: u64,
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
            client_timeout_secs: std::env::var("CLIENT_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
        }
    }
}

