// Entity module - Players, NPCs, positions (DeterministicVector2)

use super::fixed_point::DeterministicVector2;

// [2026-08-08] Allowed dead_code: fields like id and entity_type are part of the core domain model
// and will be read during snapshot serialization (Phase 3) and collision/event systems (Phases 4-6).
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entity {
    pub id: u64,
    pub name: String,
    pub position: DeterministicVector2,
    pub velocity: DeterministicVector2,
    pub entity_type: EntityType,
}

impl Entity {
    pub fn new(id: u64, name: String, entity_type: EntityType) -> Self {
        Self {
            id,
            name,
            position: DeterministicVector2::ZERO,
            velocity: DeterministicVector2::ZERO,
            entity_type,
        }
    }
}

// [2026-08-08] Allowed dead_code: NPC and Prop entity variants are reserved for upcoming world simulation phases.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityType {
    Player,
    NPC,
    Prop,
}

impl EntityType {
    pub fn as_u8(&self) -> u8 {
        match self {
            EntityType::Player => 0,
            EntityType::NPC => 1,
            EntityType::Prop => 2,
        }
    }
}
