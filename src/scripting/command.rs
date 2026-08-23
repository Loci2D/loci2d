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
    ApplyDamage {
        entity_id: u64,
        amount: i32,
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
                Command::ApplyDamage { entity_id, amount } => {
                    if instance.logging_enabled {
                        println!(
                            "[CommandBuffer] Entity {} received {} damage",
                            entity_id, amount
                        );
                    }
                }
                Command::SendEvent { event_name, data } => {
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
