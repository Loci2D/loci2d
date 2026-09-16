# loci2d Project Roadmap

> 🌐 *Read this in [Português (Brasil)](roadmap.pt-BR.md)*

This document outlines the development phases, goals, and technical milestones for **loci2d**, an authoritative 2D game server framework in Rust.

---

## Current Vision & Approach

* **Validation Phase**: Focus on getting a fully functional, stable **single instance** running locally (`127.0.0.1`).
* **Authoritative Model**: Receives client intents, processes physics/game loop, and broadcasts authoritative state.
* **Low Barrier**: Minimal complexity for indie devs and students.

---

## Roadmap Phases

### Phase 1: Game Loop & Single-Instance Core (Done)
* [x] **Network & Loop Integration**:
  - Connect UDP socket network thread to `GameLoop` via thread-safe channels (`mpsc`).
  - Implement non-blocking intent queue consumption inside fixed tick rate (e.g., 20/30 Hz).
* [x] **Localhost Single Instance**:
  - Ensure a single `Instance` manages entity positions, input application, and spatial ticks on `localhost`.
  - Multi-instance / room management is deferred to future phases.

---

### Phase 2: Lightweight Client/Session Mapping (Done)
> **Spec:** [Phase 2 Spec](roadmap-specs/phase2-client-session-mapping-spec.md) · **Reference ADRs:** [ADR-0001](adr/en/0001-authoritative-server-archoitecture.md) · [ADR-0002](adr/en/0002-instance-based-architecture.md) · [ADR-0005](adr/en/0005-cross-language-binary-serialization.md) · [ADR-0006](adr/en/0006-two-thread-network-gameloop-separation.md) · [ADR-0008](adr/en/0008-session-lifecycle-and-client-identity.md) · [ADR-0009](adr/en/0009-simplified-authentication-and-auto-join-strategy.md)

* [x] **SocketAddr to Entity Mapping**:
  - Map incoming client IP / `SocketAddr` directly to a unique `EntityID` inside the active instance.
  - Handle basic client join / disconnect / timeout detection.
* [x] **Simplified Handshake**:
  - Keep authentication minimal (no token/auth servers for now, optimized for rapid prototype validation).

---

### Phase 3: World State Broadcasting & Client Sync (Done)
> **Spec:** [Phase 3 Spec](roadmap-specs/phase3-world-state-broadcasting-spec.md) · **Reference ADRs:** [ADR-0001](adr/en/0001-authoritative-server-archoitecture.md) · [ADR-0002](adr/en/0002-instance-based-architecture.md) · [ADR-0003](adr/en/0003-2d-map-only.md) · [ADR-0005](adr/en/0005-cross-language-binary-serialization.md) · [ADR-0006](adr/en/0006-two-thread-network-gameloop-separation.md) · [ADR-0008](adr/en/0008-session-lifecycle-and-client-identity.md) · [ADR-0009](adr/en/0009-simplified-authentication-and-auto-join-strategy.md)

* [x] **Snapshot Generation**:
  - Create a `WorldState` snapshot protobuf packet representing all active entity positions and states.
* [x] **Tick Broadcast**:
  - Send state snapshots to all mapped client addresses at regular tick intervals.
* [x] **Client Integration**:
  - Validate snapshot consumption in Love2D, Godot, Python, and Rust CLI examples.

---

### Phase 4: Event Logging & Deterministic Replay System
> **Spec:** [Phase 4 Spec](roadmap-specs/phase4-deterministic-replay-spec.md) · **Reference ADRs:** [ADR-0001](adr/en/0001-authoritative-server-archoitecture.md) · [ADR-0002](adr/en/0002-instance-based-architecture.md) · [ADR-0003](adr/en/0003-2d-map-only.md) · [ADR-0005](adr/en/0005-cross-language-binary-serialization.md) · [ADR-0006](adr/en/0006-two-thread-network-gameloop-separation.md) · [ADR-0007](adr/en/0007-deterministic-simulation-and-fixed-point.md) · [ADR-0008](adr/en/0008-session-lifecycle-and-client-identity.md) · [ADR-0009](adr/en/0009-simplified-authentication-and-auto-join-strategy.md) · [ADR-0010](adr/en/0010-event-sourced-replay-format.md) · [ADR-0011](adr/en/0011-authoritative-spectator-replay-broadcasting.md)

* [x] **Deterministic Engine Core & Fixed-Point Refactoring**:
  - Replace `f32` physics simulation with fixed-point math (`I16F16`) to guarantee cross-CPU/OS arithmetic determinism.
  - Convert entity/session storage to strictly ordered collections (`BTreeMap`).
  - Upgrade game loop to a sub-millisecond Fixed-Timestep Accumulator.
* [x] **Event Sourcing Architecture & Replay Serialization**:
  - Log all incoming client intents sequentially at fixed tick boundaries into `.loci` Protobuf replay files.
  - Implement periodic canonical `WorldState` SHA-256 checkpoints for desync detection.
* [x] **Headless Replay & Desync Verification Tooling**:
  - Add CLI execution mode to replay match files offline as fast as possible, asserting bit-exact checksum matches across Linux, macOS ARM64, and Windows.
* [x] **Live Spectator Broadcast & Multi-Client Playback**:
  - Stream replay snapshots over UDP at real-time tick rates (with speed controls: 0.5x, 1x, 2x, 4x) to existing client engines (Godot, Love2D, Python, CLI) with zero client modifications.

---

