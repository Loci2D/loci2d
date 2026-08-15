use fixed::types::I16F16;
use loci2d::world::fixed_point::DeterministicVector2;
use loci2d::world::physics::{
    ColliderShape, ContactManifold, DeterministicAABB, DeterministicCircle, deterministic_distance,
    fixed_sqrt, integer_sqrt_u64, intersect_aabb_aabb, intersect_aabb_circle,
    intersect_circle_aabb, intersect_circle_circle, intersect_shapes,
};

#[test]
fn test_integer_sqrt_u64_perfect_squares() {
    assert_eq!(integer_sqrt_u64(0), 0);
    assert_eq!(integer_sqrt_u64(1), 1);
    assert_eq!(integer_sqrt_u64(4), 2);
    assert_eq!(integer_sqrt_u64(9), 3);
    assert_eq!(integer_sqrt_u64(16), 4);
    assert_eq!(integer_sqrt_u64(25), 5);
    assert_eq!(integer_sqrt_u64(100), 10);
    assert_eq!(integer_sqrt_u64(65536), 256);
    assert_eq!(integer_sqrt_u64(1_000_000), 1000);
    assert_eq!(integer_sqrt_u64(1u64 << 62), 1u64 << 31);
}

#[test]
fn test_integer_sqrt_u64_non_squares() {
    for &val in &[2, 3, 5, 8, 15, 99, 1000, 65535, 1_000_001, u64::MAX] {
        let root = integer_sqrt_u64(val);
        assert!(root * root <= val, "root^2 must be <= val for val={val}");
        if root < u32::MAX as u64 {
            let next = root + 1;
            assert!(
                next.checked_mul(next).is_none_or(|sq| sq > val),
                "(root+1)^2 must be > val for val={val}"
            );
        }
    }
}

#[test]
fn test_fixed_sqrt() {
    // Negative or zero values return 0.0
    assert_eq!(fixed_sqrt(I16F16::from_num(-10)), I16F16::ZERO);
    assert_eq!(fixed_sqrt(I16F16::ZERO), I16F16::ZERO);

    // Exact roots
    assert_eq!(fixed_sqrt(I16F16::from_num(1)), I16F16::from_num(1));
    assert_eq!(fixed_sqrt(I16F16::from_num(4)), I16F16::from_num(2));
    assert_eq!(fixed_sqrt(I16F16::from_num(9)), I16F16::from_num(3));
    assert_eq!(fixed_sqrt(I16F16::from_num(16)), I16F16::from_num(4));
    assert_eq!(fixed_sqrt(I16F16::from_num(0.25)), I16F16::from_num(0.5));

    // Approximate root: sqrt(2) approx 1.41421...
    let sqrt_2 = fixed_sqrt(I16F16::from_num(2));
    let diff = (sqrt_2.to_num::<f64>() - 2.0f64.sqrt()).abs();
    assert!(diff < 0.0001, "sqrt(2) error was {diff}");
}

#[test]
fn test_deterministic_distance() {
    // 3-4-5 Right triangle
    let p1 = DeterministicVector2::new(I16F16::from_num(0), I16F16::from_num(0));
    let p2 = DeterministicVector2::new(I16F16::from_num(3), I16F16::from_num(4));
    assert_eq!(deterministic_distance(p1, p2), I16F16::from_num(5));
    assert_eq!(deterministic_distance(p2, p1), I16F16::from_num(5));

    // Negative coordinates
    let p3 = DeterministicVector2::new(I16F16::from_num(-3), I16F16::from_num(-4));
    assert_eq!(deterministic_distance(p1, p3), I16F16::from_num(5));

    // Zero distance
    assert_eq!(deterministic_distance(p2, p2), I16F16::ZERO);
}

