# Implementation Spec — Phase 4: Deterministic Simulation & Replay Engine

> **Status:** Open / Scheduled for **Phase 4**  
> **Roadmap Phase:** Phase 4 — Event Logging & Deterministic Replay System  
> **Reference ADRs:** [ADR-0001](../adr/en/0001-authoritative-server-archoitecture.md) · [ADR-0002](../adr/en/0002-instance-based-architecture.md) · [ADR-0006](../adr/en/0006-two-thread-network-gameloop-separation.md) · [ADR-0007](../adr/en/0007-deterministic-simulation-and-fixed-point.md)

---

## 1. Context & Technical Motivation

In Phase 1–3, the server operates with a simple, float-based (`f32`) physics update and a standard OS-dependent `thread::sleep` loop. While sufficient for MVP single-instance prototyping, this approach exhibits three non-deterministic behaviors that prevent **frame-exact state replays** across different operating systems, CPUs, and runtime environments:

| Area | Current MVP Implementation | Non-Deterministic Flaw | Target Deterministic Solution |
|---|---|---|---|
| **Tick Timing** | `thread::sleep(budget - elapsed)` | Windows default timer interrupt (~15.6 ms) causes tick stutter & wall-clock drift | **Fixed-Timestep Accumulator** with sub-ms precision |
| **Data Collection** | `HashMap<u64, Entity>` | `SipHash` random seed per process causes non-deterministic iteration order | **`BTreeMap<u64, Entity>`** or sorted indexing |
| **Arithmetic** | IEEE 754 `f32` floats | Hardware float rounding (AVX vs NEON vs FMA instructions) varies between CPUs | **Fixed-Point Math (`I32F32`)** or strict integer coordinates |
| **Input Sync** | `try_recv()` immediately | Network jitter changes the tick on which an intent is applied across runs | **Tick-Indexed Intent Queue** (Event Sourcing) |

**Goal of Phase 4:** Achieve **100% bit-exact simulation determinism**, allowing any recorded stream of inputs to be replayed frame-by-frame on any platform (x86_64, ARM64, WASM) and yield the exact same `WorldState` binary checksum.

---

## 2. Architecture & Design Requirements

### 2.1 Sub-Millisecond Fixed-Timestep Accumulator Loop

Instead of resetting tick time every loop and sleeping for variable durations, the game loop maintains a precise time budget accumulator.

```rust
pub struct DeterministicLoop {
    tick_rate: u32,
    tick_duration: Duration,
    accumulator: Duration,
    last_instant: Instant,
}

impl DeterministicLoop {
    pub fn step<F>(&mut self, mut tick_fn: F)
    where
        F: FnMut(u64),
    {
        let now = Instant::now();
        self.accumulator += now.duration_since(self.last_instant);
        self.last_instant = now;

        // Process all accumulated fixed ticks
        while self.accumulator >= self.tick_duration {
            tick_fn(self.tick_count);
            self.tick_count += 1;
            self.accumulator -= self.tick_duration;
        }

        // Sleep briefly to prevent 100% CPU core pinning
        thread::sleep(Duration::from_millis(1));
    }
}
```

### 2.2 Ordered Collections (`BTreeMap`)

`HashMap` iteration order depends on random seed initialization (`RandomState`). In Phase 4, all entity containers within `Instance` are converted to `BTreeMap<u64, Entity>`.

```rust
use std::collections::BTreeMap;

pub struct Instance {
    pub id: u64,
    pub entities: BTreeMap<u64, Entity>, // Guarantees 1, 2, 3... order on all OS platforms
    pub tick_rate: u32,
    pub client_map: BTreeMap<SocketAddr, u64>,
}
```

### 2.3 Fixed-Point Mathematics (`I32F32`)

Floating-point numbers (`f32`/`f64`) do not guarantee identical binary results across x86 (AVX/SSE) and ARM64 (Apple Silicon / Raspberry Pi) due to hardware-level rounding and FMA optimizations.

Phase 4 introduces fixed-point representation for positions, velocities, and physics calculations via a library like `fixed` (e.g., `FixedI32<U16>`: 16 bits integer, 16 bits fraction):

```rust
use fixed::types::I16F16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeterministicVector2 {
    pub x: I16F16,
    pub y: I16F16,
}

impl DeterministicVector2 {
    pub const ZERO: Self = Self {
        x: I16F16::ZERO,
        y: I16F16::ZERO,
    };
}
```

### 2.4 Tick-Indexed Event Log (Event Sourcing)

All incoming intents from network sockets are stamped with a target tick or stored in an input buffer log.

```protobuf
// Replay Log Envelope Structure
message RecordedTickInputs {
  uint64 tick_number = 1;
  repeated ClientIntentEntry entries = 2;
}

message ClientIntentEntry {
  uint64 entity_id = 1;
  ClientIntent intent = 2;
}
```

During replay execution (`cargo run --bin replay replay_log.bin`), the server ignores live network sockets and feeds `RecordedTickInputs` directly into `instance.apply_intent()` on each step.

---

## 3. Implementation Checklist for Phase 4

- [ ] **`Cargo.toml`** — Add fixed-point arithmetic dependency `fixed = "1.28"` (or custom fixed-point struct)
- [ ] **`src/world/instance.rs`** — Migrate `HashMap` to `BTreeMap` for all entity stores
- [ ] **`src/world/entity.rs`** — Convert `Vector2` fields (`position`, `velocity`) to fixed-point `DeterministicVector2`
- [ ] **`src/game_loop/tick.rs`** — Implement Fixed-Timestep Accumulator pattern with sub-ms overshoot protection
- [ ] **`src/replay/log.rs`** — Implement `ReplayLogger` to record `(tick_number, entity_id, intent)` to binary file
- [ ] **`src/replay/player.rs`** — Implement `ReplayPlayer` CLI tool to load input log and verify frame-by-frame state hash
- [ ] **Automated Test** — Add cross-platform deterministic replay test asserting `WorldState` checksum matching across test runs

---

## 4. Verification & Replay Hash Validation

Phase 4 completion criteria:

1. **Replay Determinism Test**:
   - Run a simulated match with 50 entities for 1,000 ticks, saving `replay.bin`.
   - Run `cargo run --bin replay replay.bin` on Linux x86_64, Windows x86_64, and macOS ARM64.
   - Verify that the final `WorldState` SHA256 checksum is **100% identical** across all three OS architectures.

---

## 5. When You Need to Act

> [!IMPORTANT]
> **Action Timeline:** You do **not** need to implement this spec during Phase 1–3 MVP development. Keep Phase 1–3 simple with `f32` and `std::thread::sleep`.
> Refer back to this document when starting **Phase 4 (Event Logging & Deterministic Replay System)** on the project roadmap.
