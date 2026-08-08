use std::sync::mpsc;
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use std::thread;
use crate::network::packets::ClientIntent;
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

    pub fn start(
        &mut self,
        mut instance: Instance,
        intent_rx: mpsc::Receiver<(SocketAddr, ClientIntent)>,
    ) {
        self.running = true;
        let tick_duration = Duration::from_secs_f64(1.0 / self.tick_rate as f64);
        let mut tick_count = 0u64;

        while self.running {
            let start = Instant::now();

            // 1. Drain the intent queue (non-blocking)
            loop {
                match intent_rx.try_recv() {
                    Ok((addr, intent)) => instance.apply_intent(addr, intent),
                    Err(_) => break, // queue empty or channel closed
                }
            }

            // 2. Advance simulation
            instance.tick(tick_count);

            tick_count += 1;

            // 3. Sleep to maintain tick rate
            let elapsed = start.elapsed();
            if elapsed < tick_duration {
                thread::sleep(tick_duration - elapsed);
            }
        }
    }

    // [2026-08-08] Allowed dead_code: graceful shutdown method to be hooked into OS signals / server lifecycle.
    #[allow(dead_code)]
    pub fn stop(&mut self) {
        self.running = false;
    }
}

