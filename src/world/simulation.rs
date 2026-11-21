use crate::logging::{self, LogLevel};
use crate::scripting::CommandBuffer;
use crate::world::entity::EntityType;
use crate::world::fixed_point::DeterministicVector2;
use crate::world::instance::{Instance, MatchState};
use crate::world::physics::{
    ColliderShape, CollisionFilter, DeterministicCircle, TriggerEvent, TriggerEventType,
    deterministic_distance, intersect_shapes, resolve_dynamic_collision, resolve_static_collision,
    swept_circle_vs_shape, update_entity_navigation,
};
use fixed::types::I16F16;
use std::collections::{BTreeMap, BTreeSet};

/// Advance physics using deterministic fixed-point integration, resolve solid collisions against
/// static obstacles and dynamic entities, evaluate trigger sensor zones, clamp to map boundaries,
/// and sweep for timed-out sessions.
/// Returns a list of (entity_id, player_name) for any sessions that timed out during this tick.
pub fn tick(instance: &mut Instance, tick_count: u64) -> Result<Vec<(u64, String)>, String> {
    let mut cmd_buffer = CommandBuffer::new();

    if instance.state == MatchState::Running {
        let mut completed_timers = Vec::new();
        for (timer_id, timer) in instance.active_timers.iter_mut() {
            if timer.remaining_ticks > 0 {
                timer.remaining_ticks -= 1;
                if timer.remaining_ticks == 0 {
                    completed_timers.push(timer_id.clone());
                }
            }
        }
        for timer_id in completed_timers {
            instance.active_timers.remove(&timer_id);
            instance
                .script_engine
                .on_timer_complete(instance, timer_id, &mut cmd_buffer)
                .map_err(|e| e.to_string())?;
        }

        instance
            .script_engine
            .on_tick(instance, tick_count, &mut cmd_buffer)
            .map_err(|e| e.to_string())?;

        // 0. Steering & Destination Navigation Update
        for entity in instance.entities.values_mut() {
            if let Some(ref mut nav) = entity.navigation {
                update_entity_navigation(entity.position, &mut entity.velocity, nav);
            }
        }

        let log_trace_physics = instance.logging_enabled && logging::enabled(LogLevel::Trace);
        let log_debug_physics = instance.logging_enabled && logging::enabled(LogLevel::Debug);

        // 1. Velocity Integration (Candidate Next Position), Swept CCD & Map Bounds Clamping
        let mut boundary_collisions: Vec<(u64, DeterministicVector2)> = Vec::new();
        let mut swept_static_hits: BTreeMap<(u64, u64), DeterministicVector2> = BTreeMap::new();
        let mut swept_dynamic_hits: BTreeMap<(u64, u64), I16F16> = BTreeMap::new();

        struct PreStepEntity {
            id: u64,
            old_pos: DeterministicVector2,
            vel: DeterministicVector2,
            entity_type: EntityType,
            collider: Option<ColliderShape>,
            filter: CollisionFilter,
        }

        let pre_step: Vec<PreStepEntity> = instance
            .entities
            .iter()
            .map(|(&id, e)| PreStepEntity {
                id,
                old_pos: e.position,
                vel: e.velocity,
                entity_type: e.entity_type,
                collider: e.collider,
                filter: e.collision_filter,
            })
            .collect();

        let mut integrated_positions: Vec<(u64, DeterministicVector2)> =
            Vec::with_capacity(pre_step.len());

        for ent in &pre_step {
            let radius = match ent.collider {
                Some(ColliderShape::Circle(c)) => c.radius,
                Some(ColliderShape::AABB(a)) => a.half_extents().x.min(a.half_extents().y),
                None => I16F16::ZERO,
            };
            let speed = deterministic_distance(DeterministicVector2::ZERO, ent.vel);
            let use_ccd = ent.entity_type == EntityType::Projectile && speed > I16F16::ZERO;

            let unclamped_pos = if use_ccd {
                // Sweep against solid static obstacles first so walls stop & occlude the projectile
                let mut best_static: Option<(I16F16, u64, DeterministicVector2)> = None;
                for obstacle in instance.static_obstacles.values() {
                    if !obstacle.is_solid || !ent.filter.can_collide(&obstacle.filter) {
                        continue;
                    }
                    if let Some((hit_t, normal)) =
                        swept_circle_vs_shape(ent.old_pos, ent.vel, radius, &obstacle.shape)
                        && best_static.is_none_or(|(best_t, _, _)| hit_t < best_t)
                    {
                        best_static = Some((hit_t, obstacle.id, normal));
                    }
                }

                let (travel_dist, candidate_pos) =
                    if let Some((hit_t, obstacle_id, normal)) = best_static {
                        swept_static_hits.insert((obstacle_id, ent.id), normal);
                        let dir = ent.vel / speed;
                        (
                            hit_t,
                            ent.old_pos.saturating_add(DeterministicVector2::new(
                                dir.x * hit_t,
                                dir.y * hit_t,
                            )),
                        )
                    } else {
                        (speed, ent.old_pos.saturating_add(ent.vel))
                    };

                // Sweep the wall-clamped segment against dynamic entities without snagging
                let sweep_delta = candidate_pos.saturating_sub(ent.old_pos);
                for other in &pre_step {
                    if other.id == ent.id || !ent.filter.can_collide(&other.filter) {
                        continue;
                    }
                    let Some(other_shape) = other.collider.map(|s| s.at_position(other.old_pos))
                    else {
                        continue;
                    };
                    if let Some((dyn_t, _normal)) =
                        swept_circle_vs_shape(ent.old_pos, sweep_delta, radius, &other_shape)
                        && dyn_t <= travel_dist
                    {
                        let pair = (ent.id.min(other.id), ent.id.max(other.id));
                        swept_dynamic_hits
                            .entry(pair)
                            .and_modify(|existing| {
                                if dyn_t < *existing {
                                    *existing = dyn_t;
                                }
                            })
                            .or_insert(dyn_t);
                    }
                }

                candidate_pos
            } else {
                ent.old_pos.saturating_add(ent.vel)
            };

            let clamped_pos = match ent.collider {
                Some(shape) => instance
                    .map_bounds
                    .clamp_shape(&shape.at_position(unclamped_pos)),
                None => instance.map_bounds.clamp_point(unclamped_pos),
            };

            if clamped_pos != unclamped_pos {
                let diff_x = clamped_pos.x - unclamped_pos.x;
                let diff_y = clamped_pos.y - unclamped_pos.y;
                let nx = if diff_x > I16F16::ZERO {
                    I16F16::ONE
                } else if diff_x < I16F16::ZERO {
                    -I16F16::ONE
                } else {
                    I16F16::ZERO
                };
                let ny = if diff_y > I16F16::ZERO {
                    I16F16::ONE
                } else if diff_y < I16F16::ZERO {
                    -I16F16::ONE
                } else {
                    I16F16::ZERO
                };
                let normal = DeterministicVector2::new(nx, ny).normalize_or_zero();
                boundary_collisions.push((ent.id, normal));
            }

            integrated_positions.push((ent.id, clamped_pos));
        }

        for (entity_id, new_pos) in integrated_positions {
            if let Some(entity) = instance.entities.get_mut(&entity_id) {
                let old_pos = entity.position;
                entity.position = new_pos;
                if log_trace_physics && entity.position != old_pos {
                    crate::log_trace!(
                        tick = tick_count,
                        "Physics",
                        "Player {} moved to ({:.2}, {:.2})",
                        entity.name,
                        entity.position.x.to_num::<f32>(),
                        entity.position.y.to_num::<f32>()
                    );
                }
            }
        }

        // 2. Static Solid Obstacle Collision Resolution & Static/Boundary Callbacks
        // Evaluated in strict ascending obstacle.id order
        let mut current_solid_collisions = BTreeSet::new();
        let mut static_callbacks: Vec<(u64, u64, DeterministicVector2)> = Vec::new();
        for obstacle in instance.static_obstacles.values() {
            if !obstacle.is_solid {
                continue;
            }
            for entity in instance.entities.values_mut() {
                if !entity.collision_filter.can_collide(&obstacle.filter) {
                    continue;
                }
                let Some(entity_shape) = entity.current_collider() else {
                    continue;
                };
                let manifold = intersect_shapes(&entity_shape, &obstacle.shape);
                if manifold.is_colliding {
                    if !entity.is_sensor {
                        resolve_static_collision(
                            &mut entity.position,
                            &mut entity.velocity,
                            &manifold,
                        );
                    } else if manifold.penetration_depth > I16F16::ZERO
                        && manifold.normal != DeterministicVector2::ZERO
                    {
                        // Position sensor at contact surface without projecting velocity (ADR-0021)
                        // so ricochet scripts can inspect pre-impact velocity cleanly and bounce away.
                        entity.position.x += manifold.normal.x * manifold.penetration_depth;
                        entity.position.y += manifold.normal.y * manifold.penetration_depth;
                    }
                    static_callbacks.push((entity.id, obstacle.id, manifold.normal));

                    if log_debug_physics {
                        let pair = (entity.id, obstacle.id);
                        current_solid_collisions.insert(pair);
                        if !instance.active_solid_collisions.contains(&pair) {
                            crate::log_debug!(
                                tick = tick_count,
                                "Physics",
                                "Entity {} ('{}') entered static collision with obstacle #{} at ({:.2}, {:.2})",
                                entity.id,
                                entity.name,
                                obstacle.id,
                                entity.position.x.to_num::<f32>(),
                                entity.position.y.to_num::<f32>()
                            );
                        } else if log_trace_physics {
                            crate::log_trace!(
                                tick = tick_count,
                                "Physics",
                                "Entity {} ('{}') collided with obstacle #{} at ({:.2}, {:.2})",
                                entity.id,
                                entity.name,
                                obstacle.id,
                                entity.position.x.to_num::<f32>(),
                                entity.position.y.to_num::<f32>()
                            );
                        }
                    }
                    // Re-clamp to map bounds to ensure pushback didn't push outside arena
                    entity.position = match entity.current_collider() {
                        Some(shape) => instance.map_bounds.clamp_shape(&shape),
                        None => instance.map_bounds.clamp_point(entity.position),
                    };
                } else if let Some(&swept_normal) =
                    swept_static_hits.get(&(obstacle.id, entity.id))
                {
                    static_callbacks.push((entity.id, obstacle.id, swept_normal));
                }
            }
        }
        if log_debug_physics {
            instance.active_solid_collisions = current_solid_collisions;
        } else if !instance.active_solid_collisions.is_empty() {
            instance.active_solid_collisions.clear();
        }

        for (entity_id, obstacle_id, normal) in static_callbacks {
            instance
                .script_engine
                .on_static_collision(
                    instance,
                    entity_id,
                    obstacle_id,
                    normal.x.to_num::<f64>(),
                    normal.y.to_num::<f64>(),
                    &mut cmd_buffer,
                )
                .map_err(|e| e.to_string())?;
        }

        for (entity_id, normal) in boundary_collisions {
            instance
                .script_engine
                .on_boundary_collision(
                    instance,
                    entity_id,
                    normal.x.to_num::<f64>(),
                    normal.y.to_num::<f64>(),
                    &mut cmd_buffer,
                )
                .map_err(|e| e.to_string())?;
        }

        // 3. Dynamic Entity-vs-Entity Collision Resolution (50/50 Split Pushback for Rigid, Zero Pushback for Sensors)
        // Evaluated in strict ascending (entity_a.id, entity_b.id) pair order
        let mut current_dynamic_collisions = BTreeSet::new();
        let entity_ids: Vec<u64> = instance.entities.keys().copied().collect();
        for i in 0..entity_ids.len() {
            for j in (i + 1)..entity_ids.len() {
                let id_a = entity_ids[i];
                let id_b = entity_ids[j];

                let can_collide = {
                    let entity_a = &instance.entities[&id_a];
                    let entity_b = &instance.entities[&id_b];
                    entity_a
                        .collision_filter
                        .can_collide(&entity_b.collision_filter)
                        && entity_a.collider.is_some()
                        && entity_b.collider.is_some()
                };

                if !can_collide {
                    continue;
                }

                let shape_a = instance.entities[&id_a].current_collider().unwrap();
                let shape_b = instance.entities[&id_b].current_collider().unwrap();
                let manifold = intersect_shapes(&shape_a, &shape_b);
                let swept_hit = swept_dynamic_hits.contains_key(&(id_a, id_b));

                if manifold.is_colliding || swept_hit {
                    let is_sensor_pair =
                        instance.entities[&id_a].is_sensor || instance.entities[&id_b].is_sensor;

                    if !is_sensor_pair && manifold.is_colliding {
                        let mut pos_a = instance.entities[&id_a].position;
                        let mut vel_a = instance.entities[&id_a].velocity;
                        let mut pos_b = instance.entities[&id_b].position;
                        let mut vel_b = instance.entities[&id_b].velocity;

                        resolve_dynamic_collision(
                            &mut pos_a,
                            &mut vel_a,
                            &mut pos_b,
                            &mut vel_b,
                            &manifold,
                        );

                        let entity_a = instance.entities.get_mut(&id_a).unwrap();
                        entity_a.position = pos_a;
                        entity_a.velocity = vel_a;
                        if let Some(shape) = entity_a.current_collider() {
                            entity_a.position = instance.map_bounds.clamp_shape(&shape);
                        }

                        let entity_b = instance.entities.get_mut(&id_b).unwrap();
                        entity_b.position = pos_b;
                        entity_b.velocity = vel_b;
                        if let Some(shape) = entity_b.current_collider() {
                            entity_b.position = instance.map_bounds.clamp_shape(&shape);
                        }
                    }

                    instance
                        .script_engine
                        .on_collision(instance, id_a, id_b, &mut cmd_buffer)
                        .map_err(|e| e.to_string())?;

                    if log_debug_physics {
                        let pair = (id_a, id_b);
                        current_dynamic_collisions.insert(pair);
                        let pos_a = instance.entities[&id_a].position;
                        let pos_b = instance.entities[&id_b].position;
                        let name_a = &instance.entities[&id_a].name;
                        let name_b = &instance.entities[&id_b].name;
                        if !instance.active_dynamic_collisions.contains(&pair) {
                            crate::log_debug!(
                                tick = tick_count,
                                "Physics",
                                "Entity {} ('{}') entered dynamic collision with Entity {} ('{}') at ({:.2}, {:.2}) / ({:.2}, {:.2})",
                                id_a,
                                name_a,
                                id_b,
                                name_b,
                                pos_a.x.to_num::<f32>(),
                                pos_a.y.to_num::<f32>(),
                                pos_b.x.to_num::<f32>(),
                                pos_b.y.to_num::<f32>()
                            );
                        } else if log_trace_physics {
                            crate::log_trace!(
                                tick = tick_count,
                                "Physics",
                                "Entity {} ('{}') collided with Entity {} ('{}') at ({:.2}, {:.2}) / ({:.2}, {:.2})",
                                id_a,
                                name_a,
                                id_b,
                                name_b,
                                pos_a.x.to_num::<f32>(),
                                pos_a.y.to_num::<f32>(),
                                pos_b.x.to_num::<f32>(),
                                pos_b.y.to_num::<f32>()
                            );
                        }
                    }
                }
            }
        }
        if log_debug_physics {
            instance.active_dynamic_collisions = current_dynamic_collisions;
        } else if !instance.active_dynamic_collisions.is_empty() {
            instance.active_dynamic_collisions.clear();
        }

        // 4. Trigger / Sensor Zone Overlap Evaluation & Lifecycle Events
        instance.trigger_events.clear();
        let mut current_overlaps = BTreeSet::new();

        for (&trigger_id, obstacle) in &instance.static_obstacles {
            if obstacle.is_solid {
                continue;
            }
            for (&entity_id, entity) in &instance.entities {
                if !obstacle.filter.can_collide(&entity.collision_filter) {
                    continue;
                }
                let entity_shape = match entity.current_collider() {
                    Some(s) => s,
                    None => ColliderShape::Circle(DeterministicCircle::new(
                        entity.position,
                        I16F16::ZERO,
                    )),
                };
                let manifold = intersect_shapes(&entity_shape, &obstacle.shape);
                if manifold.is_colliding {
                    current_overlaps.insert((trigger_id, entity_id));
                }
            }
        }

        // Generate Enter, Stay, Exit events in strictly sorted (trigger_id, entity_id) order
        let all_pairs: BTreeSet<(u64, u64)> = instance
            .previous_trigger_overlaps
            .union(&current_overlaps)
            .copied()
            .collect();

        for (trigger_id, entity_id) in all_pairs {
            let was_present = instance
                .previous_trigger_overlaps
                .contains(&(trigger_id, entity_id));
            let is_present = current_overlaps.contains(&(trigger_id, entity_id));
            let event_type = match (was_present, is_present) {
                (false, true) => {
                    instance
                        .script_engine
                        .on_trigger_enter(instance, entity_id, trigger_id, &mut cmd_buffer)
                        .map_err(|e| e.to_string())?;
                    TriggerEventType::Enter
                }
                (true, true) => {
                    instance
                        .script_engine
                        .on_trigger_stay(instance, entity_id, trigger_id, &mut cmd_buffer)
                        .map_err(|e| e.to_string())?;
                    TriggerEventType::Stay
                }
                (true, false) => {
                    instance
                        .script_engine
                        .on_trigger_exit(instance, entity_id, trigger_id, &mut cmd_buffer)
                        .map_err(|e| e.to_string())?;
                    TriggerEventType::Exit
                }
                (false, false) => unreachable!(),
            };
            instance.trigger_events.push(TriggerEvent::new(
                trigger_id, entity_id, event_type, tick_count,
            ));
        }

        instance.previous_trigger_overlaps = current_overlaps.clone();
        instance.active_trigger_overlaps = current_overlaps;

        // 4.5 Entity Lifetime Countdown (auto-despawn expired projectiles/entities via CommandBuffer)
        for (&entity_id, entity) in instance.entities.iter_mut() {
            if let Some(ref mut remaining) = entity.remaining_lifetime_ticks {
                if *remaining > 0 {
                    *remaining -= 1;
                    if *remaining == 0 {
                        cmd_buffer.push(crate::scripting::command::Command::DestroyEntity {
                            entity_id,
                        });
                    }
                } else {
                    cmd_buffer.push(crate::scripting::command::Command::DestroyEntity {
                        entity_id,
                    });
                }
            }
        }
    }

    // 5. Check for timed out clients
    let timed_out = instance.check_timeouts();
    for (entity_id, _) in &timed_out {
        let _ = instance
            .script_engine
            .on_player_leave(instance, *entity_id, &mut cmd_buffer);
        cmd_buffer.push(crate::scripting::command::Command::DestroyEntity { entity_id: *entity_id });
    }

    cmd_buffer.flush_and_apply(instance);

    Ok(timed_out)
}
