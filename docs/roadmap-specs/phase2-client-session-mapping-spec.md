# Implementation Spec — Phase 2: Lightweight Client/Session Mapping

> **Status:** Ready for Implementation / **Phase 2**  
> **Roadmap Phase:** Phase 2 — Lightweight Client/Session Mapping  
> **Reference ADRs:** [ADR-0001](../adr/en/0001-authoritative-server-archoitecture.md) · [ADR-0002](../adr/en/0002-instance-based-architecture.md) · [ADR-0005](../adr/en/0005-cross-language-binary-serialization.md) · [ADR-0006](../adr/en/0006-two-thread-network-gameloop-separation.md)

---

## 1. Context & Current State

In **Phase 1**, `loci2d` established the two-thread game loop and single-instance core:
- The network thread decodes incoming Protobuf `GamePacket` messages over UDP and forwards `(SocketAddr, ClientIntent)` pairs via an `mpsc` channel.
- The game loop thread advances the simulation at a fixed tick rate (e.g. 30 Hz) and applies movement intents to entities.

However, client session handling in Phase 1 is a rudimentary prototype baseline:

| Component | Phase 1 Implementation | Limitation / Problem to Solve in Phase 2 |
|---|---|---|
| `world/instance.rs` | `client_map: HashMap<SocketAddr, u64>` with naive auto-join | No session tracking, no connection metadata, no player identity |
| `proto/game_packets.proto` | Only `MoveIntent`, `ActionIntent`, `PingIntent` | Missing explicit `JoinIntent` and `DisconnectIntent` |
| Entity Lifecycle | Entities are created on first packet and never removed | Disconnected or crashed clients leave orphan "ghost" entities moving indefinitely |
| Timeout Detection | None | Server has no concept of inactivity or connection loss |
| Handshake / Auth | Implicit (first packet auto-creates player entity) | No structured join handshake or client session lifecycle management |
| `src/bin/client.rs` | Sends raw intents without join/leave handshake | No way to test connection lifecycle, sessions, or graceful leaves |

**Core Problem:** In a real-time multiplayer game server, clients connect, pause, drop connection, or quit. Without explicit session mapping, timeout detection, and entity despawn routines, memory and entity counts leak over time, and the authoritative simulation becomes cluttered with abandoned entities.

---

## 2. Phase 2 Goals & Architecture

### 2.1 Primary Objectives
1. **Explicit Session Lifecycle**: Model client connections with a dedicated `ClientSession` struct tracking `session_id`, `addr: SocketAddr`, `entity_id: u64`, `player_name: String`, `last_seen: Instant`, `connected_at: Instant`, and `state: SessionState`.
2. **Protocol Extensions**: Add `JoinIntent` and `DisconnectIntent` to `proto/game_packets.proto` while retaining backward compatibility for raw intents.
3. **Inactivity & Timeout Detection**: Check client heartbeats (`last_seen`) during the tick loop against a configurable timeout threshold (`CLIENT_TIMEOUT_SECS`, default: 10s) and cleanly despawn inactive entities.
4. **Graceful Disconnect & Despawning**: Cleanly remove entities from the simulation upon receiving `DisconnectIntent`.
5. **Zero-Overhead Simplified Handshake**: Maintain minimal architecture with zero external token/auth servers, optimized for rapid prototyping and indie/educational workflows.

### 2.2 Client Session State Machine

```
                        ┌────────────────────────┐
                        │    Incoming Packet     │
                        │    (SocketAddr: IP)    │
                        └───────────┬────────────┘
                                    │
                  ┌─────────────────┴─────────────────┐
                  ▼                                   ▼
        [ Known SocketAddr ]               [ Unknown SocketAddr ]
                  │                                   │
                  │ Refresh `last_seen`               ├─ JoinIntent ──────────► Create Session & Entity
                  │ Process Intent                    └─ Other Intent ────────► Auto-Join (if enabled)
                  │                                                              or Drop Packet
                  ▼
        ┌───────────────────┐
        │  Session: ACTIVE  │
        └─────────┬─────────┘
                  │
        ┌─────────┴───────────────────────────────┐
        │                                         │
        ▼ (DisconnectIntent)                      ▼ (No packets for > CLIENT_TIMEOUT_SECS)
┌───────────────────────┐               ┌───────────────────────┐
│ Session: DISCONNECTED │               │   Session: TIMED_OUT  │
└───────────┬───────────┘               └───────────┬───────────┘
            │                                       │
            └───────────────────┬───────────────────┘
                                ▼
                    ┌───────────────────────┐
                    │ Despawn Entity from   │
                    │ Instance.entities     │
                    │ & Remove Session      │
                    └───────────────────────┘
```

