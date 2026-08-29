# Implementation Spec — Phase 6.5.0-1: Entity Data Model
> **Status:** Completed  
> **Roadmap Phase:** Phase 6.5.0-1
> **Reference ADRs:** [ADR-0007](../adr/en/0007-deterministic-simulation-and-fixed-point.md) · [ADR-0014](../adr/en/0014-embedded-lua-scripting-and-command-buffer.md) · [ADR-0015](../adr/en/0015-dedicated-roadmap-structure-phase6.5-validation-dx.md) · [ADR-0016](../adr/en/0016-data-driven-entity-properties-and-engine-agnosticism.md)

---

## 1. Context & Technical Motivation

In Phase 6, we implemented a sandboxed, deterministic embedded Lua engine. However, the Lua scripts currently lack the APIs required to build an actual game. To keep the Rust core strictly agnostic to game genres (as mandated by **ADR-0016**), all game logic state will be pushed to a dynamic, data-driven Property System. This milestone establishes the foundational data model.

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
    pub id: u64,
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
        entity_id: u64,
        key: String,
        value: String,
    },
}
```

* **Legacy Cleanup:** The existing `Command::ApplyDamage` must be removed as it violates ADR-0016. All damage and health management now belong entirely to Lua via property mutations.

#### Protobuf Serialization
To ensure the `WorldState` Protobuf snapshot remains bit-exact across architectures, the `BTreeMap` should be serialized as a repeated message of key-value pairs.

```protobuf
message Property {
    string key = 1;
    string value = 2;
}

message EntityState {
    uint64 id = 1;
    // ... physics fields ...
    repeated Property properties = 6; // Field 5 is reserved for entity_type
}
```

* **Snapshot Update:** `Instance::create_snapshot()` must be updated to populate the new `properties` field on each `EntityState` from the entity's `BTreeMap`.

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
* **Player Count:** There will be NO dedicated API for player count (e.g., `Loci.get_player_count()`). Scripts must manage this manually by incrementing/decrementing a global variable in the `on_player_join` and `on_player_leave` callbacks.

---

## 3. Testing & Verification

1. **Unit Tests:** Verify that `Command::SetEntityProperty` correctly mutates the `BTreeMap` after `flush_and_apply`.
2. **Protobuf Sync:** Inspect the byte output of `WorldState` to ensure `BTreeMap` properties are serialized correctly via the `repeated Property` message pattern.
3. **Hash Stability (Critical):** 
   * **Test:** Create an Entity. Insert `{"health": "100", "team": "red"}`. Snapshot the `WorldState` hash.
   * **Test:** Create an Entity. Insert `{"team": "red", "health": "100"}`. Snapshot the `WorldState` hash.
   * **Assert:** Both hashes MUST be identical. This proves ADR-0007 compliance and prevents cross-architecture desyncs.
