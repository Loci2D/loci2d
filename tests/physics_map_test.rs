use fixed::types::I16F16;
use loci2d::world::entity::{Entity, EntityType};
use loci2d::world::fixed_point::DeterministicVector2;
use loci2d::world::instance::Instance;
use loci2d::world::physics::{
    ColliderShape, CollisionFilter, DeterministicAABB, DeterministicCircle, MapBounds,
    StaticObstacle,
};

#[test]
fn test_map_bounds_defaults_and_properties() {
    let default_bounds = MapBounds::default_arena();
    assert_eq!(
        default_bounds.min,
        DeterministicVector2::new(I16F16::from_num(-500), I16F16::from_num(-500))
    );
    assert_eq!(
        default_bounds.max,
        DeterministicVector2::new(I16F16::from_num(500), I16F16::from_num(500))
    );
    assert_eq!(default_bounds.width(), I16F16::from_num(1000));
    assert_eq!(default_bounds.height(), I16F16::from_num(1000));
    assert_eq!(default_bounds.center(), DeterministicVector2::ZERO);

    assert!(default_bounds.contains_point(DeterministicVector2::ZERO));
    assert!(default_bounds.contains_point(DeterministicVector2::new(
        I16F16::from_num(500),
        I16F16::from_num(500)
    )));
    assert!(default_bounds.contains_point(DeterministicVector2::new(
        I16F16::from_num(-500),
        I16F16::from_num(-500)
    )));
    assert!(!default_bounds.contains_point(DeterministicVector2::new(
        I16F16::from_num(501),
        I16F16::from_num(0)
    )));
    assert!(!default_bounds.contains_point(DeterministicVector2::new(
        I16F16::from_num(0),
        I16F16::from_num(-501)
    )));

    let aabb = default_bounds.to_aabb();
    assert_eq!(aabb.min, default_bounds.min);
    assert_eq!(aabb.max, default_bounds.max);
}

#[test]
#[should_panic(expected = "Invalid map bounds: min must be <= max")]
fn test_map_bounds_invalid_construction() {
    MapBounds::new(
        DeterministicVector2::new(I16F16::from_num(100), I16F16::from_num(100)),
        DeterministicVector2::new(I16F16::from_num(50), I16F16::from_num(100)),
    );
}

#[test]
fn test_clamp_point() {
    let bounds = MapBounds::new(
        DeterministicVector2::new(I16F16::from_num(-200), I16F16::from_num(-100)),
        DeterministicVector2::new(I16F16::from_num(200), I16F16::from_num(100)),
    );

    // Inside bounds - unchanged
    let p_in = DeterministicVector2::new(I16F16::from_num(50), I16F16::from_num(-30));
    assert_eq!(bounds.clamp_point(p_in), p_in);

    // Outside along +X, -Y
    let p_out1 = DeterministicVector2::new(I16F16::from_num(350), I16F16::from_num(-250));
    assert_eq!(
        bounds.clamp_point(p_out1),
        DeterministicVector2::new(I16F16::from_num(200), I16F16::from_num(-100))
    );

    // Outside along -X, +Y
    let p_out2 = DeterministicVector2::new(I16F16::from_num(-300), I16F16::from_num(150));
    assert_eq!(
        bounds.clamp_point(p_out2),
        DeterministicVector2::new(I16F16::from_num(-200), I16F16::from_num(100))
    );
}

#[test]
fn test_clamp_circle() {
    let bounds = MapBounds::new(
        DeterministicVector2::new(I16F16::from_num(-100), I16F16::from_num(-100)),
        DeterministicVector2::new(I16F16::from_num(100), I16F16::from_num(100)),
    );
    let radius = I16F16::from_num(15);

    // Center at (90, 90) -> circle outer edge extends to (105, 105), clamped center should be (85, 85)
    let p_near_max = DeterministicVector2::new(I16F16::from_num(90), I16F16::from_num(90));
    assert_eq!(
        bounds.clamp_circle(p_near_max, radius),
        DeterministicVector2::new(I16F16::from_num(85), I16F16::from_num(85))
    );

    // Center at (-95, -50) -> clamped center should be (-85, -50)
    let p_near_min = DeterministicVector2::new(I16F16::from_num(-95), I16F16::from_num(-50));
    assert_eq!(
        bounds.clamp_circle(p_near_min, radius),
        DeterministicVector2::new(I16F16::from_num(-85), I16F16::from_num(-50))
    );

    // Center already safely inside
    let p_safe = DeterministicVector2::new(I16F16::from_num(10), I16F16::from_num(20));
    assert_eq!(bounds.clamp_circle(p_safe, radius), p_safe);
}