---

## 3. Detailed Technical Design & Changes per File

### 3.1 `proto/game_packets.proto` — Add `JoinIntent` & `DisconnectIntent`

Extend the protobuf schema with explicit join and disconnect intent payloads:

```protobuf
syntax = "proto3";

package loci2d;

// 2D Vector representation
message Vector2 {
  float x = 1;
  float y = 2;
}

// Client Intents
message JoinIntent {
  string player_name = 1;
}

message DisconnectIntent {
  string reason = 1;
}

message MoveIntent {
  Vector2 direction = 1;
}

message ActionIntent {
  uint32 ability_id = 1;
}

message PingIntent {}

message ClientIntent {
  oneof intent {
    MoveIntent move = 1;
    ActionIntent action = 2;
    PingIntent ping = 3;
    JoinIntent join = 4;
    DisconnectIntent disconnect = 5;
  }
}

// Top-level Network Game Packet (Envelope)
message GamePacket {
  uint64 sequence_id = 1;
  uint64 timestamp = 2;
  ClientIntent intent = 3;
}

// Server Response Packet (Reserved for Phase 3 state sync / ACKs)
message ServerResponse {
  uint64 sequence_id = 1;
  string status = 2;
}
```

> **Build Impact:** `prost-build` in `build.rs` will automatically generate the corresponding Rust enum variants and structs (`JoinIntent`, `DisconnectIntent`, `client_intent::Intent::Join`, `client_intent::Intent::Disconnect`).

---

### 3.2 `src/world/session.rs` — New Session Model (**New File**)

Create a dedicated session tracking module `src/world/session.rs`:

```rust
use std::net::SocketAddr;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    Active,
    TimedOut,
    Disconnected,
}

#[derive(Debug, Clone)]
pub struct ClientSession {
    pub session_id: u64,
    pub addr: SocketAddr,
    pub entity_id: u64,
    pub player_name: String,
    pub connected_at: Instant,
    pub last_seen: Instant,
    pub state: SessionState,
}

impl ClientSession {
    pub fn new(session_id: u64, addr: SocketAddr, entity_id: u64, player_name: String) -> Self {
        let now = Instant::now();
        Self {
            session_id,
            addr,
            entity_id,
            player_name,
            connected_at: now,
            last_seen: now,
            state: SessionState::Active,
        }
    }

    /// Update the heartbeat timestamp when any valid packet is received.
    pub fn refresh_activity(&mut self) {
        self.last_seen = Instant::now();
    }

    /// Check if the session has exceeded the timeout threshold.
    pub fn is_timed_out(&self, timeout_secs: u64) -> bool {
        self.last_seen.elapsed().as_secs() >= timeout_secs
    }
}
```

---

### 3.3 `src/world/entity.rs` — Entity Metadata & Ownership

Update `Entity` to store player display names or session identifiers:

```rust
use super::instance::Vector2;

#[derive(Debug, Clone)]
pub struct Entity {
    pub id: u64,
    pub name: String,
    pub position: Vector2,
    pub velocity: Vector2,
    pub entity_type: EntityType,
}

impl Entity {
    pub fn new(id: u64, name: String, entity_type: EntityType) -> Self {
        Self {
            id,
            name,
            position: Vector2 { x: 0.0, y: 0.0 },
            velocity: Vector2 { x: 0.0, y: 0.0 },
            entity_type,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityType {
    Player,
    NPC,
    Prop,
}
```

---

### 3.4 `src/config.rs` & `.env` — Session & Timeout Configuration

Add timeout settings and auto-join preferences to configuration:

**`.env` and `.env.example`:**
```env
BIND_ADDR=127.0.0.1:8080
TICK_RATE=30
CLIENT_TIMEOUT_SECS=10
AUTO_JOIN_ON_INTENT=true
```

