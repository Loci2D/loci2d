# Implementation Spec — Phase 1: Game Loop & Single-Instance Core

> **Status:** Done / Completed  
> **Roadmap Phase:** Phase 1 — Game Loop & Single-Instance Core  
> **Reference ADRs:** [ADR-0001](../adr/en/0001-authoritative-server-archoitecture.md) · [ADR-0002](../adr/en/0002-instance-based-architecture.md) · [ADR-0005](../adr/en/0005-cross-language-binary-serialization.md) · [ADR-0006](../adr/en/0006-two-thread-network-gameloop-separation.md)

---

## 1. Context & Current State

The server currently has:

| Component | Current state | Problem |
|---|---|---|
| `network/server.rs` | Blocking UDP `recv_from` in a single infinite loop | Network and game logic are coupled on the same thread |
| `game_loop/tick.rs` | `GameLoop` stub that accepts a `tick_fn` with a fixed sleep | Never invoked; does not receive intents from the network |
| `world/instance.rs` | `Instance` with `HashMap<EntityId, Entity>` | No `tick()`, no intent application |
| `world/entity.rs` | `Entity` with `Vector2` position and `EntityType` | No velocity, no position update |
| `main.rs` | Only calls `run_server()` | `GameLoop` and `Instance` are never created |

**Core problem:** The network thread is blocked on `recv_from`. It cannot simultaneously advance the game tick. Any incoming packet is only processed when the server is not "thinking" — and the server never "thinks" because there is no separate game loop.

---

## 2. Phase 1 Goal

Decouple network and game logic using **two threads** connected by an **`mpsc` channel**:

```
┌──────────────────────────────────────────────────────────────────┐
│  Network Thread                                                   │
│  UdpSocket::recv_from (blocking — correct here)                   │
│  → decodes GamePacket (prost)                                     │
│  → sends (src_addr, ClientIntent) to intent_tx                    │
└──────────────────────────┬───────────────────────────────────────┘
                           │  mpsc::channel<(SocketAddr, ClientIntent)>
                           ▼
┌──────────────────────────────────────────────────────────────────┐
│  Game Loop Thread                                                 │
│  Fixed tick: 30 Hz (~33 ms per tick)                             │
│  Per tick:                                                        │
│    1. Drain intent_rx (try_recv loop, non-blocking)              │
│    2. Apply intents to Instance entities                          │
│    3. Advance simulation (update positions)                       │
│    4. Log world state (tick N, active entities)                   │
└──────────────────────────────────────────────────────────────────┘
```

> **Why `mpsc` instead of `Arc<Mutex<VecDeque>>`?**  
> `mpsc` is the standard Rust primitive for producer→consumer inter-thread communication: no explicit lock, natural backpressure, and clear ownership semantics. There is no need for bidirectional buffer access at this stage.

---

## 3. Changes per File

### 3.1 `src/network/server.rs` — Refactor to receive `intent_tx`

**Before:** `run_server()` takes no parameters and acts as a complete server.  
**After:** `run_server(intent_tx: mpsc::Sender<(SocketAddr, ClientIntent)>, bind_addr: &str)` — only decodes and forwards intents.

```rust
// New signature
pub fn run_server(intent_tx: mpsc::Sender<(SocketAddr, ClientIntent)>, bind_addr: &str)
```

Internal changes:
- Remove `ServerResponse` and `socket.send_to` (synchronous ACK is dropped — responses will be emitted by the game loop in a future phase)
- Keep `recv_from` blocking — this is correct in this context
- On decode error, just log and continue (no panic)

```rust
// Send logic after successful decode
if let Some(intent) = packet.intent {
    let _ = intent_tx.send((src_addr, intent));
}
```

> **Note:** The `_` on `send` silently ignores a closed-channel error, which is the correct behavior during shutdown.

---

### 3.2 `src/game_loop/tick.rs` — Add `intent_rx` and `Instance` support

**Before:** `GameLoop::start<F>(tick_fn: F)` — receives a generic closure with no access to game state.  
**After:** `GameLoop::start` receives `intent_rx` and an `Instance`, and runs the loop internally.

```rust
use std::sync::mpsc;
use std::net::SocketAddr;
use crate::network::packets::ClientIntent;
use crate::world::instance::Instance;

impl GameLoop {
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
}
```

