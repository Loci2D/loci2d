# Implementation Spec — Phase 6.5.0: Entity Data Model & Game Rules API

> **Status:** Draft  
> **Roadmap Phase:** Phase 6.5.0  
> **Reference ADRs:** [ADR-0014](../adr/en/0014-embedded-lua-scripting-and-command-buffer.md) · [ADR-0015](../adr/pt/0015-estrutura-roadmap-dedicado-fase6.5-validacao-dx.md) · [ADR-0016](../adr/en/0016-data-driven-entity-properties-and-engine-agnosticism.md)

---

## 1. Context & Technical Motivation

In Phase 6, we implemented a sandboxed, deterministic embedded Lua engine. However, the Lua scripts currently lack the APIs required to build an actual game (e.g., maintaining health, assigning teams, managing match state, or spawning entities). 

Phase 6.5.0 aims to transform this foundational scripting engine into a complete "Game Rules API". To keep the Rust core strictly agnostic to game genres (as mandated by **ADR-0016**), all game logic state will be pushed to a dynamic, data-driven Property System. 

### Core Goals of Phase 6.5.0
1. **Data-Driven Entities:** Equip Rust `Entity` structs with a generic key-value store to hold Lua-defined attributes (health, team, status effects).
2. **Deterministic Mutation:** Ensure all property changes and entity spawns route through the `CommandBuffer` to maintain event-sourced replay parity.
3. **Lifecycle Control:** Allow Lua to dictate when a match starts/pauses, replacing hardcoded Rust initialization.
4. **Time & Triggers:** Provide deterministic timers/cooldowns explicitly for game logic orchestration.

---

## 2. Technical Implementation Modules

### 2.1 Entity Property System (Rust Core)
* **Storage:** Add `properties: HashMap<String, String>` to the `Entity` struct. 
* **Serialization:** Update the `WorldState` Protobuf definition to include the `properties` map, ensuring clients receive visual metadata (e.g., team colors).
* **Command Buffer Integration:** Introduce `Command::SetEntityProperty { entity_id, key, value }`.
* **Constraint:** No hardcoded game rules (`pub health: i32`) are allowed in Rust.

### 2.2 Lua API: Properties & Globals
* **Getters:** Expose `Loci.get_entity_property(entity_id, key) -> string` (synchronous Rust-to-Lua read).
* **Setters:** Expose `Loci.Commands.set_property(entity_id, key, value)` (asynchronous, pushes to CommandBuffer).
* **Match Metadata:** Expose APIs to read/write global match variables (e.g., `Loci.Commands.set_global("score_red", "1")`).

### 2.3 Match Lifecycle State Machine
* **Server State:** Add an enum `MatchState { Paused, Running }` to the `Instance`. Ticks only advance simulation and call `on_tick` when `Running`.
* **Lua Control:** Expose `Loci.Commands.start_match()` and `Loci.Commands.pause_match()`. 
* **Use Case:** Lua script waits until 6 players connect in `on_player_join` before calling `start_match()`.

### 2.4 Timer & Cooldown System
* **Rust Tracking:** Add a collection in `Instance` to track active timers: `struct ActiveTimer { id: String, remaining_ticks: u32 }`.
* **Lua API:** `Loci.Commands.start_timer(timer_id: String, delay_ticks: u32)`.
* **Callback Execution:** When `remaining_ticks` reaches 0, the `ScriptEngine` invokes a new Lua global function `on_timer_complete(timer_id)`. 

### 2.5 Action Intent Dispatching
* **Client Packets:** The client sends an `ActionIntent { ability_id: u8, target_dir: Vector2 }`.
* **Routing:** Instead of Rust interpreting the intent, the engine routes it directly to Lua via `on_action(entity_id, ability_id, x, y)`.
* **Game Logic:** Lua resolves the ability (e.g., "if ability_id == 1, spawn fireball").

### 2.6 Dynamic Entity Spawning (Resolve Stub)
* **Implementation:** The current `Command::SpawnEntity` is a logging stub. It must be updated to parse the blueprint, create an `Entity`, assign a deterministic `entity_id`, and insert it into `instance.entities`.
* **Lua API:** Ensure `Loci.Commands.spawn_entity(blueprint, x, y)` correctly populates this command.

---

## 3. Testing & Verification

1. **Unit Tests:** Verify that `set_property` commands are correctly applied to the `Entity` state after `flush_and_apply`.
2. **Determinism Verification:** Ensure `ActiveTimer` decrements and triggers `on_timer_complete` perfectly in sync with the tick rate.
3. **Protobuf Sync:** Inspect the byte output of `WorldState` to ensure `HashMap` serialization is functioning as expected.