**`src/config.rs`:**
```rust
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: String,
    pub tick_rate: u32,
    pub client_timeout_secs: u64,
    pub auto_join_on_intent: bool,
}

impl ServerConfig {
    pub fn from_env() -> Self {
        let _ = dotenvy::dotenv();

        Self {
            bind_addr: std::env::var("BIND_ADDR")
                .unwrap_or_else(|_| "127.0.0.1:8080".to_string()),
            tick_rate: std::env::var("TICK_RATE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
            client_timeout_secs: std::env::var("CLIENT_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            auto_join_on_intent: std::env::var("AUTO_JOIN_ON_INTENT")
                .map(|v| v.to_lowercase() == "true" || v == "1")
                .unwrap_or(true),
        }
    }
}
```

---

### 3.5 `src/world/instance.rs` — Full Session & Lifecycle Management

Refactor `Instance` to manage `ClientSession`s, explicit joins, graceful disconnects, and tick-based timeout detection:

```rust
use std::collections::HashMap;
use std::net::SocketAddr;
use super::entity::{Entity, EntityType};
use super::session::{ClientSession, SessionState};
use crate::network::packets::ClientIntent;

pub use crate::network::packets::Vector2;

#[derive(Debug)]
pub struct Instance {
    pub id: u64,
    pub entities: HashMap<u64, Entity>,
    pub sessions: HashMap<SocketAddr, ClientSession>,
    pub entity_to_addr: HashMap<u64, SocketAddr>,
    pub tick_rate: u32,
    pub client_timeout_secs: u64,
    pub auto_join_on_intent: bool,
    next_entity_id: u64,
    next_session_id: u64,
}

impl Instance {
    pub fn new(id: u64, tick_rate: u32, client_timeout_secs: u64, auto_join_on_intent: bool) -> Self {
        Self {
            id,
            entities: HashMap::new(),
            sessions: HashMap::new(),
            entity_to_addr: HashMap::new(),
            tick_rate,
            client_timeout_secs,
            auto_join_on_intent,
            next_entity_id: 1,
            next_session_id: 1,
        }
    }

    /// Handles an incoming client intent.
    pub fn apply_intent(&mut self, addr: SocketAddr, intent: ClientIntent) {
        use crate::network::packets::client_intent::Intent;

        let Some(inner_intent) = intent.intent else { return; };

        // 1. Handle explicit Join
        if let Intent::Join(join_intent) = &inner_intent {
            let player_name = if join_intent.player_name.trim().is_empty() {
                format!("Player_{}", self.next_entity_id)
            } else {
                join_intent.player_name.clone()
            };
            self.handle_join(addr, player_name);
            return;
        }

        // 2. Handle explicit Disconnect
        if let Intent::Disconnect(disconnect_intent) = &inner_intent {
            self.handle_disconnect(addr, &disconnect_intent.reason);
            return;
        }

        // 3. Resolve or auto-create session for gameplay intents
        let entity_id = match self.get_or_create_session(addr) {
            Some(id) => id,
            None => {
                println!("[Drop] Ignoring intent from unauthenticated client: {}", addr);
                return;
            }
        };

        // 4. Apply intent to active entity
        if let Some(entity) = self.entities.get_mut(&entity_id) {
            match inner_intent {
                Intent::Move(move_intent) => {
                    if let Some(dir) = move_intent.direction {
                        entity.velocity = Vector2 { x: dir.x, y: dir.y };
                    }
                }
                Intent::Action(action_intent) => {
                    println!("[Intent] Entity {} ({}) executed action {}", entity_id, entity.name, action_intent.ability_id);
                }
                Intent::Ping(_) => {
                    // Ping activity already refreshed via get_or_create_session
                }
                Intent::Join(_) | Intent::Disconnect(_) => unreachable!(),
            }
        }
    }

    /// Explicit client join
    pub fn handle_join(&mut self, addr: SocketAddr, player_name: String) -> u64 {
        if let Some(session) = self.sessions.get_mut(&addr) {
            session.player_name = player_name.clone();
            session.refresh_activity();
            if let Some(entity) = self.entities.get_mut(&session.entity_id) {
                entity.name = player_name;
            }
            println!("[Session] Client {} re-joined as '{}' (EntityId {})", addr, session.player_name, session.entity_id);
            return session.entity_id;
        }

        let entity_id = self.next_entity_id;
        self.next_entity_id += 1;

        let session_id = self.next_session_id;
        self.next_session_id += 1;

        let session = ClientSession::new(session_id, addr, entity_id, player_name.clone());
        let entity = Entity::new(entity_id, player_name.clone(), EntityType::Player);

        self.entities.insert(entity_id, entity);
        self.sessions.insert(addr, session);
        self.entity_to_addr.insert(entity_id, addr);

        println!("[Join] Client {} joined as '{}' (SessionId {}, EntityId {})", addr, player_name, session_id, entity_id);
        entity_id
    }

    /// Explicit client disconnect
    pub fn handle_disconnect(&mut self, addr: SocketAddr, reason: &str) {
        if let Some(mut session) = self.sessions.remove(&addr) {
            session.state = SessionState::Disconnected;
            self.entity_to_addr.remove(&session.entity_id);
            self.entities.remove(&session.entity_id);
            println!("[Disconnect] Client {} ('{}', EntityId {}) disconnected gracefully. Reason: '{}'", 
                addr, session.player_name, session.entity_id, if reason.is_empty() { "normal quit" } else { reason });
        }
    }

    /// Resolves existing session or triggers fallback auto-join
    fn get_or_create_session(&mut self, addr: SocketAddr) -> Option<u64> {
        if let Some(session) = self.sessions.get_mut(&addr) {
            session.refresh_activity();
            return Some(session.entity_id);
        }

        if self.auto_join_on_intent {
            let default_name = format!("Player_{}", self.next_entity_id);
            Some(self.handle_join(addr, default_name))
        } else {
            None
        }
    }

    /// Advance physics and sweep for timed-out sessions
    pub fn tick(&mut self, tick_count: u64) {
        // 1. Advance simulation positions
        for entity in self.entities.values_mut() {
            entity.position.x += entity.velocity.x;
            entity.position.y += entity.velocity.y;
        }

        // 2. Check for timed out clients
        self.check_timeouts();

        // 3. Log tick status
        println!("[Tick {}] {} active entities, {} active sessions", tick_count, self.entities.len(), self.sessions.len());
    }

    /// Sweep and remove inactive sessions
    pub fn check_timeouts(&mut self) {
        let timeout_secs = self.client_timeout_secs;
        let mut timed_out_addrs = Vec::new();

        for (addr, session) in &self.sessions {
            if session.is_timed_out(timeout_secs) {
                timed_out_addrs.push((*addr, session.entity_id, session.player_name.clone()));
            }
        }

        for (addr, entity_id, player_name) in timed_out_addrs {
            self.sessions.remove(&addr);
            self.entity_to_addr.remove(&entity_id);
            self.entities.remove(&entity_id);
            println!("[Timeout] Client {} ('{}', EntityId {}) timed out after {}s of inactivity", addr, player_name, entity_id, timeout_secs);
        }
    }
}
```

