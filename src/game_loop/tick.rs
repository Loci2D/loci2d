use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::thread;
use prost::Message;
use crate::network::packets::{ClientIntent, ServerPacket, server_packet};
use crate::replay::ReplayRecorder;
use crate::world::instance::Instance;

pub struct GameLoop {
    tick_rate: u32,
    running: Arc<AtomicBool>,
    recorder: Option<ReplayRecorder>,
    record_path: Option<String>,
}

impl GameLoop {
    pub fn new(tick_rate: u32) -> Self {
        Self {
            tick_rate,
            running: Arc::new(AtomicBool::new(false)),
            recorder: None,
            record_path: None,
        }
    }

    /// Enables match recording to an event-sourced `.loci` replay file.
    pub fn enable_recording(
        &mut self,
        instance_id: u64,
        seed: u64,
        map_name: String,
        checkpoint_interval: u64,
        file_path: String,
    ) {
        self.recorder = Some(ReplayRecorder::new(
            instance_id,
            self.tick_rate,
            seed,
            map_name,
            checkpoint_interval,
        ));
        self.record_path = Some(file_path);
    }

    /// Returns a reference to the active replay recorder, if recording is enabled.
    pub fn recorder(&self) -> Option<&ReplayRecorder> {
        self.recorder.as_ref()
    }

    /// Returns a cloned handle to the running atomic flag for external shutdown coordination.
    pub fn running_handle(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.running)
    }

    pub fn start(
        &mut self,
        mut instance: Instance,
        intent_rx: mpsc::Receiver<(SocketAddr, ClientIntent)>,
        socket: Arc<UdpSocket>,
    ) {
        self.running.store(true, Ordering::Relaxed);
        let tick_duration = Duration::from_secs_f64(1.0 / self.tick_rate as f64);
        let max_accumulator = tick_duration * 5; // Spiral-of-death protection clamp
        let mut accumulator = Duration::ZERO;
        let mut last_time = Instant::now();
        let mut tick_count = 0u64;
        let mut out_buf = Vec::with_capacity(2048);

        while self.running.load(Ordering::Relaxed) {
            let now = Instant::now();
            let delta = now.duration_since(last_time);
            last_time = now;

            accumulator = (accumulator + delta).min(max_accumulator);

            while accumulator >= tick_duration {
                let mut tick_entries = Vec::new();

                // 1. Drain the intent queue (non-blocking) for this fixed tick
                while let Ok((addr, intent)) = intent_rx.try_recv() {
                    if let Some(entry) = instance.apply_intent(addr, intent)
                        && self.recorder.is_some()
                    {
                        tick_entries.push(entry);
                    }
                }

                // 2. Record tick inputs if recording is enabled
                if let Some(ref mut recorder) = self.recorder {
                    recorder.record_tick(tick_count, tick_entries);
                }

                // 3. Advance deterministic simulation physics & sweep timeouts
                instance.tick(tick_count);

                // 4. Record state checkpoint if on checkpoint interval
                if let Some(ref mut recorder) = self.recorder {
                    recorder.maybe_record_checkpoint(tick_count, &instance);
                }

                // 5. Generate WorldState snapshot
                let world_state = instance.create_snapshot(tick_count);
                let packet = ServerPacket {
                    sequence_id: tick_count,
                    payload: Some(server_packet::Payload::WorldState(world_state)),
                };

                // 6. Encode packet and broadcast
                out_buf.clear();
                if let Ok(()) = packet.encode(&mut out_buf) {
                    for client_addr in instance.get_broadcast_addresses() {
                        let _ = socket.send_to(&out_buf, client_addr);
                    }
                }

                tick_count += 1;
                accumulator -= tick_duration;
            }

            // Sleep briefly to yield CPU time without missing sub-ms tick boundaries
            thread::sleep(Duration::from_micros(500));
        }

        // Flush recorded replay file on shutdown
        if let (Some(recorder), Some(path)) = (&self.recorder, &self.record_path) {
            if let Err(e) = recorder.save_to_file(path) {
                eprintln!("[Replay] Failed to save replay file '{}': {}", path, e);
            } else {
                println!("[Replay] Successfully saved replay file '{}' ({} frames, {} checkpoints)", 
                    path, recorder.frame_count(), recorder.checkpoint_count());
            }
        }
    }

    /// Graceful shutdown method to be hooked into OS signals or test harnesses.
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}