> **Timing detail:** The sleep happens *after* draining intents and advancing the simulation. This ensures that logic overhead is subtracted from the sleep time, keeping the tick rate stable.

---

### 3.3 `src/world/instance.rs` — Add `apply_intent` and `tick`

This is the central piece of Phase 1. `Instance` needs:

1. **`apply_intent(addr: SocketAddr, intent: ClientIntent)`** — translate an intent into an entity state change.
2. **`tick(tick_count: u64)`** — advance the simulation (e.g., update positions based on velocity).

#### `SocketAddr → EntityId` mapping

A simple `HashMap<SocketAddr, u64>` inside `Instance`:

```rust
pub struct Instance {
    pub id: u64,
    pub entities: HashMap<u64, Entity>,
    pub tick_rate: u32,
    pub client_map: HashMap<SocketAddr, u64>, // new field
    next_entity_id: u64,                      // new field (auto-increment)
}
```

> **Note:** Client registration is in the minimal scope of Phase 1. Phase 2 will formalize join/disconnect/timeout. For now, if an unmapped address sends an intent, a new entity is created automatically (auto-join).

#### `apply_intent`

```rust
pub fn apply_intent(&mut self, addr: SocketAddr, intent: ClientIntent) {
    use crate::network::packets::client_intent::Intent;

    // Auto-join: create entity if addr is unknown
    let entity_id = self.get_or_create_entity(addr);

    if let Some(entity) = self.entities.get_mut(&entity_id) {
        match intent.intent {
            Some(Intent::Move(move_intent)) => {
                if let Some(dir) = move_intent.direction {
                    entity.velocity = Vector2 { x: dir.x, y: dir.y };
                }
            }
            Some(Intent::Action(_)) => { /* reserved */ }
            Some(Intent::Ping(_))   => { /* log only */ }
            None => {}
        }
    }
}
```

#### `tick`

```rust
pub fn tick(&mut self, tick_count: u64) {
    for entity in self.entities.values_mut() {
        entity.position.x += entity.velocity.x;
        entity.position.y += entity.velocity.y;
    }
    println!("[Tick {}] {} active entities", tick_count, self.entities.len());
}
```

> **Velocity scale:** Phase 1 has no real `delta_time` — velocity is applied in units per tick. At 30 Hz, 1 unit/tick = 30 units/second. This can be refined in Phase 2 with an explicit delta_time.

---

### 3.4 `src/world/entity.rs` — Add `velocity` field

```rust
#[derive(Debug, Clone)]
pub struct Entity {
    pub id: u64,
    pub position: Vector2,
    pub velocity: Vector2,   // new field
    pub entity_type: EntityType,
}
```

Add a constructor `Entity::new(id, entity_type) -> Self` with position and velocity initialized to zero.

---

### 3.5 `src/config.rs` — Configuration via `.env` (**new file**)

A new `config` module reads environment variables (with fallbacks to defaults), loaded from a `.env` file via the `dotenvy` crate.

**`.env` (project root):**
```env
# loci2d server configuration
BIND_ADDR=127.0.0.1:8080
TICK_RATE=30
```

**`src/config.rs`:**
```rust
/// Configuration loaded once at startup.
pub struct ServerConfig {
    pub bind_addr: String,
    pub tick_rate: u32,
}

impl ServerConfig {
    pub fn from_env() -> Self {
        // dotenvy::dotenv() is a no-op if .env does not exist — safe in production
        let _ = dotenvy::dotenv();

        Self {
            bind_addr: std::env::var("BIND_ADDR")
                .unwrap_or_else(|_| "127.0.0.1:8080".to_string()),
            tick_rate: std::env::var("TICK_RATE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
        }
    }
}
```

> **Why `dotenvy` instead of `dotenv`?** The original `dotenv` crate is unmaintained; `dotenvy` is the actively maintained fork recommended by the Rust community.

**Add to `Cargo.toml`:**
```toml
[dependencies]
dotenvy = "0.15"
```

---

### 3.6 `src/main.rs` — Wire up the two threads

