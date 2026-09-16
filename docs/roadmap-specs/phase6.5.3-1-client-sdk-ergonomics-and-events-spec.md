# Implementation Spec — Phase 6.5.3-1: Client SDK Ergonomics, Entity Abstraction & Event Broadcasts
> **Status:** Completed
> **Roadmap Phase:** Phase 6.5.3-1
> **Reference ADRs:** ADR-0018 (Client SDKs and Network Abstraction) · ADR-0019 (Authoritative Intent Rejection Feedback) · ADR-0016 (Data-Driven Entity Properties) · ADR-0007 (Fixed-Point Arithmetic)
> **Parent Spec:** [phase6.5.3-love2d-sdk-spec.md](phase6.5.3-love2d-sdk-spec.md)

---

## 1. Context & Technical Motivation

Phase 6.5.3 successfully established the networking foundation for the Love2D client SDK (`loci_client.lua`), abstracting away UDP socket loops, Protobuf binary encoding, fixed-point conversions, and connection lifecycles.

However, during practical usage evaluation for student game developers, two major Developer Experience (DX) hurdles remain:
1. **Raw String Properties:** The engine transmits entity properties as generic string key-value pairs. Consequently, student code is forced to perform manual string comparisons (e.g. `if entity.properties["team"] == "1" then`) and conversions (`tonumber(entity.properties["hp"])`), which causes syntax errors, verbose code, and impedance mismatch.
2. **Missing Action and Match Lifecycle Signals:** Callbacks such as `loci.on_action_cast` and `loci.on_match_state_changed` remained placeholders because the Rust server did not broadcast action executions or match states to connected clients. As a result, clients cannot trigger visual effects, spell casts, sound triggers, or match over screens for actions performed by other players.

This sub-specification addresses these deficiencies directly, providing an ergonomic, high-level client abstraction and the necessary protocol extensions.

---

## 2. Architecture & Design Decisions

### 2.1 Entity Abstraction & Property Auto-Casting (Client)

In `loci_client.lua`, entities will no longer be bare dictionaries. Instead, each entity is managed as an `Entity` object with an attached metatable:

* **Automatic Property Type Casting:** When the client ingests entity properties from `WorldState`:
  * String values matching valid numbers (`"100"`, `"-2.5"`, `"0"`) are automatically parsed via `tonumber()` into Lua numbers.
  * String values `"true"` or `"false"` (case-insensitive) are converted to Lua booleans `true` and `false`.
  * All other strings remain Lua strings.
* **Ergonomic Property Access via `__index`:**
  * Direct field access: `entity.hp`, `entity.team`, `entity.max_hp` reads directly from the property table if not a native engine field.
  * Native fields (`id`, `blueprint`, `x`, `y`, `vx`, `vy`) take precedence over dynamic properties.
* **Helper Methods:**
  * `entity:is_local_player()`: Returns `true` if `entity.id == loci.my_entity_id`.
  * `entity:distance_to(other)`: Computes Euclidean distance to another entity or coordinate table `{x, y}`.
  * `entity:get(prop_name, default_value)`: Safe property getter with fallback.

### 2.2 Transient Action Broadcasts (Server -> Client)

To allow clients to trigger audio/visual effects when an entity casts an ability, the server will broadcast transient action events:

* A message `ActionBroadcast` is added to `proto/game_packets.proto`:
  ```protobuf
  message ActionBroadcast {
    uint64 entity_id = 1;
    uint32 ability_id = 2;
    Vector2 target_direction = 3;
  }
  ```
* In `WorldState`:
  ```protobuf
  repeated ActionBroadcast actions = 5;
  ```
* **Transient Semantics:** `actions` only contains actions successfully authorized and executed in the **current tick**. It is emptied after every tick snapshot broadcast, preventing memory leaks and avoiding duplication across ticks.
* **Determinism Safety:** Actions are recorded during `Intent::Action` processing in the simulation tick, maintaining bit-exact replay reproduction.

### 2.3 Authoritative Match Lifecycle Synchronization

* The protobuf schema adds `MatchLifecycleState` to `WorldState`:
  ```protobuf
  enum MatchLifecycleState {
    MATCH_RUNNING = 0;
    MATCH_PAUSED = 1;
    MATCH_ENDED = 2;
  }
  ```
  And fields in `WorldState`:
  ```protobuf
  MatchLifecycleState match_state = 6;
  string match_winner = 7;
  ```
