# Validation Criteria — Phase 6.5.4: Engine MVP Acceptance
> **Status:** Draft
> **Roadmap Phase:** Phase 6.5.4
> **Goal:** Establish the technical criteria that the student-built 3v3 Arena MVP must exercise to successfully validate the core engine's capabilities.

---

## 1. Context & Purpose

The **3v3 Arena MVP** is the first fully functional game to be built on top of the loci2d engine. Because the game itself will be developed by high school students using the Love2D SDK (`loci_client.lua`), this document does not specify game design, classes, or art assets. 

Instead, this document serves as an **Engine Acceptance Checklist**. It defines the core engine features that the MVP *must* utilize. If the MVP successfully implements these features without requiring modifications to the Rust core, the engine's architecture for Phase 6.5 is considered validated.

---

## 2. Core Validation Requirements

The student-built MVP must implement gameplay mechanics that explicitly test the following engine systems:

### 2.1. Network & Session Lifecycle
- [ ] **Connection & Disconnection:** The game must allow players to join dynamically and leave gracefully without crashing the server.
- [ ] **Timeout Handling:** The game must test scenarios where a client crashes or drops connection, ensuring the server successfully times them out and cleans up the session after `CLIENT_TIMEOUT_SECS`.

### 2.2. Physics & Movement
- [ ] **Client-Side Interpolation:** Players must use `loci.send_move()` to navigate. The client must render smooth movement using the SDK's internal velocity-based interpolation.
- [ ] **Static Collisions:** The map must contain at least one static obstacle (e.g., a wall or pillar). Players and projectiles must correctly collide with and slide against it.
- [ ] **Dynamic Collisions:** Entities (players/minions) must collide with each other (kinematic vs kinematic resolution).

### 2.3. Entity Properties & Auto-Casting
- [ ] **Property Sync:** The game must utilize dynamic properties (e.g., `hp`, `team_color`, `is_stunned`).
- [ ] **Ergonomic Access:** The client code must read these properties directly (e.g., `if entity.hp <= 0 then`) to validate the SDK's `Entity` metatable and auto-casting (string to number/boolean).
- [ ] **Reactive UI:** The client must use the `loci.on_property_changed` callback to update health bars or UI elements without polling.

### 2.4. Actions & Abilities (Spatial Queries)
- [ ] **Skillshots (Projectiles):** At least one ability must spawn a moving entity (projectile) using `ActionIntent` and `target_direction`.
- [ ] **Area of Effect (AoE) Skills:** At least one ability must validate the server's spatial queries by using `Loci.get_entities_in_radius` to deal damage or apply effects to multiple targets.
- [ ] **Action Broadcasts:** The client must use `loci.on_action_cast` to trigger visual or audio effects (e.g., a sword swing animation or fireball sound) precisely when the server authorizes the action.

### 2.5. Match Lifecycle
- [ ] **Global Properties:** The game must use `loci.globals` to display a match timer or team score (e.g., `Blue Team: 3 | Red Team: 2`).
- [ ] **Match States:** The server script must programmatically transition the match state (e.g., from `Running` to `Ended` when a team reaches a score limit or the timer runs out). The client must react via `loci.on_match_state_changed` to show a victory/defeat screen.

---

## 3. The "Standard Lua Game Template"

To assist the students in meeting these validation criteria, the core engine team will provide a **Standard Lua Game Template** (`main.lua` boilerplate). 

This boilerplate will not contain the game's logic, but will provide heavily documented skeletons for:
- Initializing the connection (`loci.connect`).
- Handling the Love2D update and draw loops.
- Empty callback functions (`loci.on_entity_spawned`, `loci.on_property_changed`, etc.) with explanatory comments.

**Success Metric:** The students should be able to build the 3v3 Arena MVP by only modifying the Lua template and server-side Lua scripts, without ever needing to compile Rust code or touch protobuf definitions.