#[test]
fn test_aabb_primitives_and_methods() {
    let min = DeterministicVector2::new(I16F16::from_num(-10), I16F16::from_num(-5));
    let max = DeterministicVector2::new(I16F16::from_num(10), I16F16::from_num(15));
    let aabb = DeterministicAABB::new(min, max);

    assert_eq!(aabb.width(), I16F16::from_num(20));
    assert_eq!(aabb.height(), I16F16::from_num(20));
    assert_eq!(
        aabb.center(),
        DeterministicVector2::new(I16F16::from_num(0), I16F16::from_num(5))
    );
    assert_eq!(
        aabb.half_extents(),
        DeterministicVector2::new(I16F16::from_num(10), I16F16::from_num(10))
    );

    let constructed =
        DeterministicAABB::from_center_half_extents(aabb.center(), aabb.half_extents());
    assert_eq!(constructed, aabb);

    // Contains point
    assert!(aabb.contains_point(DeterministicVector2::new(
        I16F16::from_num(0),
        I16F16::from_num(5)
    )));
    assert!(aabb.contains_point(min));
    assert!(aabb.contains_point(max));
    assert!(!aabb.contains_point(DeterministicVector2::new(
        I16F16::from_num(11),
        I16F16::from_num(5)
    )));

    // Intersects AABB
    let overlapping = DeterministicAABB::new(
        DeterministicVector2::new(I16F16::from_num(5), I16F16::from_num(0)),
        DeterministicVector2::new(I16F16::from_num(15), I16F16::from_num(10)),
    );
    assert!(aabb.intersects_aabb(&overlapping));

    let separate = DeterministicAABB::new(
        DeterministicVector2::new(I16F16::from_num(20), I16F16::from_num(20)),
        DeterministicVector2::new(I16F16::from_num(30), I16F16::from_num(30)),
    );
    assert!(!aabb.intersects_aabb(&separate));
}

#[test]
fn test_circle_primitives_and_methods() {
    let center = DeterministicVector2::new(I16F16::from_num(10), I16F16::from_num(20));
    let circle = DeterministicCircle::new(center, I16F16::from_num(5));

    assert!(circle.contains_point(center));
    assert!(circle.contains_point(DeterministicVector2::new(
        I16F16::from_num(13),
        I16F16::from_num(24)
    )));
    assert!(!circle.contains_point(DeterministicVector2::new(
        I16F16::from_num(16),
        I16F16::from_num(20)
    )));

    let bbox = circle.bounding_box();
    assert_eq!(
        bbox.min,
        DeterministicVector2::new(I16F16::from_num(5), I16F16::from_num(15))
    );
    assert_eq!(
        bbox.max,
        DeterministicVector2::new(I16F16::from_num(15), I16F16::from_num(25))
    );
}

