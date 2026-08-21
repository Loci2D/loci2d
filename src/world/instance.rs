use super::entity::{Entity, EntityType};
use super::physics::{
    ColliderShape, DeterministicCircle, MapBounds, NavigationComponent, StaticObstacle,
    TriggerEvent, TriggerEventType, intersect_shapes, resolve_dynamic_collision,
    resolve_static_collision, update_entity_navigation,
};
use super::session::{ClientSession, SessionState};
use crate::network::packets::{
    ClientIntent, EntityState, EntityType as ProtoEntityType, ReplayIntentEntry, WorldState,
};
use crate::scripting::ScriptEngine;
use fixed::types::I16F16;
use std::collections::{BTreeMap, BTreeSet};
use std::net::SocketAddr;
use std::path::Path;

// Re-export Vector2 from network and DeterministicVector2 from fixed_point
pub use super::fixed_point::DeterministicVector2;
pub use crate::network::packets::Vector2;

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
            map_bounds: MapBounds::default_arena(),
            static_obstacles: BTreeMap::new(),
            active_trigger_overlaps: BTreeSet::new(),
            previous_trigger_overlaps: BTreeSet::new(),
            trigger_events: Vec::new(),
            logging_enabled: false,
            script_engine: ScriptEngine::new().expect("Failed to initialize ScriptEngine"),
            next_entity_id: 1,
            next_session_id: 1,
        }
    }

    /// Evaluates a Lua script content string inside the instance's script engine.
    pub fn load_script(&mut self, script_content: &str) -> Result<(), String> {
        self.script_engine.load_script(script_content)
    }

    /// Evaluates a Lua script file from the specified path inside the instance's script engine.
    pub fn load_script_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), String> {
        self.script_engine.load_file(path)
    }

    /// Handles an incoming client intent and returns an optional replay entry for match logging.
    pub fn apply_intent(
        &mut self,
        addr: SocketAddr,
        intent: ClientIntent,
    ) -> Option<ReplayIntentEntry> {
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
                if let Some(entity) = self.entities.get_mut(&entity_id) {
                    if let Some(dir) = move_intent.direction {
                        entity.velocity = DeterministicVector2::from_proto(&dir);
                    }
                    // Direct Move intent preempts / cancels active navigation (ADR-0013)
                    if let Some(ref mut nav) = entity.navigation {
                        nav.clear();
                    }
                }
                Some(ReplayIntentEntry {
                    entity_id,
                    player_name: String::new(),
                    intent: Some(intent),
                })
            }

            // 4. Target Movement Intent (Click-to-move navigation - ADR-0013, Milestone 5.4)
            Intent::MoveToPos(move_to_pos_intent) => {
                let session = self.sessions.get_mut(&addr)?;
                session.refresh_activity();
                let entity_id = session.entity_id;
                if let (Some(target), Some(entity)) = (
                    move_to_pos_intent.target_position,
                    self.entities.get_mut(&entity_id),
                ) {
                    let target_vec = DeterministicVector2::new(
                        I16F16::from_bits(target.x_bits),
                        I16F16::from_bits(target.y_bits),
                    );
                    let nav = entity.navigation.get_or_insert_with(|| {
                        NavigationComponent::new(I16F16::from_num(1), I16F16::from_num(1))
                    });
                    nav.set_target(target_vec);
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
                    println!(
                        "[Intent] Entity {} ({}) executed action {}",
                        entity_id, entity.name, action_intent.ability_id
                    );
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
                println!(
                    "[Intent] Entity {} ({}) sent ping",
                    session.entity_id, session.player_name
                );
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
            println!(
                "[Session] Client {} re-joined as '{}' (EntityId {})",
                addr, session.player_name, session.entity_id
            );
            return session.entity_id;
        }

        let entity_id = self.next_entity_id;
        self.next_entity_id += 1;

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
            self.entities.remove(&session.entity_id);
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
    pub fn tick(&mut self, tick_count: u64) -> Vec<(u64, String)> {
        // 0. Steering & Destination Navigation Update (ADR-0013, Milestone 5.4)
        for entity in self.entities.values_mut() {
            if let Some(ref mut nav) = entity.navigation {
                update_entity_navigation(entity.position, &mut entity.velocity, nav);
            }
        }

        // 1. Velocity Integration (Candidate Next Position) & Initial Map Bounds Clamping
        for entity in self.entities.values_mut() {
            let old_pos = entity.position;
            entity.position = entity.position.saturating_add(entity.velocity);

            // Milestone 5.2: Map Boundary Constraint
            entity.position = match entity.current_collider() {
                Some(shape) => self.map_bounds.clamp_shape(&shape),
                None => self.map_bounds.clamp_point(entity.position),
            };

            if self.logging_enabled && entity.position != old_pos {
                println!(
                    "Player {} moved to ({:.2}, {:.2})",
                    entity.name,
                    entity.position.x.to_num::<f32>(),
                    entity.position.y.to_num::<f32>()
                );
            }
        }

        // 2. Static Solid Obstacle Collision Resolution (100% Pushback & Wall Sliding)
        // Evaluated in strict ascending obstacle.id order (ADR-0012)
        for obstacle in self.static_obstacles.values() {
            if !obstacle.is_solid {
                continue;
            }
            for entity in self.entities.values_mut() {
                if !entity.collision_filter.can_collide(&obstacle.filter) {
                    continue;
                }
                let Some(entity_shape) = entity.current_collider() else {
                    continue;
                };
                let manifold = intersect_shapes(&entity_shape, &obstacle.shape);
                if manifold.is_colliding {
                    resolve_static_collision(&mut entity.position, &mut entity.velocity, &manifold);
                    if self.logging_enabled {
                        println!(
                            "Player {} collided at coordinates ({:.2}, {:.2})",
                            entity.name,
                            entity.position.x.to_num::<f32>(),
                            entity.position.y.to_num::<f32>()
                        );
                    }
                    // Re-clamp to map bounds to ensure pushback didn't push outside arena
                    entity.position = match entity.current_collider() {
                        Some(shape) => self.map_bounds.clamp_shape(&shape),
                        None => self.map_bounds.clamp_point(entity.position),
                    };
                }
            }
        }

        // 3. Dynamic Entity-vs-Entity Collision Resolution (50/50 Split Pushback)
        // Evaluated in strict ascending (entity_a.id, entity_b.id) pair order
        let entity_ids: Vec<u64> = self.entities.keys().copied().collect();
        for i in 0..entity_ids.len() {
            for j in (i + 1)..entity_ids.len() {
                let id_a = entity_ids[i];
                let id_b = entity_ids[j];

                let can_collide = {
                    let entity_a = &self.entities[&id_a];
                    let entity_b = &self.entities[&id_b];
                    entity_a
                        .collision_filter
                        .can_collide(&entity_b.collision_filter)
                        && entity_a.collider.is_some()
                        && entity_b.collider.is_some()
                };

                if !can_collide {
                    continue;
                }

                let shape_a = self.entities[&id_a].current_collider().unwrap();
                let shape_b = self.entities[&id_b].current_collider().unwrap();
                let manifold = intersect_shapes(&shape_a, &shape_b);

                if manifold.is_colliding {
                    let mut pos_a = self.entities[&id_a].position;
                    let mut vel_a = self.entities[&id_a].velocity;
                    let mut pos_b = self.entities[&id_b].position;
                    let mut vel_b = self.entities[&id_b].velocity;

                    resolve_dynamic_collision(
                        &mut pos_a, &mut vel_a, &mut pos_b, &mut vel_b, &manifold,
                    );

                    if self.logging_enabled {
                        println!(
                            "Player {} collided at coordinates ({:.2}, {:.2})",
                            self.entities[&id_a].name,
                            pos_a.x.to_num::<f32>(),
                            pos_a.y.to_num::<f32>()
                        );
                        println!(
                            "Player {} collided at coordinates ({:.2}, {:.2})",
                            self.entities[&id_b].name,
                            pos_b.x.to_num::<f32>(),
                            pos_b.y.to_num::<f32>()
                        );
                    }

                    let entity_a = self.entities.get_mut(&id_a).unwrap();
                    entity_a.position = pos_a;
                    entity_a.velocity = vel_a;
                    if let Some(shape) = entity_a.current_collider() {
                        entity_a.position = self.map_bounds.clamp_shape(&shape);
                    }

                    let entity_b = self.entities.get_mut(&id_b).unwrap();
                    entity_b.position = pos_b;
                    entity_b.velocity = vel_b;
                    if let Some(shape) = entity_b.current_collider() {
                        entity_b.position = self.map_bounds.clamp_shape(&shape);
                    }
                }
            }
        }

        // 4. Trigger / Sensor Zone Overlap Evaluation & Lifecycle Events
        self.trigger_events.clear();
        let mut current_overlaps = BTreeSet::new();

        for (&trigger_id, obstacle) in &self.static_obstacles {
            if obstacle.is_solid {
                continue;
            }
            for (&entity_id, entity) in &self.entities {
                if !obstacle.filter.can_collide(&entity.collision_filter) {
                    continue;
                }
                let entity_shape = match entity.current_collider() {
                    Some(s) => s,
                    None => ColliderShape::Circle(DeterministicCircle::new(
                        entity.position,
                        I16F16::ZERO,
                    )),
                };
                let manifold = intersect_shapes(&entity_shape, &obstacle.shape);
                if manifold.is_colliding {
                    current_overlaps.insert((trigger_id, entity_id));
                }
            }
        }

        // Generate Enter, Stay, Exit events in strictly sorted (trigger_id, entity_id) order
        let all_pairs: BTreeSet<(u64, u64)> = self
            .previous_trigger_overlaps
            .union(&current_overlaps)
            .copied()
            .collect();

        for (trigger_id, entity_id) in all_pairs {
            let was_present = self
                .previous_trigger_overlaps
                .contains(&(trigger_id, entity_id));
            let is_present = current_overlaps.contains(&(trigger_id, entity_id));
            let event_type = match (was_present, is_present) {
                (false, true) => TriggerEventType::Enter,
                (true, true) => TriggerEventType::Stay,
                (true, false) => TriggerEventType::Exit,
                (false, false) => unreachable!(),
            };
            self.trigger_events.push(TriggerEvent::new(
                trigger_id, entity_id, event_type, tick_count,
            ));
        }

        self.previous_trigger_overlaps = current_overlaps.clone();
        self.active_trigger_overlaps = current_overlaps;

        // 5. Check for timed out clients
        let timed_out = self.check_timeouts();

        // println!(
        //     "[Tick {}] {} active entities, {} active sessions",
        //     tick_count,
        //     self.entities.len(),
        //     self.sessions.len()
        // );
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
            })
            .collect();

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

        let Some(ClientIntent {
            intent: Some(ref inner_intent),
        }) = entry.intent
        else {
            return;
        };

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
                    let entity = Entity::new(entry.entity_id, player_name, EntityType::Player)
                        .with_default_navigation(I16F16::from_num(1), I16F16::from_num(1))
                        .with_circle_collider(I16F16::from_num(2));
                    self.entities.insert(entry.entity_id, entity);
                }
            }
            Intent::Disconnect(_) => {
                self.entities.remove(&entry.entity_id);
            }
            Intent::Move(move_intent) => {
                if let Some(entity) = self.entities.get_mut(&entry.entity_id) {
                    if let Some(dir) = move_intent.direction {
                        entity.velocity = DeterministicVector2::from_proto(&dir);
                    }
                    if let Some(ref mut nav) = entity.navigation {
                        nav.clear();
                    }
                }
            }
            Intent::MoveToPos(move_to_pos_intent) => {
                if let (Some(target), Some(entity)) = (
                    move_to_pos_intent.target_position,
                    self.entities.get_mut(&entry.entity_id),
                ) {
                    let target_vec = DeterministicVector2::new(
                        I16F16::from_bits(target.x_bits),
                        I16F16::from_bits(target.y_bits),
                    );
                    let nav = entity.navigation.get_or_insert_with(|| {
                        NavigationComponent::new(I16F16::from_num(1), I16F16::from_num(1))
                    });
                    nav.set_target(target_vec);
                }
            }
            Intent::Action(action_intent) => {
                if let Some(entity) = self.entities.get_mut(&entry.entity_id) {
                    println!(
                        "[Replay] Entity {} ({}) executed action {}",
                        entry.entity_id, entity.name, action_intent.ability_id
                    );
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
        ActionIntent, ClientIntent, DisconnectIntent, JoinIntent, MoveIntent, PingIntent,
        client_intent,
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
            intent: Some(client_intent::Intent::Action(ActionIntent {
                ability_id: 1,
            })),
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
            intent: Some(client_intent::Intent::Action(ActionIntent {
                ability_id: 42,
            })),
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

    #[test]
    fn test_instance_map_bounds_clamping_on_tick() {
        use crate::world::physics::MapBounds;
        use fixed::types::I16F16;

        let mut instance = Instance::new(1, 30, 10);
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

        instance.tick(1);

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

        let mut instance = Instance::new(1, 30, 10);
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

        let mut instance = Instance::new(1, 30, 10);
        let addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();

        // 1. Join
        let join_intent = ClientIntent {
            intent: Some(client_intent::Intent::Join(JoinIntent {
                player_name: "Alice".to_string(),
            })),
        };
        instance.apply_intent(addr, join_intent);

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
        instance.apply_intent(addr, move_to_pos);

        let entity = instance.get_entity(1).unwrap();
        assert!(entity.navigation.as_ref().unwrap().is_navigating());
        assert_eq!(
            entity.navigation.as_ref().unwrap().target,
            Some(DeterministicVector2::new(target_x, target_y))
        );

        // 3. Tick: entity moves toward (10, 0) with move_speed = 1.0
        instance.tick(1);
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
        instance.apply_intent(addr, move_intent);

        let entity = instance.get_entity(1).unwrap();
        assert!(!entity.navigation.as_ref().unwrap().is_navigating());
        assert_eq!(
            entity.velocity,
            DeterministicVector2::new(I16F16::ZERO, I16F16::from_num(-2))
        );
    }
}
