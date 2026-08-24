# Implementation Spec — Phase 6.5.0-3: Gameplay Actions & Physics
> **Status:** Ready to review
> **Roadmap Phase:** Phase 6.5.0-3
> **Reference ADRs:** [ADR-0007](../adr/en/0007-deterministic-simulation-and-fixed-point.md) · [ADR-0014](../adr/en/0014-embedded-lua-scripting-and-command-buffer.md)

---

## 1. Context & Technical Motivation

This milestone connects player inputs and physical mechanics to the Lua rules engine. It addresses critical gaps for Battle Arena gameplay: handling directional abilities, spawning new entities dynamically (e.g., projectiles), and mutating movement physics on the fly.

---

## 2. Technical Implementation Modules

### 2.1 Action Intent Dispatching

When a client wants to cast an ability, they send an `ActionIntent` over the network. Most game abilities are **directional** (e.g., "fire projectile toward cursor"), so the intent must carry a target direction.

#### Protobuf Update
```protobuf
message ActionIntent {
  uint32 ability_id = 1;
  Vector2 target_direction = 2;  // Normalized aim direction from client input
}
```

* **Rust Routing:** The Rust `IntentHandler` receives the packet, validates the player is connected, and routes it directly to Lua via `ScriptEngine::on_action(entity_id, ability_id, dir_x, dir_y)`.
* **Agnostic Core:** Rust does *not* know what `ability_id == 1` means. It merely passes the intent and direction to the Lua ruleset. If the client omits `target_direction`, Rust passes `(0, 0)` to Lua.

---

### 2.2 Dynamic Entity Spawning (Resolve Stub)

The `Command::SpawnEntity` variant currently only logs to the console. It must be actualized to support projectiles and dynamic map elements.

* **Implementation:** When `Command::SpawnEntity { blueprint, position }` is applied:
  1. Generate a deterministic `entity_id` (e.g., incrementing an `entity_id_counter` on the `Instance`).
  2. Create a new `Entity` instance.
  3. Insert it into `instance.entities`.
  4. Optionally trigger `on_entity_spawned(entity_id, blueprint)` in Lua.
* **Lua API:** `Loci.Commands.spawn_entity({ blueprint = "name", x = 0, y = 0 })`.

---

### 2.3 Entity Physics Configuration

Currently, `NavigationComponent` parameters (`move_speed`, `arrival_tolerance`) are hardcoded at entity creation time (see [Issue #4](https://github.com/lamfsantos/loci2d/issues/4)). For a Battle Arena, Lua must be able to configure these values at runtime.

#### Command Buffer Integration
```rust
pub enum Command {
    // ... existing commands ...
    SetMoveSpeed {
        entity_id: u64,
        speed: I16F16,
    },
}
```

* **Lua API:** `Loci.Commands.set_move_speed(entity_id, speed)` where `speed` is a number (e.g., `2.5`). The Rust side converts to `I16F16` for deterministic physics.
* **Application:** When `flush_and_apply` processes this command, it updates `entity.navigation.move_speed`. If the entity has no `NavigationComponent`, the command is a no-op.

---

## 3. Testing & Verification

1. **Action Direction Passthrough:** Verify that `on_action(entity_id, ability_id, dir_x, dir_y)` receives correct fixed-point-converted direction values from the client's `ActionIntent.target_direction`.
2. **Move Speed Mutation:** Verify that `Command::SetMoveSpeed` correctly updates `NavigationComponent.move_speed` and that the entity's velocity magnitude changes on the next tick.
3. **Spawn Entity:** Verify that calling `Loci.Commands.spawn_entity` increases the entity count in the `BTreeMap` and correctly syncs to clients.