---

### 3.6 `src/game_loop/tick.rs` & `src/main.rs` — Configuration Wiring

Update `main.rs` to pass configuration parameters (`client_timeout_secs`, `auto_join_on_intent`) into `Instance`:

```rust
// In src/main.rs
let instance = Instance::new(
    1, 
    cfg.tick_rate, 
    cfg.client_timeout_secs, 
    cfg.auto_join_on_intent
);
```

---

### 3.7 `src/bin/client.rs` — Interactive Testing Client

Enhance the Rust CLI test client with `join`, `leave`, `ping`, and informative status prompts:

```rust
// Commands supported in Phase 2 client:
// join <player_name>  -> Sends JoinIntent
// leave [reason]      -> Sends DisconnectIntent and resets local state
// ping                -> Sends PingIntent (heartbeat)
// move <x> <y>        -> Sends MoveIntent
// action <id>         -> Sends ActionIntent
// quit                -> Sends DisconnectIntent then exits process
```

---

## 4. Simplified Handshake & Security Rationale

In accordance with [ADR-0001](../adr/en/0001-authoritative-server-archoitecture.md) and [ADR-0002](../adr/en/0002-instance-based-architecture.md):
- **Why No Token/Auth Server?** Introducing OAuth, JWT, or database authentication during single-instance prototyping introduces immense setup friction for students and game engine developers.
- **Identity Mechanism**: Client identity is bound to `SocketAddr` within the instance. The client supplies a human-readable display name (`player_name`) via `JoinIntent`.
- **Packet Spoofing**: Since this phase targets localhost/LAN validation and trusted private game rooms, cryptographic packet signing is explicitly deferred to **Phase 5 (Authentication & Security)**.