#[test]
fn test_intersect_aabb_aabb() {
    // Non-colliding separate boxes
    let a = DeterministicAABB::new(
        DeterministicVector2::new(I16F16::from_num(0), I16F16::from_num(0)),
        DeterministicVector2::new(I16F16::from_num(10), I16F16::from_num(10)),
    );
    let b = DeterministicAABB::new(
        DeterministicVector2::new(I16F16::from_num(15), I16F16::from_num(0)),
        DeterministicVector2::new(I16F16::from_num(25), I16F16::from_num(10)),
    );
    assert_eq!(intersect_aabb_aabb(&a, &b), ContactManifold::NONE);

    // Overlap on X only
    let c = DeterministicAABB::new(
        DeterministicVector2::new(I16F16::from_num(5), I16F16::from_num(15)),
        DeterministicVector2::new(I16F16::from_num(15), I16F16::from_num(25)),
    );
    assert_eq!(intersect_aabb_aabb(&a, &c), ContactManifold::NONE);

    // Touching on edge (zero overlap) -> non-colliding
    let touching = DeterministicAABB::new(
        DeterministicVector2::new(I16F16::from_num(10), I16F16::from_num(0)),
        DeterministicVector2::new(I16F16::from_num(20), I16F16::from_num(10)),
    );
    assert_eq!(intersect_aabb_aabb(&a, &touching), ContactManifold::NONE);

    // Box A is to the right of Box B (Overlap X = 2, Overlap Y = 10 -> MTV along X)
    // Box A: [8, 18] x [0, 10], Box B: [0, 10] x [0, 10]
    let a_right = DeterministicAABB::new(
        DeterministicVector2::new(I16F16::from_num(8), I16F16::from_num(0)),
        DeterministicVector2::new(I16F16::from_num(18), I16F16::from_num(10)),
    );
    let manifold = intersect_aabb_aabb(&a_right, &a);
    assert!(manifold.is_colliding);
    assert_eq!(manifold.normal, DeterministicVector2::UNIT_X); // Normal points from B to A (+X)
    assert_eq!(manifold.penetration_depth, I16F16::from_num(2));

    // Box A is above Box B (Overlap X = 10, Overlap Y = 3 -> MTV along Y)
    let a_top = DeterministicAABB::new(
        DeterministicVector2::new(I16F16::from_num(0), I16F16::from_num(7)),
        DeterministicVector2::new(I16F16::from_num(10), I16F16::from_num(17)),
    );
    let manifold_top = intersect_aabb_aabb(&a_top, &a);
    assert!(manifold_top.is_colliding);
    assert_eq!(manifold_top.normal, DeterministicVector2::UNIT_Y); // Normal points from B to A (+Y)
    assert_eq!(manifold_top.penetration_depth, I16F16::from_num(3));

    // Concentric identical center boxes fallback
    let manifold_same = intersect_aabb_aabb(&a, &a);
    assert!(manifold_same.is_colliding);
    assert_eq!(manifold_same.normal, DeterministicVector2::UNIT_X);
    assert_eq!(manifold_same.penetration_depth, I16F16::from_num(10));
}

#[test]
fn test_intersect_circle_circle() {
    // Separate circles
    let c1 = DeterministicCircle::new(
        DeterministicVector2::new(I16F16::from_num(0), I16F16::from_num(0)),
        I16F16::from_num(3),
    );
    let c2 = DeterministicCircle::new(
        DeterministicVector2::new(I16F16::from_num(10), I16F16::from_num(0)),
        I16F16::from_num(3),
    );
    assert_eq!(intersect_circle_circle(&c1, &c2), ContactManifold::NONE);

    // Touching circles (dist = 6 == radii_sum = 6) -> non-colliding
    let c_touch = DeterministicCircle::new(
        DeterministicVector2::new(I16F16::from_num(6), I16F16::from_num(0)),
        I16F16::from_num(3),
    );
    assert_eq!(
        intersect_circle_circle(&c1, &c_touch),
        ContactManifold::NONE
    );

    // Overlapping along X: C3 at (4, 0), C1 at (0, 0), radius 3 each. Dist = 4, radii_sum = 6, depth = 2.
    // Intersecting C3 (A) vs C1 (B): normal points from B to A -> (+1, 0)
    let c3 = DeterministicCircle::new(
        DeterministicVector2::new(I16F16::from_num(4), I16F16::from_num(0)),
        I16F16::from_num(3),
    );
    let m = intersect_circle_circle(&c3, &c1);
    assert!(m.is_colliding);
    assert_eq!(m.normal, DeterministicVector2::UNIT_X);
    assert_eq!(m.penetration_depth, I16F16::from_num(2));

    // Overlapping along diagonal: C4 at (3, 4) with radius 4 vs C1 at (0, 0) with radius 3.
    // Dist = 5, radii_sum = 7, depth = 2.
    // Normal from C1 to C4 = (3/5, 4/5) = (0.6, 0.8)
    let c4 = DeterministicCircle::new(
        DeterministicVector2::new(I16F16::from_num(3), I16F16::from_num(4)),
        I16F16::from_num(4),
    );
    let m_diag = intersect_circle_circle(&c4, &c1);
    assert!(m_diag.is_colliding);
    assert_eq!(m_diag.normal.x, I16F16::from_num(3) / I16F16::from_num(5));
    assert_eq!(m_diag.normal.y, I16F16::from_num(4) / I16F16::from_num(5));
    assert_eq!(m_diag.penetration_depth, I16F16::from_num(2));

    // Concentric circles (dist == 0) fallback: normal = (1, 0), depth = radii_sum = 6
    let m_concentric = intersect_circle_circle(&c1, &c1);
    assert!(m_concentric.is_colliding);
    assert_eq!(m_concentric.normal, DeterministicVector2::UNIT_X);
    assert_eq!(m_concentric.penetration_depth, I16F16::from_num(6));
}