* In `loci_client.lua`:
  * Maps enum values to clean strings: `"running"`, `"paused"`, `"ended"`.
  * Detects state transitions and invokes `loci.on_match_state_changed(new_state, winner_data)`.

---

## 3. Detailed Technical Design

### 3.1 Protobuf Protocol Extensions (`proto/game_packets.proto`)

```protobuf
enum MatchLifecycleState {
  MATCH_RUNNING = 0;
  MATCH_PAUSED = 1;
  MATCH_ENDED = 2;
}

message ActionBroadcast {
  uint64 entity_id = 1;
  uint32 ability_id = 2;
  Vector2 target_direction = 3;
}

message WorldState {
  uint64 tick = 1;
  uint64 timestamp = 2;
  repeated EntityState entities = 3;
  repeated Property globals = 4;
  repeated ActionBroadcast actions = 5;
  MatchLifecycleState match_state = 6;
  string match_winner = 7;
}
```

### 3.2 Rust Server Implementation

1. **`Instance` Buffer:** Add `pub tick_actions: Vec<ActionBroadcast>` to `Instance`.
2. **Intent Execution:** In `src/world/intent_handler.rs`, when `Intent::Action` is authorized (returns `IntentResult::Ok`), push `ActionBroadcast` into `instance.tick_actions`.
3. **Snapshot Creation:** In `Instance::create_snapshot`, clone `self.tick_actions`, and map `self.state` to `MatchLifecycleState` and `winner_data`.
4. **Lifecycle Clearing:** In `src/game_loop/tick.rs` and `src/replay/player.rs`, clear `instance.tick_actions` after generating the snapshot for each tick.

### 3.3 Love2D SDK Implementation (`sdks/love2d/loci_client.lua`)

1. **Entity Class / Metatable:**
   ```lua
   local Entity = {}
   Entity.__index = function(t, k)
       local method = Entity[k]
       if method ~= nil then return method end
       local props = rawget(t, "properties")
       if props ~= nil then
           return props[k]
       end
       return nil
   end
   ```
2. **Type Casting Helper:**
   ```lua
   local function cast_property_value(val)
       if type(val) ~= "string" then return val end
       if val == "true" or val == "True" then return true end
       if val == "false" or val == "False" then return false end
       local num = tonumber(val)
       if num ~= nil then return num end
       return val
   end
   ```
3. **Action Dispatch:**
   Iterate over `state.actions`, convert `target_direction` from fixed-point to Lua float, and invoke `loci.on_action_cast(entity, action.ability_id, dir_x, dir_y)`.
4. **Match State Dispatch:**
   Compare `state.match_state` with `loci.match_state`. If changed, update and trigger `loci.on_match_state_changed(state_str, winner_str)`.

---

## 4. Implementation Checklist

- [x] **1. Protobuf Extensions**
  - [x] Add `ActionBroadcast` and `MatchLifecycleState` to `proto/game_packets.proto`.
  - [x] Synchronize `sdks/love2d/lib/game_packets.proto`.
- [x] **2. Rust Server Pipeline**
  - [x] Add `tick_actions` buffer to `Instance` in `src/world/instance.rs`.
  - [x] Record authorized actions in `src/world/intent_handler.rs`.
  - [x] Populate `actions`, `match_state`, and `match_winner` in `Instance::create_snapshot`.
  - [x] Clear `tick_actions` after each tick in `src/game_loop/tick.rs` and `src/replay/player.rs`.
- [x] **3. Love2D SDK Improvements (`loci_client.lua`)**
  - [x] Implement `Entity` metatable and property auto-casting.
  - [x] Add entity helper methods: `is_local_player()`, `distance_to()`, `get()`.
  - [x] Implement action broadcast ingestion and trigger `loci.on_action_cast`.
  - [x] Implement match lifecycle state tracking and trigger `loci.on_match_state_changed`.
  - [x] Add spatial/filtering helpers: `loci.get_entities_by_blueprint()`, `loci.get_entities_in_radius()`.
- [x] **4. Client Example & Documentation**
  - [x] Update `examples/love2d/main.lua` to leverage typed properties and action FX.
  - [x] Update `sdks/love2d/docs/index.html` with typed properties and callback guides.
- [x] **5. Verification**
  - [x] Run `cargo check` and `cargo test`.
  - [x] Run `cargo run --bin benchmark` to verify determinism and performance.
