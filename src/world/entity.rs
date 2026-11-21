use super::fixed_point::DeterministicVector2;
use super::physics::{
    ColliderShape, CollisionFilter, DeterministicAABB, DeterministicCircle, NavigationComponent,
};
use fixed::types::I16F16;
use std::collections::BTreeMap;

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
    pub navigation: Option<NavigationComponent>,
    // Phase 6.5.0 Additions (Data-Driven Properties)
    pub properties: BTreeMap<String, String>,
    // ADR-0021 Additions (Physical Projectiles & Sensors)
    pub is_sensor: bool,
    pub remaining_lifetime_ticks: Option<u32>,
}

impl Entity {
    pub fn new(id: u64, name: String, entity_type: EntityType) -> Self {
        let collision_filter = if entity_type == EntityType::Projectile {
            CollisionFilter::default_projectile()
        } else {
            CollisionFilter::default_player()
        };
        let is_sensor = matches!(entity_type, EntityType::Projectile);
        Self {
            id,
            name,
            position: DeterministicVector2::ZERO,
            velocity: DeterministicVector2::ZERO,
            entity_type,
            collider: None,
            collision_filter,
            navigation: None,
            properties: BTreeMap::new(),
            is_sensor,
            remaining_lifetime_ticks: None,
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

    pub fn with_navigation(mut self, nav: NavigationComponent) -> Self {
        self.navigation = Some(nav);
        self
    }

    pub fn with_default_navigation(
        mut self,
        move_speed: I16F16,
        arrival_tolerance: I16F16,
    ) -> Self {
        self.navigation = Some(NavigationComponent::new(move_speed, arrival_tolerance));
        self
    }

    pub fn with_sensor(mut self, is_sensor: bool) -> Self {
        self.is_sensor = is_sensor;
        self
    }

    pub fn with_lifetime_ticks(mut self, ticks: Option<u32>) -> Self {
        self.remaining_lifetime_ticks = ticks;
        self
    }

    /// Returns the entity's collider shape translated to its current position.
    pub fn current_collider(&self) -> Option<ColliderShape> {
        self.collider.map(|shape| shape.at_position(self.position))
    }
}

// [2026-08-08] Allowed dead_code: NPC and Prop entity variants are reserved for upcoming world simulation phases.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityType {
    Player,
    NPC,
    Prop,
    Projectile,
}

impl EntityType {
    pub fn as_u8(&self) -> u8 {
        match self {
            EntityType::Player => 0,
            EntityType::NPC => 1,
            EntityType::Prop => 2,
            EntityType::Projectile => 3,
        }
    }
}
