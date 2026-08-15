// Static map geometry, collision filtering, and world boundary clamping.
// Guarantees 100% bit-exact determinism across platforms (ADR-0003, ADR-0007, ADR-0012).

use fixed::types::I16F16;
use serde::{Deserialize, Serialize};
use crate::world::fixed_point::DeterministicVector2;
use crate::world::physics::primitives::{ColliderShape, DeterministicAABB};

/// 16-bit bitmask filter for collision layers and masks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CollisionFilter {
    pub layer: u16,
    pub mask: u16,
}

impl CollisionFilter {
    pub const NONE: u16 = 0;
    pub const SOLID_WALL: u16 = 1 << 0;
    pub const PLAYER: u16 = 1 << 1;
    pub const TRIGGER_ZONE: u16 = 1 << 2;
    pub const PROJECTILE: u16 = 1 << 3;

    pub const fn new(layer: u16, mask: u16) -> Self {
        Self { layer, mask }
    }

    pub const fn default_solid_wall() -> Self {
        Self {
            layer: Self::SOLID_WALL,
            mask: Self::PLAYER | Self::PROJECTILE,
        }
    }

    pub const fn default_player() -> Self {
        Self {
            layer: Self::PLAYER,
            mask: Self::SOLID_WALL | Self::PLAYER | Self::TRIGGER_ZONE | Self::PROJECTILE,
        }
    }

    pub const fn default_trigger_zone() -> Self {
        Self {
            layer: Self::TRIGGER_ZONE,
            mask: Self::PLAYER,
        }
    }

    pub const fn default_projectile() -> Self {
        Self {
            layer: Self::PROJECTILE,
            mask: Self::SOLID_WALL | Self::PLAYER,
        }
    }

    /// Bidirectional layer-mask collision check.
    /// Collision can only occur if A's mask includes B's layer AND B's mask includes A's layer.
    #[inline]
    pub const fn can_collide(&self, other: &Self) -> bool {
        (self.mask & other.layer) != 0 && (other.mask & self.layer) != 0
    }
}

impl Default for CollisionFilter {
    fn default() -> Self {
        Self::default_player()
    }
}

/// Static obstacle representing walls, pillars, or trigger sensors in the map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticObstacle {
    pub id: u64,
    pub shape: ColliderShape,
    pub filter: CollisionFilter,
    pub is_solid: bool, // True for physical blockers/walls, False for non-solid trigger sensors
}

impl StaticObstacle {
    pub fn new(id: u64, shape: ColliderShape, filter: CollisionFilter, is_solid: bool) -> Self {
        Self {
            id,
            shape,
            filter,
            is_solid,
        }
    }

    pub fn solid_wall(id: u64, shape: ColliderShape) -> Self {
        Self {
            id,
            shape,
            filter: CollisionFilter::default_solid_wall(),
            is_solid: true,
        }
    }

    pub fn trigger_zone(id: u64, shape: ColliderShape) -> Self {
        Self {
            id,
            shape,
            filter: CollisionFilter::default_trigger_zone(),
            is_solid: false,
        }
    }
}

/// Axis-Aligned rectangular map boundary defining the playable arena limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapBounds {
    pub min: DeterministicVector2,
    pub max: DeterministicVector2,
}

impl MapBounds {
    pub fn new(min: DeterministicVector2, max: DeterministicVector2) -> Self {
        assert!(min.x <= max.x && min.y <= max.y, "Invalid map bounds: min must be <= max");
        Self { min, max }
    }

    /// Default arena boundaries: [-500, +500] along X and Y axes (1000x1000 units playable area).
    pub fn default_arena() -> Self {
        Self {
            min: DeterministicVector2::new(I16F16::from_num(-500), I16F16::from_num(-500)),
            max: DeterministicVector2::new(I16F16::from_num(500), I16F16::from_num(500)),
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
    pub fn contains_point(&self, point: DeterministicVector2) -> bool {
        point.x >= self.min.x && point.x <= self.max.x && point.y >= self.min.y && point.y <= self.max.y
    }

    #[inline]
    pub fn to_aabb(&self) -> DeterministicAABB {
        DeterministicAABB::new(self.min, self.max)
    }

    /// Clamps a single point position within the map boundary limits.
    #[inline]
    pub fn clamp_point(&self, point: DeterministicVector2) -> DeterministicVector2 {
        DeterministicVector2::new(
            point.x.clamp(self.min.x, self.max.x),
            point.y.clamp(self.min.y, self.max.y),
        )
    }

    /// Clamps a circle's center so the entire circular area remains inside the map boundaries.
    /// If arena dimensions are smaller than 2 * radius, centers to the midpoint safely without panicking.
    pub fn clamp_circle(&self, center: DeterministicVector2, radius: I16F16) -> DeterministicVector2 {
        let (min_x, max_x) = if self.max.x - self.min.x <= radius * 2 {
            let mid = (self.min.x + self.max.x) / 2;
            (mid, mid)
        } else {
            (self.min.x + radius, self.max.x - radius)
        };

        let (min_y, max_y) = if self.max.y - self.min.y <= radius * 2 {
            let mid = (self.min.y + self.max.y) / 2;
            (mid, mid)
        } else {
            (self.min.y + radius, self.max.y - radius)
        };

        DeterministicVector2::new(
            center.x.clamp(min_x, max_x),
            center.y.clamp(min_y, max_y),
        )
    }

    /// Clamps an AABB's center so the entire rectangular area remains inside the map boundaries.
    /// If arena dimensions are smaller than 2 * half_extents, centers to the midpoint safely without panicking.
    pub fn clamp_aabb(&self, center: DeterministicVector2, half_extents: DeterministicVector2) -> DeterministicVector2 {
        let (min_x, max_x) = if self.max.x - self.min.x <= half_extents.x * 2 {
            let mid = (self.min.x + self.max.x) / 2;
            (mid, mid)
        } else {
            (self.min.x + half_extents.x, self.max.x - half_extents.x)
        };

        let (min_y, max_y) = if self.max.y - self.min.y <= half_extents.y * 2 {
            let mid = (self.min.y + self.max.y) / 2;
            (mid, mid)
        } else {
            (self.min.y + half_extents.y, self.max.y - half_extents.y)
        };

        DeterministicVector2::new(
            center.x.clamp(min_x, max_x),
            center.y.clamp(min_y, max_y),
        )
    }

    /// Clamps an entity's center based on its `ColliderShape`.
    #[inline]
    pub fn clamp_shape(&self, shape: &ColliderShape) -> DeterministicVector2 {
        match shape {
            ColliderShape::AABB(aabb) => self.clamp_aabb(aabb.center(), aabb.half_extents()),
            ColliderShape::Circle(circle) => self.clamp_circle(circle.center, circle.radius),
        }
    }
}

impl Default for MapBounds {
    fn default() -> Self {
        Self::default_arena()
    }
}
