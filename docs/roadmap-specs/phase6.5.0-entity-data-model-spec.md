# Implementation Spec — Phase 6.5.0: Entity Data Model & Game Rules API

> **Status:** Draft  
> **Roadmap Phase:** Phase 6.5.0  
> **Reference ADRs:** [ADR-0014](../adr/en/0014-embedded-lua-scripting-and-command-buffer.md) · [ADR-0015](../adr/pt/0015-estrutura-roadmap-dedicado-fase6.5-validacao-dx.md) · [ADR-0016](../adr/en/0016-data-driven-entity-properties-and-engine-agnosticism.md)

---

## 1. Context & Technical Motivation

In Phase 6, we implemented a sandboxed, deterministic embedded Lua engine. However, the Lua scripts currently lack the APIs required to build an actual game (e.g., maintaining health, assigning teams, managing match state, or spawning entities). 

Phase 6.5.0 aims to transform this foundational scripting engine into a complete "Game Rules API". To keep the Rust core strictly agnostic to game genres (as mandated by **ADR-0016**), all game logic state will be pushed to a dynamic, data-driven Property System. 

> [!CAUTION]
> **Architecture Rule (ADR-0007):** Every new collection introduced in this phase MUST use `BTreeMap` or `BTreeSet` to guarantee deterministic iteration order and stable hashes across different process executions. `HashMap` and `HashSet` are strictly forbidden for game state storage.

---

## 2. Technical Implementation Modules

### 2.1 Entity Property System (Rust Core)

To support arbitrary data without hardcoding rules, entities will store a deterministic property map.

#### Data Structure
```rust
use std::collections::BTreeMap;

pub struct Entity {
    pub id: u32,
    pub position: DeterministicVector2,
    pub velocity: DeterministicVector2,
    // ... other physics/spatial fields ...
    
    /// Game logic properties (e.g., "health": "100", "team": "red")
    /// MUST be BTreeMap to guarantee deterministic serialization order (ADR-0007).
    pub properties: BTreeMap<String, String>,
}
```

#### Command Buffer Integration
To allow Lua to mutate properties safely, we add a new command variant:
```rust
pub enum Command {
    // ... existing commands ...
    SetEntityProperty {
        entity_id: u32,
        key: String,
        value: String,
    },
}
```

