use fixed::types::I16F16;
use loci2d::world::entity::{Entity, EntityType};
use loci2d::world::fixed_point::DeterministicVector2;
use loci2d::world::instance::Instance;
use loci2d::world::physics::{
    ColliderShape, ContactManifold, DeterministicAABB, DeterministicCircle,
    StaticObstacle, TriggerEventType, fixed_raycast,
    resolve_dynamic_collision, resolve_static_collision,
};
use std::collections::BTreeMap;

#[test]
fn test_resolve_static_collision_pushback() {
    let mut pos = DeterministicVector2::new(I16F16::from_num(10), I16F16::from_num(20));
    let mut vel = DeterministicVector2::new(I16F16::from_num(-5), I16F16::from_num(0));

    let manifold = ContactManifold {
        is_colliding: true,
        normal: DeterministicVector2::UNIT_X, // Normal points away from wall (+X)
        penetration_depth: I16F16::from_num(3),
    };

    resolve_static_collision(&mut pos, &mut vel, &manifold);

    // Entity pushed out by 100% of depth (3 units along +X)
    assert_eq!(pos.x, I16F16::from_num(13));
    assert_eq!(pos.y, I16F16::from_num(20));

    // Inward velocity (-5, 0) canceled by wall sliding along normal (1, 0)
    // dot = (-5 * 1) = -5 < 0 -> vel.x -= 1 * (-5) = 0
    assert_eq!(vel.x, I16F16::ZERO);
    assert_eq!(vel.y, I16F16::ZERO);
}

#[test]
fn test_resolve_static_collision_wall_sliding() {
    // Entity moving diagonally down-right (10, -10) into a floor at Y = 0 (normal +Y)
    let mut pos = DeterministicVector2::new(I16F16::from_num(50), I16F16::from_num(-2));
    let mut vel = DeterministicVector2::new(I16F16::from_num(10), I16F16::from_num(-10));

    let manifold = ContactManifold {
        is_colliding: true,
        normal: DeterministicVector2::UNIT_Y,
        penetration_depth: I16F16::from_num(2),
    };

    resolve_static_collision(&mut pos, &mut vel, &manifold);

    // Pushed up to Y = 0
    assert_eq!(pos.x, I16F16::from_num(50));
    assert_eq!(pos.y, I16F16::ZERO);

    // Horizontal sliding velocity along X is preserved, downward Y velocity is eliminated
    assert_eq!(vel.x, I16F16::from_num(10));
    assert_eq!(vel.y, I16F16::ZERO);
}

#[test]
fn test_resolve_dynamic_collision_50_50_split() {
    // Entity A at (10, 0), Entity B at (8, 0).
    // Penetration depth = 2.0. Normal points from B to A (+X).
    let mut pos_a = DeterministicVector2::new(I16F16::from_num(10), I16F16::ZERO);
    let mut vel_a = DeterministicVector2::new(I16F16::from_num(-4), I16F16::ZERO);
    let mut pos_b = DeterministicVector2::new(I16F16::from_num(8), I16F16::ZERO);
    let mut vel_b = DeterministicVector2::new(I16F16::from_num(4), I16F16::ZERO);

    let manifold = ContactManifold {
        is_colliding: true,
        normal: DeterministicVector2::UNIT_X,
        penetration_depth: I16F16::from_num(2),
    };

    resolve_dynamic_collision(
        &mut pos_a,
        &mut vel_a,
        &mut pos_b,
        &mut vel_b,
        &manifold,
    );

    // 50/50 pushback: A pushed +1.0 along +X (to 11.0), B pushed -1.0 along +X (to 7.0)
    assert_eq!(pos_a.x, I16F16::from_num(11));
    assert_eq!(pos_b.x, I16F16::from_num(7));

    // Inward velocities canceled
    assert_eq!(vel_a.x, I16F16::ZERO);
    assert_eq!(vel_b.x, I16F16::ZERO);
}

