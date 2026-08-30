use super::entity::{Entity, EntityType};
use super::physics::{MapBounds, StaticObstacle, TriggerEvent};
use super::session::{ClientSession, SessionState};
use crate::network::packets::client_intent::Intent;
use crate::network::packets::{
    ClientIntent, EntityState, EntityType as ProtoEntityType, Property, ReplayIntentEntry,
    WorldState,
};
use crate::scripting::{CommandBuffer, ScriptEngine};
use fixed::types::I16F16;
use std::collections::{BTreeMap, BTreeSet};
use std::net::SocketAddr;
use std::path::Path;

// Re-export Vector2 from network and DeterministicVector2 from fixed_point
pub use super::fixed_point::DeterministicVector2;
pub use crate::network::packets::Vector2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchState {
    Paused,
    Running,
    Ended { winner_data: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveTimer {
    pub timer_id: String,
    pub remaining_ticks: u32,
}

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
    // Phase 5 Additions:
    pub map_bounds: MapBounds,
    pub static_obstacles: BTreeMap<u64, StaticObstacle>,
    pub active_trigger_overlaps: BTreeSet<(u64, u64)>,
    pub previous_trigger_overlaps: BTreeSet<(u64, u64)>,
    pub trigger_events: Vec<TriggerEvent>,
    pub logging_enabled: bool,
    // Phase 6 Additions:
    pub script_engine: ScriptEngine,
    pub script_hash: String,
    pub globals: BTreeMap<String, String>,
    pub state: MatchState,
    pub active_timers: BTreeMap<String, ActiveTimer>,
    // NOTE: Cell<u64> is intentionally not PartialEq-comparable. Instance equality
    // must be established via canonical_hash(), not structural comparison.
    // TODO(Phase 7+): If Instance is ever moved to a multi-threaded runtime,
    // replace Cell<u64> with AtomicU64 for Send + Sync compliance.
    next_entity_id: std::cell::Cell<u64>,
    next_session_id: u64,
}

impl Instance {
    /// Allocates and returns a unique deterministic entity ID.
    pub fn allocate_entity_id(&self) -> u64 {
        let id = self.next_entity_id.get();
        self.next_entity_id.set(id + 1);
        id
    }

    pub fn new(id: u64, tick_rate: u32, client_timeout_secs: u64, seed: u64) -> Self {
        Self {
            id,
            entities: BTreeMap::new(),
            sessions: BTreeMap::new(),
            entity_to_addr: BTreeMap::new(),
            tick_rate,
            client_timeout_secs,
            map_bounds: MapBounds::default_arena(),
            static_obstacles: BTreeMap::new(),
            active_trigger_overlaps: BTreeSet::new(),
            previous_trigger_overlaps: BTreeSet::new(),
            trigger_events: Vec::new(),
            logging_enabled: false,
            script_engine: ScriptEngine::new(seed).expect("Failed to initialize ScriptEngine"),
            script_hash: String::new(),
            globals: BTreeMap::new(),
            state: MatchState::Running,
            active_timers: BTreeMap::new(),
            next_entity_id: std::cell::Cell::new(1),
            next_session_id: 1,
        }
    }

    /// Evaluates a Lua script content string inside the instance's script engine.
    pub fn load_script(&mut self, script_content: &str) -> Result<(), String> {
        self.script_engine.load_script(script_content)?;

        let mut cmd_buffer = CommandBuffer::new();
        self.script_engine
            .on_init(self, &mut cmd_buffer)
            .map_err(|e| e.to_string())?;
        cmd_buffer.flush_and_apply(self);

        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(script_content.as_bytes());
        let hash_bytes: [u8; 32] = hasher.finalize().into();
        self.script_hash = hash_bytes.iter().map(|b| format!("{:02x}", b)).collect();
        Ok(())
    }

    /// Evaluates a Lua script file from the specified path inside the instance's script engine.
    pub fn load_script_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), String> {
        let path_ref = path.as_ref();
        let content = std::fs::read_to_string(path_ref)
            .map_err(|e| format!("Failed to read script file '{}': {e}", path_ref.display()))?;
        self.load_script(&content)
    }

    /// Handles an incoming client intent and returns an optional replay entry for match logging.
    pub fn apply_intent(
        &mut self,
        addr: SocketAddr,
        intent: ClientIntent,
    ) -> Result<Option<ReplayIntentEntry>, String> {
        let inner_intent = match intent.intent.as_ref() {
            Some(i) => i,
            None => return Ok(None),
        };

        if matches!(self.state, MatchState::Ended { .. }) {
            if matches!(
                inner_intent,
                Intent::Action(_) | Intent::Move(_) | Intent::MoveToPos(_)
            ) {
                return Ok(None);
            }
        }

        let (entity_id, player_name) = match inner_intent {
            Intent::Join(join_intent) => {
                let player_name = if join_intent.player_name.trim().is_empty() {
                    format!("Player_{}", self.next_entity_id.get())
                } else {
                    join_intent.player_name.clone()
                };
                let entity_id = self.handle_join(addr, player_name.clone());
                (entity_id, player_name)
            }
            Intent::Disconnect(disconnect_intent) => {
                let session = match self.sessions.get(&addr) {
                    Some(s) => s,
                    None => return Ok(None),
                };
                let entity_id = session.entity_id;
                let player_name = session.player_name.clone();
                self.handle_disconnect(addr, &disconnect_intent.reason);
                (entity_id, player_name)
            }
            _ => {
                let session = match self.sessions.get_mut(&addr) {
                    Some(s) => s,
                    None => return Ok(None),
                };
                session.refresh_activity();
                (session.entity_id, String::new())
            }
        };

        crate::world::intent_handler::apply_resolved_intent(
            self,
            entity_id,
            player_name.clone(),
            inner_intent,
        )?;

        let intent_for_replay = if matches!(inner_intent, Intent::Ping(_)) {
            None
        } else {
            Some(ReplayIntentEntry {
                entity_id,
                player_name,
                intent: Some(intent),
            })
        };

        Ok(intent_for_replay)
    }

    /// Explicit client join
    pub fn handle_join(&mut self, addr: SocketAddr, player_name: String) -> u64 {
        if let Some(session) = self.sessions.get_mut(&addr) {
            session.player_name = player_name.clone();
            session.refresh_activity();
            if let Some(entity) = self.entities.get_mut(&session.entity_id) {
                entity.name = player_name;
            }
            println!(
                "[Session] Client {} re-joined as '{}' (EntityId {})",
                addr, session.player_name, session.entity_id
            );
            return session.entity_id;
        }

        let entity_id = self.allocate_entity_id();

        let session_id = self.next_session_id;
        self.next_session_id += 1;

        let session = ClientSession::new(session_id, addr, entity_id, player_name.clone());
        let entity = Entity::new(entity_id, player_name.clone(), EntityType::Player)
            .with_default_navigation(I16F16::from_num(1), I16F16::from_num(1))
            .with_circle_collider(I16F16::from_num(2));

        self.entities.insert(entity_id, entity);
        self.sessions.insert(addr, session);
        self.entity_to_addr.insert(entity_id, addr);

        println!(
            "[Join] Client {} joined as '{}' (SessionId {}, EntityId {})",
            addr, player_name, session_id, entity_id
        );
        entity_id
    }

    /// Explicit client disconnect
    pub fn handle_disconnect(&mut self, addr: SocketAddr, reason: &str) {
        if let Some(mut session) = self.sessions.remove(&addr) {
            session.state = SessionState::Disconnected;
            self.entity_to_addr.remove(&session.entity_id);
            // We do not remove the entity from self.entities here, to allow on_player_leave
            // to access its properties. The intent handler will issue a DestroyEntity command.
            let display_reason = if reason.trim().is_empty() {
                "normal quit"
            } else {
                reason
            };
            println!(
                "[Disconnect] Client {} ('{}', EntityId {}) disconnected gracefully. Reason: '{}'",
                addr, session.player_name, session.entity_id, display_reason
            );
        }
    }

    /// Advance physics using deterministic fixed-point integration, resolve solid collisions against
    /// static obstacles and dynamic entities, evaluate trigger sensor zones, clamp to map boundaries,
    /// and sweep for timed-out sessions.
    /// Returns a list of (entity_id, player_name) for any sessions that timed out during this tick.
    pub fn tick(&mut self, tick_count: u64) -> Result<Vec<(u64, String)>, String> {
        crate::world::simulation::tick(self, tick_count)
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
            // We do not remove the entity from self.entities here.
            // simulation.rs will dispatch on_player_leave and push a DestroyEntity command.
            // removed -> self.entities.remove(&session.entity_id);
            timed_out_entities.push((entity_id, player_name.clone()));
            println!(
                "[Timeout] Client {} ('{}', EntityId {}) timed out after {}s of inactivity",
                addr, player_name, entity_id, timeout_secs
            );
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

    pub fn add_static_obstacle(&mut self, obstacle: StaticObstacle) {
        self.static_obstacles.insert(obstacle.id, obstacle);
    }

    pub fn remove_static_obstacle(&mut self, obstacle_id: u64) -> Option<StaticObstacle> {
        self.static_obstacles.remove(&obstacle_id)
    }

    pub fn get_static_obstacle(&self, obstacle_id: u64) -> Option<&StaticObstacle> {
        self.static_obstacles.get(&obstacle_id)
    }

    pub fn set_map_bounds(&mut self, bounds: MapBounds) {
        self.map_bounds = bounds;
    }

    /// Generates a complete WorldState snapshot representing all active entities.
    pub fn create_snapshot(&self, tick: u64) -> WorldState {
        let entities = self
            .entities
            .values()
            .map(|e| EntityState {
                id: e.id,
                name: e.name.clone(),
                position: Some(e.position.to_proto()),
                velocity: Some(e.velocity.to_proto()),
                entity_type: match e.entity_type {
                    EntityType::Player => ProtoEntityType::Player as i32,
                    EntityType::NPC => ProtoEntityType::Npc as i32,
                    EntityType::Prop => ProtoEntityType::Prop as i32,
                },
                properties: e
                    .properties
                    .iter()
                    .map(|(k, v)| Property {
                        key: k.clone(),
                        value: v.clone(),
                    })
                    .collect(),
            })
            .collect();

        let timestamp = tick * (1000 / self.tick_rate as u64);

        WorldState {
            tick,
            timestamp,
            entities,
            globals: self
                .globals
                .iter()
                .map(|(k, v)| Property {
                    key: k.clone(),
                    value: v.clone(),
                })
                .collect(),
        }
    }

    /// Returns a list of all active client destination addresses for broadcasting.
    pub fn get_broadcast_addresses(&self) -> Vec<SocketAddr> {
        self.sessions.keys().copied().collect()
    }

    /// Applies a recorded replay intent entry directly by entity_id without requiring network sockets.
    pub fn apply_replay_entry(&mut self, entry: &ReplayIntentEntry) {
        let Some(ClientIntent {
            intent: Some(ref inner_intent),
        }) = entry.intent
        else {
            return;
        };

        if let Err(e) = crate::world::intent_handler::apply_resolved_intent(
            self,
            entry.entity_id,
            entry.player_name.clone(),
            inner_intent,
        ) {
            eprintln!("[Replay] Error applying intent: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::packets::{
        ActionIntent, ClientIntent, DisconnectIntent, JoinIntent, MoveIntent, PingIntent,
        client_intent,
    };

    #[test]
    fn test_explicit_join_and_move_intent() {
        let mut instance = Instance::new(1, 30, 10, 42);
        let addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();

        // 1. Explicit Join
        let join_intent = ClientIntent {
            intent: Some(client_intent::Intent::Join(JoinIntent {
                player_name: "Alice".to_string(),
            })),
        };
        let _ = instance.apply_intent(addr, join_intent);

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
        let _ = instance.apply_intent(addr, move_intent);

        let entity = instance.get_entity(entity_id).unwrap();
        assert_eq!(entity.velocity.to_f32(), (2.5, -1.0));

        // 3. Tick
        let _ = instance.tick(1);

        let updated_entity = instance.get_entity(entity_id).unwrap();
        assert_eq!(updated_entity.position.to_f32(), (2.5, -1.0));
    }

    #[test]
    fn test_explicit_disconnect() {
        let mut instance = Instance::new(1, 30, 10, 42);
        let addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();

        // Join
        let join_intent = ClientIntent {
            intent: Some(client_intent::Intent::Join(JoinIntent {
                player_name: "Bob".to_string(),
            })),
        };
        let _ = instance.apply_intent(addr, join_intent);
        assert_eq!(instance.sessions.len(), 1);
        assert_eq!(instance.entities.len(), 1);

        // Disconnect
        let disconnect_intent = ClientIntent {
            intent: Some(client_intent::Intent::Disconnect(DisconnectIntent {
                reason: "Leaving match".to_string(),
            })),
        };
        let _ = instance.apply_intent(addr, disconnect_intent);

        assert_eq!(instance.sessions.len(), 0);
        assert_eq!(instance.entities.len(), 0);
        assert_eq!(instance.entity_to_addr.len(), 0);
    }

    #[test]
    fn test_unjoined_client_intents_dropped() {
        let mut instance = Instance::new(1, 30, 10, 42);
        let addr: SocketAddr = "127.0.0.1:34567".parse().unwrap();

        // Send move intent without prior join — should be dropped
        let move_intent = ClientIntent {
            intent: Some(client_intent::Intent::Move(MoveIntent {
                direction: Some(DeterministicVector2::from_f32(1.0, 1.0).to_proto()),
            })),
        };
        let _ = instance.apply_intent(addr, move_intent);

        assert_eq!(instance.sessions.len(), 0);
        assert_eq!(instance.entities.len(), 0);

        // Send action intent without prior join — should be dropped
        let action_intent = ClientIntent {
            intent: Some(client_intent::Intent::Action(ActionIntent {
                ability_id: 1,
                target_direction: None,
            })),
        };
        let _ = instance.apply_intent(addr, action_intent);
        assert_eq!(instance.sessions.len(), 0);
        assert_eq!(instance.entities.len(), 0);

        // Send ping intent without prior join — should be dropped
        let ping_intent = ClientIntent {
            intent: Some(client_intent::Intent::Ping(PingIntent {})),
        };
        let _ = instance.apply_intent(addr, ping_intent);
        assert_eq!(instance.sessions.len(), 0);
        assert_eq!(instance.entities.len(), 0);
    }

    #[test]
    fn test_timeout_detection() {
        let mut instance = Instance::new(1, 30, 0, 42); // 0-second timeout for immediate expiry
        let addr: SocketAddr = "127.0.0.1:45678".parse().unwrap();

        let join_intent = ClientIntent {
            intent: Some(client_intent::Intent::Join(JoinIntent {
                player_name: "Charlie".to_string(),
            })),
        };
        let _ = instance.apply_intent(addr, join_intent);
        assert_eq!(instance.sessions.len(), 1);
        assert_eq!(instance.entities.len(), 1);

        // Advance tick, should trigger check_timeouts and clean up
        let _ = instance.tick(1);

        assert_eq!(instance.sessions.len(), 0);
        assert_eq!(instance.entities.len(), 0);
        assert_eq!(instance.entity_to_addr.len(), 0);
    }

    #[test]
    fn test_ping_and_action_intents() {
        let mut instance = Instance::new(1, 30, 10, 42);
        let addr: SocketAddr = "127.0.0.1:56789".parse().unwrap();

        let join_intent = ClientIntent {
            intent: Some(client_intent::Intent::Join(JoinIntent {
                player_name: "Dave".to_string(),
            })),
        };
        let _ = instance.apply_intent(addr, join_intent);
        assert_eq!(instance.sessions.len(), 1);

        let ping_intent = ClientIntent {
            intent: Some(client_intent::Intent::Ping(PingIntent {})),
        };
        let _ = instance.apply_intent(addr, ping_intent);
        assert_eq!(instance.sessions.len(), 1);

        let action_intent = ClientIntent {
            intent: Some(client_intent::Intent::Action(ActionIntent {
                ability_id: 42,
                target_direction: None,
            })),
        };
        let _ = instance.apply_intent(addr, action_intent);
        assert_eq!(instance.sessions.len(), 1);
    }

    #[test]
    fn test_rejoin_updates_player_name() {
        let mut instance = Instance::new(1, 30, 10, 42);
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
        let mut instance = Instance::new(1, 30, 10, 42);
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

    #[test]
    fn test_instance_map_bounds_clamping_on_tick() {
        use crate::world::physics::MapBounds;
        use fixed::types::I16F16;
        let mut instance = Instance::new(1, 30, 10, 42);
        instance.set_map_bounds(MapBounds::new(
            DeterministicVector2::new(I16F16::from_num(-100), I16F16::from_num(-100)),
            DeterministicVector2::new(I16F16::from_num(100), I16F16::from_num(100)),
        ));

        // 1. Point entity (no collider)
        let mut e1 = Entity::new(1, "PointEntity".to_string(), EntityType::Player);
        e1.position = DeterministicVector2::new(I16F16::from_num(90), I16F16::from_num(90));
        e1.velocity = DeterministicVector2::new(I16F16::from_num(30), I16F16::from_num(30)); // would reach 120, 120
        instance.add_entity(e1);

        // 2. Circle entity (radius 10)
        let mut e2 = Entity::new(2, "CircleEntity".to_string(), EntityType::Player)
            .with_circle_collider(I16F16::from_num(10));
        e2.position = DeterministicVector2::new(I16F16::from_num(85), I16F16::from_num(-85));
        e2.velocity = DeterministicVector2::new(I16F16::from_num(20), I16F16::from_num(-20)); // would reach 105, -105
        instance.add_entity(e2);

        // 3. AABB entity (half extents 15, 15)
        let mut e3 =
            Entity::new(3, "AABBEntity".to_string(), EntityType::Player).with_aabb_collider(
                DeterministicVector2::new(I16F16::from_num(15), I16F16::from_num(15)),
            );
        e3.position = DeterministicVector2::new(I16F16::from_num(-80), I16F16::from_num(0));
        e3.velocity = DeterministicVector2::new(I16F16::from_num(-30), I16F16::from_num(0)); // would reach -110, 0
        instance.add_entity(e3);

        let _ = instance.tick(1);

        // e1 clamped to (100, 100)
        let updated_e1 = instance.get_entity(1).unwrap();
        assert_eq!(
            updated_e1.position,
            DeterministicVector2::new(I16F16::from_num(100), I16F16::from_num(100))
        );

        // e2 clamped to (90, -90) because radius is 10 and max is 100 / min is -100
        let updated_e2 = instance.get_entity(2).unwrap();
        assert_eq!(
            updated_e2.position,
            DeterministicVector2::new(I16F16::from_num(90), I16F16::from_num(-90))
        );

        // e3 clamped to (-85, 0) because half_extent.x is 15 and min is -100
        let updated_e3 = instance.get_entity(3).unwrap();
        assert_eq!(
            updated_e3.position,
            DeterministicVector2::new(I16F16::from_num(-85), I16F16::from_num(0))
        );
    }

    #[test]
    fn test_instance_static_obstacle_management() {
        use crate::world::physics::{ColliderShape, DeterministicCircle, StaticObstacle};
        use fixed::types::I16F16;

        let mut instance = Instance::new(1, 30, 10, 42);
        let obs1 = StaticObstacle::solid_wall(
            1,
            ColliderShape::Circle(DeterministicCircle::new(
                DeterministicVector2::new(I16F16::from_num(10), I16F16::from_num(20)),
                I16F16::from_num(5),
            )),
        );
        let obs2 = StaticObstacle::trigger_zone(
            2,
            ColliderShape::Circle(DeterministicCircle::new(
                DeterministicVector2::new(I16F16::from_num(50), I16F16::from_num(50)),
                I16F16::from_num(10),
            )),
        );

        instance.add_static_obstacle(obs1.clone());
        instance.add_static_obstacle(obs2.clone());

        assert_eq!(instance.static_obstacles.len(), 2);
        assert_eq!(instance.get_static_obstacle(1), Some(&obs1));
        assert_eq!(instance.get_static_obstacle(2), Some(&obs2));

        let removed = instance.remove_static_obstacle(1);
        assert_eq!(removed, Some(obs1));
        assert_eq!(instance.static_obstacles.len(), 1);
        assert_eq!(instance.get_static_obstacle(1), None);
    }

    #[test]
    fn test_explicit_move_to_pos_intent_and_preemption() {
        use crate::network::packets::MoveToPositionIntent;
        let mut instance = Instance::new(1, 30, 10, 42);
        let addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();

        // 1. Join
        let join_intent = ClientIntent {
            intent: Some(client_intent::Intent::Join(JoinIntent {
                player_name: "Alice".to_string(),
            })),
        };
        let _ = instance.apply_intent(addr, join_intent);

        // 2. MoveToPos Intent
        let target_x = I16F16::from_num(10);
        let target_y = I16F16::from_num(0);
        let move_to_pos = ClientIntent {
            intent: Some(client_intent::Intent::MoveToPos(MoveToPositionIntent {
                target_position: Some(Vector2 {
                    x_bits: target_x.to_bits(),
                    y_bits: target_y.to_bits(),
                }),
            })),
        };
        let _ = instance.apply_intent(addr, move_to_pos);

        let entity = instance.get_entity(1).unwrap();
        assert!(entity.navigation.as_ref().unwrap().is_navigating());
        assert_eq!(
            entity.navigation.as_ref().unwrap().target,
            Some(DeterministicVector2::new(target_x, target_y))
        );

        // 3. Tick: entity moves toward (10, 0) with move_speed = 1.0
        let _ = instance.tick(1);
        let entity = instance.get_entity(1).unwrap();
        assert_eq!(
            entity.velocity,
            DeterministicVector2::new(I16F16::from_num(1), I16F16::ZERO)
        );
        assert_eq!(
            entity.position,
            DeterministicVector2::new(I16F16::from_num(1), I16F16::ZERO)
        );

        // 4. Preemption by direct Move intent
        let move_intent = ClientIntent {
            intent: Some(client_intent::Intent::Move(MoveIntent {
                direction: Some(
                    DeterministicVector2::new(I16F16::ZERO, I16F16::from_num(-2)).to_proto(),
                ),
            })),
        };
        let _ = instance.apply_intent(addr, move_intent);

        let entity = instance.get_entity(1).unwrap();
        assert!(!entity.navigation.as_ref().unwrap().is_navigating());
        assert_eq!(
            entity.velocity,
            DeterministicVector2::new(I16F16::ZERO, I16F16::from_num(-2))
        );
    }

    #[test]
    fn test_property_commands() {
        use crate::scripting::command::{Command, CommandBuffer};

        let mut instance = Instance::new(1, 30, 10, 42);
        let e = Entity::new(1, "Player1".to_string(), EntityType::Player);
        instance.add_entity(e);

        let mut cmd_buf = CommandBuffer::new();
        cmd_buf.push(Command::SetEntityProperty {
            entity_id: 1,
            key: "health".to_string(),
            value: "100".to_string(),
        });
        cmd_buf.push(Command::SetGlobalProperty {
            key: "round_number".to_string(),
            value: "2".to_string(),
        });

        cmd_buf.flush_and_apply(&mut instance);

        assert_eq!(
            instance
                .get_entity(1)
                .unwrap()
                .properties
                .get("health")
                .unwrap(),
            "100"
        );
        assert_eq!(instance.globals.get("round_number").unwrap(), "2");
    }

    #[test]
    fn test_hash_stability() {
        use prost::Message;

        let mut instance1 = Instance::new(1, 30, 10, 42);
        let mut e1 = Entity::new(1, "Player1".to_string(), EntityType::Player);
        e1.properties
            .insert("health".to_string(), "100".to_string());
        e1.properties.insert("team".to_string(), "red".to_string());
        instance1.add_entity(e1);
        let snap1 = instance1.create_snapshot(1);
        let mut buf1 = Vec::new();
        snap1.encode(&mut buf1).unwrap();

        let mut instance2 = Instance::new(1, 30, 10, 42);
        let mut e2 = Entity::new(1, "Player1".to_string(), EntityType::Player);
        // Insert in reverse order to ensure BTreeMap sorts it internally
        e2.properties.insert("team".to_string(), "red".to_string());
        e2.properties
            .insert("health".to_string(), "100".to_string());
        instance2.add_entity(e2);
        let snap2 = instance2.create_snapshot(1);
        let mut buf2 = Vec::new();
        snap2.encode(&mut buf2).unwrap();

        // Prove ADR-0007 compliance: iteration order and hence serialization bytes are identical
        assert_eq!(buf1, buf2);
    }

    #[test]
    fn test_timer_determinism_and_execution_order() {
        use crate::world::instance::ActiveTimer;
        use crate::world::instance::MatchState;

        let mut instance = Instance::new(1, 30, 10, 12345);
        instance.state = MatchState::Running;

        // Add timers out of alphabetical order
        instance.active_timers.insert(
            "timer_B".to_string(),
            ActiveTimer {
                timer_id: "timer_B".to_string(),
                remaining_ticks: 1,
            },
        );
        instance.active_timers.insert(
            "timer_A".to_string(),
            ActiveTimer {
                timer_id: "timer_A".to_string(),
                remaining_ticks: 1,
            },
        );

        // Add a script that logs timer completion
        instance
            .script_engine
            .load_script(
                r#"
            _G.timer_log = _G.timer_log or {}
            function on_timer_complete(timer_id)
                table.insert(_G.timer_log, timer_id)
            end
        "#,
            )
            .unwrap();

        let _ = instance.tick(1);

        assert!(instance.active_timers.is_empty());

        // Verify execution order in Lua
        instance
            .script_engine
            .lua()
            .scope(|_scope| {
                let log: mlua::Table = instance
                    .script_engine
                    .lua()
                    .globals()
                    .get("timer_log")
                    .unwrap();
                let first: String = log.get(1).unwrap();
                let second: String = log.get(2).unwrap();

                assert_eq!(first, "timer_A");
                assert_eq!(second, "timer_B");
                Ok::<(), mlua::Error>(())
            })
            .unwrap();
    }

    #[test]
    fn test_match_state_machine_lock() {
        use crate::world::instance::MatchState;

        let mut instance = Instance::new(1, 30, 10, 12345);
        instance.state = MatchState::Paused;

        // Load a script to track callbacks
        instance
            .script_engine
            .load_script(
                r#"
            _G.tick_called = false
            _G.join_called = false
            
            function on_tick(tick)
                _G.tick_called = true
            end
            
            function on_player_join(entity_id)
                _G.join_called = true
            end
        "#,
            )
            .unwrap();

        let _ = instance.tick(1);

        // Tick should be skipped because we're paused
        let tick_called: bool = instance
            .script_engine
            .lua()
            .globals()
            .get("tick_called")
            .unwrap();
        assert!(!tick_called);

        // But join events should still process
        use crate::network::packets::{ClientIntent, JoinIntent, client_intent};
        let join_intent = ClientIntent {
            intent: Some(client_intent::Intent::Join(JoinIntent {
                player_name: "Alice".to_string(),
            })),
        };
        let _ = instance
            .apply_intent("127.0.0.1:1234".parse().unwrap(), join_intent)
            .unwrap();

        let join_called: bool = instance
            .script_engine
            .lua()
            .globals()
            .get("join_called")
            .unwrap();
        assert!(join_called);
    }

    #[test]
    fn test_intent_filtering_when_ended() {
        use crate::network::packets::{
            ActionIntent, ClientIntent, MoveIntent, PingIntent, Vector2, client_intent,
        };
        use crate::world::instance::MatchState;

        let mut instance = Instance::new(1, 30, 10, 12345);
        instance.state = MatchState::Ended {
            winner_data: "Alice".to_string(),
        };

        let action_intent = ClientIntent {
            intent: Some(client_intent::Intent::Action(ActionIntent {
                ability_id: 1,
                target_direction: None,
            })),
        };

        let move_intent = ClientIntent {
            intent: Some(client_intent::Intent::Move(MoveIntent {
                direction: Some(Vector2 {
                    x_bits: 1,
                    y_bits: 0,
                }),
            })),
        };

        let ping_intent = ClientIntent {
            intent: Some(client_intent::Intent::Ping(PingIntent {})),
        };

        let addr: std::net::SocketAddr = "127.0.0.1:1234".parse().unwrap();

        // These should be ignored and return Ok(None)
        let res_action = instance.apply_intent(addr, action_intent).unwrap();
        assert!(res_action.is_none());

        let res_move = instance.apply_intent(addr, move_intent).unwrap();
        assert!(res_move.is_none());

        let res_ping = instance.apply_intent(addr, ping_intent);
        assert!(res_ping.is_ok());
    }
}