#### Protobuf Serialization
To ensure the `WorldState` Protobuf snapshot remains bit-exact across architectures, the `BTreeMap` should be serialized as a repeated message of key-value pairs (which naturally maintains the `BTreeMap`'s sorted order), rather than a Protobuf `map` (which depending on the Rust Protobuf crate, might deserialize into a non-deterministic `HashMap`).

```protobuf
message Property {
    string key = 1;
    string value = 2;
}

message EntityState {
    uint32 id = 1;
    // ... physics fields ...
    repeated Property properties = 5;
}
```

---

### 2.2 Lua API: Properties & Globals

The `ScriptEngine` must bind Rust functions to allow Lua scripts to read and enqueue mutations to the property system.

#### Entity Properties
* **Getter (Synchronous):** `Loci.get_entity_property(entity_id, key)` reads directly from the `Instance`'s entity map.
* **Setter (Asynchronous):** `Loci.Commands.set_property(entity_id, key, value)` pushes a `Command::SetEntityProperty` to the buffer.

```lua
-- Example usage in Lua
local health = tonumber(Loci.get_entity_property(1, "health"))
if health < 50 then
    Loci.Commands.set_property(1, "status", "critical")
end
```

#### Global Match Variables & Player Count
Some properties belong to the match itself (e.g., round number, red team score).
* **Storage:** Add `pub globals: BTreeMap<String, String>` to the `Instance` struct.
* **Getters/Setters:** Expose `Loci.get_global(key)` and `Loci.Commands.set_global(key, value)`.
* **Player Count:** There will be NO dedicated API for player count (e.g., `Loci.get_player_count()`). Scripts must manage this manually by incrementing/decrementing a global variable in the `on_player_join` and `on_player_leave` callbacks. This keeps the engine's API surface minimal.

---

### 2.3 Match Lifecycle State Machine

Currently, the server `Instance` ticks unconditionally. We need to introduce a lifecycle so Lua can pause the simulation (e.g., waiting for players to connect in a lobby) and permanently end the match when a win condition is met.

```rust
pub enum MatchState {
    Paused,
    Running,
    Ended { winner_data: String },
}

pub struct Instance {
    // ...
    pub state: MatchState,
}
```

* **Logic Update:** In `Instance::tick()`, skip physics updates and `on_tick` Lua callbacks if `state` is not `Running`. If `Ended`, the server should also stop accepting client `ActionIntent` packets and finalize the replay file.
* **Lua Control:** Expose `Loci.Commands.start_match()`, `Loci.Commands.pause_match()`, and `Loci.Commands.end_match(winner_data: string)`.
* **Use Case:** Lua script waits until 6 players connect in `on_player_join` before calling `start_match()`. When a team's base is destroyed, it calls `end_match("red_team_won")`.

---

### 2.4 Timer & Cooldown System

Timers are essential for game logic (e.g., "apply poison damage over 5 seconds", "ability cooldown"). These must be resolved deterministically on the server tick.

#### Storage (ADR-0007 Compliant)
```rust
pub struct ActiveTimer {
    pub timer_id: String,
    pub remaining_ticks: u32,
}

pub struct Instance {
    // ...
    /// Key: timer_id. MUST be BTreeMap to guarantee stable execution order if multiple 
    /// timers expire on the exact same tick.
    pub active_timers: BTreeMap<String, ActiveTimer>, 
}
```

#### Execution Flow
1. **Lua requests timer:** `Loci.Commands.start_timer("spawn_wave_1", 300)` (300 ticks).
2. **Command Buffer:** Applies `Command::StartTimer` to the `Instance`, inserting into `active_timers`.
3. **Tick Logic:** `Instance::tick()` decrements `remaining_ticks` for all timers.
4. **Trigger:** If a timer reaches `0`, it is removed from the `BTreeMap` and its ID is passed to `ScriptEngine::on_timer_complete(timer_id)`.

---

### 2.5 Action Intent Dispatching

When a client wants to cast an ability or perform a game-specific action, they send an `ActionIntent` over the network.

* **Client Packet:** `ActionIntent { ability_id: u8, target_dir: Vector2 }`
* **Rust Routing:** The Rust `IntentHandler` receives the packet, validates the player is connected, and routes it directly to Lua via `ScriptEngine::on_action(entity_id, ability_id, target_dir.x, target_dir.y)`.
* **Agnostic Core:** Rust does *not* know what `ability_id == 1` means. It merely passes the intent to the Lua ruleset.

---

### 2.6 Dynamic Entity Spawning (Resolve Stub)

The `Command::SpawnEntity` variant currently only logs to the console. It must be actualized to support projectiles and dynamic map elements.

* **Implementation:** When `Command::SpawnEntity { blueprint, position }` is applied:
  1. Generate a deterministic `entity_id` (e.g., incrementing an `entity_id_counter` on the `Instance`).
  2. Create a new `Entity` instance.
  3. Insert it into `instance.entities`.
  4. Optionally trigger `on_entity_spawned(entity_id, blueprint)` in Lua.
* **Lua API:** `Loci.Commands.spawn_entity(blueprint: string, x: number, y: number)`.

---

## 3. Testing & Verification

1. **Unit Tests:** Verify that `Command::SetEntityProperty` correctly mutates the `BTreeMap` after `flush_and_apply`.
2. **Determinism Verification:** Ensure `ActiveTimer` decrements precisely on `Instance::tick()` and that multiple timers expiring on the same tick trigger `on_timer_complete` in alphabetical order (guaranteed by `BTreeMap`).
3. **Protobuf Sync:** Inspect the byte output of `WorldState` to ensure `BTreeMap` properties are serialized correctly via the `repeated Property` message pattern.
4. **Hash Stability (Critical):** 
   * **Test:** Create an Entity. Insert `{"health": "100", "team": "red"}`. Snapshot the `WorldState` hash.
   * **Test:** Create an Entity. Insert `{"team": "red", "health": "100"}`. Snapshot the `WorldState` hash.
   * **Assert:** Both hashes MUST be identical. This proves ADR-0007 compliance and prevents cross-architecture desyncs.
