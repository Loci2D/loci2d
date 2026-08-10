use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::thread;
use prost::Message;
use crate::network::packets::{ClientIntent, ServerPacket, server_packet};
use crate::world::instance::Instance;

pub struct GameLoop {
    tick_rate: u32,
    running: bool,
}

impl GameLoop {
    pub fn new(tick_rate: u32) -> Self {
        Self {
            tick_rate,
            running: false,
        }
    }

    #[allow(clippy::while_immutable_condition)]
    pub fn start(
        &mut self,
        mut instance: Instance,
        intent_rx: mpsc::Receiver<(SocketAddr, ClientIntent)>,
        socket: Arc<UdpSocket>,
    ) {
        self.running = true;
        let tick_duration = Duration::from_secs_f64(1.0 / self.tick_rate as f64);
        let max_accumulator = tick_duration * 5; // Spiral-of-death protection clamp
        let mut accumulator = Duration::ZERO;
        let mut last_time = Instant::now();
        let mut tick_count = 0u64;
        let mut out_buf = Vec::with_capacity(2048);

        while self.running {
            let now = Instant::now();
            let mut delta = now.duration_since(last_time);
            last_time = now;

            if delta > max_accumulator {
                delta = max_accumulator;
            }
            accumulator += delta;

            while accumulator >= tick_duration {
                // 1. Drain the intent queue (non-blocking) for this fixed tick
                while let Ok((addr, intent)) = intent_rx.try_recv() {
                    instance.apply_intent(addr, intent);
                }

                // 2. Advance deterministic simulation physics & sweep timeouts
                instance.tick(tick_count);

                // 3. Generate WorldState snapshot
                let world_state = instance.create_snapshot(tick_count);
                let packet = ServerPacket {
                    sequence_id: tick_count,
                    payload: Some(server_packet::Payload::WorldState(world_state)),
                };

                // 4. Encode packet
                out_buf.clear();
                if let Ok(()) = packet.encode(&mut out_buf) {
                    // 5. Broadcast to all active sessions
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
    }

    // [2026-08-08] Allowed dead_code: graceful shutdown method to be hooked into OS signals / server lifecycle.
    #[allow(dead_code)]
    pub fn stop(&mut self) {
        self.running = false;
    }
}
