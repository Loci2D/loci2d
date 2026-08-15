// Instance module - Logic for a specific room/instance (tick rate, entity list, session mapping)

use std::collections::BTreeMap;
use std::net::SocketAddr;
use super::entity::{Entity, EntityType};
use super::session::{ClientSession, SessionState};
use crate::network::packets::{
    ClientIntent, EntityState, ReplayIntentEntry, WorldState, EntityType as ProtoEntityType,
};

// Re-export Vector2 from network and DeterministicVector2 from fixed_point
pub use crate::network::packets::Vector2;
pub use super::fixed_point::DeterministicVector2;

// [2026-08-08] Allowed dead_code: fields like id and tick_rate are essential metadata for multi-room management (Phase 5).
#[allow(dead_code)]
#[derive(Debug)]
pub struct Instance {
    pub id: u64,
    pub entities: BTreeMap<u64, Entity>,
    pub sessions: BTreeMap<SocketAddr, ClientSession>,
    pub entity_to_addr: BTreeMap<u64, SocketAddr>,
    pub tick_rate: u32, // ticks per second
    pub client_timeout_secs: u64,
    next_entity_id: u64,
    next_session_id: u64,
}

impl Instance {
    pub fn new(id: u64, tick_rate: u32, client_timeout_secs: u64) -> Self {
        Self {
            id,
            entities: BTreeMap::new(),
            sessions: BTreeMap::new(),
            entity_to_addr: BTreeMap::new(),
            tick_rate,
            client_timeout_secs,
            next_entity_id: 1,
            next_session_id: 1,
        }
    }

    /// Handles an incoming client intent and returns an optional replay entry for match logging.
    pub fn apply_intent(&mut self, addr: SocketAddr, intent: ClientIntent) -> Option<ReplayIntentEntry> {
        use crate::network::packets::client_intent::Intent;

        let inner_intent = intent.intent.as_ref()?;

        match inner_intent {
            // 1. Explicit Join Handshake
            Intent::Join(join_intent) => {
                let player_name = if join_intent.player_name.trim().is_empty() {
                    format!("Player_{}", self.next_entity_id)
                } else {
                    join_intent.player_name.clone()
                };
                let entity_id = self.handle_join(addr, player_name.clone());
                Some(ReplayIntentEntry {
                    entity_id,
                    player_name,
                    intent: Some(intent),
                })
            }

            // 2. Explicit Disconnect
            Intent::Disconnect(disconnect_intent) => {
                let session = self.sessions.get(&addr)?;
                let entity_id = session.entity_id;
                let player_name = session.player_name.clone();
                self.handle_disconnect(addr, &disconnect_intent.reason);
                Some(ReplayIntentEntry {
                    entity_id,
                    player_name,
                    intent: Some(intent),
                })
            }

            // 3. Movement Intent (requires active session)
            Intent::Move(move_intent) => {
                let session = self.sessions.get_mut(&addr)?;
                session.refresh_activity();
                let entity_id = session.entity_id;
                if let (Some(dir), Some(entity)) = (move_intent.direction, self.entities.get_mut(&entity_id)) {
                    entity.velocity = DeterministicVector2::from_proto(&dir);
                }
                Some(ReplayIntentEntry {
                    entity_id,
                    player_name: String::new(),
                    intent: Some(intent),
                })
            }

            // 4. Target Movement Intent (Phase 6 click-to-move reserved)
            Intent::MoveToPos(move_to_pos_intent) => {
                let session = self.sessions.get_mut(&addr)?;
                session.refresh_activity();
                let entity_id = session.entity_id;
                if let Some(target) = move_to_pos_intent.target_position {
                    let target_vec = DeterministicVector2::from_proto(&target);
                    let (tx, ty) = target_vec.to_f32();
                    println!("[Intent] Entity {} ({}) requested move to target ({:.1}, {:.1})",
                        session.entity_id, session.player_name, tx, ty);
                }
                Some(ReplayIntentEntry {
                    entity_id,
                    player_name: String::new(),
                    intent: Some(intent),
                })
            }

            // 5. Action Intent (requires active session)
            Intent::Action(action_intent) => {
                let session = self.sessions.get_mut(&addr)?;
                session.refresh_activity();
                let entity_id = session.entity_id;
                if let Some(entity) = self.entities.get_mut(&entity_id) {
                    println!("[Intent] Entity {} ({}) executed action {}", entity_id, entity.name, action_intent.ability_id);
                }
                Some(ReplayIntentEntry {
                    entity_id,
                    player_name: String::new(),
                    intent: Some(intent),
                })
            }

            // 6. Ping / Heartbeat Intent (requires active session)
            Intent::Ping(_) => {
                let session = self.sessions.get_mut(&addr)?;
                session.refresh_activity();
                println!("[Intent] Entity {} ({}) sent ping", session.entity_id, session.player_name);
                // Pings are heartbeats and do not mutate simulation state
                None
            }
        }
    }

