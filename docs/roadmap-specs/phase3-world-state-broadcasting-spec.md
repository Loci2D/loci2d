# Implementation Spec — Phase 3: World State Broadcasting & Client Sync

> **Status:** Completed  
> **Roadmap Phase:** Phase 3 — World State Broadcasting & Client Sync  
> **Reference ADRs:** [ADR-0001](../adr/en/0001-authoritative-server-archoitecture.md) · [ADR-0002](../adr/en/0002-instance-based-architecture.md) · [ADR-0003](../adr/en/0003-2d-map-only.md) · [ADR-0005](../adr/en/0005-cross-language-binary-serialization.md) · [ADR-0006](../adr/en/0006-two-thread-network-gameloop-separation.md) · [ADR-0008](../adr/en/0008-session-lifecycle-and-client-identity.md) · [ADR-0009](../adr/en/0009-simplified-authentication-and-auto-join-strategy.md)

---

## 1. Context & Current State

In **Phase 1** and **Phase 2**, `loci2d` established:
1. A two-thread network and game-loop architecture connected via `mpsc` channels ([ADR-0006](../adr/en/0006-two-thread-network-gameloop-separation.md)).
2. Cross-language binary packet serialization via Protocol Buffers v3 ([ADR-0005](../adr/en/0005-cross-language-binary-serialization.md)).
3. Explicit client session lifecycle management, heartbeat timeout detection, and entity despawning ([ADR-0008](../adr/en/0008-session-lifecycle-and-client-identity.md), [ADR-0009](../adr/en/0009-simplified-authentication-and-auto-join-strategy.md)).

However, the communication channel is currently **unidirectional (inbound only)**:

| Subsystem | Current State (Phase 2) | Phase 3 Target |
|---|---|---|
| Inbound Traffic | Clients send `GamePacket` (`JoinIntent`, `MoveIntent`, `DisconnectIntent`, etc.) | Unchanged (inbound intent streaming continues) |
| Outbound Traffic | None (`UdpSocket` is only read by the network thread) | Game loop broadcasts authoritative `WorldState` snapshots back to connected clients |
| Network Socket Sharing | `UdpSocket` owned exclusively by the network thread | `Arc<UdpSocket>` shared between network (inbound `recv_from`) and game loop (outbound `send_to`) |
| Protobuf Schema | `ServerResponse` exists as placeholder | `ServerPacket` envelope containing `WorldState` (tick, timestamp, `repeated EntityState`) and `ServerResponse` |
| Client Rendering | Clients send inputs blindly without receiving server world state | Clients receive binary `WorldState` snapshots every tick and render authoritative entity positions |

**Core Problem:** An authoritative multiplayer game server cannot function without transmitting its official world state back to clients. In Phase 3, we close the loop: the server broadcasts authoritative world snapshots at the fixed tick rate, enabling client game engines (Godot, Love2D, Python, CLI) to render multi-entity movement in real time.

---

## 2. Phase 3 Goals & Architecture

### 2.1 Primary Objectives

1. **Protobuf Snapshot Schema**: Formalize `EntityState`, `WorldState`, and the top-level `ServerPacket` envelope in `proto/game_packets.proto`.
2. **Bidirectional Socket Sharing (`Arc<UdpSocket>`)**: Share the bound `UdpSocket` across threads using `Arc<UdpSocket>` to enable the game loop thread to call `send_to` concurrently with the network thread's blocking `recv_from` ([ADR-0006](../adr/en/0006-two-thread-network-gameloop-separation.md)).
3. **Tick-Based Snapshot Generation**: Implement `Instance::create_snapshot(tick_count)` to generate structured world snapshots containing all active entity positions, velocities, names, and types.
4. **Authoritative World Broadcasting**: After advancing simulation physics and sweeping timeouts on each tick, serialize and broadcast the `WorldState` snapshot to all active client `SocketAddr`s in `instance.sessions`.
5. **Multi-Language Client Synchronization**:
   - **Rust CLI Client**: Add background listener thread to display active entities and tick updates.
   - **Love2D (Lua)**: Decode `WorldState` in `love.update()` and visually render all players on screen in `love.draw()`.
   - **Python**: Receive and print world state snapshots in real time.
   - **Godot 4 (GDScript)**: Poll and decode `ServerPacket` for spatial node synchronization.

---

