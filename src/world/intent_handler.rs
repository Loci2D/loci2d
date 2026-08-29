use crate::network::packets::client_intent::Intent;
use crate::scripting::command::CommandBuffer;
use crate::world::entity::{Entity, EntityType};
use crate::world::fixed_point::DeterministicVector2;
use crate::world::instance::Instance;
use crate::world::physics::navigation::NavigationComponent;
use fixed::types::I16F16;

/// Applies a resolved intent to the instance. This is used by both live client intents
/// and replay playback to ensure identical behavior and identical Lua callback invocation.
pub fn apply_resolved_intent(
    instance: &mut Instance,
    entity_id: u64,
    player_name: String,
    intent: &Intent,
) -> Result<(), String> {
    match intent {
        Intent::Join(join_intent) => {
            let final_name = if join_intent.player_name.trim().is_empty() {
                player_name
            } else {
                join_intent.player_name.clone()
            };

            if let Some(existing) = instance.entities.get_mut(&entity_id) {
                existing.name = final_name;
            } else {
                let entity = Entity::new(entity_id, final_name, EntityType::Player)
                    .with_default_navigation(I16F16::from_num(1), I16F16::from_num(1))
                    .with_circle_collider(I16F16::from_num(2));
                instance.entities.insert(entity_id, entity);
            }

            let mut cmd_buffer = CommandBuffer::new();
            instance
                .script_engine
                .on_player_join(instance, entity_id, &mut cmd_buffer)
                .map_err(|e| e.to_string())?;
            cmd_buffer.flush_and_apply(instance);
        }
        Intent::Disconnect(_) => {
            instance.entities.remove(&entity_id);

            let mut cmd_buffer = CommandBuffer::new();
            instance
                .script_engine
                .on_player_leave(instance, entity_id, &mut cmd_buffer)
                .map_err(|e| e.to_string())?;
            cmd_buffer.flush_and_apply(instance);
        }
        Intent::Move(move_intent) => {
            if let Some(entity) = instance.entities.get_mut(&entity_id) {
                if let Some(dir) = &move_intent.direction {
                    entity.velocity = DeterministicVector2::from_proto(dir);
                }
                if let Some(ref mut nav) = entity.navigation {
                    nav.clear();
                }
            }
        }
        Intent::MoveToPos(move_to_pos_intent) => {
            if let (Some(target), Some(entity)) = (
                &move_to_pos_intent.target_position,
                instance.entities.get_mut(&entity_id),
            ) {
                let target_vec = DeterministicVector2::new(
                    I16F16::from_bits(target.x_bits),
                    I16F16::from_bits(target.y_bits),
                );
                let nav = entity.navigation.get_or_insert_with(|| {
                    NavigationComponent::new(I16F16::from_num(1), I16F16::from_num(1))
                });
                nav.set_target(target_vec);
            }
        }
        Intent::Action(action_intent) => {
            if let Some(entity) = instance.entities.get(&entity_id) {
                if instance.logging_enabled {
                    println!(
                        "[Intent] Entity {} ({}) executed action {}",
                        entity_id, entity.name, action_intent.ability_id
                    );
                }
            }

            let (dir_x, dir_y) = match &action_intent.target_direction {
                Some(dir) => {
                    let vec = DeterministicVector2::from_proto(dir);
                    (vec.x.to_num::<f64>(), vec.y.to_num::<f64>())
                }
                None => (0.0, 0.0),
            };

            let mut cmd_buffer = CommandBuffer::new();
            instance
                .script_engine
                .on_action(
                    instance,
                    entity_id,
                    action_intent.ability_id,
                    dir_x,
                    dir_y,
                    &mut cmd_buffer,
                )
                .map_err(|e| e.to_string())?;
            cmd_buffer.flush_and_apply(instance);
        }
        Intent::Ping(_) => {
            // Heartbeat, no state mutation
        }
    }
    Ok(())
}