    /// Explicit client join
    pub fn handle_join(&mut self, addr: SocketAddr, player_name: String) -> u64 {
        if let Some(session) = self.sessions.get_mut(&addr) {
            session.player_name = player_name.clone();
            session.refresh_activity();
            if let Some(entity) = self.entities.get_mut(&session.entity_id) {
                entity.name = player_name;
            }
            println!("[Session] Client {} re-joined as '{}' (EntityId {})", addr, session.player_name, session.entity_id);
            return session.entity_id;
        }

        let entity_id = self.next_entity_id;
        self.next_entity_id += 1;

        let session_id = self.next_session_id;
        self.next_session_id += 1;

        let session = ClientSession::new(session_id, addr, entity_id, player_name.clone());
        let entity = Entity::new(entity_id, player_name.clone(), EntityType::Player);

        self.entities.insert(entity_id, entity);
        self.sessions.insert(addr, session);
        self.entity_to_addr.insert(entity_id, addr);

        println!("[Join] Client {} joined as '{}' (SessionId {}, EntityId {})", addr, player_name, session_id, entity_id);
        entity_id
    }

    /// Explicit client disconnect
    pub fn handle_disconnect(&mut self, addr: SocketAddr, reason: &str) {
        if let Some(mut session) = self.sessions.remove(&addr) {
            session.state = SessionState::Disconnected;
            self.entity_to_addr.remove(&session.entity_id);
            self.entities.remove(&session.entity_id);
            let display_reason = if reason.trim().is_empty() { "normal quit" } else { reason };
            println!("[Disconnect] Client {} ('{}', EntityId {}) disconnected gracefully. Reason: '{}'", 
                addr, session.player_name, session.entity_id, display_reason);
        }
    }

    /// Advance physics using deterministic fixed-point addition and sweep for timed-out sessions.
    /// Returns a list of (entity_id, player_name) for any sessions that timed out during this tick.
    pub fn tick(&mut self, tick_count: u64) -> Vec<(u64, String)> {
        for entity in self.entities.values_mut() {
            entity.position = entity.position.saturating_add(entity.velocity);
        }

        // Check for timed out clients
        let timed_out = self.check_timeouts();

        println!("[Tick {}] {} active entities, {} active sessions", tick_count, self.entities.len(), self.sessions.len());
        timed_out
    }

    /// Sweep and remove inactive sessions, returning the removed (entity_id, player_name) pairs.
    pub fn check_timeouts(&mut self) -> Vec<(u64, String)> {
        let timeout_secs = self.client_timeout_secs;
        let mut timed_out_addrs = Vec::new();

        for (addr, session) in &self.sessions {
            if session.is_timed_out(timeout_secs) {
                timed_out_addrs.push((*addr, session.entity_id, session.player_name.clone()));
            }
        }

        let mut timed_out_entities = Vec::new();
        for (addr, entity_id, player_name) in timed_out_addrs {
            self.sessions.remove(&addr);
            self.entity_to_addr.remove(&entity_id);
            self.entities.remove(&entity_id);
            timed_out_entities.push((entity_id, player_name.clone()));
            println!("[Timeout] Client {} ('{}', EntityId {}) timed out after {}s of inactivity", addr, player_name, entity_id, timeout_secs);
        }
        timed_out_entities
    }

    // [2026-08-08] Allowed dead_code: entity/session lifecycle helper methods for upcoming phases.
    #[allow(dead_code)]
    pub fn add_entity(&mut self, entity: Entity) {
        self.entities.insert(entity.id, entity);
    }

    #[allow(dead_code)]
    pub fn remove_entity(&mut self, entity_id: u64) -> Option<Entity> {
        self.entities.remove(&entity_id)
    }

    pub fn get_entity(&self, entity_id: u64) -> Option<&Entity> {
        self.entities.get(&entity_id)
    }

    #[allow(dead_code)]
    pub fn get_session(&self, addr: &SocketAddr) -> Option<&ClientSession> {
        self.sessions.get(addr)
    }

    /// Generates a complete WorldState snapshot representing all active entities.
    pub fn create_snapshot(&self, tick: u64) -> WorldState {
        let entities = self.entities.values().map(|e| {
            EntityState {
                id: e.id,
                name: e.name.clone(),
                position: Some(e.position.to_proto()),
                velocity: Some(e.velocity.to_proto()),
                entity_type: match e.entity_type {
                    EntityType::Player => ProtoEntityType::Player as i32,
                    EntityType::NPC => ProtoEntityType::Npc as i32,
                    EntityType::Prop => ProtoEntityType::Prop as i32,
                },
            }
        }).collect();

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        WorldState {
            tick,
            timestamp,
            entities,
        }
    }