### 2.2 End-to-End Bidirectional Data Flow

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                                    loci2d Server                                       │
│                                                                                        │
│   ┌────────────────────────────────┐                 ┌─────────────────────────────┐   │
│   │ Network Thread                 │                 │ Game Loop Thread            │   │
│   │ Arc<UdpSocket>::recv_from()    │                 │ Fixed Tick Rate (e.g. 30Hz) │   │
│   │ (Blocking Inbound)             │                 │ (Simulation & Physics)      │   │
│   └───────────────┬────────────────┘                 └──────────────┬──────────────┘   │
│                   │                                                 │                  │
│                   │ mpsc::channel<(SocketAddr, ClientIntent)>       │                  │
│                   └───────────────────────► 1. Drain Intents        │                  │
│                                             2. Advance Simulation   │                  │
│                                             3. Sweep Timeouts       │                  │
│                                             4. Create Snapshot ─────┘                  │
│                                                        │                               │
│                                                        ▼                               │
│                                             Arc<UdpSocket>::send_to()                  │
│                                             (Outbound Binary Broadcast)                │
└────────────────────────────────────────────────────────┬───────────────────────────────┘
                                                         │
                             UDP Broadcast (WorldState)  │
                                                         ▼
                ┌────────────────────────────────────────────────────────┐
                │                  Connected Clients                     │
                ├────────────────────┬───────────────────┬───────────────┤
                │ Godot 4 (GDScript) │   Love2D (Lua)    │ Python / CLI  │
                │ Node2D Transforms  │ Canvas 2D Render  │ State Logging │
                └────────────────────┴───────────────────┴───────────────┘
```

---

## 3. Detailed Technical Design & Changes per File

### 3.1 `proto/game_packets.proto` — Formalize Outbound Snapshot Schema

Extend the protobuf schema with `EntityType`, `EntityState`, `WorldState`, and top-level `ServerPacket` envelope:

```protobuf
syntax = "proto3";

package loci2d;

// 2D Vector representation (ADR-0003)
message Vector2 {
  float x = 1;
  float y = 2;
}

// Client Intents (Inbound)
message JoinIntent {
  string player_name = 1;
}

message DisconnectIntent {
  string reason = 1;
}

// Continuous Directional Movement (Joystick / WASD / Analog Stick)
message MoveIntent {
  Vector2 direction = 1; // Heading vector with magnitude <= 1.0
}

// Destination / Target Movement (Mouse Click-to-Move / Screen Tap - Phase 6)
message MoveToPositionIntent {
  Vector2 target_position = 1; // Absolute map coordinates
}

message ActionIntent {
  uint32 ability_id = 1;
}

message PingIntent {}

message ClientIntent {
  oneof intent {
    MoveIntent move = 1;
    MoveToPositionIntent move_to_pos = 2; // Reserved for Phase 6 click-to-move
    ActionIntent action = 3;
    PingIntent ping = 4;
    JoinIntent join = 5;
    DisconnectIntent disconnect = 6;
  }
}

// Top-level Inbound Game Packet Envelope
message GamePacket {
  uint64 sequence_id = 1;
  uint64 timestamp = 2;
  ClientIntent intent = 3;
}

// ==========================================
// Phase 3 Outbound Protocols (Server -> Client)
// ==========================================

enum EntityType {
  PLAYER = 0;
  NPC = 1;
  PROP = 2;
}

// Individual Entity State in World Snapshot
message EntityState {
  uint64 id = 1;
  string name = 2;
  Vector2 position = 3;
  Vector2 velocity = 4;
  EntityType entity_type = 5;
}

// Full Authoritative World Snapshot
message WorldState {
  uint64 tick = 1;
  uint64 timestamp = 2;
  repeated EntityState entities = 3;
}

// Direct Acknowledgment / Server Response
message ServerResponse {
  uint64 sequence_id = 1;
  string status = 2;
}

// Top-level Outbound Server Packet Envelope
message ServerPacket {
  uint64 sequence_id = 1;
  oneof payload {
    WorldState world_state = 2;
    ServerResponse response = 3;
  }
}
```

---

### 3.2 `src/network/server.rs` — Socket Sharing via `Arc<UdpSocket>`

Refactor `run_server` to accept a shared `Arc<UdpSocket>` instead of binding internally:

```rust
use std::net::SocketAddr;
use std::sync::mpsc;
use std::sync::Arc;
use std::net::UdpSocket;
use chrono::Local;
use prost::Message;
use super::packets::{GamePacket, ClientIntent};

pub fn run_server(socket: Arc<UdpSocket>, intent_tx: mpsc::Sender<(SocketAddr, ClientIntent)>) {
    let mut buf = [0u8; 2048];

    loop {
        match socket.recv_from(&mut buf) {
            Ok((num_bytes, src_addr)) => {
                let received_data = &buf[..num_bytes];
                
                match GamePacket::decode(received_data) {
                    Ok(packet) => {
                        if let Some(intent) = packet.intent {
                            let _ = intent_tx.send((src_addr, intent));
                        }
                    }
                    Err(e) => {
                        println!("[{}] Failed to deserialize GamePacket from {}: {}", 
                            Local::now().format("%Y-%m-%d %H:%M:%S"),
                            src_addr,
                            e
                        );
                    }
                }
            }
            Err(e) => {
                println!("[{}] Error receiving data: {}", Local::now().format("%Y-%m-%d %H:%M:%S"), e);
            }
        }
    }
}
```

---

### 3.3 `src/world/instance.rs` — Snapshot Generation

Add `create_snapshot` to `Instance` to build `WorldState` packets:

```rust
use crate::network::packets::{EntityState, WorldState, EntityType as ProtoEntityType};

