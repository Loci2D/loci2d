use crate::world::instance::DeterministicVector2;
use crate::world::instance::{ActiveTimer, Instance, MatchState};

use fixed::types::I16F16;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    SpawnEntity {
        entity_id: u64,
        blueprint: String,
        position: DeterministicVector2,
        entity_type: String,
        move_speed: I16F16,
        radius: I16F16,
        properties: std::collections::BTreeMap<String, String>,
    },
    DestroyEntity {
        entity_id: u64,
    },
    SetPosition {
        entity_id: u64,
        position: DeterministicVector2,
    },
    SetVelocity {
        entity_id: u64,
        velocity: DeterministicVector2,
    },
    SetMoveSpeed {
        entity_id: u64,
        speed: I16F16,
    },
    SetNavigationTarget {
        entity_id: u64,
        target: DeterministicVector2,
    },
    SetEntityProperty {
        entity_id: u64,
        key: String,
        value: String,
    },
    SetGlobalProperty {
        key: String,
        value: String,
    },
    SendEvent {
        event_name: String,
        data: String,
    },
    StartMatch,
    PauseMatch,
    EndMatch {
        winner_data: String,
    },
    StartTimer {
        timer_id: String,
        remaining_ticks: u32,
    },
}

#[derive(Debug, Default)]
pub struct CommandBuffer {
    pub commands: Vec<Command>,
}

impl CommandBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, command: Command) {
        self.commands.push(command);
    }

    /// Flush the command buffer and apply commands to the instance.
    pub fn flush_and_apply(&mut self, instance: &mut Instance) {
        for command in self.commands.drain(..) {
            match command {
                Command::SpawnEntity {
                    entity_id,
                    blueprint,
                    position,
                    entity_type,
                    move_speed,
                    radius,
                    properties,
                } => {
                    let parsed_type = match entity_type.as_str() {
                        "Player" => crate::world::entity::EntityType::Player,
                        "Enemy" | "NPC" => crate::world::entity::EntityType::NPC,
                        _ => crate::world::entity::EntityType::Prop,
                    };
                    
                    let mut entity = crate::world::entity::Entity::new(
                        entity_id,
                        blueprint.clone(),
                        parsed_type,
                    )
                    .with_default_navigation(move_speed, I16F16::from_num(1))
                    .with_circle_collider(radius);
                    
                    for (k, v) in properties {
                        entity.properties.insert(k, v);
                    }

                    let pos_for_log = position;
                    entity.position = position;
                    
                    instance.entities.insert(entity_id, entity);
                    
                    if instance.logging_enabled {
                        println!(
                            "[CommandBuffer] Spawned entity {} from blueprint '{}' at ({}, {})",
                            entity_id,
                            blueprint,
                            pos_for_log.x.to_num::<f64>(),
                            pos_for_log.y.to_num::<f64>()
                        );
                    }
                }
                Command::DestroyEntity { entity_id } => {
                    instance.entities.remove(&entity_id);
                }
                Command::SetPosition {
                    entity_id,
                    position,
                } => {
                    if let Some(entity) = instance.entities.get_mut(&entity_id) {
                        entity.position = position;
                    }
                }
                Command::SetNavigationTarget { entity_id, target } => {
                    if let Some(entity) = instance.entities.get_mut(&entity_id) {
                        let nav = entity.navigation.get_or_insert_with(|| {
                            crate::world::physics::navigation::NavigationComponent::new(
                                I16F16::from_num(1),
                                I16F16::from_num(1),
                            )
                        });
                        nav.set_target(target);
                    }
                }
                Command::SetVelocity {
                    entity_id,
                    velocity,
                } => {
                    if let Some(entity) = instance.entities.get_mut(&entity_id) {
                        entity.velocity = velocity;
                        if let Some(ref mut nav) = entity.navigation {
                            nav.clear();
                        }
                    }
                }
                Command::SetMoveSpeed { entity_id, speed } => {
                    if let Some(entity) = instance.entities.get_mut(&entity_id) {
                        if let Some(ref mut nav) = entity.navigation {
                            nav.move_speed = speed;
                        }
                    }
                }
                Command::SetEntityProperty {
                    entity_id,
                    key,
                    value,
                } => {
                    if let Some(entity) = instance.entities.get_mut(&entity_id) {
                        entity.properties.insert(key, value);
                    }
                }
                Command::SetGlobalProperty { key, value } => {
                    instance.globals.insert(key, value);
                }
                Command::SendEvent { event_name, data } => {
                    // TODO(Phase X): Implement actual event broadcasting to clients
                    if instance.logging_enabled {
                        println!(
                            "[CommandBuffer] Broadcasting event '{}' with data '{}'",
                            event_name, data
                        );
                    }
                }
                Command::StartMatch => {
                    instance.state = MatchState::Running;
                }
                Command::PauseMatch => {
                    instance.state = MatchState::Paused;
                }
                Command::EndMatch { winner_data } => {
                    instance.state = MatchState::Ended { winner_data };
                }
                Command::StartTimer {
                    timer_id,
                    remaining_ticks,
                } => {
                    // Note: If a timer with the same timer_id already exists, it is silently overwritten.
                    // This allows scripts to easily reset or restart active timers.
                    instance.active_timers.insert(
                        timer_id.clone(),
                        ActiveTimer {
                            timer_id,
                            remaining_ticks,
                        },
                    );
                }
            }
        }
    }
}
