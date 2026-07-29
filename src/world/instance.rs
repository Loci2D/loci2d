// Instance module - Logic for a specific room/instance (tick rate, entity list)
// TODO: Implement instance management, tick rate control, and entity lists

use std::collections::HashMap;
use super::entity::Entity;

// Re-export Vector2 from network for now
pub use crate::network::packets::Vector2;

#[derive(Debug)]
pub struct Instance {
    pub id: u64,
    pub entities: HashMap<u64, Entity>,
    pub tick_rate: u32, // ticks per second
}

impl Instance {
    pub fn new(id: u64, tick_rate: u32) -> Self {
        Self {
            id,
            entities: HashMap::new(),
            tick_rate,
        }
    }

    pub fn add_entity(&mut self, entity: Entity) {
        self.entities.insert(entity.id, entity);
    }

    pub fn remove_entity(&mut self, entity_id: u64) -> Option<Entity> {
        self.entities.remove(&entity_id)
    }

    pub fn get_entity(&self, entity_id: u64) -> Option<&Entity> {
        self.entities.get(&entity_id)
    }
}