---

## 5. Implementation Checklist

- [ ] **`proto/game_packets.proto`** — Add `JoinIntent` and `DisconnectIntent` messages, and register them in `ClientIntent.oneof`
- [ ] **`build.rs` / `Cargo.toml`** — Verify `cargo check` compiles updated protobuf definitions with `prost`
- [ ] **`src/world/session.rs`** — Implement `ClientSession`, `SessionState`, and `is_timed_out`
- [ ] **`src/world/mod.rs`** — Register and export `pub mod session;`
- [ ] **`src/world/entity.rs`** — Add `name: String` field to `Entity` and update `Entity::new` constructor
- [ ] **`src/config.rs`** — Add `client_timeout_secs` (default `10`) and `auto_join_on_intent` (default `true`)
- [ ] **`.env` / `.env.example`** — Document `CLIENT_TIMEOUT_SECS` and `AUTO_JOIN_ON_INTENT`
- [ ] **`src/world/instance.rs`** — Implement `handle_join`, `handle_disconnect`, `check_timeouts`, and update `apply_intent` & `tick`
- [ ] **`src/main.rs`** — Initialize `Instance` with timeout and auto-join settings from `ServerConfig`
- [ ] **`src/bin/client.rs`** — Add CLI commands for `join`, `leave`, `ping`, and auto-disconnect on `quit`
- [ ] **Unit Tests** — Add unit tests in `src/world/instance.rs` verifying:
  - Explicit join registers session and creates entity with custom name
  - Explicit disconnect removes entity and cleans session map
  - Inactivity timeout drops session and despawns entity after configured threshold
  - Fallback auto-join behavior for backwards compatibility
- [ ] **Integration Verification** — Test multi-client interaction with CLI and Python clients

---

## 6. Files Unchanged in This Phase

| File / Subsystem | Reason |
|---|---|
| `src/network/server.rs` | Socket decoding forwards `(SocketAddr, ClientIntent)` directly; intent extraction remains transparent |
| `src/game_loop/tick.rs` | Game loop advances `instance.tick()` which internally invokes timeout sweeping |
| `src/scripting/` | Lua integration reserved for Phase 5 |
| `proto/game_packets.proto` (`ServerResponse`) | Outbound state sync and broadcast packets will be formalized in Phase 3 |

---

## 7. Phase 2 Completion Criteria

Phase 2 is considered complete when:

1. **Clean Compilation**: `cargo build` and `cargo test` pass with **zero warnings and zero errors**.
2. **Explicit Join Handshake**:
   - Running `cargo run --bin client` and executing `join Arthur` creates `Entity` named `"Arthur"`.
   - Server logs: `[Join] Client 127.0.0.1:XXXXX joined as 'Arthur' (SessionId 1, EntityId 1)`.
3. **Graceful Disconnect**:
   - Sending `leave` or `quit` in the client immediately despawns the entity from the server.
   - Server logs: `[Disconnect] Client 127.0.0.1:XXXXX ('Arthur', EntityId 1) disconnected gracefully.`
   - Next tick shows `0 active entities, 0 active sessions`.
4. **Automatic Timeout Sweep**:
   - Starting a client, joining, and terminating the client process without `leave`.
   - After `CLIENT_TIMEOUT_SECS` (e.g. 10s), server logs: `[Timeout] Client 127.0.0.1:XXXXX ('Arthur', EntityId 1) timed out after 10s of inactivity`.
   - The entity is automatically removed from the simulation.
5. **Backwards-Compatible Auto-Join**:
   - Legacy scripts sending raw `move 1.0 0.0` without a prior `join` command are automatically registered as `Player_<id>` without crashing.