#[test]
fn test_resolve_dynamic_collision_lsb_remainder_preservation() {
    // Odd bit representation depth: 3 raw bits (3 / 65536)
    let depth = I16F16::from_bits(3);
    let mut pos_a = DeterministicVector2::ZERO;
    let mut vel_a = DeterministicVector2::ZERO;
    let mut pos_b = DeterministicVector2::ZERO;
    let mut vel_b = DeterministicVector2::ZERO;

    let manifold = ContactManifold {
        is_colliding: true,
        normal: DeterministicVector2::UNIT_X,
        penetration_depth: depth,
    };

    resolve_dynamic_collision(
        &mut pos_a,
        &mut vel_a,
        &mut pos_b,
        &mut vel_b,
        &manifold,
    );

    // Half depth = 1 bit, remainder = 1 bit.
    // Push A = 2 bits, Push B = 1 bit. Total separation = 3 bits (100% exact).
    assert_eq!(pos_a.x.to_bits(), 2);
    assert_eq!(pos_b.x.to_bits(), -1);
    assert_eq!(pos_a.x.to_bits() - pos_b.x.to_bits(), 3);
}

#[test]
fn test_instance_static_wall_blocking_and_sliding() {
    let mut instance = Instance::new(1, 30, 60);

    // Static solid wall from [50, 60] x [-100, 100]
    let wall_shape = ColliderShape::AABB(DeterministicAABB::new(
        DeterministicVector2::new(I16F16::from_num(50), I16F16::from_num(-100)),
        DeterministicVector2::new(I16F16::from_num(60), I16F16::from_num(100)),
    ));
    instance.add_static_obstacle(StaticObstacle::solid_wall(10, wall_shape));

    // Player with circle collider radius = 5, starting at (40, 0), velocity = (10, 5)
    let mut player = Entity::new(1, "Player1".to_string(), EntityType::Player)
        .with_circle_collider(I16F16::from_num(5));
    player.position = DeterministicVector2::new(I16F16::from_num(40), I16F16::ZERO);
    player.velocity = DeterministicVector2::new(I16F16::from_num(10), I16F16::from_num(5));
    instance.add_entity(player);

    // Tick 1:
    // Integration: position -> (50, 5). Circle edge at X = 55 penetrates wall (min.x = 50) by 5 units.
    // Collision resolution pushes player out to X = 45 (contact normal = (-1, 0)).
    // Velocity along X is zeroed, velocity along Y (5) is preserved for sliding!
    instance.tick(1);

    let p = instance.get_entity(1).unwrap();
    assert_eq!(p.position.x, I16F16::from_num(45));
    assert_eq!(p.position.y, I16F16::from_num(5));
    assert_eq!(p.velocity.x, I16F16::ZERO);
    assert_eq!(p.velocity.y, I16F16::from_num(5));

    // Tick 2:
    // Slides along wall from Y = 5 to Y = 10, staying at X = 45
    instance.tick(2);

    let p2 = instance.get_entity(1).unwrap();
    assert_eq!(p2.position.x, I16F16::from_num(45));
    assert_eq!(p2.position.y, I16F16::from_num(10));
}

#[test]
fn test_instance_dynamic_entity_collision() {
    let mut instance = Instance::new(1, 30, 60);

    // Player 1 at (-6, 0), moving right (+4, 0), radius = 5
    let mut p1 = Entity::new(1, "P1".to_string(), EntityType::Player)
        .with_circle_collider(I16F16::from_num(5));
    p1.position = DeterministicVector2::new(I16F16::from_num(-6), I16F16::ZERO);
    p1.velocity = DeterministicVector2::new(I16F16::from_num(4), I16F16::ZERO);
    instance.add_entity(p1);

    // Player 2 at (6, 0), moving left (-4, 0), radius = 5
    let mut p2 = Entity::new(2, "P2".to_string(), EntityType::Player)
        .with_circle_collider(I16F16::from_num(5));
    p2.position = DeterministicVector2::new(I16F16::from_num(6), I16F16::ZERO);
    p2.velocity = DeterministicVector2::new(I16F16::from_num(-4), I16F16::ZERO);
    instance.add_entity(p2);

    // Tick 1:
    // Integration moves p1 to (-2, 0) and p2 to (+2, 0).
    // Distance = 4, radii sum = 10 -> overlap = 6.
    // Normal from B (+2) to A (-2) is (-1, 0).
    // 50/50 pushback separates them by 3 each: p1 to (-5, 0), p2 to (+5, 0).
    // Inward velocities (4 and -4) are canceled to 0.
    instance.tick(1);

    let p1_after = instance.get_entity(1).unwrap();
    let p2_after = instance.get_entity(2).unwrap();

    // Distance between centers is exactly 10 (touching, separated)
    let dist = (p2_after.position.x - p1_after.position.x).abs();
    assert_eq!(dist, I16F16::from_num(10));
    assert_eq!(p1_after.position.x, I16F16::from_num(-5));
    assert_eq!(p2_after.position.x, I16F16::from_num(5));
    assert_eq!(p1_after.velocity.x, I16F16::ZERO);
    assert_eq!(p2_after.velocity.x, I16F16::ZERO);
}

