# Phase 6.5 Roadmap: Validation, DX & API Stabilization

> **Reference ADRs:** [ADR-0015](adr/en/0015-dedicated-roadmap-structure-phase6.5-validation-dx.md)
>
> 🌐 *Read this in [Português (Brasil)](phase6.5-roadmap.pt-BR.md)*

This document details the sub-milestones for Phase 6.5 of the **loci2d** project. Phase 6.5 acts as a "strategic pause" (v0.6.x) to consolidate the engine's core features into a playable, developer-friendly multiplayer ecosystem before introducing multi-room infrastructure.

---

## 6.5.0-1: Entity Data Model & Global Properties
> **Reference ADRs:** [ADR-0014](adr/en/0014-embedded-lua-scripting-and-command-buffer.md) · [ADR-0016](adr/en/0016-data-driven-entity-properties-and-engine-agnosticism.md)
> **Spec:** [phase6.5.0-1-data-model-spec.md](roadmap-specs/phase6.5.0-1-data-model-spec.md)

* [x] **Entity Property System (Data-Driven Design):**
  - Add a flexible key-value property map to the `Entity` struct in Rust.
* [x] **Lua Property Interface:**
  - Expose `Loci.get_entity_property` and `Loci.Commands.set_property` to Lua scripts.
* [x] **Global Match Metadata:**
  - Allow Lua to manage global match variables (e.g., match score, player count) via the generic property system (`globals`).

---

## 6.5.0-2: Match Lifecycle & Timers
> **Spec:** [phase6.5.0-2-match-lifecycle-spec.md](roadmap-specs/phase6.5.0-2-match-lifecycle-spec.md)

* [x] **Match Lifecycle State Machine:**
  - Introduce a `Paused`/`Running`/`Ended` state machine to the server instance.
  - Expose `Loci.Commands.start_match()`, `Loci.Commands.pause_match()`, and `Loci.Commands.end_match()` to Lua scripts.
* [x] **Timer & Cooldown System:**
  - Expose `Loci.Commands.start_timer(timer_id, delay_ticks)` and route expiration to a new Lua callback `on_timer_complete(timer_id)`.

---

## 6.5.0-3: Gameplay Actions & Physics Configuration
> **Spec:** [phase6.5.0-3-gameplay-actions-spec.md](roadmap-specs/phase6.5.0-3-gameplay-actions-spec.md)

* [x] **Action/Ability Dispatch:**
  - Expand `ActionIntent` with a `target_direction` field for directional abilities.
  - Route the intent to a new Lua callback `on_action(entity_id, ability_id, dir_x, dir_y)`.
* [x] **Dynamic Entity Spawning:**
  - Resolve the `SpawnEntity` CommandBuffer stub so scripts can dynamically spawn projectiles.
* [x] **Entity Physics Configuration:**
  - Expose `Loci.Commands.set_move_speed(entity_id, speed)` to allow Lua scripts to configure per-entity movement speed at runtime.

---

## 6.5.1: Architecture, Safety & Determinism Validation

* [x] **Strict State vs. Scripting Separation:**
  - Decouple canonical match state (pure deterministic physics, spatial data) from Lua script execution.
* [x] **Safe Dispatch Layer:**
  - Implement a safe command/intent dispatch layer for Lua scripts to prevent unauthorized state corruption and ensure deterministic replay integrity.
* [x] **Determinism CI Suite & Benchmarks:**
  - Establish a rigorous cross-platform continuous integration (CI) test suite to mathematically prove `I16F16` fixed-point determinism across ARM64 and x86_64 architectures.
  - Update `benchmark.rs` and core tests to stress-test Lua Gameplay Actions and ensure they maintain determinism.

---

## 6.5.2: API Refinement & SDK Preparation

* [x] **Strict Intent Interception (Safe Layer Fix):**
  - Refactor `intent_handler.rs` so `Intent::Move` and `Intent::MoveToPos` do not directly mutate Entity velocity and navigation.
  - Expose new Lua hooks (`on_move_intent(entity_id, dir_x, dir_y)`) allowing scripts to validate and apply movement via `Loci.Commands`.
