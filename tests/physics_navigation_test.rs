use fixed::types::I16F16;
use loci2d::network::packets::{
    ClientIntent, MoveIntent, MoveToPositionIntent, Vector2, client_intent,
};
use loci2d::world::entity::{Entity, EntityType};
use loci2d::world::fixed_point::DeterministicVector2;
use loci2d::world::instance::Instance;
use loci2d::world::physics::{
    ColliderShape, CollisionFilter, DeterministicAABB, MapBounds, NavigationComponent,
    StaticObstacle, deterministic_distance,
};
use std::net::SocketAddr;

#[test]
fn test_click_to_move_straight_axis_arrival() {
    let mut instance = Instance::new(1, 30, 10);
    let mut entity = Entity::new(1, "Player1".to_string(), EntityType::Player)
        .with_aabb_collider(DeterministicVector2::new(
            I16F16::from_num(1),
            I16F16::from_num(1),
        ))
        .with_navigation(NavigationComponent::new(
            I16F16::from_num(1), // move_speed = 1.0
            I16F16::from_num(1), // arrival_tolerance = 1.0
        ));
    entity.position = DeterministicVector2::ZERO;

    // Navigate to (10, 0)
    let target = DeterministicVector2::new(I16F16::from_num(10), I16F16::ZERO);
    entity.navigation.as_mut().unwrap().set_target(target);
    instance.add_entity(entity);

    // Simulate 20 ticks
    for tick in 1..=20 {
        instance.tick(tick);
        let e = instance.get_entity(1).unwrap();

        if tick <= 10 {
            // Ticks 1..=10: moving right at 1.0 unit per tick towards (10, 0)
            assert_eq!(e.velocity.x, I16F16::from_num(1));
            assert_eq!(e.velocity.y, I16F16::ZERO);
            assert_eq!(e.position.x, I16F16::from_num(tick as i16));
        } else {
            // At tick 11 and beyond: entity is at (10, 0), stopped with no target
            assert_eq!(e.position.x, I16F16::from_num(10));
            assert_eq!(e.position.y, I16F16::ZERO);
            assert_eq!(e.velocity, DeterministicVector2::ZERO);
            assert!(
                !e.navigation.as_ref().unwrap().is_navigating(),
                "Navigation should be completed with no target"
            );
        }
    }

    let final_entity = instance.get_entity(1).unwrap();
    assert_eq!(
        final_entity.position,
        DeterministicVector2::new(I16F16::from_num(10), I16F16::ZERO)
    );
    assert_eq!(final_entity.velocity, DeterministicVector2::ZERO);
}

#[test]
fn test_click_to_move_diagonal_arrival() {
    let mut instance = Instance::new(1, 30, 10);
    let mut entity = Entity::new(1, "Player1".to_string(), EntityType::Player)
        .with_circle_collider(I16F16::from_num(1))
        .with_navigation(NavigationComponent::new(
            I16F16::from_num(2), // move_speed = 2.0
            I16F16::from_num(2), // arrival_tolerance = 2.0
        ));
    entity.position = DeterministicVector2::ZERO;

    // Navigate to (20, 20)
    let target = DeterministicVector2::new(I16F16::from_num(20), I16F16::from_num(20));
    entity.navigation.as_mut().unwrap().set_target(target);
    instance.add_entity(entity);

    // Simulate 30 ticks
    for tick in 1..=30 {
        instance.tick(tick);
    }

    let e = instance.get_entity(1).unwrap();
    let dist_to_target = deterministic_distance(e.position, target);
    assert!(
        dist_to_target <= I16F16::from_num(2),
        "Entity should arrive within arrival tolerance (dist={:?})",
        dist_to_target
    );
    assert_eq!(e.velocity, DeterministicVector2::ZERO);
    assert!(!e.navigation.as_ref().unwrap().is_navigating());
}

