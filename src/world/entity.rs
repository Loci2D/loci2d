// Entity module - Players, NPCs, positions (Vector2)
// TODO: Implement entity types, positions, and game object logic

use super::instance::Vector2;

#[derive(Debug, Clone)]
pub struct Entity {
    pub id: u64,
    pub position: Vector2,
    pub velocity: Vector2,
    pub entity_type: EntityType,
}

impl Entity {
    pub fn new(id: u64, entity_type: EntityType) -> Self {
        Self {
            id,
            position: Vector2 { x: 0.0, y: 0.0 },
            velocity: Vector2 { x: 0.0, y: 0.0 },
            entity_type,
        }
    }
}

#[derive(Debug, Clone)]
pub enum EntityType {
    Player,
    NPC,
    Prop,
}

