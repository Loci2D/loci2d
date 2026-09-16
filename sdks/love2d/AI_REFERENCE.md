# Loci2D Client SDK - AI Agent Reference

> **Context:** You are an AI assistant helping a developer build a multiplayer 2D game using Love2D and the `loci2d` game engine. 

## Engine Architecture (For the AI)
- `loci2d` is an authoritative Rust server that handles all physics, collisions, and state.
- The developer is using the `loci_client.lua` SDK, which abstracts away all UDP sockets, Protocol Buffers, and network interpolation. **DO NOT generate any `luasocket` or `protobuf` code.**
- The server uses fixed-point math internally, but the SDK automatically converts this. **Always use standard Lua floats** for coordinates (`x`, `y`).
- The SDK automatically manages the player's connection and session heartbeat.

## Public SDK API

### 1. Connection Lifecycle
- `loci.connect(ip, port, player_name)`: Connects to the server.
- `loci.disconnect(reason)`: Disconnects gracefully.
- `loci.update(dt)`: **MUST** be called every frame inside `love.update(dt)`. It processes all network packets and handles entity interpolation.

### 2. State Access
- `loci.get_entities()`: Returns a table of all entities currently visible to the client.
- `loci.get_my_entity()`: Returns the local player's `Entity` object.
- `loci.get_globals()`: Returns a table of global match properties (e.g., scores, timers).
- `loci.get_entities_in_radius(center_x, center_y, radius)`: Spatial query helper.

### 3. The `Entity` Object
Entities are NOT simple tables. They have a metatable that automatically casts properties.
- **Transform**: `entity.x`, `entity.y`, `entity.vx`, `entity.vy` (all floats).
- **Metadata**: `entity.id`, `entity.blueprint` (string).
- **Dynamic Properties**: Accessed directly via `entity.prop_name` (e.g., `entity.hp`, `entity.team`). Values are automatically cast to `number` or `boolean`.
- **Methods**: 
  - `entity:is_local_player()` -> bool
  - `entity:distance_to(other_entity_or_table)` -> number

### 4. Sending Intents (Client -> Server)
- `loci.send_move(dir_x, dir_y)`: Sends a movement intent. The vector does not need to be normalized.
- `loci.send_action(ability_id, aim_world_x, aim_world_y)`: Casts an ability. The coordinates must be **World Space** coordinates, not screen space.

### 5. Event Callbacks (Server -> Client)
The developer must assign functions to these callbacks to react to server events:
- `loci.on_entity_spawned = function(entity)`
- `loci.on_entity_despawned = function(entity_id)`
- `loci.on_property_changed = function(entity, key, old_val, new_val)`: Ideal for updating UI or VFX.
- `loci.on_action_cast = function(entity, ability_id, dir_x, dir_y)`: Fired when an entity successfully casts an ability. Trigger particle effects or sounds here.
- `loci.on_match_state_changed = function(state, winner)`: `state` is "running", "paused", or "ended".
- `loci.on_intent_rejected = function(reason)`: Fired if the server denies a client action.

## AI Instructions
When asked to implement a game feature:
1. Put all visual/audio logic inside the event callbacks (e.g., play attack sound in `on_action_cast`).
2. Map Love2D inputs (`love.keypressed`, `love.mousepressed`) directly to `loci.send_move` and `loci.send_action`.
3. Read game state directly from `loci.get_entities()` inside `love.draw` to render the game. Do not maintain a separate list of game objects.
