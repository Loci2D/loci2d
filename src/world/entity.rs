use fixed::types::I16F16;
use super::fixed_point::DeterministicVector2;
use super::physics::{ColliderShape, CollisionFilter, DeterministicAABB, DeterministicCircle};

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
    // Phase 5 Additions:
    pub collider: Option<ColliderShape>,
    pub collision_filter: CollisionFilter,
}

impl Entity {
    pub fn new(id: u64, name: String, entity_type: EntityType) -> Self {
        Self {
            id,
            name,
            position: DeterministicVector2::ZERO,
            velocity: DeterministicVector2::ZERO,
            entity_type,
            collider: None,
            collision_filter: CollisionFilter::default_player(),
        }
    }

    pub fn with_collider(mut self, collider: ColliderShape) -> Self {
        self.collider = Some(collider);
        self
    }

    pub fn with_circle_collider(mut self, radius: I16F16) -> Self {
        self.collider = Some(ColliderShape::Circle(DeterministicCircle::new(
            self.position,
            radius,
        )));
        self
    }

    pub fn with_aabb_collider(mut self, half_extents: DeterministicVector2) -> Self {
        self.collider = Some(ColliderShape::AABB(
            DeterministicAABB::from_center_half_extents(self.position, half_extents),
        ));
        self
    }

    pub fn with_collision_filter(mut self, filter: CollisionFilter) -> Self {
        self.collision_filter = filter;
        self
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
