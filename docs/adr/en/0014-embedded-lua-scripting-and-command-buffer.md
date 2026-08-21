# ADR 0014: Embedded Lua Scripting and Deterministic Command Buffer

## Status

Accepted (Phase 6)

## Context

To enable rapid prototyping, modding, and game logic customization without recompiling the Rust server, `loci2d` needs an embedded scripting language. 

The language must be lightweight, easy for indie developers to learn, and capable of running in a sandboxed, deterministic environment to preserve event-sourced replays ([ADR-0010](0010-event-sourced-replay-format.md)) and deterministic simulation ([ADR-0007](0007-deterministic-simulation-and-fixed-point.md)).

Furthermore, allowing user-provided scripts to mutate the canonical game state (entities, physics tree) directly mid-tick poses significant stability risks. For example, if a Lua script destroys an entity during an `on_collision` callback while the Rust physics engine is actively iterating over the collision tree, it causes iterator invalidation (crashing the engine) or mid-tick state corruption. For the engine to support complex, high-interaction games like MOBAs, the architecture must be indestructible against user-script errors.

## Decision

We adopt **Lua (via `mlua`) with Strict Sandboxing and a Deferred Command Buffer Architecture**.

### 1. Choice of Lua
Lua is the industry standard for game scripting. Its VM is extremely lightweight, its C-FFI compatibility (`mlua`) makes it fast, and it has a very low barrier to entry. This aligns perfectly with the project's goal of abstracting netcode complexity for indie developers and students. Alternatives like WASM or Rhai were discarded due to higher complexity or lack of widespread industry adoption.

### 2. Deterministic Sandboxing
To prevent server exploits and guarantee that replays remain 100% deterministic across different operating systems:
- Dangerous standard libraries (`io`, `os`, `package`, `debug`) are strictly disabled.
- Non-deterministic iteration (`pairs()`) is removed from the global environment, forcing the use of deterministic arrays (`ipairs()`).
- The default `math.random` function is overridden with a deterministic PRNG seeded by the `Instance` seed.
- A strict instruction limit hook (DoS protection) is enforced per callback. If a script enters an infinite loop, the Instance gracefully aborts and disconnects clients without crashing the main server binary.

### 3. The Command Buffer Pattern (Deferred Execution)
To prevent iterator invalidation and ensure robust state decoupling:
- Lua scripts **cannot** mutate the canonical game state directly during a tick (e.g., inside `on_collision` or `on_tick`).
- Instead, the Rust-to-Lua API is backed by a `CommandBuffer`. Functions like `spawn_entity`, `destroy_entity`, or `set_position` push intents into this buffer.
- Rust flushes and applies the Command Buffer sequentially at the end of the tick, ensuring absolute safety for internal engine iterators.
- **DX (Developer Experience) Mitigation:** To prevent the Command Buffer from being frustrating for indie developers, entity creation commands pre-allocate and return an `EntityID` immediately. This allows the Lua script to queue subsequent commands referencing the new entity within the same tick, hiding the deferred complexity behind a synchronous-feeling API.

## Consequences

**Positive:**
- **Low Barrier to Entry:** Game logic can be written rapidly in Lua without Rust knowledge.
- **Rock-Solid Stability:** The Command Buffer protects the engine from mid-tick state corruption and iterator invalidation, scaling safely to MOBA-level complexity.
- **Determinism Maintained:** Sandboxing and PRNG overrides preserve the event-sourced replay architecture.
- **Server Security:** Disabling `io`/`os` and enforcing instruction limits prevents one malicious/buggy room from taking down the entire server.

**Negative:**
- **Deferred Complexity:** Deferred execution introduces slight conceptual complexity for developers. The Rust-to-Lua API must be carefully designed to abstract this (e.g., pre-allocating IDs) to mitigate DX friction.
- **No Direct File Access:** Game developers cannot save/load external files directly from Lua, requiring the Rust engine to handle all asset/map loading before passing data to the Lua context.
