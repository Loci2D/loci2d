// Canonical state hashing for determinism verification and replay checkpoints (ADR-0007, ADR-0010).

use crate::world::instance::Instance;
use sha2::{Digest, Sha256};

/// Computes a canonical SHA-256 hash of the instance world state on a given tick.
/// Guarantees bit-exact hashing across CPU architectures by iterating entities
/// in sorted key order (BTreeMap) and hashing raw integer bits of fixed-point coordinates.
pub fn compute_canonical_state_hash(instance: &Instance, tick: u64) -> [u8; 32] {
    let mut hasher = Sha256::new();

    // 1. Hash tick number (big-endian)
    hasher.update(tick.to_be_bytes());

    // 2. Hash MatchState
    match &instance.state {
        crate::world::instance::MatchState::Paused => hasher.update([0]),
        crate::world::instance::MatchState::Running => hasher.update([1]),
        crate::world::instance::MatchState::Ended { winner_data } => {
            hasher.update([2]);
            hasher.update((winner_data.len() as u32).to_be_bytes());
            hasher.update(winner_data.as_bytes());
        }
    }

    // 3. Hash globals
    hasher.update((instance.globals.len() as u32).to_be_bytes());
    for (k, v) in &instance.globals {
        hasher.update((k.len() as u32).to_be_bytes());
        hasher.update(k.as_bytes());
        hasher.update((v.len() as u32).to_be_bytes());
        hasher.update(v.as_bytes());
    }

    // 4. Hash active_timers
    hasher.update((instance.active_timers.len() as u32).to_be_bytes());
    for (timer_id, timer) in &instance.active_timers {
        hasher.update((timer_id.len() as u32).to_be_bytes());
        hasher.update(timer_id.as_bytes());
        hasher.update(timer.remaining_ticks.to_be_bytes());
    }

    // 5. Hash active entity count
    let entity_count = instance.entities.len() as u32;
    hasher.update(entity_count.to_be_bytes());

    // 6. Iterate through entities in strict BTreeMap key order (1, 2, 3...)
    for (entity_id, entity) in &instance.entities {
        hasher.update(entity_id.to_be_bytes());
        hasher.update((entity.name.len() as u32).to_be_bytes());
        hasher.update(entity.name.as_bytes());

        // Fixed-point raw integer bits (100% bit-exact across platforms)
        hasher.update(entity.position.x.to_bits().to_be_bytes());
        hasher.update(entity.position.y.to_bits().to_be_bytes());
        hasher.update(entity.velocity.x.to_bits().to_be_bytes());
        hasher.update(entity.velocity.y.to_bits().to_be_bytes());

        let type_id = entity.entity_type.as_u8();
        hasher.update([type_id]);

        // Hash navigation component
        if let Some(nav) = &entity.navigation {
            hasher.update([1]);
            if let Some(t) = nav.target {
                hasher.update([1]);
                hasher.update(t.x.to_bits().to_be_bytes());
                hasher.update(t.y.to_bits().to_be_bytes());
            } else {
                hasher.update([0]);
            }
            hasher.update(nav.arrival_tolerance.to_bits().to_be_bytes());
            hasher.update(nav.move_speed.to_bits().to_be_bytes());
            
            hasher.update((nav.waypoints.len() as u32).to_be_bytes());
            for wp in &nav.waypoints {
                hasher.update(wp.x.to_bits().to_be_bytes());
                hasher.update(wp.y.to_bits().to_be_bytes());
            }
            hasher.update((nav.current_waypoint_index as u32).to_be_bytes());
        } else {
            hasher.update([0]);
        }

        // Hash entity properties
        hasher.update((entity.properties.len() as u32).to_be_bytes());
        for (k, v) in &entity.properties {
            hasher.update((k.len() as u32).to_be_bytes());
            hasher.update(k.as_bytes());
            hasher.update((v.len() as u32).to_be_bytes());
            hasher.update(v.as_bytes());
        }
    }

    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::entity::{Entity, EntityType};
    use crate::world::fixed_point::DeterministicVector2;

    #[test]
    fn test_canonical_state_hash_determinism() {
        let mut inst1 = Instance::new(1, 30, 10, 42);
        let mut inst2 = Instance::new(1, 30, 10, 42);

        // Add entities in different insertion order to inst1 vs inst2
        let mut e1 = Entity::new(1, "Alice".to_string(), EntityType::Player);
        e1.position = DeterministicVector2::from_f32(10.5, -4.0);
        e1.velocity = DeterministicVector2::from_f32(1.0, 0.0);

        let mut e2 = Entity::new(2, "Bob".to_string(), EntityType::Player);
        e2.position = DeterministicVector2::from_f32(0.0, 5.25);
        e2.velocity = DeterministicVector2::from_f32(0.0, -1.0);

        // inst1: e1 then e2
        inst1.add_entity(e1.clone());
        inst1.add_entity(e2.clone());

        // inst2: e2 then e1 (different insertion order, but BTreeMap sorts them identically)
        inst2.add_entity(e2);
        inst2.add_entity(e1);

        let hash1 = compute_canonical_state_hash(&inst1, 100);
        let hash2 = compute_canonical_state_hash(&inst2, 100);

        assert_eq!(
            hash1, hash2,
            "BTreeMap must guarantee identical state hash regardless of insertion order"
        );
    }

    #[test]
    fn test_canonical_state_hash_changes_on_state_diff() {
        let mut inst = Instance::new(1, 30, 10, 42);
        let mut e = Entity::new(1, "Alice".to_string(), EntityType::Player);
        e.velocity = DeterministicVector2::from_f32(1.0, 2.0);
        inst.add_entity(e);

        let hash1 = compute_canonical_state_hash(&inst, 1);
        let hash2 = compute_canonical_state_hash(&inst, 2); // Different tick
        assert_ne!(hash1, hash2);

        let _ = inst.tick(2); // Position advances by velocity
        let hash3 = compute_canonical_state_hash(&inst, 2);
        assert_ne!(hash2, hash3);
    }
}