#[test]
fn test_multi_waypoint_path_traversal() {
    let mut instance = Instance::new(1, 30, 10);
    let mut entity = Entity::new(1, "Pathfinder".to_string(), EntityType::Player)
        .with_circle_collider(I16F16::from_num(1))
        .with_navigation(NavigationComponent::new(
            I16F16::from_num(1), // move_speed = 1.0
            I16F16::from_num(1), // arrival_tolerance = 1.0
        ));
    entity.position = DeterministicVector2::ZERO;

    // Waypoints forming a square path: (5, 0) -> (5, 5) -> (0, 5) -> (0, 0)
    let wp1 = DeterministicVector2::new(I16F16::from_num(5), I16F16::ZERO);
    let wp2 = DeterministicVector2::new(I16F16::from_num(5), I16F16::from_num(5));
    let wp3 = DeterministicVector2::new(I16F16::ZERO, I16F16::from_num(5));
    let wp4 = DeterministicVector2::new(I16F16::ZERO, I16F16::ZERO);

    entity
        .navigation
        .as_mut()
        .unwrap()
        .set_waypoints(vec![wp1, wp2, wp3, wp4]);
    instance.add_entity(entity);

    // Ticks 1..=5: moving to wp1 (5, 0)
    for tick in 1..=5 {
        instance.tick(tick);
    }
    let e = instance.get_entity(1).unwrap();
    assert_eq!(e.position, wp1);

    // Ticks 6..=10: transitioning and moving to wp2 (5, 5)
    for tick in 6..=10 {
        instance.tick(tick);
    }
    let e = instance.get_entity(1).unwrap();
    assert_eq!(e.position, wp2);

    // Ticks 11..=15: transitioning and moving to wp3 (0, 5)
    for tick in 11..=15 {
        instance.tick(tick);
    }
    let e = instance.get_entity(1).unwrap();
    assert_eq!(e.position, wp3);

    // Ticks 16..=20: transitioning and moving to wp4 (0, 0)
    for tick in 16..=20 {
        instance.tick(tick);
    }
    let e = instance.get_entity(1).unwrap();
    assert_eq!(e.position, wp4);

    // Tick 21: final arrival evaluated and stopped
    instance.tick(21);
    let e = instance.get_entity(1).unwrap();
    assert_eq!(e.position, wp4);
    assert_eq!(e.velocity, DeterministicVector2::ZERO);
    assert!(
        !e.navigation.as_ref().unwrap().is_navigating(),
        "Should complete entire waypoint sequence and stop"
    );
}

#[test]
fn test_wasd_move_preempts_navigation() {
    let mut instance = Instance::new(1, 30, 10);
    let addr: SocketAddr = "127.0.0.1:20001".parse().unwrap();

    // 1. Join
    let join_intent = ClientIntent {
        intent: Some(client_intent::Intent::Join(
            loci2d::network::packets::JoinIntent {
                player_name: "PreemptTest".to_string(),
            },
        )),
    };
    instance.apply_intent(addr, join_intent);

    // 2. Issue MoveToPos towards (100, 100)
    let move_to_pos = ClientIntent {
        intent: Some(client_intent::Intent::MoveToPos(MoveToPositionIntent {
            target_position: Some(Vector2 {
                x_bits: I16F16::from_num(100).to_bits(),
                y_bits: I16F16::from_num(100).to_bits(),
            }),
        })),
    };
    instance.apply_intent(addr, move_to_pos);

    // Advance 3 ticks
    instance.tick(1);
    instance.tick(2);
    instance.tick(3);

    let e = instance.get_entity(1).unwrap();
    assert!(e.navigation.as_ref().unwrap().is_navigating());

    // 3. Preempt with raw WASD Move intent pointing left (-2, 0)
    let wasd_intent = ClientIntent {
        intent: Some(client_intent::Intent::Move(MoveIntent {
            direction: Some(
                DeterministicVector2::new(I16F16::from_num(-2), I16F16::ZERO).to_proto(),
            ),
        })),
    };
    instance.apply_intent(addr, wasd_intent);

    let e = instance.get_entity(1).unwrap();
    assert!(
        !e.navigation.as_ref().unwrap().is_navigating(),
        "Direct WASD move must cancel active navigation"
    );
    assert_eq!(e.velocity.x, I16F16::from_num(-2));
    assert_eq!(e.velocity.y, I16F16::ZERO);

    // Advance next tick and ensure it moves according to WASD velocity
    let prev_pos = e.position;
    instance.tick(4);
    let e = instance.get_entity(1).unwrap();
    assert_eq!(
        e.position.x,
        prev_pos.x.saturating_add(I16F16::from_num(-2))
    );
}

