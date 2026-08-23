# ADR 0016: Data-Driven Entity Properties and Engine Agnosticism

## Status

Accepted (Phase 6.5.0)

## Context

As part of Phase 6.5.0, the `loci2d` scripting API is being expanded to support concrete game rules (e.g., determining if an entity has a specific "team" to implement friendly fire, or checking "health" to apply damage). 

The natural and easiest approach in Rust would be to add these fields directly to the `Entity` struct (`pub health: i32`, `pub team_id: u8`). However, `loci2d` is designed to be a generic 2D authoritative server framework, not tied to a specific genre (like a 3v3 MOBA or a Battle Royale). Hardcoding game-specific attributes into the core Rust engine pollutes the architecture, makes the engine less flexible, and blurs the line between engine infrastructure and game logic.

Furthermore, if the engine dictates what properties exist, developers would need to modify the Rust source code and recompile the server binary every time they wanted to add a new mechanic (e.g., "mana", "stamina", "armor").

## Decision

We will adopt a strict **Data-Driven Design** for entity attributes by implementing a generic Property System.

1. **Generic Storage:** The Rust `Entity` struct will only hold a flexible key-value property map (e.g., `properties: HashMap<String, String>` or similar generic value storage).
2. **Agnostic Core:** The Rust core must remain completely ignorant of game rules. Hardcoded gameplay fields (like `health`, `team`, `mana`) are strictly forbidden in the core Rust structs.
3. **Lua Ownership:** The Lua scripting layer is solely responsible for defining, setting, getting, and interpreting the meaning of these properties via `Loci.get_entity_property` and `Loci.Commands.set_property`.
4. **State Synchronization:** The generic property map will be serialized and broadcasted to clients as part of the `WorldState` Protobuf snapshot, allowing clients to render visuals (like red/blue team outlines) based on the dynamic properties.

## Consequences

**Positive:**
- **Absolute Flexibility:** Developers can build any type of 2D game (FFA, Team Deathmatch, PvE) solely through Lua scripts without ever touching Rust code.
- **Clear Architectural Boundary:** The engine (Rust) focuses exclusively on performance, networking, and deterministic physics. The game logic (Lua) focuses entirely on rules and state interpretation.
- **Future-Proof:** Prevents structural bloat in the `Entity` struct as the framework evolves.

**Negative:**
- **Serialization Overhead:** Serializing a dynamic hash map into Protobuf for network broadcasting consumes more bandwidth and CPU cycles compared to serializing static, strongly-typed fields.
- **Loss of Type Safety:** Properties retrieved in Lua or Rust are generic values (e.g., Strings or basic enums), meaning developers must handle type parsing and validation manually in their scripts.
