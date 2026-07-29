// Tick module - The decoupled game loop (e.g., 20 fixed ticks per second)
// TODO: Implement fixed tick rate game loop separate from network I/O

//placeholder

use std::time::{Duration, Instant};
use std::thread;

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

    pub fn start<F>(&mut self, mut tick_fn: F)
    where
        F: FnMut(u64) + 'static,
    {
        self.running = true;
        let tick_duration = Duration::from_secs_f64(1.0 / self.tick_rate as f64);
        let mut tick_count = 0u64;

        while self.running {
            let start = Instant::now();
            
            tick_fn(tick_count);
            tick_count += 1;

            let elapsed = start.elapsed();
            if elapsed < tick_duration {
                thread::sleep(tick_duration - elapsed);
            }
        }
    }

    pub fn stop(&mut self) {
        self.running = false;
    }
}