#[test]
fn test_intersect_circle_aabb_outside_and_corners() {
    let aabb = DeterministicAABB::new(
        DeterministicVector2::new(I16F16::from_num(0), I16F16::from_num(0)),
        DeterministicVector2::new(I16F16::from_num(10), I16F16::from_num(10)),
    );

    // Circle completely outside
    let circle_out = DeterministicCircle::new(
        DeterministicVector2::new(I16F16::from_num(15), I16F16::from_num(5)),
        I16F16::from_num(2),
    );
    assert_eq!(
        intersect_circle_aabb(&circle_out, &aabb),
        ContactManifold::NONE
    );

    // Circle penetrating right edge: Center at (11, 5), radius 3 -> closest point is (10, 5), dist = 1, depth = 2.
    // Normal points from AABB (B) to Circle (A) -> (+1, 0)
    let circle_right = DeterministicCircle::new(
        DeterministicVector2::new(I16F16::from_num(11), I16F16::from_num(5)),
        I16F16::from_num(3),
    );
    let m_right = intersect_circle_aabb(&circle_right, &aabb);
    assert!(m_right.is_colliding);
    assert_eq!(m_right.normal, DeterministicVector2::UNIT_X);
    assert_eq!(m_right.penetration_depth, I16F16::from_num(2));

    // Symmetry test: intersect_aabb_circle
    let m_aabb_circle = intersect_aabb_circle(&aabb, &circle_right);
    assert!(m_aabb_circle.is_colliding);
    assert_eq!(m_aabb_circle.normal, -DeterministicVector2::UNIT_X);
    assert_eq!(m_aabb_circle.penetration_depth, I16F16::from_num(2));

    // Circle penetrating corner: Center at (13, 14), radius 6.
    // Closest point on AABB is top-right corner (10, 10).
    // Vector (13-10, 14-10) = (3, 4), dist = 5. Radius = 6 -> depth = 1.
    // Normal = (3/5, 4/5)
    let circle_corner = DeterministicCircle::new(
        DeterministicVector2::new(I16F16::from_num(13), I16F16::from_num(14)),
        I16F16::from_num(6),
    );
    let m_corner = intersect_circle_aabb(&circle_corner, &aabb);
    assert!(m_corner.is_colliding);
    assert_eq!(m_corner.normal.x, I16F16::from_num(3) / I16F16::from_num(5));
    assert_eq!(m_corner.normal.y, I16F16::from_num(4) / I16F16::from_num(5));
    assert_eq!(m_corner.penetration_depth, I16F16::from_num(1));
}

