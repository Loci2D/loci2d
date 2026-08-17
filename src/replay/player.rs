// Replay player for deterministic match playback and offline verification (ADR-0010, ADR-0011).

use super::hash::compute_canonical_state_hash;
use crate::network::packets::{
    ClientIntent, ReplayCheckpoint, ReplayFile, ReplayHeader, ReplayTickFrame, ServerPacket,
    client_intent, server_packet,
};
use crate::world::instance::Instance;
use prost::Message;
use std::collections::BTreeMap;
use std::net::{SocketAddr, UdpSocket};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

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
        writeln!(
            f,
            "================================================================================"
        )?;
        writeln!(
            f,
            "                       REPLAY STATE DESYNC DETECTED                             "
        )?;
        writeln!(
            f,
            "================================================================================"
        )?;
        writeln!(f, "Desync Tick:         {}", self.tick)?;
        writeln!(f, "Expected SHA-256:    {}", self.expected_hash)?;
        writeln!(f, "Actual SHA-256:      {}", self.actual_hash)?;
        writeln!(f, "Expected Entities:   {}", self.expected_entities)?;
        writeln!(f, "Actual Entities:     {}", self.actual_entities)?;
        writeln!(f, "--- Entity State Breakdown ---")?;
        for entry in &self.entity_summary {
            writeln!(f, "  {}", entry)?;
        }
        writeln!(
            f,
            "================================================================================"
        )
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
            return Err(format!(
                "Invalid magic bytes: '{}', expected 'LOCI_REPLAY'",
                header.magic
            )
            .into());
        }
        if header.version != 1 {
            return Err(
                format!("Unsupported replay version: {}, expected 1", header.version).into(),
            );
        }

        Ok(Self { replay })
    }

    /// Runs headless verification against recorded state checksums as fast as possible.
    /// Reads and verifies all frames, applying them to a newly created empty instance.
    pub fn verify_determinism(&mut self) -> Result<VerificationReport, DesyncReport> {
        let header = self.replay.header.as_ref().unwrap();
        let mut instance = Instance::new(header.instance_id, header.tick_rate, 60);
        self.verify_determinism_with_instance(&mut instance)
    }

    /// Reads and verifies all frames, applying them to the provided instance.
    /// This is useful when the instance needs to be pre-configured with map geometry.
    pub fn verify_determinism_with_instance(
        &mut self,
        instance: &mut Instance,
    ) -> Result<VerificationReport, DesyncReport> {

        let frames_by_tick: BTreeMap<u64, &ReplayTickFrame> =
            self.replay.frames.iter().map(|f| (f.tick, f)).collect();

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

    /// Streams the replay match in real-time (or at scaled speed) and broadcasts
    /// Streams the replay match in real-time (or at scaled speed) and broadcasts
    /// authoritative WorldState snapshots over UDP to all connected spectator clients.
    pub fn broadcast_live(
        &mut self,
        socket: Arc<UdpSocket>,
        intent_rx: mpsc::Receiver<(SocketAddr, ClientIntent)>,
        speed: f32,
        max_spectators: usize,
        running: Arc<AtomicBool>,
    ) {
        let max_spectators = if max_spectators == 0 {
            128
        } else {
            max_spectators
        };
        const TERMINAL_FRAME_RETRIES: usize = 3;
        const TERMINAL_FRAME_DELAY_MS: u64 = 15;

        let header = self.replay.header.as_ref().unwrap();
        let mut instance = Instance::new(header.instance_id, header.tick_rate, 60);

        let original_speed = speed;
        let speed = if speed <= 0.0 {
            1.0
        } else {
            speed.clamp(0.1, 10.0)
        };
        if (speed - original_speed).abs() > 0.001 {
            println!(
                "[Spectator] Playback speed clamped from {:.1}x to {:.1}x",
                original_speed, speed
            );
        }

        let base_tick_hz = header.tick_rate as f64 * speed as f64;
        let tick_duration = Duration::from_secs_f64(1.0 / base_tick_hz);
        let max_accumulator = tick_duration * 5;

        let frames_by_tick: BTreeMap<u64, &ReplayTickFrame> =
            self.replay.frames.iter().map(|f| (f.tick, f)).collect();

        let end_tick = frames_by_tick.keys().next_back().copied().unwrap_or(0).max(
            self.replay
                .checkpoints
                .iter()
                .map(|c| c.tick)
                .max()
                .unwrap_or(0),
        );

        // Track connected spectator clients (SocketAddr -> last activity timestamp)
        let mut spectators: BTreeMap<SocketAddr, Instant> = BTreeMap::new();
        let spectator_timeout = Duration::from_secs(10);

        let mut accumulator = Duration::ZERO;
        let mut last_time = Instant::now();
        let mut tick_count = 0u64;
        let mut out_buf = Vec::with_capacity(2048);

        running.store(true, Ordering::Relaxed);
        println!(
            "[Spectator] Replay broadcast started ({} Hz at {:.1}x speed, total ticks: {})",
            header.tick_rate, speed, end_tick
        );

        while running.load(Ordering::Relaxed) && tick_count <= end_tick {
            let now = Instant::now();
            let delta = now.duration_since(last_time);
            last_time = now;

            accumulator = (accumulator + delta).min(max_accumulator);

            while accumulator >= tick_duration && tick_count <= end_tick {
                // 1. Drain incoming spectator intents & register/refresh spectator sessions
                while let Ok((addr, intent)) = intent_rx.try_recv() {
                    let now = Instant::now();
                    if spectators.contains_key(&addr) {
                        spectators.insert(addr, now);
                    } else if spectators.len() < max_spectators {
                        spectators.insert(addr, now);
                        println!(
                            "[Spectator] New spectator client registered: {} ({}/{} active)",
                            addr,
                            spectators.len(),
                            max_spectators
                        );
                    } else {
                        println!(
                            "[Spectator] Rejected spectator client {}: maximum capacity ({} spectators) reached",
                            addr, max_spectators
                        );
                    }

                    if let Some(client_intent::Intent::Disconnect(_)) = intent.intent {
                        spectators.remove(&addr);
                        println!("[Spectator] Spectator client disconnected: {}", addr);
                    }
                }

                // 2. Sweep timed-out spectators (> 10s inactivity)
                let now = Instant::now();
                spectators.retain(|addr, last_seen| {
                    if now.duration_since(*last_seen) > spectator_timeout {
                        println!("[Spectator] Spectator client {} timed out", addr);
                        false
                    } else {
                        true
                    }
                });

                // 3. Apply recorded match intents for this fixed tick
                if let Some(frame) = frames_by_tick.get(&tick_count) {
                    for entry in &frame.entries {
                        instance.apply_replay_entry(entry);
                    }
                }

                // 4. Advance deterministic simulation physics
                instance.tick(tick_count);

                // 5. Generate WorldState snapshot
                let world_state = instance.create_snapshot(tick_count);
                let packet = ServerPacket {
                    sequence_id: tick_count,
                    payload: Some(server_packet::Payload::WorldState(world_state)),
                };

                // 6. Encode and broadcast to all active spectators
                out_buf.clear();
                if let Ok(()) = packet.encode(&mut out_buf) {
                    for spectator_addr in spectators.keys() {
                        let _ = socket.send_to(&out_buf, spectator_addr);
                    }
                }

                tick_count += 1;
                accumulator -= tick_duration;
            }

            thread::sleep(Duration::from_micros(500));
        }

        // Send trailing terminal frames so connected spectator clients reliably receive the final state
        let terminal_state = instance.create_snapshot(tick_count);
        let terminal_packet = ServerPacket {
            sequence_id: tick_count,
            payload: Some(server_packet::Payload::WorldState(terminal_state)),
        };
        out_buf.clear();
        if let Ok(()) = terminal_packet.encode(&mut out_buf) {
            for spectator_addr in spectators.keys() {
                for _ in 0..TERMINAL_FRAME_RETRIES {
                    let _ = socket.send_to(&out_buf, spectator_addr);
                    thread::sleep(Duration::from_millis(TERMINAL_FRAME_DELAY_MS));
                }
            }
        }

        println!(
            "[Spectator] Replay broadcast finished at tick {}.",
            tick_count.saturating_sub(1)
        );
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
        ClientIntent, JoinIntent, MoveIntent, ReplayIntentEntry, Vector2, client_intent,
    };
    use crate::replay::ReplayRecorder;

    #[test]
    fn test_replay_player_verify_success() {
        let mut recorder = ReplayRecorder::new(1, 30, 42, "test_arena".to_string(), 10);

        // Tick 1: Join Alice
        recorder.record_tick(
            1,
            vec![ReplayIntentEntry {
                entity_id: 1,
                player_name: "Alice".to_string(),
                intent: Some(ClientIntent {
                    intent: Some(client_intent::Intent::Join(JoinIntent {
                        player_name: "Alice".to_string(),
                    })),
                }),
            }],
        );

        // Tick 2: Move Alice
        recorder.record_tick(
            2,
            vec![ReplayIntentEntry {
                entity_id: 1,
                player_name: String::new(),
                intent: Some(ClientIntent {
                    intent: Some(client_intent::Intent::Move(MoveIntent {
                        direction: Some(Vector2 {
                            x_bits: (2.0f32 * 65536.0) as i32,
                            y_bits: (1.0f32 * 65536.0) as i32,
                        }),
                    })),
                }),
            }],
        );

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
        let report = player
            .verify_determinism()
            .expect("Verification should pass");

        assert_eq!(report.total_ticks, 10);
        assert_eq!(report.verified_checkpoints, 1);
        assert!(!report.final_hash.is_empty());
    }

    #[test]
    fn test_replay_player_detects_desync() {
        let mut recorder = ReplayRecorder::new(1, 30, 42, "test_arena".to_string(), 5);

        recorder.record_tick(
            1,
            vec![ReplayIntentEntry {
                entity_id: 1,
                player_name: "Bob".to_string(),
                intent: Some(ClientIntent {
                    intent: Some(client_intent::Intent::Join(JoinIntent {
                        player_name: "Bob".to_string(),
                    })),
                }),
            }],
        );

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