### Phase 5: Deterministic Physics & Collision Engine
> **Spec:** [Phase 5 Spec](roadmap-specs/phase5-physics-and-collision-spec.md) · **Reference ADRs:** [ADR-0001](adr/en/0001-authoritative-server-archoitecture.md) · [ADR-0002](adr/en/0002-instance-based-architecture.md) · [ADR-0003](adr/en/0003-2d-map-only.md) · [ADR-0005](adr/en/0005-cross-language-binary-serialization.md) · [ADR-0006](adr/en/0006-two-thread-network-gameloop-separation.md) · [ADR-0007](adr/en/0007-deterministic-simulation-and-fixed-point.md) · [ADR-0008](adr/en/0008-session-lifecycle-and-client-identity.md) · [ADR-0010](adr/en/0010-event-sourced-replay-format.md) · [ADR-0011](adr/en/0011-authoritative-spectator-replay-broadcasting.md) · [ADR-0012](adr/en/0012-deterministic-2d-collision-and-kinematic-resolution.md) · [ADR-0013](adr/en/0013-server-authoritative-destination-steering-and-navigation.md)

* [x] **Deterministic Collision Detection**:
  - Implement fixed-point AABB (Axis-Aligned Bounding Box) and Circle colliders using `I16F16`.
* [x] **Static Map Geometry & Boundaries**:
  - Define world boundaries and static colliders (walls, obstacles) loaded from map files/configuration.
* [x] **Collision Resolution & Trigger Zones**:
  - Implement solid obstacle pushback and trigger/sensor zones (`on_overlap_enter` / `on_overlap_exit`).
* [x] **Click-to-Move Steering & Navigation**:
  - Implement destination-based steering (`MoveToPositionIntent`) with basic obstacle navigation for RTS/MOBA controls.
* [-] **[Deferred to Phase 8] Spatial Partitioning (Optional Optimization)**:
  - Add a fixed spatial hash grid for efficient broadphase collision queries.

---

### Phase 6: Embedded Scripting & Game Logic (Lua Engine)

* [x] **Lua VM Integration (`mlua`)**:
  - Embed a sandboxed, deterministic Lua runtime into the game instance.
* [x] **Rust-to-Lua Engine API**:
  - Expose entity manipulation, fixed-point vectors, intent hooks, and collision/trigger callbacks to Lua.
* [x] **Event-Driven Gameplay Callbacks**:
  - Implement lifecycle hooks: `on_init`, `on_tick`, `on_player_join`, `on_player_leave`, `on_collision`.
* [-] **[Deferred to Phase 9] Hot-Reloadable Game Rules**:
  - Support reloading script files at runtime without restarting the server binary.
* [x] **Deterministic Script Execution**:
  - Ensure Lua callbacks execute strictly at deterministic tick boundaries to preserve `.loci` replay parity.

---

### Phase 6.5: Validation, Developer Experience (DX) & API Stabilization
> **Milestone Focus:** Feature freeze on new engine features to focus on v0.6.x stability, DX, client abstraction, and user-friendly documentation before moving to multi-room infrastructure.

> [!NOTE]
> **Strategic Pause (v0.6.x Consolidation):** Once Phase 6 is reached, `loci2d` achieves a fully playable, deterministic LAN multiplayer stack. New feature development will be temporarily delayed to validate the engine, gather playtester feedback, and refine the public API.
> 
> Due to the heterogeneous nature of this milestone (spanning client SDKs, documentation, and engine architecture), the detailed roadmap and sub-specifications for Phase 6.5 have been moved to a dedicated document.
> 
> **[View the full Phase 6.5 Roadmap here](phase6.5-roadmap.md)** (See [ADR 0015](adr/en/0015-dedicated-roadmap-structure-phase6.5-validation-dx.md) for details).

---

### Phase 7: Multi-Instance & Room Management

* [ ] **Dynamic Room Lifecycle**:
  - Allocate, configure (map, scripts, tick rate, max players), and teardown game rooms dynamically.
* [ ] **Packet Demultiplexing & Room Routing**:
  - Route incoming client UDP packets to target instances via Room ID headers or port allocation.
* [ ] **Concurrent Room Execution**:
  - Execute multiple room loops in parallel across a threadpool / async worker tasks with memory isolation.
* [ ] **Per-Room Replay Logging**:
  - Isolate `.loci` match replay recordings per room instance.
* [ ] **Server Room Administration & Metrics**:
  - CLI commands and metrics to monitor active rooms, player counts, and tick health.

---

### Phase 8: Production Hardening, Security & Authentication

* [ ] **Session Authentication & Tokens**:
  - Implement secure handshake validation, session tokens, and optional external auth webhooks.
* [ ] **Intent Validation & Anti-Tamper**:
  - Enforce server-side intent rate-limiting, packet sequence integrity, and velocity/teleportation sanity checks.
* [ ] **Lua Sandbox Security Hardening**:
  - Implement OOM protection (memory limits per instance), disable dangerous base globals (`dofile`, `load`, `getmetatable`), and establish CI penetration tests to prevent Sandbox escapes.
* [ ] **Production Observability & Deployment**:
  - Structured logging, Prometheus metrics, and production containerization.

---

### Phase 9: Script Versioning & Hot-Reloading

* [ ] **Cross-Version Replay Support**:
  - Bundle or version Lua scripts to ensure clients can watch old `.loci` replays with the exact logic used during that match.
* [ ] **Immutable Live Instances**:
  - Guarantee that hot-reloading scripts on the server only affects new instances; running matches must finish on their original script versions to maintain determinism.