#[test]
fn test_trigger_zones_lifecycle_enter_stay_exit() {
    let mut instance = Instance::new(1, 30, 60);

    // Non-solid trigger zone at [100, 200] x [100, 200]
    let trigger_shape = ColliderShape::AABB(DeterministicAABB::new(
        DeterministicVector2::new(I16F16::from_num(100), I16F16::from_num(100)),
        DeterministicVector2::new(I16F16::from_num(200), I16F16::from_num(200)),
    ));
    instance.add_static_obstacle(StaticObstacle::trigger_zone(500, trigger_shape));

    // Player starting outside at (80, 150), moving right (+30, 0)
    let mut player = Entity::new(1, "Traveler".to_string(), EntityType::Player)
        .with_circle_collider(I16F16::from_num(5));
    player.position = DeterministicVector2::new(I16F16::from_num(80), I16F16::from_num(150));
    player.velocity = DeterministicVector2::new(I16F16::from_num(30), I16F16::ZERO);
    instance.add_entity(player);

    // Tick 1: Player moves to (110, 150) -> Inside trigger zone!
    // Should emit Enter event
    instance.tick(1);

    assert_eq!(instance.trigger_events.len(), 1);
    assert_eq!(instance.trigger_events[0].trigger_id, 500);
    assert_eq!(instance.trigger_events[0].entity_id, 1);
    assert_eq!(
        instance.trigger_events[0].event_type,
        TriggerEventType::Enter
    );
    assert_eq!(instance.trigger_events[0].tick, 1);
    assert!(instance.active_trigger_overlaps.contains(&(500, 1)));

    // Tick 2: Player moves to (140, 150) -> Still inside trigger zone!
    // Should emit Stay event
    instance.tick(2);

    assert_eq!(instance.trigger_events.len(), 1);
    assert_eq!(instance.trigger_events[0].trigger_id, 500);
    assert_eq!(instance.trigger_events[0].entity_id, 1);
    assert_eq!(
        instance.trigger_events[0].event_type,
        TriggerEventType::Stay
    );
    assert_eq!(instance.trigger_events[0].tick, 2);
    assert!(instance.active_trigger_overlaps.contains(&(500, 1)));

    // Tick 3: Player moves to (170, 150) -> Still inside
    instance.tick(3);
    assert_eq!(
        instance.trigger_events[0].event_type,
        TriggerEventType::Stay
    );

    // Tick 4: Player moves with +50 velocity to (220, 150) -> Outside trigger zone!
    // Should emit Exit event
    if let Some(p) = instance.entities.get_mut(&1) {
        p.velocity = DeterministicVector2::new(I16F16::from_num(50), I16F16::ZERO);
    }
    instance.tick(4);

    assert_eq!(instance.trigger_events.len(), 1);
    assert_eq!(instance.trigger_events[0].trigger_id, 500);
    assert_eq!(instance.trigger_events[0].entity_id, 1);
    assert_eq!(
        instance.trigger_events[0].event_type,
        TriggerEventType::Exit
    );
    assert_eq!(instance.trigger_events[0].tick, 4);
    assert!(!instance.active_trigger_overlaps.contains(&(500, 1)));

    // Tick 5: Player is at (270, 150) -> Far outside
    // No trigger events emitted
    instance.tick(5);
    assert_eq!(instance.trigger_events.len(), 0);
}

