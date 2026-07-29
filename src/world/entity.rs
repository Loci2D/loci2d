// Entity module - Players, NPCs, positions (Vector2)
// TODO: Implement entity types, positions, and game object logic

use super::instance::Vector2;

#[derive(Debug, Clone)]
pub struct Entity {
    pub id: u64,
    pub position: Vector2,
    pub entity_type: EntityType,
}

#[derive(Debug, Clone)]
pub enum EntityType {
    Player,
    NPC,
    Prop,
}
