use crate::world::instance::DeterministicVector2;
use crate::world::instance::{ActiveTimer, Instance, MatchState};
use crate::world::physics::{ColliderShape, CollisionFilter, MapBounds, StaticObstacle};

use fixed::types::I16F16;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    SpawnEntity {
        entity_id: u64,
        blueprint: String,
        position: DeterministicVector2,
        velocity: Option<DeterministicVector2>,
        entity_type: String,
        move_speed: I16F16,
        radius: I16F16,
        is_sensor: Option<bool>,
        lifetime_ticks: Option<u32>,
        collision_layer: Option<u16>,
        collision_mask: Option<u16>,
        properties: std::collections::BTreeMap<String, String>,
    },
    SetSensor {
        entity_id: u64,
        is_sensor: bool,
    },
    SetLifetime {
        entity_id: u64,
        lifetime_ticks: Option<u32>,
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
    SetCollisionFilter {
        entity_id: u64,
        layer: u16,
        mask: u16,
    },
    SpawnStaticObstacle {
        obstacle_id: u64,
        blueprint: String,
        shape: ColliderShape,
        filter: CollisionFilter,
        is_solid: bool,
        collision_type: String,
        properties: std::collections::BTreeMap<String, String>,
    },
    DestroyStaticObstacle {
        obstacle_id: u64,
    },
    SetObstacleProperty {
        obstacle_id: u64,
        key: String,
        value: String,
    },
    SetMapBounds {
        bounds: MapBounds,
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
                    velocity,
                    entity_type,
                    move_speed,
                    radius,
                    is_sensor,
                    lifetime_ticks,
                    collision_layer,
                    collision_mask,
                    properties,
                } => {
                    let parsed_type = match entity_type.as_str() {
                        "Player" | "player" => crate::world::entity::EntityType::Player,
                        "Enemy" | "NPC" | "npc" | "enemy" => crate::world::entity::EntityType::NPC,
                        "Projectile" | "projectile" => crate::world::entity::EntityType::Projectile,
                        "Prop" | "prop" => crate::world::entity::EntityType::Prop,
                        unknown => {
                            if instance.logging_enabled {
                                crate::log_warn!(
                                    "CommandBuffer",
                                    "Unknown entity_type '{}', defaulting to Prop",
                                    unknown
                                );
                            }
                            crate::world::entity::EntityType::Prop
                        }
                    };

                    let mut entity = crate::world::entity::Entity::new(
                        entity_id,
                        blueprint.clone(),
                        parsed_type,
                    )
                    .with_circle_collider(radius);

                    if parsed_type != crate::world::entity::EntityType::Projectile {
                        entity = entity.with_default_navigation(move_speed, I16F16::from_num(1));
                    }

                    if let Some(vel) = velocity {
                        entity.velocity = vel;
                    }
                    if let Some(sensor) = is_sensor {
                        entity.is_sensor = sensor;
                    }
                    entity.remaining_lifetime_ticks = lifetime_ticks.filter(|&t| t > 0);

                    if collision_layer.is_some() || collision_mask.is_some() {
                        entity.collision_filter = CollisionFilter::new(
                            collision_layer.unwrap_or(entity.collision_filter.layer),
                            collision_mask.unwrap_or(entity.collision_filter.mask),
                        );
                    }

                    for (k, v) in properties {
                        entity.properties.insert(k, v);
                    }

                    let pos_for_log = position;
                    entity.position = position;

                    instance.entities.insert(entity_id, entity);

                    if instance.logging_enabled {
                        crate::log_debug!(
                            "CommandBuffer",
                            "Spawned entity {} from blueprint '{}' at ({}, {})",
                            entity_id,
                            blueprint,
                            pos_for_log.x.to_num::<f64>(),
                            pos_for_log.y.to_num::<f64>()
                        );
                    }
                }
                Command::SetSensor {
                    entity_id,
                    is_sensor,
                } => {
                    if let Some(entity) = instance.entities.get_mut(&entity_id) {
                        entity.is_sensor = is_sensor;
                    }
                }
                Command::SetLifetime {
                    entity_id,
                    lifetime_ticks,
                } => {
                    if let Some(entity) = instance.entities.get_mut(&entity_id) {
                        entity.remaining_lifetime_ticks = lifetime_ticks.filter(|&t| t > 0);
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
                            if instance.logging_enabled {
                                crate::log_warn!(
                                    "Lua",
                                    "SetNavigationTarget called on entity {} without a NavigationComponent; creating one with speed 0.0",
                                    entity_id
                                );
                            }
                            crate::world::physics::navigation::NavigationComponent::new(
                                I16F16::from_num(0),
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
                    if let Some(entity) = instance.entities.get_mut(&entity_id)
                        && let Some(ref mut nav) = entity.navigation {
                            nav.move_speed = speed;
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
                        crate::log_debug!(
                            "CommandBuffer",
                            "Broadcasting event '{}' with data '{}'",
                            event_name,
                            data
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
                Command::SetCollisionFilter { entity_id, layer, mask } => {
                    if let Some(entity) = instance.entities.get_mut(&entity_id) {
                        entity.collision_filter = CollisionFilter::new(layer, mask);
                        if instance.logging_enabled {
                            crate::log_debug!(
                                "CommandBuffer",
                                "SetCollisionFilter entity={} layer={} mask={}",
                                entity_id,
                                layer,
                                mask
                            );
                        }
                    }
                }
                Command::SpawnStaticObstacle {
                    obstacle_id,
                    blueprint,
                    shape,
                    filter,
                    is_solid,
                    collision_type,
                    properties,
                } => {
                    let obstacle = StaticObstacle::new(obstacle_id, shape, filter, is_solid)
                        .with_blueprint(blueprint.clone())
                        .with_collision_type(collision_type)
                        .with_properties(properties);
                    instance.add_static_obstacle(obstacle);
                    if instance.logging_enabled {
                        println!(
                            "[CommandBuffer] Spawned static obstacle {} from blueprint '{}' (is_solid={})",
                            obstacle_id, blueprint, is_solid
                        );
                    }
                }
                Command::DestroyStaticObstacle { obstacle_id } => {
                    instance.remove_static_obstacle(obstacle_id);
                    if instance.logging_enabled {
                        println!("[CommandBuffer] Destroyed static obstacle {}", obstacle_id);
                    }
                }
                Command::SetObstacleProperty {
                    obstacle_id,
                    key,
                    value,
                } => {
                    if let Some(obstacle) = instance.static_obstacles.get_mut(&obstacle_id) {
                        obstacle.properties.insert(key, value);
                    }
                }
                Command::SetMapBounds { bounds } => {
                    instance.set_map_bounds(bounds);
                }
            }
        }
    }
}