#[test]
fn test_multiple_triggers_and_entities_deterministic_ordering() {
    let mut instance = Instance::new(1, 30, 60);

    // Two trigger zones
    let tz1 = StaticObstacle::trigger_zone(
        10,
        ColliderShape::AABB(DeterministicAABB::new(
            DeterministicVector2::new(I16F16::from_num(-50), I16F16::from_num(-50)),
            DeterministicVector2::new(I16F16::from_num(50), I16F16::from_num(50)),
        )),
    );
    let tz2 = StaticObstacle::trigger_zone(
        20,
        ColliderShape::AABB(DeterministicAABB::new(
            DeterministicVector2::new(I16F16::from_num(-50), I16F16::from_num(-50)),
            DeterministicVector2::new(I16F16::from_num(50), I16F16::from_num(50)),
        )),
    );
    instance.add_static_obstacle(tz2); // Insert 20 first
    instance.add_static_obstacle(tz1); // Insert 10 second

    // Three players inside the trigger zones
    let mut e3 = Entity::new(3, "P3".to_string(), EntityType::Player)
        .with_circle_collider(I16F16::from_num(2));
    let mut e1 = Entity::new(1, "P1".to_string(), EntityType::Player)
        .with_circle_collider(I16F16::from_num(2));
    let mut e2 = Entity::new(2, "P2".to_string(), EntityType::Player)
        .with_circle_collider(I16F16::from_num(2));

    e1.position = DeterministicVector2::ZERO;
    e2.position = DeterministicVector2::ZERO;
    e3.position = DeterministicVector2::ZERO;

    instance.add_entity(e3);
    instance.add_entity(e1);
    instance.add_entity(e2);

    instance.tick(1);

    // Events must be ordered strictly by (trigger_id, entity_id):
    // (10, 1), (10, 2), (10, 3), (20, 1), (20, 2), (20, 3)
    assert_eq!(instance.trigger_events.len(), 6);
    assert_eq!(
        (
            instance.trigger_events[0].trigger_id,
            instance.trigger_events[0].entity_id
        ),
        (10, 1)
    );
    assert_eq!(
        (
            instance.trigger_events[1].trigger_id,
            instance.trigger_events[1].entity_id
        ),
        (10, 2)
    );
    assert_eq!(
        (
            instance.trigger_events[2].trigger_id,
            instance.trigger_events[2].entity_id
        ),
        (10, 3)
    );
    assert_eq!(
        (
            instance.trigger_events[3].trigger_id,
            instance.trigger_events[3].entity_id
        ),
        (20, 1)
    );
    assert_eq!(
        (
            instance.trigger_events[4].trigger_id,
            instance.trigger_events[4].entity_id
        ),
        (20, 2)
    );
    assert_eq!(
        (
            instance.trigger_events[5].trigger_id,
            instance.trigger_events[5].entity_id
        ),
        (20, 3)
    );
}

#[test]
fn test_fixed_raycast_aabb_hit_and_miss() {
    let mut obstacles = BTreeMap::new();
    let aabb_obstacle = StaticObstacle::solid_wall(
        1,
        ColliderShape::AABB(DeterministicAABB::new(
            DeterministicVector2::new(I16F16::from_num(20), I16F16::from_num(-10)),
            DeterministicVector2::new(I16F16::from_num(40), I16F16::from_num(10)),
        )),
    );
    obstacles.insert(1, aabb_obstacle);

    // Ray shooting along +X from (0, 0)
    let origin = DeterministicVector2::ZERO;
    let direction = DeterministicVector2::UNIT_X;
    let max_dist = I16F16::from_num(100);

    let hit = fixed_raycast(origin, direction, max_dist, &obstacles);
    assert!(hit.is_some());
    let h = hit.unwrap();
    assert_eq!(h.obstacle_id, 1);
    assert_eq!(h.point.x, I16F16::from_num(20));
    assert_eq!(h.point.y, I16F16::ZERO);
    assert_eq!(h.normal, -DeterministicVector2::UNIT_X); // Normal points outward from wall (-X)
    assert_eq!(
        h.fraction,
        I16F16::from_num(20) / I16F16::from_num(100)
    );

    // Ray shooting in opposite direction (-X) misses
    let miss = fixed_raycast(origin, -DeterministicVector2::UNIT_X, max_dist, &obstacles);
    assert!(miss.is_none());

    // Ray shooting with max_dist = 10 (wall is at 20) misses due to range
    let short_ray = fixed_raycast(
        origin,
        direction,
        I16F16::from_num(10),
        &obstacles,
    );
    assert!(short_ray.is_none());
}

