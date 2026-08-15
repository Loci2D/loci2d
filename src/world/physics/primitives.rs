// Geometric primitives for 2D deterministic collision detection (ADR-0003, ADR-0012).

use fixed::types::I16F16;
use serde::{Deserialize, Serialize};
use crate::world::fixed_point::DeterministicVector2;
use crate::world::physics::math::deterministic_distance;

/// Axis-Aligned Bounding Box defined by min and max extents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DeterministicAABB {
    pub min: DeterministicVector2,
    pub max: DeterministicVector2,
}

impl DeterministicAABB {
    pub fn new(min: DeterministicVector2, max: DeterministicVector2) -> Self {
        assert!(min.x <= max.x && min.y <= max.y, "Invalid AABB bounds: min must be <= max");
        Self { min, max }
    }

    pub fn from_center_half_extents(center: DeterministicVector2, half_extents: DeterministicVector2) -> Self {
        Self {
            min: DeterministicVector2::new(
                center.x.saturating_sub(half_extents.x),
                center.y.saturating_sub(half_extents.y),
            ),
            max: DeterministicVector2::new(
                center.x.saturating_add(half_extents.x),
                center.y.saturating_add(half_extents.y),
            ),
        }
    }

    #[inline]
    pub fn width(&self) -> I16F16 {
        self.max.x - self.min.x
    }

    #[inline]
    pub fn height(&self) -> I16F16 {
        self.max.y - self.min.y
    }

    #[inline]
    pub fn center(&self) -> DeterministicVector2 {
        DeterministicVector2::new(
            (self.min.x + self.max.x) / 2,
            (self.min.y + self.max.y) / 2,
        )
    }

    #[inline]
    pub fn half_extents(&self) -> DeterministicVector2 {
        DeterministicVector2::new(self.width() / 2, self.height() / 2)
    }

    #[inline]
    pub fn contains_point(&self, point: DeterministicVector2) -> bool {
        point.x >= self.min.x && point.x <= self.max.x && point.y >= self.min.y && point.y <= self.max.y
    }

    #[inline]
    pub fn intersects_aabb(&self, other: &Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }

    #[inline]
    pub fn bounding_box(&self) -> DeterministicAABB {
        *self
    }
}

/// Circle collider defined by center position and radius.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DeterministicCircle {
    pub center: DeterministicVector2,
    pub radius: I16F16,
}

impl DeterministicCircle {
    pub fn new(center: DeterministicVector2, radius: I16F16) -> Self {
        assert!(radius >= I16F16::ZERO, "Collider radius must be non-negative");
        Self { center, radius }
    }

    #[inline]
    pub fn contains_point(&self, point: DeterministicVector2) -> bool {
        deterministic_distance(self.center, point) <= self.radius
    }

    #[inline]
    pub fn bounding_box(&self) -> DeterministicAABB {
        DeterministicAABB::from_center_half_extents(
            self.center,
            DeterministicVector2::new(self.radius, self.radius),
        )
    }
}

/// Unified Collider Shape representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ColliderShape {
    AABB(DeterministicAABB),
    Circle(DeterministicCircle),
}

impl ColliderShape {
    #[inline]
    pub fn bounding_box(&self) -> DeterministicAABB {
        match self {
            ColliderShape::AABB(aabb) => aabb.bounding_box(),
            ColliderShape::Circle(circle) => circle.bounding_box(),
        }
    }

    #[inline]
    pub fn center(&self) -> DeterministicVector2 {
        match self {
            ColliderShape::AABB(aabb) => aabb.center(),
            ColliderShape::Circle(circle) => circle.center,
        }
    }
}
