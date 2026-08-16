// 2D intersection algorithms and Minimum Translation Vector (MTV) manifold calculation.
// Guarantees 100% bit-exact determinism across platforms (ADR-0007, ADR-0012).

use crate::world::fixed_point::DeterministicVector2;
use crate::world::physics::map::StaticObstacle;
use crate::world::physics::math::{deterministic_distance, integer_sqrt_u64};
use crate::world::physics::primitives::{ColliderShape, DeterministicAABB, DeterministicCircle};
use fixed::types::I16F16;
use std::collections::BTreeMap;

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

/// Kinematic Minimum Translation Vector (MTV) pushback and wall-sliding resolution
/// for a dynamic entity colliding against a static obstacle.
///
/// Pushes the entity out of the static obstacle by 100% of the penetration depth
/// and projects the velocity along the contact tangent to produce smooth sliding.
pub fn resolve_static_collision(
    pos: &mut DeterministicVector2,
    vel: &mut DeterministicVector2,
    manifold: &ContactManifold,
) {
    if !manifold.is_colliding || manifold.penetration_depth <= I16F16::ZERO {
        return;
    }
    if manifold.normal == DeterministicVector2::ZERO {
        return;
    }

    pos.x += manifold.normal.x * manifold.penetration_depth;
    pos.y += manifold.normal.y * manifold.penetration_depth;

    let dot = vel.dot(manifold.normal);
    if dot < I16F16::ZERO {
        vel.x -= manifold.normal.x * dot;
        vel.y -= manifold.normal.y * dot;
    }
}

/// Kinematic Minimum Translation Vector (MTV) 50/50 split pushback and velocity projection
/// for two dynamic entities colliding against each other.
///
/// Distributes penetration depth equally between entity A and entity B, protecting against
/// integer division LSB loss by giving any odd remainder unit to entity A.
pub fn resolve_dynamic_collision(
    pos_a: &mut DeterministicVector2,
    vel_a: &mut DeterministicVector2,
    pos_b: &mut DeterministicVector2,
    vel_b: &mut DeterministicVector2,
    manifold: &ContactManifold,
) {
    if !manifold.is_colliding || manifold.penetration_depth <= I16F16::ZERO {
        return;
    }
    if manifold.normal == DeterministicVector2::ZERO {
        return;
    }

    // Fixing LSB (Least Significant Bit) loss in integer division
    let half_depth = manifold.penetration_depth / 2;
    let remainder = manifold.penetration_depth - (half_depth * I16F16::from_num(2));
    let push_a = half_depth + remainder; // Ensuring 100% total separation
    let push_b = half_depth;

    pos_a.x += manifold.normal.x * push_a;
    pos_a.y += manifold.normal.y * push_a;
    pos_b.x -= manifold.normal.x * push_b;
    pos_b.y -= manifold.normal.y * push_b;

    let dot_a = vel_a.dot(manifold.normal);
    if dot_a < I16F16::ZERO {
        vel_a.x -= manifold.normal.x * dot_a;
        vel_a.y -= manifold.normal.y * dot_a;
    }

    let normal_b = -manifold.normal;
    let dot_b = vel_b.dot(normal_b);
    if dot_b < I16F16::ZERO {
        vel_b.x -= normal_b.x * dot_b;
        vel_b.y -= normal_b.y * dot_b;
    }
}

/// Result of a deterministic 2D raycast query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RaycastHit {
    pub point: DeterministicVector2,
    pub normal: DeterministicVector2,
    pub fraction: I16F16, // Range [0.0, 1.0] representing distance / max_distance
    pub obstacle_id: u64,
}

