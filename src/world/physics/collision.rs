// 2D intersection algorithms and Minimum Translation Vector (MTV) manifold calculation.
// Guarantees 100% bit-exact determinism across platforms (ADR-0007, ADR-0012).

use crate::world::fixed_point::DeterministicVector2;
use crate::world::physics::math::deterministic_distance;
use crate::world::physics::primitives::{ColliderShape, DeterministicAABB, DeterministicCircle};
use fixed::types::I16F16;

/// Contact manifold resulting from narrowphase intersection test.
///
/// **Convention**: `normal` is the separation direction that points **from B to A**.
/// Moving A by `+normal * penetration_depth` separates A from B.
/// Moving B by `-normal * penetration_depth` separates B from A.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContactManifold {
    pub is_colliding: bool,
    pub normal: DeterministicVector2,
    pub penetration_depth: I16F16,
}

impl ContactManifold {
    pub const NONE: Self = Self {
        is_colliding: false,
        normal: DeterministicVector2::ZERO,
        penetration_depth: I16F16::ZERO,
    };
}

/// Intersects two Axis-Aligned Bounding Boxes.
/// Normal points from B to A (direction to push A out of B).
pub fn intersect_aabb_aabb(a: &DeterministicAABB, b: &DeterministicAABB) -> ContactManifold {
    let overlap_x = a.max.x.min(b.max.x) - a.min.x.max(b.min.x);
    let overlap_y = a.max.y.min(b.max.y) - a.min.y.max(b.min.y);

    if overlap_x <= I16F16::ZERO || overlap_y <= I16F16::ZERO {
        return ContactManifold::NONE;
    }

    let a_center = a.center();
    let b_center = b.center();

    if a_center == b_center {
        ContactManifold {
            is_colliding: true,
            normal: DeterministicVector2::UNIT_X, // Deterministic fallback: (1, 0)
            penetration_depth: overlap_x,
        }
    } else if overlap_x < overlap_y {
        let sign_x = if a_center.x >= b_center.x {
            I16F16::ONE
        } else {
            -I16F16::ONE
        };
        ContactManifold {
            is_colliding: true,
            normal: DeterministicVector2::new(sign_x, I16F16::ZERO),
            penetration_depth: overlap_x,
        }
    } else {
        let sign_y = if a_center.y >= b_center.y {
            I16F16::ONE
        } else {
            -I16F16::ONE
        };
        ContactManifold {
            is_colliding: true,
            normal: DeterministicVector2::new(I16F16::ZERO, sign_y),
            penetration_depth: overlap_y,
        }
    }
}

/// Intersects two Circles.
/// Normal points from B to A (direction to push A out of B).
pub fn intersect_circle_circle(
    a: &DeterministicCircle,
    b: &DeterministicCircle,
) -> ContactManifold {
    let dist = deterministic_distance(a.center, b.center);
    let radii_sum = a.radius + b.radius;

    if dist >= radii_sum {
        return ContactManifold::NONE;
    }

    if dist == I16F16::ZERO {
        // Zero-distance fallback: concentric circles separate along +X
        ContactManifold {
            is_colliding: true,
            normal: DeterministicVector2::UNIT_X,
            penetration_depth: radii_sum,
        }
    } else {
        let normal = (a.center - b.center) / dist;
        let penetration_depth = radii_sum - dist;
        ContactManifold {
            is_colliding: true,
            normal,
            penetration_depth,
        }
    }
}

/// Intersects a Circle (A) with an AABB (B).
/// Normal points from AABB (B) to Circle (A) (direction to push Circle out of AABB).
pub fn intersect_circle_aabb(
    circle: &DeterministicCircle,
    aabb: &DeterministicAABB,
) -> ContactManifold {
    let px = circle.center.x.clamp(aabb.min.x, aabb.max.x);
    let py = circle.center.y.clamp(aabb.min.y, aabb.max.y);
    let closest_point = DeterministicVector2::new(px, py);

    if closest_point != circle.center {
        // Circle center is outside AABB
        let dist = deterministic_distance(circle.center, closest_point);
        if dist >= circle.radius {
            return ContactManifold::NONE;
        }

        let normal = if dist == I16F16::ZERO {
            DeterministicVector2::UNIT_X
        } else {
            (circle.center - closest_point) / dist
        };
        let penetration_depth = circle.radius - dist;

        ContactManifold {
            is_colliding: true,
            normal,
            penetration_depth,
        }
    } else {
        // Circle center is strictly inside AABB
        let dist_left = circle.center.x - aabb.min.x;
        let dist_right = aabb.max.x - circle.center.x;
        let dist_bottom = circle.center.y - aabb.min.y;
        let dist_top = aabb.max.y - circle.center.y;

        // Find minimum distance with deterministic tie-break ordering (+X, -X, +Y, -Y)
        let mut min_dist = dist_right;
        let mut normal = DeterministicVector2::UNIT_X;

        if dist_left < min_dist {
            min_dist = dist_left;
            normal = -DeterministicVector2::UNIT_X;
        }
        if dist_top < min_dist {
            min_dist = dist_top;
            normal = DeterministicVector2::UNIT_Y;
        }
        if dist_bottom < min_dist {
            min_dist = dist_bottom;
            normal = -DeterministicVector2::UNIT_Y;
        }

        ContactManifold {
            is_colliding: true,
            normal,
            penetration_depth: circle.radius + min_dist,
        }
    }
}

/// Intersects an AABB (A) with a Circle (B).
/// Normal points from Circle (B) to AABB (A) (direction to push AABB out of Circle).
pub fn intersect_aabb_circle(
    aabb: &DeterministicAABB,
    circle: &DeterministicCircle,
) -> ContactManifold {
    let manifold = intersect_circle_aabb(circle, aabb);
    if !manifold.is_colliding {
        ContactManifold::NONE
    } else {
        ContactManifold {
            is_colliding: true,
            normal: -manifold.normal,
            penetration_depth: manifold.penetration_depth,
        }
    }
}

/// Unified narrowphase intersection dispatch between any two collider shapes.
/// Normal points from shape_b to shape_a.
pub fn intersect_shapes(shape_a: &ColliderShape, shape_b: &ColliderShape) -> ContactManifold {
    match (shape_a, shape_b) {
        (ColliderShape::AABB(a), ColliderShape::AABB(b)) => intersect_aabb_aabb(a, b),
        (ColliderShape::Circle(a), ColliderShape::Circle(b)) => intersect_circle_circle(a, b),
        (ColliderShape::AABB(a), ColliderShape::Circle(b)) => intersect_aabb_circle(a, b),
        (ColliderShape::Circle(a), ColliderShape::AABB(b)) => intersect_circle_aabb(a, b),
    }
}
