# Implementation Spec — Phase 6.5.4: Lua Standard Game Template
> **Status:** Ready to Implement
> **Roadmap Phase:** Phase 6.5.4
> **Reference ADRs:** [ADR-0014](../adr/pt/0014-scripting-embutido-lua-e-command-buffer.md), [ADR-0015](../adr/pt/0015-estrutura-roadmap-dedicado-fase6.5-validacao-dx.md)

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
│   └── skills.lua    # Logic for handling ActionIntents and projectile/skill spawning.
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
    command: ["--script-path", "/game/main.lua"]
```
* **Rationale:** This drastically lowers the barrier to entry. Students just need Docker installed, and they can run `docker compose up` to boot the authoritative server with their Lua scripts immediately. We use `--script-path /game/main.lua` because the template entry point sits directly at `/game/main.lua`, and the engine's VFS crawler automatically discovers and bundles all sibling Lua modules in `/game/src/` into the virtual filesystem.

### 2.3 Documentation Portal & Ecosystem Separation

As the Loci2D project expands into an ecosystem (Engine + Arena MVP + Client SDKs), the comprehensive documentation will be extracted into a dedicated repository (e.g., `loci2d-docs`). This portal will use a static site generator (like VitePress or Docusaurus) to host the exhaustive Lua API Reference, Client SDK setup guides, and a case study of the `loci-arena` MVP. The `lua-template` described in this spec will serve as the official foundational boilerplate that the documentation portal references in its quickstarts.

### 2.4 Best Practices Documented (Inline Comments)

The code within these files must serve as living documentation. Crucial topics to cover via inline comments:

1. **Fixed-Point Arithmetic & Determinism:** Explain why developers must use `Loci.Vector2(x, y)` and deterministic fixed-point math considerations (instead of non-deterministic OS floats or unseeded random) to preserve `.loci` replay determinism.
2. **Safe Entity Retrieval:** Demonstrate the proper use of read-only queries such as `Loci.get_entity_position(id)` and `Loci.get_entities_in_radius(pos, radius)` without mutating internal state directly.
3. **Command Buffer (Mutations):** Emphasize that state must only be altered via `Loci.Commands` (e.g., `Loci.Commands.set_property`, `Loci.Commands.set_velocity`, `Loci.Commands.spawn_entity`) rather than mutating local Lua tables, ensuring the Rust engine serializes all mutations into the authoritative `WorldState` snapshot.

---

## 3. Template Submodules Specification

### 3.1 `main.lua` (The Router)

The entry point delegates all lifecycle and intent events directly to specialized submodules:

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
    return player.handle_join(entity_id)
end

function on_player_leave(entity_id)
    player.handle_leave(entity_id)
end

function on_move_intent(entity_id, dir_x, dir_y)
    return movement.apply_input(entity_id, dir_x, dir_y)
end

function on_action(entity_id, ability_id, dir_x, dir_y)
    return skills.cast(entity_id, ability_id, dir_x, dir_y)
end
```

### 3.2 `src/config.lua` (Globals & Setup)

Exposes configuration constants and initializes global world state via `Loci.Commands.set_global`:

```lua
-- src/config.lua
local config = {}

config.MOVE_SPEED = 5.0
config.INITIAL_HP = "100"
config.DEFAULT_TEAM = "1"
config.PROJECTILE_SPEED = 12.0
config.PROJECTILE_RADIUS = 0.5
config.PROJECTILE_DAMAGE = "25"

function config.setup()
    Loci.Log.info("[Config] Initializing game world...")
    Loci.Commands.set_global("match_state", "active")
    Loci.Commands.set_global("server_name", "Loci2D Standard Template Arena")
end

return config
```

### 3.3 `src/player.lua` (Lifecycle)

Handles player joining and disconnecting, initializing base properties and cleaning up entities:

