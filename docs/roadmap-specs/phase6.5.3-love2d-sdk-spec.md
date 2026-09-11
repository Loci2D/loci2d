# Implementation Spec — Phase 6.5.3: Love2D SDK & MVP Client
> **Status:** Draft
> **Roadmap Phase:** Phase 6.5.3
> **Reference ADRs:** (TBD based on Protobuf Lua implementation)

---

## 1. Context & Technical Motivation

To successfully build the 3v3 Arena MVP, the high school students will use **Love2D**. Because they lack knowledge of netcode, UDP sockets, and Protocol Buffers, we must provide an ergonomic, high-level SDK (`loci_client.lua`). 

The primary goal of this SDK is to **completely abstract the network layer**. The developers (students) should only interact with game state (Entities, Properties) and high-level intents (Move, Cast Ability), treating the multiplayer game almost as if it were a local single-player game.

---

## 2. Architecture Overview

The `loci_client.lua` module will be internally divided into three layers, but only the top layer is exposed to the developer:

1. **Network & Protocol Layer (Internal):** Manages the `luasocket` UDP connection and serializes/deserializes Protobuf payloads.
2. **State Manager (Internal):** Maintains the canonical in-memory representation of the world based on server broadcasts. Handles the automatic type-casting of Entity properties (e.g., converting the string `"100"` to the Lua number `100` for HP).
3. **API Facade (Public):** The interface exposed to the students, consisting of commands (Client -> Server) and callbacks (Server -> Client).

---

## 3. Client SDK API Mapping

This section maps the low-level engine concepts to the high-level Lua functions the students will call in their Love2D code.

### 3.1 Initialization & Loop
These are the standard hooks the student must place in their `main.lua`.

* **`loci.connect(ip, port, player_name)`**
  * *Internal:* Establishes the UDP socket and sends a `ConnectRequest` protobuf.
* **`loci.update(dt)`**
  * *Internal:* Called inside `love.update(dt)`. Polls the UDP socket for new packets, updates the internal State Manager, applies interpolation, and fires callbacks if state changed.
* **`loci.get_entities()`**
  * *Returns:* A Lua table containing all currently active `Entity` objects in the match.
* **`loci.get_my_entity()`**
  * *Returns:* The `Entity` object representing the local player (useful for centering the camera).

### 3.2 Sending Intents (Client -> Server)
Students will call these functions based on player input (keyboard/mouse).

* **`loci.send_move(dir_x, dir_y)`**
  * *Internal:* Packages a `MoveIntent` protobuf packet.
* **`loci.send_action(ability_id, aim_x, aim_y)`**
  * *Internal:* Packages an `ActionIntent` protobuf packet. Used for casting the 3v3 Arena skills (e.g., `ability_id = 1` for basic attack, `ability_id = 2` for ultimate).
* **`loci.send_ping()`**
  * *Internal:* Sends a ping request to measure latency.

### 3.3 Signals & Callbacks (Server -> Client)
Students will define these functions in their code. The SDK will trigger them when the server sends new state.

* **`loci.on_entity_spawned = function(entity)`**
  * Triggered when a new player or projectile enters the simulation.
* **`loci.on_entity_despawned = function(entity_id)`**
  * Triggered when an entity is removed (e.g., a projectile explodes, or a player dies).
* **`loci.on_property_changed = function(entity, key, old_val, new_val)`**
  * The most powerful hook for UI and FX. 
  * *Example Use Case:* If `key == "hp"`, update the health bar. If `key == "team"`, change the sprite color to Red or Blue.
* **`loci.on_match_state_changed = function(state)`**
  * Triggered when the match starts, pauses, or ends.
* **`loci.on_action_cast = function(entity, ability_id, dir_x, dir_y)`**
  * Triggered when the server confirms an action was cast.
  * *Example Use Case:* Play the sword swing animation or spawn particle effects at the entity's position.

---

## 4. High-Level Entity Abstraction

The `Entity` object passed to the callbacks or retrieved via `loci.get_entities()` is a high-level Lua table, *not* a raw Protobuf struct.

**Expected Structure:**
```lua
{
  id = 12345,
  x = 150.5,           -- Interpolated automatically by the SDK
  y = 200.0,
  velocity_x = 0.0,
  velocity_y = 0.0,
  properties = {       -- Automatically casted from string to native Lua types
    hp = 100,
    max_hp = 100,
    team = "blue",
    is_stunned = false
  }
}
```

## 5. Next Steps / Implementation Plan
1. **Choose Protobuf Library:** Evaluate pure-Lua protobuf libraries vs C-bindings for Love2D to ensure easy distribution for the students.
2. **Implement Network Scaffold:** Set up the UDP send/receive loop in `loci.update(dt)`.
3. **Build State Manager:** Parse `WorldState` packets and build the high-level `Entity` tables.
4. **Wire Callbacks:** Trigger `on_property_changed` by diffing the old state with the new state.