impl Instance {
    /// Generates a complete WorldState snapshot representing all active entities.
    pub fn create_snapshot(&self, tick: u64) -> WorldState {
        let entities = self.entities.values().map(|e| {
            EntityState {
                id: e.id,
                name: e.name.clone(),
                position: Some(e.position.clone()),
                velocity: Some(e.velocity.clone()),
                entity_type: match e.entity_type {
                    EntityType::Player => ProtoEntityType::Player as i32,
                    EntityType::NPC => ProtoEntityType::Npc as i32,
                    EntityType::Prop => ProtoEntityType::Prop as i32,
                },
            }
        }).collect();

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        WorldState {
            tick,
            timestamp,
            entities,
        }
    }

    /// Returns a list of all active client destination addresses for broadcasting.
    pub fn get_broadcast_addresses(&self) -> Vec<SocketAddr> {
        self.sessions.keys().copied().collect()
    }
}
```

---

### 3.4 `src/game_loop/tick.rs` — Outbound Snapshot Broadcast

Update `GameLoop` to receive `Arc<UdpSocket>` and broadcast serialized snapshots to all active sessions:

```rust
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
        let mut tick_count = 0u64;
        let mut out_buf = Vec::with_capacity(2048);

        while self.running {
            let start = Instant::now();

            // 1. Drain the intent queue (non-blocking)
            while let Ok((addr, intent)) = intent_rx.try_recv() {
                instance.apply_intent(addr, intent);
            }

            // 2. Advance simulation physics & sweep timeouts
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

            // 6. Sleep remaining tick budget
            let elapsed = start.elapsed();
            if elapsed < tick_duration {
                thread::sleep(tick_duration - elapsed);
            }
        }
    }

    #[allow(dead_code)]
    pub fn stop(&mut self) {
        self.running = false;
    }
}
```

---

### 3.5 `src/main.rs` — Application Wiring

Update `main.rs` to bind the socket once and pass `Arc<UdpSocket>` to both threads:

```rust
use std::net::UdpSocket;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use loci2d::config::ServerConfig;
use loci2d::game_loop::tick::GameLoop;
use loci2d::network::server::run_server;
use loci2d::world::instance::Instance;