#[test]
fn test_clamp_aabb() {
    let bounds = MapBounds::new(
        DeterministicVector2::new(I16F16::from_num(-200), I16F16::from_num(-200)),
        DeterministicVector2::new(I16F16::from_num(200), I16F16::from_num(200)),
    );
    let half_extents = DeterministicVector2::new(I16F16::from_num(25), I16F16::from_num(40));

    // Center at (190, 180) -> box extends to (215, 220), clamped center must be (175, 160)
    let center = DeterministicVector2::new(I16F16::from_num(190), I16F16::from_num(180));
    assert_eq!(
        bounds.clamp_aabb(center, half_extents),
        DeterministicVector2::new(I16F16::from_num(175), I16F16::from_num(160))
    );

    // Center at (-250, -250) -> clamped center must be (-175, -160)
    let far_center = DeterministicVector2::new(I16F16::from_num(-250), I16F16::from_num(-250));
    assert_eq!(
        bounds.clamp_aabb(far_center, half_extents),
        DeterministicVector2::new(I16F16::from_num(-175), I16F16::from_num(-160))
    );
}

#[test]
fn test_clamp_small_or_degenerate_arenas_no_panic() {
    // Narrow arena: width = 10 (from -5 to +5), height = 10 (from -5 to +5)
    let narrow_bounds = MapBounds::new(
        DeterministicVector2::new(I16F16::from_num(-5), I16F16::from_num(-5)),
        DeterministicVector2::new(I16F16::from_num(5), I16F16::from_num(5)),
    );

    // Circle radius = 20 (diameter 40 > arena width 10)
    // Must not panic and must center at midpoint (0, 0)
    let circle_center = DeterministicVector2::new(I16F16::from_num(100), I16F16::from_num(-50));
    let clamped_circle = narrow_bounds.clamp_circle(circle_center, I16F16::from_num(20));
    assert_eq!(clamped_circle, DeterministicVector2::ZERO);

    // AABB half_extents = (10, 15) > arena limits
    let aabb_center = DeterministicVector2::new(I16F16::from_num(-20), I16F16::from_num(30));
    let clamped_aabb = narrow_bounds.clamp_aabb(
        aabb_center,
        DeterministicVector2::new(I16F16::from_num(10), I16F16::from_num(15)),
    );
    assert_eq!(clamped_aabb, DeterministicVector2::ZERO);
}

#[test]
fn test_clamp_shape_dispatch() {
    let bounds = MapBounds::default_arena();

    let circle_shape = ColliderShape::Circle(DeterministicCircle::new(
        DeterministicVector2::new(I16F16::from_num(520), I16F16::from_num(0)),
        I16F16::from_num(10),
    ));
    assert_eq!(
        bounds.clamp_shape(&circle_shape),
        DeterministicVector2::new(I16F16::from_num(490), I16F16::from_num(0))
    );

    let aabb_shape = ColliderShape::AABB(DeterministicAABB::from_center_half_extents(
        DeterministicVector2::new(I16F16::from_num(0), I16F16::from_num(-520)),
        DeterministicVector2::new(I16F16::from_num(20), I16F16::from_num(30)),
    ));
    assert_eq!(
        bounds.clamp_shape(&aabb_shape),
        DeterministicVector2::new(I16F16::from_num(0), I16F16::from_num(-470))
    );
}

