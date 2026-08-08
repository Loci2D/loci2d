// Session module - Client connection lifecycle, heartbeats, and identity mapping

use std::net::SocketAddr;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    Active,
    TimedOut,
    Disconnected,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ClientSession {
    pub session_id: u64,
    pub addr: SocketAddr,
    pub entity_id: u64,
    pub player_name: String,
    pub connected_at: Instant,
    pub last_seen: Instant,
    pub state: SessionState,
}

impl ClientSession {
    pub fn new(session_id: u64, addr: SocketAddr, entity_id: u64, player_name: String) -> Self {
        let now = Instant::now();
        Self {
            session_id,
            addr,
            entity_id,
            player_name,
            connected_at: now,
            last_seen: now,
            state: SessionState::Active,
        }
    }

    /// Update the heartbeat timestamp when any valid packet is received.
    pub fn refresh_activity(&mut self) {
        self.last_seen = Instant::now();
    }

    /// Check if the session has exceeded the timeout threshold.
    pub fn is_timed_out(&self, timeout_secs: u64) -> bool {
        self.last_seen.elapsed().as_secs() >= timeout_secs
    }
}