fn main() {
    let cfg = ServerConfig::from_env();
    println!("[Config] bind_addr={} tick_rate={} Hz client_timeout={}s", 
        cfg.bind_addr, cfg.tick_rate, cfg.client_timeout_secs);

    let socket = UdpSocket::bind(&cfg.bind_addr).expect("Failed to bind UDP socket");
    let socket = Arc::new(socket);
    println!("[Server] Bound to {}", socket.local_addr().unwrap());

    let (intent_tx, intent_rx) = mpsc::channel();
    let tick_rate = cfg.tick_rate;
    let client_timeout_secs = cfg.client_timeout_secs;

    // Network thread (inbound)
    let net_socket = Arc::clone(&socket);
    let net_thread = thread::spawn(move || {
        run_server(net_socket, intent_tx);
    });

    // Game loop thread (simulation & outbound broadcast)
    let loop_socket = Arc::clone(&socket);
    let loop_thread = thread::spawn(move || {
        let instance = Instance::new(1, tick_rate, client_timeout_secs);
        let mut game_loop = GameLoop::new(tick_rate);
        game_loop.start(instance, intent_rx, loop_socket);
    });

    let _ = net_thread.join();
    let _ = loop_thread.join();
}
```

---

### 3.6 `src/bin/client.rs` — Asynchronous Snapshot Receiver

Update `src/bin/client.rs` to run a background listener thread that decodes incoming `ServerPacket`s:

```rust
// Background listener receives and prints incoming WorldState updates:
// [2026-08-08 18:30:01] [Snapshot Tick 120] 2 entities in world:
//   - Entity 1 ("Arthur", Player) @ (12.5, -4.0), vel=(1.0, 0.0)
//   - Entity 2 ("Lancelot", Player) @ (0.0, 0.0), vel=(0.0, 0.0)
```

---

### 3.7 Multi-Language Client Examples

#### Python (`examples/python/client.py`)
- Run a non-blocking receiver loop to decode `ServerPacket.world_state` and display active entities.

#### Love2D (`examples/love2d/main.lua`)
- In `love.update(dt)`: Decode `ServerPacket` via `lua-protobuf` into `current_world_state`.
- In `love.draw()`: Render each entity in `current_world_state.entities` as a 2D shape (circle/rectangle) at `(pos.x, pos.y)` with player name labels above them.

#### Godot 4 (`examples/godot/NetworkClient.gd`)
- In `_process(delta)`: Read packets with `_udp.get_packet()`, decode `ServerPacket`, and emit a `world_state_updated(entities)` signal.

---

## 4. ADR Compliance & Design Rationale

| Decision / Rule | ADR Reference | How Phase 3 Complies |
|---|---|---|
| **Authoritative State** | [ADR-0001](../adr/en/0001-authoritative-server-archoitecture.md) | World state is computed solely by the server simulation; clients only receive and render authoritative positions. |
| **Instance Isolation** | [ADR-0002](../adr/en/0002-instance-based-architecture.md) | Snapshots are generated strictly from the active `Instance` and sent only to sessions mapped within that instance. |
| **2D Coordinates Only** | [ADR-0003](../adr/en/0003-2d-map-only.md) | `EntityState` uses `Vector2` (`x, y`), omitting all 3D volumetric / rotation fields. |
| **Binary Protobuf Contract** | [ADR-0005](../adr/en/0005-cross-language-binary-serialization.md) | Formalized in `proto/game_packets.proto` via `ServerPacket` envelope with typed field numbers. |
| **Thread Decoupling** | [ADR-0006](../adr/en/0006-two-thread-network-gameloop-separation.md) | `Arc<UdpSocket>` enables non-blocking outbound broadcasts from the game loop while isolating inbound `recv_from`. |
| **Explicit Session Addressing** | [ADR-0008](../adr/en/0008-session-lifecycle-and-client-identity.md) | Outbound packets are broadcast strictly to active `SocketAddr`s registered in `instance.sessions`. |

---

## 5. Implementation Checklist

- [x] **`proto/game_packets.proto`** — Add `EntityType`, `EntityState`, `WorldState`, and `ServerPacket` envelope definitions.
- [x] **`src/network/packets.rs`** — Add unit tests for encoding and decoding `ServerPacket` and `WorldState`.
- [x] **`src/network/server.rs`** — Update `run_server` to accept `Arc<UdpSocket>`.
- [x] **`src/world/instance.rs`** — Implement `create_snapshot(tick)` and `get_broadcast_addresses()`.
- [x] **`src/game_loop/tick.rs`** — Update `GameLoop::start` to accept `Arc<UdpSocket>` and broadcast snapshots to active sessions every tick.
- [x] **`src/main.rs`** — Initialize `Arc<UdpSocket>` and wire it into `net_thread` and `loop_thread`.
- [x] **`src/bin/client.rs`** — Add background UDP listener thread to receive and format `WorldState` snapshots in real time.
- [x] **Integration Tests (`tests/broadcasting_integration_test.rs`)** — Verify that:
  - Connecting client receives `WorldState` containing its entity on the very next tick.
  - Applying `MoveIntent` changes position in subsequent received snapshots.
  - Multiple connected clients receive snapshots containing all peers' entities.
  - Disconnected or timed-out clients disappear from subsequent snapshots.
- [x] **Client Examples** — Update `examples/python/client.py`, `examples/love2d/main.lua`, and `examples/godot/NetworkClient.gd` to consume and render snapshots.
- [x] **Documentation** — Update `docs/roadmap.md` and `README.md`.

---

## 6. Phase 3 Completion Criteria

Phase 3 is considered complete when:

1. **Clean Build & Tests**: `cargo build` and `cargo test --all-targets` pass with **zero warnings and zero errors**.
2. **Authoritative World State Sync**:
   - Starting server: `cargo run`.
   - Running client: `cargo run --bin client` and executing `join Alice`.
   - Client logs: `[Snapshot Tick 1] 1 entity: Alice @ (0.0, 0.0)`.
   - Client executes `move 2.0 1.0` $\rightarrow$ subsequent snapshots reflect updated positions: `Alice @ (2.0, 1.0)`.
3. **Multi-Client World Visibility**:
   - Starting a second client `cargo run --bin client` and executing `join Bob`.
   - Both clients receive snapshots containing **both** `Alice` and `Bob` with their respective positions and velocities.
4. **Despawn Synchronization**:
   - Client `Bob` executes `leave` or disconnects $\rightarrow$ Client `Alice` receives next snapshot with `Bob` cleanly omitted.
5. **Love2D 2D Rendering**:
   - Running Love2D client renders moving player shapes and name tags directly on screen driven by live UDP `WorldState` snapshots.
