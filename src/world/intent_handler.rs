use crate::network::packets::client_intent::Intent;
use crate::scripting::command::CommandBuffer;
use crate::world::entity::{Entity, EntityType};
use crate::world::fixed_point::DeterministicVector2;
use crate::world::instance::Instance;

use fixed::types::I16F16;

pub enum IntentResult {
    Ok,
    Rejected(String),
    FatalError(String),
}

/// Applies a resolved intent to the instance. This is used by both live client intents
/// and replay playback to ensure identical behavior and identical Lua callback invocation.
pub fn apply_resolved_intent(
    instance: &mut Instance,
    entity_id: u64,
    player_name: String,
    intent: &Intent,
) -> IntentResult {
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
            if let Err(e) = instance.script_engine.on_player_join(instance, entity_id, &mut cmd_buffer) {
                return IntentResult::FatalError(e.to_string());
            }
            cmd_buffer.flush_and_apply(instance);
        }
        Intent::Disconnect(_) => {
            let mut cmd_buffer = CommandBuffer::new();
            if let Err(e) = instance.script_engine.on_player_leave(instance, entity_id, &mut cmd_buffer) {
                return IntentResult::FatalError(e.to_string());
            }
            cmd_buffer.push(crate::scripting::command::Command::DestroyEntity { entity_id });
            cmd_buffer.flush_and_apply(instance);
        }
        Intent::Move(move_intent) => {
            let (dir_x, dir_y) = match &move_intent.direction {
                Some(dir) => {
                    let vec = DeterministicVector2::from_proto(dir);
                    (vec.x.to_num::<f64>(), vec.y.to_num::<f64>())
                }
                None => (0.0, 0.0),
            };

            let mut cmd_buffer = CommandBuffer::new();
            match instance.script_engine.on_move_intent(instance, entity_id, dir_x, dir_y, &mut cmd_buffer) {
                Ok(Some(reason)) => return IntentResult::Rejected(reason),
                Ok(None) => {},
                Err(e) => return IntentResult::FatalError(e.to_string()),
            }
            cmd_buffer.flush_and_apply(instance);
        }
        Intent::MoveToPos(move_to_pos_intent) => {
            let (target_x, target_y) = match &move_to_pos_intent.target_position {
                Some(target) => {
                    let vec = DeterministicVector2::new(
                        I16F16::from_bits(target.x_bits),
                        I16F16::from_bits(target.y_bits),
                    );
                    (vec.x.to_num::<f64>(), vec.y.to_num::<f64>())
                }
                None => (0.0, 0.0),
            };

            let mut cmd_buffer = CommandBuffer::new();
            match instance.script_engine.on_nav_intent(instance, entity_id, target_x, target_y, &mut cmd_buffer) {
                Ok(Some(reason)) => return IntentResult::Rejected(reason),
                Ok(None) => {},
                Err(e) => return IntentResult::FatalError(e.to_string()),
            }
            cmd_buffer.flush_and_apply(instance);
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
            match instance.script_engine.on_action(
                instance,
                entity_id,
                action_intent.ability_id,
                dir_x,
                dir_y,
                &mut cmd_buffer,
            ) {
                Ok(Some(reason)) => return IntentResult::Rejected(reason),
                Ok(None) => {},
                Err(e) => return IntentResult::FatalError(e.to_string()),
            }
            cmd_buffer.flush_and_apply(instance);
        }
        Intent::Ping(_) => {
            // Heartbeat, no state mutation
        }
    }
    IntentResult::Ok
}