    /// Returns a list of all active client destination addresses for broadcasting.
    pub fn get_broadcast_addresses(&self) -> Vec<SocketAddr> {
        self.sessions.keys().copied().collect()
    }

    /// Applies a recorded replay intent entry directly by entity_id without requiring network sockets.
    pub fn apply_replay_entry(&mut self, entry: &ReplayIntentEntry) {
        use crate::network::packets::client_intent::Intent;

        let Some(ClientIntent { intent: Some(ref inner_intent) }) = entry.intent else { return; };

        match inner_intent {
            Intent::Join(join_intent) => {
                let player_name = if join_intent.player_name.trim().is_empty() {
                    entry.player_name.clone()
                } else {
                    join_intent.player_name.clone()
                };
                if let Some(existing) = self.entities.get_mut(&entry.entity_id) {
                    existing.name = player_name;
                } else {
                    let entity = Entity::new(entry.entity_id, player_name, EntityType::Player);
                    self.entities.insert(entry.entity_id, entity);
                }
            }
            Intent::Disconnect(_) => {
                self.entities.remove(&entry.entity_id);
            }
            Intent::Move(move_intent) => {
                if let (Some(dir), Some(entity)) = (move_intent.direction, self.entities.get_mut(&entry.entity_id)) {
                    entity.velocity = DeterministicVector2::from_proto(&dir);
                }
            }
            Intent::MoveToPos(_) => {}
            Intent::Action(action_intent) => {
                if let Some(entity) = self.entities.get_mut(&entry.entity_id) {
                    println!("[Replay] Entity {} ({}) executed action {}", entry.entity_id, entity.name, action_intent.ability_id);
                }
            }
            Intent::Ping(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::packets::{
        client_intent, ActionIntent, ClientIntent, DisconnectIntent, JoinIntent, MoveIntent, PingIntent,
    };

    #[test]
    fn test_explicit_join_and_move_intent() {
        let mut instance = Instance::new(1, 30, 10);
        let addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();

        // 1. Explicit Join
        let join_intent = ClientIntent {
            intent: Some(client_intent::Intent::Join(JoinIntent {
                player_name: "Alice".to_string(),
            })),
        };
        instance.apply_intent(addr, join_intent);

        assert_eq!(instance.sessions.len(), 1);
        assert_eq!(instance.entities.len(), 1);

        let entity_id = {
            let session = instance.sessions.get(&addr).expect("Session should exist");
            assert_eq!(session.player_name, "Alice");
            assert_eq!(session.state, SessionState::Active);
            session.entity_id
        };

        let entity = instance.get_entity(entity_id).expect("Entity should exist");
        assert_eq!(entity.name, "Alice");
        assert_eq!(entity.position, DeterministicVector2::ZERO);

        // 2. Move Intent
        let move_intent = ClientIntent {
            intent: Some(client_intent::Intent::Move(MoveIntent {
                direction: Some(DeterministicVector2::from_f32(2.5, -1.0).to_proto()),
            })),
        };
        instance.apply_intent(addr, move_intent);

        let entity = instance.get_entity(entity_id).unwrap();
        assert_eq!(entity.velocity.to_f32(), (2.5, -1.0));

        // 3. Tick
        instance.tick(1);

        let updated_entity = instance.get_entity(entity_id).unwrap();
        assert_eq!(updated_entity.position.to_f32(), (2.5, -1.0));
    }

    #[test]
    fn test_explicit_disconnect() {
        let mut instance = Instance::new(1, 30, 10);
        let addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();

        // Join
        let join_intent = ClientIntent {
            intent: Some(client_intent::Intent::Join(JoinIntent {
                player_name: "Bob".to_string(),
            })),
        };
        instance.apply_intent(addr, join_intent);
        assert_eq!(instance.sessions.len(), 1);
        assert_eq!(instance.entities.len(), 1);

        // Disconnect
        let disconnect_intent = ClientIntent {
            intent: Some(client_intent::Intent::Disconnect(DisconnectIntent {
                reason: "Leaving match".to_string(),
            })),
        };
        instance.apply_intent(addr, disconnect_intent);

        assert_eq!(instance.sessions.len(), 0);
        assert_eq!(instance.entities.len(), 0);
        assert_eq!(instance.entity_to_addr.len(), 0);
    }

    #[test]
    fn test_unjoined_client_intents_dropped() {
        let mut instance = Instance::new(1, 30, 10);
        let addr: SocketAddr = "127.0.0.1:34567".parse().unwrap();

        // Send move intent without prior join — should be dropped
        let move_intent = ClientIntent {
            intent: Some(client_intent::Intent::Move(MoveIntent {
                direction: Some(DeterministicVector2::from_f32(1.0, 1.0).to_proto()),
            })),
        };
        instance.apply_intent(addr, move_intent);

        assert_eq!(instance.sessions.len(), 0);
        assert_eq!(instance.entities.len(), 0);

        // Send action intent without prior join — should be dropped
        let action_intent = ClientIntent {
            intent: Some(client_intent::Intent::Action(ActionIntent { ability_id: 1 })),
        };
        instance.apply_intent(addr, action_intent);
        assert_eq!(instance.sessions.len(), 0);
        assert_eq!(instance.entities.len(), 0);

        // Send ping intent without prior join — should be dropped
        let ping_intent = ClientIntent {
            intent: Some(client_intent::Intent::Ping(PingIntent {})),
        };
        instance.apply_intent(addr, ping_intent);
        assert_eq!(instance.sessions.len(), 0);
        assert_eq!(instance.entities.len(), 0);
    }

    #[test]
    fn test_timeout_detection() {
        let mut instance = Instance::new(1, 30, 0); // 0-second timeout for immediate expiry
        let addr: SocketAddr = "127.0.0.1:45678".parse().unwrap();

        let join_intent = ClientIntent {
            intent: Some(client_intent::Intent::Join(JoinIntent {
                player_name: "Charlie".to_string(),
            })),
        };
        instance.apply_intent(addr, join_intent);
        assert_eq!(instance.sessions.len(), 1);
        assert_eq!(instance.entities.len(), 1);

        // Advance tick, should trigger check_timeouts and clean up
        instance.tick(1);

        assert_eq!(instance.sessions.len(), 0);
        assert_eq!(instance.entities.len(), 0);
        assert_eq!(instance.entity_to_addr.len(), 0);
    }

    #[test]
    fn test_ping_and_action_intents() {
        let mut instance = Instance::new(1, 30, 10);
        let addr: SocketAddr = "127.0.0.1:56789".parse().unwrap();

        let join_intent = ClientIntent {
            intent: Some(client_intent::Intent::Join(JoinIntent {
                player_name: "Dave".to_string(),
            })),
        };
        instance.apply_intent(addr, join_intent);
        assert_eq!(instance.sessions.len(), 1);

        let ping_intent = ClientIntent {
            intent: Some(client_intent::Intent::Ping(PingIntent {})),
        };
        instance.apply_intent(addr, ping_intent);
        assert_eq!(instance.sessions.len(), 1);

        let action_intent = ClientIntent {
            intent: Some(client_intent::Intent::Action(ActionIntent { ability_id: 42 })),
        };
        instance.apply_intent(addr, action_intent);
        assert_eq!(instance.sessions.len(), 1);
    }

    #[test]
    fn test_rejoin_updates_player_name() {
        let mut instance = Instance::new(1, 30, 10);
        let addr: SocketAddr = "127.0.0.1:60001".parse().unwrap();

        // Initial join
        instance.handle_join(addr, "InitialName".to_string());
        {
            let session = instance.sessions.get(&addr).unwrap();
            let entity = instance.get_entity(session.entity_id).unwrap();
            assert_eq!(entity.name, "InitialName");
        }

        // Rejoin with new name
        instance.handle_join(addr, "NewName".to_string());
        {
            let session = instance.sessions.get(&addr).unwrap();
            let entity = instance.get_entity(session.entity_id).unwrap();
            assert_eq!(entity.name, "NewName");
            assert_eq!(session.player_name, "NewName");
        }
    }

    #[test]
    fn test_create_snapshot_and_broadcast_addresses() {
        let mut instance = Instance::new(1, 30, 10);
        let addr1: SocketAddr = "127.0.0.1:50001".parse().unwrap();
        let addr2: SocketAddr = "127.0.0.1:50002".parse().unwrap();

        instance.handle_join(addr1, "Alice".to_string());
        instance.handle_join(addr2, "Bob".to_string());

        let addrs = instance.get_broadcast_addresses();
        assert_eq!(addrs.len(), 2);
        assert!(addrs.contains(&addr1));
        assert!(addrs.contains(&addr2));

        let snapshot = instance.create_snapshot(42);
        assert_eq!(snapshot.tick, 42);
        assert!(snapshot.timestamp > 0);
        assert_eq!(snapshot.entities.len(), 2);

        let names: Vec<String> = snapshot.entities.iter().map(|e| e.name.clone()).collect();
        assert!(names.contains(&"Alice".to_string()));
        assert!(names.contains(&"Bob".to_string()));
    }
}
