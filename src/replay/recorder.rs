// Replay recorder for event-sourced match logging (ADR-0010).

use super::hash::compute_canonical_state_hash;
use crate::network::packets::{
    ReplayCheckpoint, ReplayFile, ReplayHeader, ReplayIntentEntry, ReplayTickFrame,
};
use crate::world::instance::Instance;
use prost::Message;
use std::path::Path;

pub const DEFAULT_CHECKPOINT_INTERVAL_TICKS: u64 = 60; // 2.0 seconds at 30 Hz

#[derive(Debug, Clone)]
pub struct ReplayRecorder {
    header: ReplayHeader,
    frames: Vec<ReplayTickFrame>,
    checkpoints: Vec<ReplayCheckpoint>,
    checkpoint_interval_ticks: u64,
}

impl ReplayRecorder {
    pub fn new(
        instance_id: u64,
        tick_rate: u32,
        seed: u64,
        map_name: String,
        checkpoint_interval: u64,
    ) -> Self {
        let interval = if checkpoint_interval == 0 {
            DEFAULT_CHECKPOINT_INTERVAL_TICKS
        } else {
            checkpoint_interval
        };
        let map = if map_name.trim().is_empty() {
            "default_arena".to_string()
        } else {
            map_name
        };

        Self {
            header: ReplayHeader {
                magic: "LOCI_REPLAY".to_string(),
                version: 1,
                tick_rate,
                start_timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                instance_id,
                random_seed: seed,
                map_name: map,
            },
            frames: Vec::new(),
            checkpoints: Vec::new(),
            checkpoint_interval_ticks: interval,
        }
    }

    /// Records input events for a specific tick frame. Empty entries are ignored.
    pub fn record_tick(&mut self, tick: u64, entries: Vec<ReplayIntentEntry>) {
        if !entries.is_empty() {
            self.frames.push(ReplayTickFrame { tick, entries });
        }
    }

    /// Records a canonical state hash checkpoint if the current tick is on the checkpoint interval.
    /// Returns true if a checkpoint was recorded.
    #[allow(clippy::manual_is_multiple_of)]
    pub fn maybe_record_checkpoint(&mut self, tick: u64, instance: &Instance) -> bool {
        if tick > 0 && tick % self.checkpoint_interval_ticks == 0 {
            let state_hash = compute_canonical_state_hash(instance, tick);
            let active_entities = instance.entities.len() as u32;
            self.checkpoints.push(ReplayCheckpoint {
                tick,
                state_sha256: state_hash.to_vec(),
                active_entities,
            });
            true
        } else {
            false
        }
    }

    /// Explicitly records a checkpoint (e.g. at match end).
    pub fn record_checkpoint(&mut self, tick: u64, state_hash: [u8; 32], active_entities: u32) {
        self.checkpoints.push(ReplayCheckpoint {
            tick,
            state_sha256: state_hash.to_vec(),
            active_entities,
        });
    }

    /// Encodes the replay data into a Proto3 binary payload.
    pub fn to_bytes(&self) -> Result<Vec<u8>, prost::EncodeError> {
        let replay_file = ReplayFile {
            header: Some(self.header.clone()),
            frames: self.frames.clone(),
            checkpoints: self.checkpoints.clone(),
        };
        let mut buf = Vec::with_capacity(replay_file.encoded_len());
        replay_file.encode(&mut buf)?;
        Ok(buf)
    }

    /// Saves the replay file to disk (.loci).
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let bytes = self
            .to_bytes()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, bytes)
    }

    pub fn header(&self) -> &ReplayHeader {
        &self.header
    }

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    pub fn checkpoint_count(&self) -> usize {
        self.checkpoints.len()
    }

    pub fn frames(&self) -> &[ReplayTickFrame] {
        &self.frames
    }

    pub fn checkpoints(&self) -> &[ReplayCheckpoint] {
        &self.checkpoints
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::packets::{ClientIntent, JoinIntent, MoveIntent, Vector2, client_intent};

    #[test]
    fn test_replay_recorder_roundtrip() {
        let mut recorder = ReplayRecorder::new(1, 30, 12345, "custom_map".to_string(), 60);

        let join_intent = ClientIntent {
            intent: Some(client_intent::Intent::Join(JoinIntent {
                player_name: "Alice".to_string(),
            })),
        };
        recorder.record_tick(
            1,
            vec![ReplayIntentEntry {
                entity_id: 1,
                player_name: "Alice".to_string(),
                intent: Some(join_intent),
            }],
        );

        let move_intent = ClientIntent {
            intent: Some(client_intent::Intent::Move(MoveIntent {
                direction: Some(Vector2 {
                    x_bits: (1.0f32 * 65536.0) as i32,
                    y_bits: 0,
                }),
            })),
        };
        recorder.record_tick(
            2,
            vec![ReplayIntentEntry {
                entity_id: 1,
                player_name: String::new(),
                intent: Some(move_intent),
            }],
        );

        recorder.record_checkpoint(60, [0xAA; 32], 1);

        assert_eq!(recorder.frame_count(), 2);
        assert_eq!(recorder.checkpoint_count(), 1);

        let bytes = recorder.to_bytes().expect("Failed to encode bytes");
        let decoded = ReplayFile::decode(&bytes[..]).expect("Failed to decode ReplayFile");

        let header = decoded.header.unwrap();
        assert_eq!(header.magic, "LOCI_REPLAY");
        assert_eq!(header.map_name, "custom_map");
        assert_eq!(decoded.frames.len(), 2);
        assert_eq!(decoded.checkpoints.len(), 1);
        assert_eq!(decoded.checkpoints[0].state_sha256, vec![0xAA; 32]);
    }
}