```rust
mod config;
use std::sync::mpsc;
use std::thread;
use network::server::run_server;
use game_loop::tick::GameLoop;
use world::instance::Instance;
use config::ServerConfig;

fn main() {
    let cfg = ServerConfig::from_env();
    println!("[Config] bind_addr={} tick_rate={} Hz", cfg.bind_addr, cfg.tick_rate);

    let (intent_tx, intent_rx) = mpsc::channel();
    let bind_addr = cfg.bind_addr.clone();
    let tick_rate = cfg.tick_rate;

    // Network thread: produces intents
    let net_thread = thread::spawn(move || {
        run_server(intent_tx, &bind_addr);
    });

    // Game loop thread: consumes intents and advances simulation
    let loop_thread = thread::spawn(move || {
        let instance = Instance::new(1, tick_rate);
        let mut game_loop = GameLoop::new(tick_rate);
        game_loop.start(instance, intent_rx);
    });

    // Wait for both (they run indefinitely in production)
    let _ = net_thread.join();
    let _ = loop_thread.join();
}
```

---

## 4. Implementation Checklist

- [x] **`Cargo.toml`** — Add dependency `dotenvy = "0.15"`
- [x] **`.env`** — Create at project root with `BIND_ADDR=127.0.0.1:8080` and `TICK_RATE=30`
- [x] **`.gitignore`** — Ensure `.env` is ignored (future secrets); add `.env.example` with documented defaults (committed)
- [x] **`src/config.rs`** — New module with `ServerConfig::from_env()` reading `BIND_ADDR` and `TICK_RATE`
- [x] **`network/server.rs`** — Refactor `run_server` to receive `intent_tx: Sender<(SocketAddr, ClientIntent)>` and `bind_addr: &str`
- [x] **`network/server.rs`** — Remove synchronous `ServerResponse` send (no ACK for now)
- [x] **`world/entity.rs`** — Add `velocity: Vector2` field and `Entity::new()` constructor
- [x] **`world/instance.rs`** — Add `client_map: HashMap<SocketAddr, u64>` and `next_entity_id`
- [x] **`world/instance.rs`** — Implement `get_or_create_entity(addr) -> u64`
- [x] **`world/instance.rs`** — Implement `apply_intent(addr, intent)`
- [x] **`world/instance.rs`** — Implement `tick(tick_count)`
- [x] **`game_loop/tick.rs`** — Refactor `GameLoop::start` to receive `Instance` + `intent_rx`
- [x] **`main.rs`** — Read `ServerConfig`, create `mpsc` channel, spawn both threads
- [x] **Build** — `cargo build` with no errors or warnings
- [x] **Manual test** — Send `MoveIntent` via example client and verify tick log with position updating

---

## 5. Files Unchanged in This Phase

| File | Reason |
|---|---|
| `proto/game_packets.proto` | Schema already covers `MoveIntent` with `Vector2` direction |
| `src/world/mod.rs` | Re-exports only |
| `src/game_loop/mod.rs` | Re-export of `tick` only |
| `src/scripting/` | Reserved for Phase 5 (Lua engine) |
| `docs/adr/` | ADR-0006 added — documents the two-thread `mpsc` architecture, its known limitations, and the migration path to async in Phase 5 |

**New files added in this phase:**

| File | Description |
|---|---|
| `src/config.rs` | Reads `.env` via `dotenvy`, exposes `ServerConfig` |
| `.env` | Local configuration values (not committed) |
| `.env.example` | Configuration template for new contributors (committed) |

---

## 6. Phase 1 Completion Criteria

Phase 1 is complete when:

1. `cargo build` passes with no errors
2. `cargo run` starts with:
   - `[Config] bind_addr=127.0.0.1:8080 tick_rate=30 Hz`
   - `[HH:MM:SS] UDP Server listening on 127.0.0.1:8080`
   - `[Tick 0] 0 active entities` (incrementing every ~33ms)
3. Setting `TICK_RATE=20` in `.env` changes the tick rate to 20 Hz without recompiling
4. Sending a `GamePacket` with `MoveIntent { direction: {x:1.0, y:0.0} }` shows in the log:
   - Entity created on auto-join
   - Entity position incrementing each tick
5. The 30 Hz loop is **not** blocked by incoming network packets (threads are independent)

---

## 7. Out of Scope for This Phase (Future Work)

| Feature | Phase |
|---|---|
| `WorldState` broadcast to clients | Phase 3 |
| Formal join/disconnect/timeout handshake | Phase 2 |
| `SocketAddr→EntityId` mapping with auth | Phase 2 |
| Real `delta_time` in tick | Phase 2 (refinement) |
| Lua scripting / `mlua` | Phase 5 |
| Multi-instance / room manager | Phase 5 |
