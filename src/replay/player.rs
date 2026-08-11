// Replay player for deterministic match playback and offline verification (ADR-0010, ADR-0011).

use std::collections::BTreeMap;
use std::path::Path;
use prost::Message;
use crate::network::packets::{ReplayCheckpoint, ReplayFile, ReplayHeader, ReplayTickFrame};
use crate::world::instance::Instance;
use super::hash::compute_canonical_state_hash;

/// Diagnostic report generated when a state checksum mismatch occurs during replay verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesyncReport {
    pub tick: u64,
    pub expected_hash: String,
    pub actual_hash: String,
    pub expected_entities: u32,
    pub actual_entities: usize,
    pub entity_summary: Vec<String>,
}

impl std::fmt::Display for DesyncReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "================================================================================")?;
        writeln!(f, "                       REPLAY STATE DESYNC DETECTED                             ")?;
        writeln!(f, "================================================================================")?;
        writeln!(f, "Desync Tick:         {}", self.tick)?;
        writeln!(f, "Expected SHA-256:    {}", self.expected_hash)?;
        writeln!(f, "Actual SHA-256:      {}", self.actual_hash)?;
        writeln!(f, "Expected Entities:   {}", self.expected_entities)?;
        writeln!(f, "Actual Entities:     {}", self.actual_entities)?;
        writeln!(f, "--- Entity State Breakdown ---")?;
        for entry in &self.entity_summary {
            writeln!(f, "  {}", entry)?;
        }
        writeln!(f, "================================================================================")
    }
}

/// Report summarizing a successful deterministic replay verification run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationReport {
    pub total_ticks: u64,
    pub total_frames: usize,
    pub verified_checkpoints: usize,
    pub final_hash: String,
}

impl std::fmt::Display for VerificationReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Replay Verification Passed: {} ticks, {} frames, {} checkpoints verified. Final SHA-256: {}",
            self.total_ticks, self.total_frames, self.verified_checkpoints, self.final_hash
        )
    }
}

pub struct ReplayPlayer {
    replay: ReplayFile,
}

impl ReplayPlayer {
    /// Loads and parses a .loci replay file from disk.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let bytes = std::fs::read(path)?;
        Self::from_bytes(&bytes)
    }

    /// Parses a .loci replay file from raw bytes, verifying header magic and version.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let replay = ReplayFile::decode(bytes)?;

        let header = replay.header.as_ref().ok_or("Missing replay header")?;
        if header.magic != "LOCI_REPLAY" {
            return Err(format!("Invalid magic bytes: '{}', expected 'LOCI_REPLAY'", header.magic).into());
        }
        if header.version != 1 {
            return Err(format!("Unsupported replay version: {}, expected 1", header.version).into());
        }

        Ok(Self { replay })
    }

    /// Runs headless verification against recorded state checksums as fast as possible.
    pub fn verify_determinism(&mut self) -> Result<VerificationReport, DesyncReport> {
        let header = self.replay.header.as_ref().unwrap();
        let mut instance = Instance::new(header.instance_id, header.tick_rate, 60);

        let frames_by_tick: BTreeMap<u64, &ReplayTickFrame> = self
            .replay
            .frames
            .iter()
            .map(|f| (f.tick, f))
            .collect();

        let checkpoints_by_tick: BTreeMap<u64, &ReplayCheckpoint> = self
            .replay
            .checkpoints
            .iter()
            .map(|c| (c.tick, c))
            .collect();

        let end_tick = frames_by_tick
            .keys()
            .next_back()
            .copied()
            .unwrap_or(0)
            .max(checkpoints_by_tick.keys().next_back().copied().unwrap_or(0));

        for tick in 0..=end_tick {
            // 1. Apply recorded intents for this fixed tick
            if let Some(frame) = frames_by_tick.get(&tick) {
                for entry in &frame.entries {
                    instance.apply_replay_entry(entry);
                }
            }

            // 2. Advance simulation physics
            instance.tick(tick);

            // 3. Check for state hash checkpoint verification
            if let Some(checkpoint) = checkpoints_by_tick.get(&tick) {
                let actual_hash = compute_canonical_state_hash(&instance, tick);
                if actual_hash.as_slice() != checkpoint.state_sha256.as_slice() {
                    let entity_summary = instance
                        .entities
                        .values()
                        .map(|e| {
                            let (px, py) = e.position.to_f32();
                            let (vx, vy) = e.velocity.to_f32();
                            format!(
                                "Entity {} ('{}') @ ({:.2}, {:.2}) vel=({:.2}, {:.2})",
                                e.id, e.name, px, py, vx, vy
                            )
                        })
                        .collect();

                    return Err(DesyncReport {
                        tick,
                        expected_hash: hex_encode(&checkpoint.state_sha256),
                        actual_hash: hex_encode(&actual_hash),
                        expected_entities: checkpoint.active_entities,
                        actual_entities: instance.entities.len(),
                        entity_summary,
                    });
                }
            }
        }

        let final_hash = hex_encode(&compute_canonical_state_hash(&instance, end_tick));

        Ok(VerificationReport {
            total_ticks: end_tick,
            total_frames: self.replay.frames.len(),
            verified_checkpoints: checkpoints_by_tick.len(),
            final_hash,
        })
    }

    pub fn header(&self) -> &ReplayHeader {
        self.replay.header.as_ref().unwrap()
    }

    pub fn frames(&self) -> &[ReplayTickFrame] {
        &self.replay.frames
    }

    pub fn checkpoints(&self) -> &[ReplayCheckpoint] {
        &self.replay.checkpoints
    }
}

