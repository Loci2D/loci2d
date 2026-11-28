use fixed::types::I16F16;
use loci2d::world::fixed_point::DeterministicVector2;
use loci2d::world::physics::{
    DeterministicAABB, DeterministicCircle, deterministic_distance, integer_sqrt_u64,
    integer_sqrt_u128, intersect_circle_aabb, intersect_circle_circle,
};

#[test]
fn test_large_distance_exceeding_naive_fixed_point_threshold() {
    // In I16F16, any delta >= sqrt(32767) ≈ 181.01 causes naive delta^2 to overflow I16F16::MAX.
    // Our 64-bit intermediate calculation must operate flawlessly without overflow.
    let p1 = DeterministicVector2::new(I16F16::from_num(-300), I16F16::from_num(-300));
    let p2 = DeterministicVector2::new(I16F16::from_num(300), I16F16::from_num(300));

    // Expected distance = sqrt(600^2 + 600^2) = sqrt(720000) ≈ 848.528137...
    let dist = deterministic_distance(p1, p2);
    let expected = (600.0f64 * 600.0 + 600.0 * 600.0).sqrt();
    let diff = (dist.to_num::<f64>() - expected).abs();
    assert!(
        diff < 0.01,
        "Distance on 600-unit span failed: got {dist}, expected {expected}"
    );
}

#[test]
fn test_extreme_coordinates_distance() {
    // Test points spanning 10,000 units across arena
    let p1 = DeterministicVector2::new(I16F16::from_num(-5000), I16F16::from_num(-5000));
    let p2 = DeterministicVector2::new(I16F16::from_num(5000), I16F16::from_num(5000));

    let dist = deterministic_distance(p1, p2);
    let expected = (10000.0f64 * 10000.0 + 10000.0 * 10000.0).sqrt();
    let diff = (dist.to_num::<f64>() - expected).abs();
    assert!(
        diff < 0.05,
        "Extreme 10,000 unit distance failed: got {dist}, expected {expected}"
    );

    // Test straight line spanning 30,000 units
    let p_left = DeterministicVector2::new(I16F16::from_num(-15000), I16F16::from_num(0));
    let p_right = DeterministicVector2::new(I16F16::from_num(15000), I16F16::from_num(0));
    assert_eq!(
        deterministic_distance(p_left, p_right),
        I16F16::from_num(30000)
    );
}

#[test]
fn test_vector_operations_at_large_coordinates() {
    // Large dot product
    let v1 = DeterministicVector2::new(I16F16::from_num(1000), I16F16::from_num(1000));
    let v2 = DeterministicVector2::new(I16F16::from_num(10), I16F16::from_num(-10));
    assert_eq!(v1.dot(v2), I16F16::ZERO);

    let v_large = DeterministicVector2::new(I16F16::from_num(500), I16F16::from_num(0));
    let v_unit = DeterministicVector2::new(I16F16::from_num(1), I16F16::from_num(0));
    assert_eq!(v_large.dot(v_unit), I16F16::from_num(500));

    // Large vector length & normalize
    let v_345_large = DeterministicVector2::new(I16F16::from_num(3000), I16F16::from_num(4000));
    assert_eq!(v_345_large.length(), I16F16::from_num(5000));

    let norm = v_345_large.normalize_or_zero();
    assert_eq!(norm.x, I16F16::from_num(3) / I16F16::from_num(5));
    assert_eq!(norm.y, I16F16::from_num(4) / I16F16::from_num(5));
}

#[test]
fn test_collision_at_large_coordinates() {
    // Two large circles at far coordinates (x ≈ 10000)
    let c1 = DeterministicCircle::new(
        DeterministicVector2::new(I16F16::from_num(10000), I16F16::from_num(10000)),
        I16F16::from_num(100),
    );
    let c2 = DeterministicCircle::new(
        DeterministicVector2::new(I16F16::from_num(10150), I16F16::from_num(10000)),
        I16F16::from_num(100),
    );

    // Radii sum = 200, dist = 150 -> depth = 50
    let m = intersect_circle_circle(&c2, &c1);
    assert!(m.is_colliding);
    assert_eq!(m.normal, DeterministicVector2::UNIT_X);
    assert_eq!(m.penetration_depth, I16F16::from_num(50));

    // AABB and Circle at far coordinates
    let aabb = DeterministicAABB::new(
        DeterministicVector2::new(I16F16::from_num(9000), I16F16::from_num(9000)),
        DeterministicVector2::new(I16F16::from_num(9200), I16F16::from_num(9200)),
    );
    let c3 = DeterministicCircle::new(
        DeterministicVector2::new(I16F16::from_num(9250), I16F16::from_num(9100)),
        I16F16::from_num(100),
    );
    // Closest point on AABB is (9200, 9100), dist = 50, radius = 100 -> depth = 50
    let m_box = intersect_circle_aabb(&c3, &aabb);
    assert!(m_box.is_colliding);
    assert_eq!(m_box.normal, DeterministicVector2::UNIT_X);
    assert_eq!(m_box.penetration_depth, I16F16::from_num(50));
}

#[test]
fn test_distance_beyond_46k_units_no_overflow_and_saturates() {
    // 50,000-unit span exceeds the ~46,340-unit i64 squared-raw threshold.
    let a = DeterministicVector2::new(I16F16::from_num(-25_000), I16F16::ZERO);
    let b = DeterministicVector2::new(I16F16::from_num(25_000), I16F16::ZERO);
    assert_eq!(deterministic_distance(a, b), I16F16::MAX);
    assert_eq!(deterministic_distance(b, a), I16F16::MAX);

    // Full I16F16 domain extremes (~92,681.9 units diagonal, exceeding u64::MAX squared-raw).
    let min_min = DeterministicVector2::new(I16F16::MIN, I16F16::MIN);
    let max_max = DeterministicVector2::new(I16F16::MAX, I16F16::MAX);
    assert_eq!(deterministic_distance(min_min, max_max), I16F16::MAX);

    let min_max = DeterministicVector2::new(I16F16::MIN, I16F16::MAX);
    let max_min = DeterministicVector2::new(I16F16::MAX, I16F16::MIN);
    assert_eq!(deterministic_distance(min_max, max_min), I16F16::MAX);

    // Near-max non-saturating diagonal: 23,000 * sqrt(2) ≈ 32,526.9119 < I16F16::MAX.
    let d1 = DeterministicVector2::new(I16F16::from_num(-11_500), I16F16::from_num(-11_500));
    let d2 = DeterministicVector2::new(I16F16::from_num(11_500), I16F16::from_num(11_500));
    let dist = deterministic_distance(d1, d2);
    let expected = (23_000.0f64 * 23_000.0 + 23_000.0 * 23_000.0).sqrt();
    let diff = (dist.to_num::<f64>() - expected).abs();
    assert!(
        diff < 0.05,
        "Near-max diagonal distance failed: got {dist}, expected {expected}"
    );
}

#[test]
fn test_integer_sqrt_u128_parity_and_large_values() {
    let samples = [0u64, 1, 2, 3, 4, 65_535, 65_536, u32::MAX as u64, u64::MAX];
    for val in samples {
        assert_eq!(
            integer_sqrt_u128(val as u128),
            integer_sqrt_u64(val) as u128,
            "Parity mismatch for val={val}"
        );
    }

    assert_eq!(integer_sqrt_u128(1u128 << 66), 1u128 << 33);
    assert_eq!(integer_sqrt_u128(u128::MAX), u64::MAX as u128);
}