```lua
-- src/player.lua
local config = require("src.config")

local player = {}

function player.handle_join(entity_id)
    Loci.Log.info("[Player] Player connected with entity ID: " .. tostring(entity_id))

    -- Assign initial baseline properties and navigation move speed
    Loci.Commands.set_property(entity_id, "hp", config.INITIAL_HP)
    Loci.Commands.set_property(entity_id, "team", config.DEFAULT_TEAM)
    Loci.Commands.set_move_speed(entity_id, config.MOVE_SPEED)

    return true
end

function player.handle_leave(entity_id)
    Loci.Log.info("[Player] Player disconnected: " .. tostring(entity_id))
    Loci.Commands.destroy_entity(entity_id)
end

return player
```

### 3.4 `src/movement.lua` (Movement & Intent Handling)

Validates directional input vectors and translates them into physical velocities:

```lua
-- src/movement.lua
local config = require("src.config")

local movement = {}

function movement.apply_input(entity_id, dir_x, dir_y)
    -- Normalize input vector if analog magnitude exceeds 1.0
    local len_sq = dir_x * dir_x + dir_y * dir_y
    if len_sq > 0 then
        local len = math.sqrt(len_sq)
        if len > 1.0 then
            dir_x = dir_x / len
            dir_y = dir_y / len
        end
    end

    local vel_x = dir_x * config.MOVE_SPEED
    local vel_y = dir_y * config.MOVE_SPEED

    -- Apply velocity mutation via CommandBuffer to preserve snapshot determinism
    Loci.Commands.set_velocity(entity_id, Loci.Vector2(vel_x, vel_y))
    return true
end

return movement
```

### 3.5 `src/skills.lua` (Actions & Physics)

Demonstrates entity queries (`Loci.get_entity_position`), aiming calculation, and dynamic entity spawning (`Loci.Commands.spawn_entity`):

```lua
-- src/skills.lua
local config = require("src.config")

local skills = {}

function skills.cast(entity_id, ability_id, dir_x, dir_y)
    -- Normalize aim direction
    local len = math.sqrt(dir_x * dir_x + dir_y * dir_y)
    if len > 0 then
        dir_x = dir_x / len
        dir_y = dir_y / len
    else
        dir_x = 1.0
        dir_y = 0.0
    end

    -- Safely read caster position without mutating engine state
    local caster_pos = Loci.get_entity_position(entity_id)
    if not caster_pos then
        return false, "Caster entity not found"
    end

    -- Offset spawn position in aiming direction
    local spawn_x = caster_pos.x + dir_x * 1.5
    local spawn_y = caster_pos.y + dir_y * 1.5

    -- Allocate and spawn a projectile entity via the CommandBuffer
    local proj_id = Loci.Commands.spawn_entity({
        blueprint = "fireball",
        position = Loci.Vector2(spawn_x, spawn_y),
        entity_type = "Projectile",
        move_speed = config.PROJECTILE_SPEED,
        radius = config.PROJECTILE_RADIUS,
        properties = {
            damage = config.PROJECTILE_DAMAGE,
            caster = tostring(entity_id)
        }
    })

    -- Set projectile velocity in aiming direction
    Loci.Commands.set_velocity(
        proj_id, 
        Loci.Vector2(dir_x * config.PROJECTILE_SPEED, dir_y * config.PROJECTILE_SPEED)
    )

    Loci.Log.info(string.format("[Skills] Entity %d cast ability %d -> spawned projectile %d", entity_id, ability_id, proj_id))
    return true
end

return skills
```

---

## 4. Verification

1. **Run Locally via Cargo:**
   ```bash
   cargo run -- --script-path examples/lua-template/main.lua
   ```
   Verify that all submodules load cleanly via VFS and that `on_init` runs without errors.
2. **Run Containerized via Docker:**
   ```bash
   cd examples/lua-template && docker compose up
   ```
   Verify that the containerized engine runs and discovers the Lua modules.
3. **Replay Validation:** Join with a client, perform actions (move, cast skills), disconnect, and ensure the resulting `.loci` file replays perfectly with matching checksums.
4. **Module Resolution:** Ensure that all nested `require("src.module")` calls successfully resolve via the VFS without triggering sandbox path traversal errors.
