use crate::world::instance::DeterministicVector2;
use crate::world::instance::Instance;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    SpawnEntity {
        blueprint: String,
        position: DeterministicVector2,
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
                Command::SpawnEntity { blueprint, position } => {
                    // TODO(Phase X): Implement actual entity spawning from blueprint
                    if instance.logging_enabled {
                        println!(
                            "[CommandBuffer] Spawning entity from blueprint '{}' at ({}, {})",
                            blueprint,
                            position.x.to_num::<f64>(),
                            position.y.to_num::<f64>()
                        );
                    }
                }
                Command::DestroyEntity { entity_id } => {
                    instance.entities.remove(&entity_id);
                }
                Command::SetPosition { entity_id, position } => {
                    if let Some(entity) = instance.entities.get_mut(&entity_id) {
                        entity.position = position;
                    }
                }
                Command::SetVelocity { entity_id, velocity } => {
                    if let Some(entity) = instance.entities.get_mut(&entity_id) {
                        entity.velocity = velocity;
                    }
                }
                Command::SetEntityProperty { entity_id, key, value } => {
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
            }
        }
    }
}