#[test]
fn test_new_moveto_preempts_prior_target() {
    let mut instance = Instance::new(1, 30, 10);
    let addr: SocketAddr = "127.0.0.1:20002".parse().unwrap();

    // 1. Join
    instance.apply_intent(
        addr,
        ClientIntent {
            intent: Some(client_intent::Intent::Join(
                loci2d::network::packets::JoinIntent {
                    player_name: "RetargetPlayer".to_string(),
                },
            )),
        },
    );

    // 2. MoveTo (50, 0)
    instance.apply_intent(
        addr,
        ClientIntent {
            intent: Some(client_intent::Intent::MoveToPos(MoveToPositionIntent {
                target_position: Some(Vector2 {
                    x_bits: I16F16::from_num(50).to_bits(),
                    y_bits: I16F16::ZERO.to_bits(),
                }),
            })),
        },
    );
    instance.tick(1);
    let e = instance.get_entity(1).unwrap();
    assert_eq!(
        e.navigation.as_ref().unwrap().target,
        Some(DeterministicVector2::new(
            I16F16::from_num(50),
            I16F16::ZERO
        ))
    );
    assert_eq!(e.velocity.x, I16F16::from_num(1));

    // 3. New MoveTo (-20, 0)
    instance.apply_intent(
        addr,
        ClientIntent {
            intent: Some(client_intent::Intent::MoveToPos(MoveToPositionIntent {
                target_position: Some(Vector2 {
                    x_bits: I16F16::from_num(-20).to_bits(),
                    y_bits: I16F16::ZERO.to_bits(),
                }),
            })),
        },
    );
    instance.tick(2);
    let e = instance.get_entity(1).unwrap();
    assert_eq!(
        e.navigation.as_ref().unwrap().target,
        Some(DeterministicVector2::new(
            I16F16::from_num(-20),
            I16F16::ZERO
        ))
    );
    assert_eq!(e.velocity.x, I16F16::from_num(-1));
}

#[test]
fn test_navigation_respects_static_obstacles() {
    let mut instance = Instance::new(1, 30, 10);
    // Add solid wall from x=8 to x=12, y=-10 to y=10
    let wall = StaticObstacle::solid_wall(
        100,
        ColliderShape::AABB(DeterministicAABB::new(
            DeterministicVector2::new(I16F16::from_num(8), I16F16::from_num(-10)),
            DeterministicVector2::new(I16F16::from_num(12), I16F16::from_num(10)),
        )),
    );
    instance.add_static_obstacle(wall);

    // Entity at (0, 0) with radius 1.0 navigating to (20, 0)
    let mut entity = Entity::new(1, "BlockedWalker".to_string(), EntityType::Player)
        .with_circle_collider(I16F16::from_num(1))
        .with_collision_filter(CollisionFilter::default_player())
        .with_navigation(NavigationComponent::new(
            I16F16::from_num(1),
            I16F16::from_num(1),
        ));
    entity.position = DeterministicVector2::ZERO;
    entity
        .navigation
        .as_mut()
        .unwrap()
        .set_target(DeterministicVector2::new(
            I16F16::from_num(20),
            I16F16::ZERO,
        ));
    instance.add_entity(entity);

    // Simulate 20 ticks
    for tick in 1..=20 {
        instance.tick(tick);
    }

    let e = instance.get_entity(1).unwrap();
    // Entity should be stopped at the left face of the wall (x = 8 - 1 = 7)
    assert!(
        e.position.x <= I16F16::from_num(7),
        "Entity must not penetrate solid wall (x={:?})",
        e.position.x
    );
}

#[test]
fn test_navigation_map_bounds_clamping() {
    let mut instance = Instance::new(1, 30, 10);
    instance.set_map_bounds(MapBounds::new(
        DeterministicVector2::new(I16F16::from_num(-50), I16F16::from_num(-50)),
        DeterministicVector2::new(I16F16::from_num(50), I16F16::from_num(50)),
    ));

    let mut entity = Entity::new(1, "OutboundWalker".to_string(), EntityType::Player)
        .with_circle_collider(I16F16::from_num(2))
        .with_navigation(NavigationComponent::new(
            I16F16::from_num(5),
            I16F16::from_num(2),
        ));
    entity.position = DeterministicVector2::ZERO;
    // Destination far outside map bounds
    entity
        .navigation
        .as_mut()
        .unwrap()
        .set_target(DeterministicVector2::new(
            I16F16::from_num(500),
            I16F16::ZERO,
        ));
    instance.add_entity(entity);

    for tick in 1..=30 {
        instance.tick(tick);
    }

    let e = instance.get_entity(1).unwrap();
    // Clamped at 50 - radius (2) = 48
    assert_eq!(
        e.position.x,
        I16F16::from_num(48),
        "Entity should be clamped safely at arena boundary"
    );
}

#[test]
fn test_push_waypoint_queue() {
    let mut nav = NavigationComponent::new(I16F16::from_num(1), I16F16::from_num(1));
    let wp1 = DeterministicVector2::new(I16F16::from_num(10), I16F16::ZERO);
    let wp2 = DeterministicVector2::new(I16F16::from_num(20), I16F16::ZERO);

    nav.push_waypoint(wp1);
    assert_eq!(nav.target, Some(wp1));
    assert_eq!(nav.waypoints.len(), 1);

    nav.push_waypoint(wp2);
    assert_eq!(nav.target, Some(wp1));
    assert_eq!(nav.waypoints.len(), 2);
}