#[test]
fn test_fixed_raycast_circle_hit() {
    let mut obstacles = BTreeMap::new();
    let circle_obstacle = StaticObstacle::solid_wall(
        2,
        ColliderShape::Circle(DeterministicCircle::new(
            DeterministicVector2::new(I16F16::from_num(50), I16F16::ZERO),
            I16F16::from_num(10),
        )),
    );
    obstacles.insert(2, circle_obstacle);

    let origin = DeterministicVector2::ZERO;
    let direction = DeterministicVector2::UNIT_X;
    let max_dist = I16F16::from_num(100);

    let hit = fixed_raycast(origin, direction, max_dist, &obstacles);
    assert!(hit.is_some());
    let h = hit.unwrap();
    assert_eq!(h.obstacle_id, 2);
    // Circle center at 50, radius 10 -> hit at X = 40
    assert_eq!(h.point.x, I16F16::from_num(40));
    assert_eq!(h.point.y, I16F16::ZERO);
    assert_eq!(h.normal, -DeterministicVector2::UNIT_X);
    assert_eq!(
        h.fraction,
        I16F16::from_num(40) / I16F16::from_num(100)
    );
}

#[test]
fn test_fixed_raycast_occlusion_and_filter() {
    let mut obstacles = BTreeMap::new();

    // Trigger zone (non-solid) at X = 10..20 -> Should be ignored by raycast
    let trigger = StaticObstacle::trigger_zone(
        1,
        ColliderShape::AABB(DeterministicAABB::new(
            DeterministicVector2::new(I16F16::from_num(10), I16F16::from_num(-10)),
            DeterministicVector2::new(I16F16::from_num(20), I16F16::from_num(10)),
        )),
    );
    // Near solid wall at X = 30..40
    let wall_near = StaticObstacle::solid_wall(
        2,
        ColliderShape::AABB(DeterministicAABB::new(
            DeterministicVector2::new(I16F16::from_num(30), I16F16::from_num(-10)),
            DeterministicVector2::new(I16F16::from_num(40), I16F16::from_num(10)),
        )),
    );
    // Far solid wall at X = 60..70
    let wall_far = StaticObstacle::solid_wall(
        3,
        ColliderShape::AABB(DeterministicAABB::new(
            DeterministicVector2::new(I16F16::from_num(60), I16F16::from_num(-10)),
            DeterministicVector2::new(I16F16::from_num(70), I16F16::from_num(10)),
        )),
    );

    obstacles.insert(1, trigger);
    obstacles.insert(2, wall_near);
    obstacles.insert(3, wall_far);

    let origin = DeterministicVector2::ZERO;
    let direction = DeterministicVector2::UNIT_X;
    let max_dist = I16F16::from_num(100);

    let hit = fixed_raycast(origin, direction, max_dist, &obstacles);
    assert!(hit.is_some());
    let h = hit.unwrap();
    // Non-solid trigger at 10 was skipped, near wall at 30 was hit, far wall at 60 was occluded
    assert_eq!(h.obstacle_id, 2);
    assert_eq!(h.point.x, I16F16::from_num(30));
    assert_eq!(
        h.fraction,
        I16F16::from_num(30) / I16F16::from_num(100)
    );
}