#[test]
fn test_intersect_circle_aabb_center_inside() {
    let aabb = DeterministicAABB::new(
        DeterministicVector2::new(I16F16::from_num(0), I16F16::from_num(0)),
        DeterministicVector2::new(I16F16::from_num(10), I16F16::from_num(10)),
    );

    // Circle center inside at (9, 5), radius 2. Closest edge is right (dist = 10 - 9 = 1).
    // Normal = (+1, 0), depth = radius (2) + min_dist (1) = 3.
    let c_inside_right = DeterministicCircle::new(
        DeterministicVector2::new(I16F16::from_num(9), I16F16::from_num(5)),
        I16F16::from_num(2),
    );
    let m = intersect_circle_aabb(&c_inside_right, &aabb);
    assert!(m.is_colliding);
    assert_eq!(m.normal, DeterministicVector2::UNIT_X);
    assert_eq!(m.penetration_depth, I16F16::from_num(3));

    // Circle center inside at (1, 5), radius 2. Closest edge is left (dist = 1 - 0 = 1).
    // Normal = (-1, 0), depth = 2 + 1 = 3.
    let c_inside_left = DeterministicCircle::new(
        DeterministicVector2::new(I16F16::from_num(1), I16F16::from_num(5)),
        I16F16::from_num(2),
    );
    let m_left = intersect_circle_aabb(&c_inside_left, &aabb);
    assert!(m_left.is_colliding);
    assert_eq!(m_left.normal, -DeterministicVector2::UNIT_X);
    assert_eq!(m_left.penetration_depth, I16F16::from_num(3));

    // Circle center inside at (5, 9), radius 2. Closest edge is top (dist = 10 - 9 = 1).
    // Normal = (0, 1), depth = 3.
    let c_inside_top = DeterministicCircle::new(
        DeterministicVector2::new(I16F16::from_num(5), I16F16::from_num(9)),
        I16F16::from_num(2),
    );
    let m_top = intersect_circle_aabb(&c_inside_top, &aabb);
    assert!(m_top.is_colliding);
    assert_eq!(m_top.normal, DeterministicVector2::UNIT_Y);
    assert_eq!(m_top.penetration_depth, I16F16::from_num(3));

    // Circle center inside at (5, 1), radius 2. Closest edge is bottom (dist = 1 - 0 = 1).
    // Normal = (0, -1), depth = 3.
    let c_inside_bottom = DeterministicCircle::new(
        DeterministicVector2::new(I16F16::from_num(5), I16F16::from_num(1)),
        I16F16::from_num(2),
    );
    let m_bot = intersect_circle_aabb(&c_inside_bottom, &aabb);
    assert!(m_bot.is_colliding);
    assert_eq!(m_bot.normal, -DeterministicVector2::UNIT_Y);
    assert_eq!(m_bot.penetration_depth, I16F16::from_num(3));

    // Circle center at exact center (5, 5), radius 2. Distances to all 4 edges are 5.
    // Deterministic tie-break order selects +X.
    let c_center = DeterministicCircle::new(
        DeterministicVector2::new(I16F16::from_num(5), I16F16::from_num(5)),
        I16F16::from_num(2),
    );
    let m_center = intersect_circle_aabb(&c_center, &aabb);
    assert!(m_center.is_colliding);
    assert_eq!(m_center.normal, DeterministicVector2::UNIT_X);
    assert_eq!(m_center.penetration_depth, I16F16::from_num(7));
}

#[test]
fn test_intersect_shapes_dispatcher() {
    let aabb = DeterministicAABB::new(
        DeterministicVector2::new(I16F16::from_num(0), I16F16::from_num(0)),
        DeterministicVector2::new(I16F16::from_num(10), I16F16::from_num(10)),
    );
    let circle = DeterministicCircle::new(
        DeterministicVector2::new(I16F16::from_num(11), I16F16::from_num(5)),
        I16F16::from_num(3),
    );

    let shape_aabb = ColliderShape::AABB(aabb);
    let shape_circle = ColliderShape::Circle(circle);

    assert_eq!(shape_aabb.bounding_box(), aabb);
    assert_eq!(shape_circle.bounding_box(), circle.bounding_box());
    assert_eq!(shape_aabb.center(), aabb.center());
    assert_eq!(shape_circle.center(), circle.center);

    let m1 = intersect_shapes(&shape_circle, &shape_aabb);
    assert!(m1.is_colliding);
    assert_eq!(m1.normal, DeterministicVector2::UNIT_X);

    let m2 = intersect_shapes(&shape_aabb, &shape_circle);
    assert!(m2.is_colliding);
    assert_eq!(m2.normal, -DeterministicVector2::UNIT_X);
}
