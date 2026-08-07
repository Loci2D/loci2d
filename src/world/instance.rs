// Instance module - Logic for a specific room/instance (tick rate, entity list)
// TODO: Implement instance management, tick rate control, and entity lists

use std::collections::HashMap;
use std::net::SocketAddr;
use super::entity::{Entity, EntityType};
use crate::network::packets::ClientIntent;

// Re-export Vector2 from network for now
pub use crate::network::packets::Vector2;

#[derive(Debug)]
pub struct Instance {
    pub id: u64,
    pub entities: HashMap<u64, Entity>,
    pub tick_rate: u32, // ticks per second
    pub client_map: HashMap<SocketAddr, u64>,
    next_entity_id: u64,
}

impl Instance {
    pub fn new(id: u64, tick_rate: u32) -> Self {
        Self {
            id,
            entities: HashMap::new(),
            tick_rate,
            client_map: HashMap::new(),
            next_entity_id: 1,
        }
    }

    pub fn get_or_create_entity(&mut self, addr: SocketAddr) -> u64 {
        if let Some(&entity_id) = self.client_map.get(&addr) {
            entity_id
        } else {
            let entity_id = self.next_entity_id;
            self.next_entity_id += 1;
            let entity = Entity::new(entity_id, EntityType::Player);
            self.entities.insert(entity_id, entity);
            self.client_map.insert(addr, entity_id);
            println!("[Auto-Join] New client {} mapped to EntityId {}", addr, entity_id);
            entity_id
        }
    }

    pub fn apply_intent(&mut self, addr: SocketAddr, intent: ClientIntent) {
        use crate::network::packets::client_intent::Intent;

        let entity_id = self.get_or_create_entity(addr);

        if let Some(entity) = self.entities.get_mut(&entity_id) {
            match intent.intent {
                Some(Intent::Move(move_intent)) => {
                    if let Some(dir) = move_intent.direction {
                        entity.velocity = Vector2 { x: dir.x, y: dir.y };
                    }
                }
                Some(Intent::Action(action_intent)) => {
                    println!("[Intent] Entity {} executed action {}", entity_id, action_intent.ability_id);
                }
                Some(Intent::Ping(_)) => {
                    println!("[Intent] Entity {} sent ping", entity_id);
                }
                None => {}
            }
        }
    }

    pub fn tick(&mut self, tick_count: u64) {
        for entity in self.entities.values_mut() {
            entity.position.x += entity.velocity.x;
            entity.position.y += entity.velocity.y;
        }
        println!("[Tick {}] {} active entities", tick_count, self.entities.len());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::packets::{client_intent, ClientIntent, MoveIntent};

    #[test]
    fn test_apply_move_intent_and_tick() {
        let mut instance = Instance::new(1, 30);
        let addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();

        let move_intent = ClientIntent {
            intent: Some(client_intent::Intent::Move(MoveIntent {
                direction: Some(Vector2 { x: 2.5, y: -1.0 }),
            })),
        };

        instance.apply_intent(addr, move_intent);

        let entity_id = *instance.client_map.get(&addr).expect("Client should be mapped");
        let entity = instance.get_entity(entity_id).expect("Entity should exist");
        assert_eq!(entity.velocity.x, 2.5);
        assert_eq!(entity.velocity.y, -1.0);
        assert_eq!(entity.position.x, 0.0);
        assert_eq!(entity.position.y, 0.0);

        instance.tick(1);

        let updated_entity = instance.get_entity(entity_id).unwrap();
        assert_eq!(updated_entity.position.x, 2.5);
        assert_eq!(updated_entity.position.y, -1.0);
    }
}


