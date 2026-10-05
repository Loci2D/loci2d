# Implementation Spec — Phase 6.5.4: Lua Standard Game Template
> **Status:** Pending
> **Roadmap Phase:** Phase 6.5.4
> **Reference ADRs:** [ADR-0014](../adr/pt/0014-scripting-embutido-lua-e-command-buffer.md)

---

## 1. Context & Technical Motivation

With the introduction of multi-file module loading (`require`) via Virtual Filesystem (VFS) in Issue #31 and safer entity access in Issue #19, it's now possible to build complex games using `loci2d` without cramming all logic into a monolithic `main.lua` file. 

To guide students and developers towards the pit of success, we need a standard boilerplate (template). This template establishes the official multi-file architecture to decouple game logic (lifecycle, config, skills, math) while demonstrating best practices in fixed-point math and state immutability.

---

## 2. Technical Implementation Modules

### 2.1 Directory Structure & Entry Point

The template will be hosted at `examples/lua-template/` and should follow a modular hierarchy:

```text
examples/lua-template/
├── docker-compose.yml # Containerized engine runner (Zero-install DX)
├── main.lua          # Entry point. Receives engine hooks and delegates them.
├── src/
│   ├── config.lua    # Global properties, map info, and initial setups.
│   ├── player.lua    # Handles on_player_join, on_player_leave, and disconnects.
│   ├── movement.lua  # Logic for handling Move/MoveToPos intents.
│   └── skills.lua    # Logic for handling ActionIntents and area effects.
```

* **Rationale:** A modular structure keeps files small, isolated, and easy to maintain. `main.lua` becomes merely a router for the engine callbacks.

### 2.2 Developer Experience (DX): Docker Compose (Zero-Install)

To eliminate the need for students to install the Rust toolchain (`cargo`, `rustc`) to test their games, the template will include a `docker-compose.yml` file. This mounts the local directory into the pre-compiled engine container:

```yaml
services:
  server:
    image: loci2d/engine:latest
    ports:
      - "8000:8000/udp"
    volumes:
      - ./:/game
    command: ["--scripts-dir", "/game"]
```
* **Rationale:** This drastically lowers the barrier to entry. Students just need Docker installed, and they can run `docker compose up` to boot the authoritative server with their Lua scripts immediately.

### 2.3 Documentation Portal & Ecosystem Separation

As the Loci2D project expands into an ecosystem (Engine + Arena MVP + Client SDKs), the comprehensive documentation will be extracted into a dedicated repository (e.g., `loci2d-docs`). This portal will use a static site generator (like VitePress or Docusaurus) to host the exhaustive Lua API Reference, Client SDK setup guides, and a case study of the `loci-arena` MVP. The `lua-template` described in this spec will serve as the official foundational boilerplate that the documentation portal references in its quickstarts.

### 2.4 Best Practices Documented (Inline Comments)

The code within these files must serve as living documentation. Crucial topics to cover via inline comments:

1. **Fixed-Point Arithmetic:** Explain why developers must use `DeterministicVector2` and fixed-point math functions (instead of standard Lua `math` or floats) to preserve `.loci` replay determinism.
2. **Safe Entity Retrieval:** Demonstrate the proper use of `Loci.get_entities()` in the context of Issue #19, showing how to read properties without mutating the engine's internal cache.
3. **Command Buffer (Mutations):** Emphasize that state must only be altered via `Loci.Commands` (e.g., `Loci.Commands.set_property`, `Loci.Commands.set_position`) rather than mutating local Lua state, ensuring the Rust side serializes these changes into the authoritative `WorldState` snapshot.

---

## 3. Template Submodules Specification

### 3.1 `main.lua` (The Router)
```lua
-- main.lua
local config = require("src.config")
local player = require("src.player")
local movement = require("src.movement")
local skills = require("src.skills")

function on_init()
    config.setup()
end

function on_player_join(entity_id)
    player.spawn_new(entity_id)
end

function on_action(entity_id, ability_id, dir_x, dir_y)
    skills.cast(entity_id, ability_id, dir_x, dir_y)
end

function on_move_intent(entity_id, dir_x, dir_y)
    movement.apply_input(entity_id, dir_x, dir_y)
end
```

### 3.2 `src/config.lua` (Globals & Setup)
Demonstrates setting up the map, global timers, or match states using `Loci.Commands.set_global_property`.

### 3.3 `src/player.lua` (Lifecycle)
Demonstrates assigning base attributes (HP, Team, Move Speed) using `Loci.Commands.set_property` and `Loci.Commands.set_move_speed` upon a player's arrival.

### 3.4 `src/skills.lua` (Actions & Physics)
Demonstrates reading the `dir_x` and `dir_y` vector from an `ActionIntent`, normalizing it, and spawning a projectile entity via `Loci.Commands.spawn_entity` using fixed-point math calculations.

---

## 4. Verification

1. **Run as Example:** Verify that running `docker compose up` (or `cargo run`) boots the server with the template scripts without errors.
2. **Replay Validation:** Join with a client, perform actions (move, cast skills), disconnect, and ensure the resulting `.loci` file replays perfectly with matching checksums.
3. **Module Resolution:** Ensure that all nested `require("src.module")` calls successfully resolve via the VFS without triggering sandbox path traversal errors.