#[test]
fn test_collision_filter_rules() {
    let player = CollisionFilter::default_player();
    let wall = CollisionFilter::default_solid_wall();
    let trigger = CollisionFilter::default_trigger_zone();
    let projectile = CollisionFilter::default_projectile();

    // Player collides with Wall, Player, Trigger, Projectile
    assert!(player.can_collide(&wall));
    assert!(wall.can_collide(&player));

    assert!(player.can_collide(&player));

    assert!(player.can_collide(&trigger));
    assert!(trigger.can_collide(&player));

    assert!(player.can_collide(&projectile));
    assert!(projectile.can_collide(&player));

    // Walls do not collide with Walls
    assert!(!wall.can_collide(&wall));

    // Trigger zones do not collide with Trigger zones or Walls
    assert!(!trigger.can_collide(&trigger));
    assert!(!trigger.can_collide(&wall));
    assert!(!wall.can_collide(&trigger));

    // Projectiles collide with Walls
    assert!(projectile.can_collide(&wall));
    assert!(wall.can_collide(&projectile));
}

#[test]
fn test_static_obstacle_constructors() {
    let wall_shape = ColliderShape::AABB(DeterministicAABB::new(
        DeterministicVector2::new(I16F16::from_num(10), I16F16::from_num(10)),
        DeterministicVector2::new(I16F16::from_num(20), I16F16::from_num(20)),
    ));
    let wall = StaticObstacle::solid_wall(101, wall_shape);
    assert_eq!(wall.id, 101);
    assert!(wall.is_solid);
    assert_eq!(wall.filter.layer, CollisionFilter::SOLID_WALL);

    let trigger_shape = ColliderShape::Circle(DeterministicCircle::new(
        DeterministicVector2::new(I16F16::from_num(100), I16F16::from_num(100)),
        I16F16::from_num(25),
    ));
    let trigger = StaticObstacle::trigger_zone(102, trigger_shape);
    assert_eq!(trigger.id, 102);
    assert!(!trigger.is_solid);
    assert_eq!(trigger.filter.layer, CollisionFilter::TRIGGER_ZONE);
}

#[test]
fn test_entity_with_colliders_and_filter() {
    let entity = Entity::new(1, "Player1".to_string(), EntityType::Player)
        .with_circle_collider(I16F16::from_num(8))
        .with_collision_filter(CollisionFilter::default_player());

    assert_eq!(entity.id, 1);
    assert_eq!(
        entity.collider,
        Some(ColliderShape::Circle(DeterministicCircle::new(
            DeterministicVector2::ZERO,
            I16F16::from_num(8)
        )))
    );
    assert_eq!(entity.collision_filter, CollisionFilter::default_player());
}

#[test]
fn test_instance_multi_entity_boundary_clamping_simulation() {
    let mut instance = Instance::new(1, 30, 60, 42);
    // Custom arena bounds: [-300, +300]
    instance.set_map_bounds(MapBounds::new(
        DeterministicVector2::new(I16F16::from_num(-300), I16F16::from_num(-300)),
        DeterministicVector2::new(I16F16::from_num(300), I16F16::from_num(300)),
    ));

    // Player 1: Moving right at high velocity (+50 per tick) starting at (280, 0)
    let mut p1 = Entity::new(1, "RunnerRight".to_string(), EntityType::Player)
        .with_circle_collider(I16F16::from_num(10));
    p1.position = DeterministicVector2::new(I16F16::from_num(280), I16F16::from_num(0));
    p1.velocity = DeterministicVector2::new(I16F16::from_num(50), I16F16::from_num(0));
    instance.add_entity(p1);

    // Player 2: Moving top-left (-40, +40) starting at (-270, 270)
    let mut p2 = Entity::new(2, "RunnerTopLeft".to_string(), EntityType::Player)
        .with_circle_collider(I16F16::from_num(15));
    p2.position = DeterministicVector2::new(I16F16::from_num(-270), I16F16::from_num(270));
    p2.velocity = DeterministicVector2::new(I16F16::from_num(-40), I16F16::from_num(40));
    instance.add_entity(p2);

    // Advance 5 ticks
    for tick in 1..=5 {
        instance.tick(tick);
    }

    // p1 clamped at max_x - radius = 300 - 10 = 290
    let updated_p1 = instance.get_entity(1).unwrap();
    assert_eq!(updated_p1.position.x, I16F16::from_num(290));
    assert_eq!(updated_p1.position.y, I16F16::from_num(0));

    // p2 clamped at (-285, 285) because radius = 15
    let updated_p2 = instance.get_entity(2).unwrap();
    assert_eq!(updated_p2.position.x, I16F16::from_num(-285));
    assert_eq!(updated_p2.position.y, I16F16::from_num(285));
}