* [x] **Lua API Consistency & Getters:**
  - Standardize Vector2 arguments across the Lua API (e.g., `on_action`, `set_position`) so they accept consistent formats.
  - Implement basic Getters (`get_velocity`, `get_move_speed`, `get_entity_name`) to prevent state duplication in Lua.
* [x] **Blueprint Extensibility:**
  - Enhance `SpawnEntity` to accept configuration parameters (or integrate a registry) rather than hardcoding EntityType and Components.
* [x] **API Ergonomics for Skills (MOBA Support):**
  - Add `Loci.get_entities_in_radius` for Spatial Queries (AoE skills).
  - Expose fixed-point math methods (`distance_to`, `normalize`, `length`) to `DeterministicVector2` to prevent floating-point determinism loss.

---

## 6.5.3: Client SDKs & Wrappers
> **Specs:** [phase6.5.3-love2d-sdk-spec.md](roadmap-specs/phase6.5.3-love2d-sdk-spec.md) · [phase6.5.3-1-client-sdk-ergonomics-and-events-spec.md](roadmap-specs/phase6.5.3-1-client-sdk-ergonomics-and-events-spec.md)

* [x] **Love2D SDK Spec:** Write `phase6.5.3-love2d-sdk-spec.md` detailing the client architecture, ensuring all netcode is abstracted away from the devs.
* [x] **Love2D Wrapper (`loci_client.lua`):** (PRIORITY)
  - Create a lightweight Lua module for Love2D developers to connect to the engine effortlessly, abstracting UDP sockets and Protobuf completely.
* [x] **Phase 6.5.3-1: High-Level Entity Abstraction & Property Auto-Casting:**
  - Create in-memory `Entity` objects with metatables to automatically cast property strings to numbers/booleans and provide direct field access (`entity.hp`).
* [x] **Phase 6.5.3-1: Action & Match State Broadcasts:**
  - Extend Protobuf with transient `ActionBroadcast` and `MatchLifecycleState`, updating the server and SDK to fire `on_action_cast` and `on_match_state_changed`.

---

## 6.5.4: Reference Examples & Templates

* [ ] **3v3 Arena MVP (Love2D):** Build a fully functional 3v3 Arena game using the Love2D SDK to validate the engine.
* [ ] **Standard Lua Game Template:** Create a boilerplate `main.lua` file with heavily documented callbacks (`on_init`, `on_player_join`, etc.) establishing the official structure for students to build game rules.
* [ ] **Refactor CLI Example:** Update the headless rust client example to reflect recent architectural changes.

---

## 6.5.5: User-Friendly Documentation

* [ ] **Quickstart Tutorial:**
  - Author a "15-Minute First Multiplayer Game" tutorial (e.g., a simple 2D Arena or Tag game).
* [ ] **Lua API Reference:**
  - Create a comprehensive, copy-paste-ready Lua scripting API reference guide with clear use cases for every engine callback.

---

## 6.5.6: LAN Playtesting & Empirical Feedback

* [ ] **Multi-Device Sessions:**
  - Run physical LAN playtest sessions with high school and university students to identify UX/DX friction points and API ergonomics issues.
* [ ] **Network Stability Metrics:**
  - Measure tick stability, desync resilience, and packet latency under real local network conditions.

---

## Future Work (Out of Scope for Phase 6.5 MVP)

* [ ] **Godot 4 Wrapper (`LociClient.gd`):** Build an ergonomic GDScript client module to eliminate boilerplate UDP socket/Protobuf parsing for end users.
* [ ] **Python SDK:** Expose a Python SDK for AI training, data analysis, or rapid prototyping.
* [ ] **Refactor Godot Example:** Rebuild the Godot demo using the new `LociClient.gd` SDK.
* [ ] **Refactor Python Example:** Showcase basic interactions using the new Python SDK.
