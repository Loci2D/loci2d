use crate::world::instance::DeterministicVector2;
use crate::world::instance::Instance;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    SpawnEntity {
        blueprint: String,
        position: DeterministicVector2,
    },
    // Future commands could go here
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
                    // For Phase 6.3, we'll just log this intent as a demonstration.
                    // True blueprint handling and dynamic entity spawning is part of Phase 7 content definitions.
                    if instance.logging_enabled {
                        println!("[CommandBuffer] Spawning entity from blueprint '{}' at ({}, {})", 
                                 blueprint, 
                                 position.x.to_num::<f64>(), 
                                 position.y.to_num::<f64>());
                    }
                }
            }
        }
    }
}