pub fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(&mut s, "{:02x}", b);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::packets::{
        client_intent, ClientIntent, JoinIntent, MoveIntent, ReplayIntentEntry, Vector2,
    };
    use crate::replay::ReplayRecorder;

    #[test]
    fn test_replay_player_verify_success() {
        let mut recorder = ReplayRecorder::new(1, 30, 42, "test_arena".to_string(), 10);

        // Tick 1: Join Alice
        recorder.record_tick(1, vec![ReplayIntentEntry {
            entity_id: 1,
            player_name: "Alice".to_string(),
            intent: Some(ClientIntent {
                intent: Some(client_intent::Intent::Join(JoinIntent {
                    player_name: "Alice".to_string(),
                })),
            }),
        }]);

        // Tick 2: Move Alice
        recorder.record_tick(2, vec![ReplayIntentEntry {
            entity_id: 1,
            player_name: String::new(),
            intent: Some(ClientIntent {
                intent: Some(client_intent::Intent::Move(MoveIntent {
                    direction: Some(Vector2 { x: 2.0, y: 1.0 }),
                })),
            }),
        }]);

        // Simulate instance ticks to record authentic checkpoints
        let mut sim_instance = Instance::new(1, 30, 10);
        sim_instance.handle_join("127.0.0.1:1000".parse().unwrap(), "Alice".to_string());
        sim_instance.tick(1);

        sim_instance.entities.get_mut(&1).unwrap().velocity =
            crate::world::fixed_point::DeterministicVector2::from_f32(2.0, 1.0);
        for t in 2..=10 {
            sim_instance.tick(t);
        }

        recorder.maybe_record_checkpoint(10, &sim_instance);

        let bytes = recorder.to_bytes().unwrap();
        let mut player = ReplayPlayer::from_bytes(&bytes).unwrap();
        let report = player.verify_determinism().expect("Verification should pass");

        assert_eq!(report.total_ticks, 10);
        assert_eq!(report.verified_checkpoints, 1);
        assert!(!report.final_hash.is_empty());
    }

    #[test]
    fn test_replay_player_detects_desync() {
        let mut recorder = ReplayRecorder::new(1, 30, 42, "test_arena".to_string(), 5);

        recorder.record_tick(1, vec![ReplayIntentEntry {
            entity_id: 1,
            player_name: "Bob".to_string(),
            intent: Some(ClientIntent {
                intent: Some(client_intent::Intent::Join(JoinIntent {
                    player_name: "Bob".to_string(),
                })),
            }),
        }]);

        // Record a forged/corrupted checkpoint hash
        recorder.record_checkpoint(5, [0xFF; 32], 1);

        let bytes = recorder.to_bytes().unwrap();
        let mut player = ReplayPlayer::from_bytes(&bytes).unwrap();
        let err = player.verify_determinism().expect_err("Must detect desync");

        assert_eq!(err.tick, 5);
        assert_eq!(err.expected_hash, hex_encode(&[0xFF; 32]));
        assert_ne!(err.actual_hash, err.expected_hash);
        assert_eq!(err.expected_entities, 1);
        assert_eq!(err.actual_entities, 1);
    }

    #[test]
    fn test_replay_player_accessors() {
        let recorder = ReplayRecorder::new(1, 30, 42, "test_arena".to_string(), 10);
        let bytes = recorder.to_bytes().unwrap();
        let player = ReplayPlayer::from_bytes(&bytes).unwrap();

        assert_eq!(player.header().magic, "LOCI_REPLAY");
        assert_eq!(player.header().map_name, "test_arena");
        assert_eq!(player.frames().len(), 0);
        assert_eq!(player.checkpoints().len(), 0);
    }
}
