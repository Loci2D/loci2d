use crate::scripting::CommandBuffer;
use crate::world::instance::{Instance, MatchState};
use crate::world::physics::{
    ColliderShape, DeterministicCircle, TriggerEvent, TriggerEventType, intersect_shapes,
    resolve_dynamic_collision, resolve_static_collision, update_entity_navigation,
};
use fixed::types::I16F16;
use std::collections::BTreeSet;

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

        // 1. Velocity Integration (Candidate Next Position) & Initial Map Bounds Clamping
        for entity in instance.entities.values_mut() {
            let old_pos = entity.position;
            entity.position = entity.position.saturating_add(entity.velocity);

            entity.position = match entity.current_collider() {
                Some(shape) => instance.map_bounds.clamp_shape(&shape),
                None => instance.map_bounds.clamp_point(entity.position),
            };

            if instance.logging_enabled && entity.position != old_pos {
                println!(
                    "Player {} moved to ({:.2}, {:.2})",
                    entity.name,
                    entity.position.x.to_num::<f32>(),
                    entity.position.y.to_num::<f32>()
                );
            }
        }

        // 2. Static Solid Obstacle Collision Resolution (100% Pushback & Wall Sliding)
        // Evaluated in strict ascending obstacle.id order
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
                    resolve_static_collision(&mut entity.position, &mut entity.velocity, &manifold);
                    if instance.logging_enabled {
                        println!(
                            "Player {} collided at coordinates ({:.2}, {:.2})",
                            entity.name,
                            entity.position.x.to_num::<f32>(),
                            entity.position.y.to_num::<f32>()
                        );
                    }
                    // Re-clamp to map bounds to ensure pushback didn't push outside arena
                    entity.position = match entity.current_collider() {
                        Some(shape) => instance.map_bounds.clamp_shape(&shape),
                        None => instance.map_bounds.clamp_point(entity.position),
                    };
                }
            }
        }

        // 3. Dynamic Entity-vs-Entity Collision Resolution (50/50 Split Pushback)
        // Evaluated in strict ascending (entity_a.id, entity_b.id) pair order
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

                if manifold.is_colliding {
                    let mut pos_a = instance.entities[&id_a].position;
                    let mut vel_a = instance.entities[&id_a].velocity;
                    let mut pos_b = instance.entities[&id_b].position;
                    let mut vel_b = instance.entities[&id_b].velocity;

                    resolve_dynamic_collision(
                        &mut pos_a, &mut vel_a, &mut pos_b, &mut vel_b, &manifold,
                    );

                    instance
                        .script_engine
                        .on_collision(instance, id_a, id_b, &mut cmd_buffer)
                        .map_err(|e| e.to_string())?;

                    if instance.logging_enabled {
                        println!(
                            "Player {} collided at coordinates ({:.2}, {:.2})",
                            instance.entities[&id_a].name,
                            pos_a.x.to_num::<f32>(),
                            pos_a.y.to_num::<f32>()
                        );
                        println!(
                            "Player {} collided at coordinates ({:.2}, {:.2})",
                            instance.entities[&id_b].name,
                            pos_b.x.to_num::<f32>(),
                            pos_b.y.to_num::<f32>()
                        );
                    }

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
            }
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
    }

    // 5. Check for timed out clients
    let timed_out = instance.check_timeouts();

    cmd_buffer.flush_and_apply(instance);

    Ok(timed_out)
}