fn ray_intersect_aabb(
    origin: DeterministicVector2,
    dir: DeterministicVector2,
    max_distance: I16F16,
    aabb: &DeterministicAABB,
) -> Option<(I16F16, DeterministicVector2)> {
    if aabb.contains_point(origin) {
        return Some((I16F16::ZERO, -dir));
    }

    let mut t_min = -I16F16::MAX;
    let mut t_max = I16F16::MAX;
    let mut normal = DeterministicVector2::ZERO;

    // X slab
    if dir.x != I16F16::ZERO {
        let t1 = (aabb.min.x - origin.x) / dir.x;
        let t2 = (aabb.max.x - origin.x) / dir.x;
        let (t_enter_x, t_exit_x, norm_x) = if t1 < t2 {
            (t1, t2, DeterministicVector2::new(-I16F16::ONE, I16F16::ZERO))
        } else {
            (t2, t1, DeterministicVector2::new(I16F16::ONE, I16F16::ZERO))
        };
        if t_enter_x > t_min {
            t_min = t_enter_x;
            normal = norm_x;
        }
        if t_exit_x < t_max {
            t_max = t_exit_x;
        }
    } else if origin.x < aabb.min.x || origin.x > aabb.max.x {
        return None;
    }

    // Y slab
    if dir.y != I16F16::ZERO {
        let t1 = (aabb.min.y - origin.y) / dir.y;
        let t2 = (aabb.max.y - origin.y) / dir.y;
        let (t_enter_y, t_exit_y, norm_y) = if t1 < t2 {
            (t1, t2, DeterministicVector2::new(I16F16::ZERO, -I16F16::ONE))
        } else {
            (t2, t1, DeterministicVector2::new(I16F16::ZERO, I16F16::ONE))
        };
        if t_enter_y > t_min {
            t_min = t_enter_y;
            normal = norm_y;
        }
        if t_exit_y < t_max {
            t_max = t_exit_y;
        }
    } else if origin.y < aabb.min.y || origin.y > aabb.max.y {
        return None;
    }

    if t_min <= t_max && t_max >= I16F16::ZERO && t_min <= max_distance {
        let hit_t = if t_min < I16F16::ZERO {
            I16F16::ZERO
        } else {
            t_min
        };
        Some((hit_t, normal))
    } else {
        None
    }
}

fn ray_intersect_circle(
    origin: DeterministicVector2,
    dir: DeterministicVector2,
    max_distance: I16F16,
    circle: &DeterministicCircle,
) -> Option<(I16F16, DeterministicVector2)> {
    let dx_raw = (circle.center.x - origin.x).to_bits() as i64;
    let dy_raw = (circle.center.y - origin.y).to_bits() as i64;
    let dist_sq_raw = dx_raw * dx_raw + dy_raw * dy_raw;

    let r_raw = circle.radius.to_bits() as i64;
    let r_sq_raw = r_raw * r_raw;

    if dist_sq_raw <= r_sq_raw {
        // Origin inside circle
        return Some((I16F16::ZERO, -dir));
    }

    let v = circle.center - origin;
    let t_proj = v.dot(dir);
    if t_proj < I16F16::ZERO {
        return None;
    }

    let p_close = origin + dir * t_proj;
    let px_raw = (circle.center.x - p_close.x).to_bits() as i64;
    let py_raw = (circle.center.y - p_close.y).to_bits() as i64;
    let d_perp_sq_raw = px_raw * px_raw + py_raw * py_raw;

    if d_perp_sq_raw > r_sq_raw {
        return None;
    }

    let d_half_sq_raw = (r_sq_raw - d_perp_sq_raw) as u64;
    let d_half_raw = integer_sqrt_u64(d_half_sq_raw);
    let d_half = I16F16::from_bits(d_half_raw as i32);
    let t_hit = t_proj - d_half;

    if t_hit > max_distance || t_hit < I16F16::ZERO {
        return None;
    }

    let hit_point = origin + dir * t_hit;
    let normal = if hit_point == circle.center {
        -dir
    } else {
        (hit_point - circle.center).normalize_or_zero()
    };

    Some((t_hit, normal))
}

/// Casts a ray across static obstacles in the instance world.
///
/// Returns the nearest solid obstacle intersection hit within `max_distance`,
/// or `None` if no solid obstacle is hit.
pub fn fixed_raycast(
    origin: DeterministicVector2,
    direction: DeterministicVector2,
    max_distance: I16F16,
    obstacles: &BTreeMap<u64, StaticObstacle>,
) -> Option<RaycastHit> {
    let dir = direction.normalize_or_zero();
    if dir == DeterministicVector2::ZERO || max_distance <= I16F16::ZERO {
        return None;
    }

    let mut closest_hit: Option<RaycastHit> = None;
    let mut min_t = max_distance;

    for (&obstacle_id, obstacle) in obstacles {
        if !obstacle.is_solid {
            continue;
        }

        let hit_result = match &obstacle.shape {
            ColliderShape::AABB(aabb) => ray_intersect_aabb(origin, dir, min_t, aabb),
            ColliderShape::Circle(circle) => ray_intersect_circle(origin, dir, min_t, circle),
        };

        if let Some((hit_t, normal)) = hit_result.filter(|&(hit_t, _)| hit_t <= min_t) {
            min_t = hit_t;
            let fraction = hit_t / max_distance;
            let point = origin + dir * hit_t;
            closest_hit = Some(RaycastHit {
                point,
                normal,
                fraction,
                obstacle_id,
            });
        }
    }

    closest_hit
}
