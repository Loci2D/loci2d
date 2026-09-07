# Phase 6.5.2: API Refinement & SDK Preparation

> **Status:** Draft

## Objective
Refine the Lua Engine API to remove inconsistencies, patch holes in the safe dispatch model, and prepare the server layer for ergonomic consumption by future Wrappers and SDKs.

## Motivation
During the finalization of Phase 6.5.1 (Architecture & Safety), we noticed that `Loci.Commands` correctly solves the unsafe mutation problem, but some parts of the Rust code (especially `intent_handler.rs`) still bypass Lua. For a Client Wrapper (like Godot or Love2D) to fully leverage server-side logic, the SDK requires total flexibility, including movement interception, reading basic status, and parameter passing standardization.

---

## Implementation Specifications

### 1. Strict Movement Intent Interception (Safe Layer Fix)

**Problem:** Currently, `Intent::Move` and `Intent::MoveToPos` directly alter `entity.velocity` and `entity.navigation` in `intent_handler.rs` *before* any Lua script can analyze the action.

**Solution:**
- Remove direct mutation of physics properties in `intent_handler.rs`.
- Instead of altering the state, propagate these intents to the `ScriptEngine`, which will invoke two new callbacks:
  - `on_move_intent(entity_id, dir_x, dir_y)`
  - `on_nav_intent(entity_id, target_x, target_y)`
- **Expected Flow:** The Lua script decides whether to accept the movement intent by validating game rules (e.g., checking `stunned`, `dead`). If accepted, it triggers `Loci.Commands.set_velocity` or equivalent.
- **Advantage:** This gives ultimate authority to the Script.

### 2. Lua API Consistency & Basic Getters

**Problem:** There are two ways to handle `Vector2` (`x,y` parameters vs UserData), vital getters are missing, and properties purely deal with strings.

**Solutions - Part A (Vectors):**
- Standardize whether hooks (`on_action`, `on_move_intent`) provide exploded variables (`x, y` f64) or instantiate the `Loci.Vector2` UserData class. The recommendation to avoid heavy instantiation is to continue passing `f64`, but adjust the setter syntax (`set_velocity`, `set_position`) so they accept both `(x, y)` floats and `Loci.Vector2`. This facilitates writing and adapting Wrappers.

**Solutions - Part B (Getters):**
- Implement the following functions in the `Loci` table:
  - `Loci.get_velocity(entity_id)` -> returns `x, y`
  - `Loci.get_move_speed(entity_id)` -> returns `speed`
  - `Loci.get_entity_name(entity_id)` -> returns `string`
- **Note on Properties:** Properties in `BTreeMap<String, String>` will continue using Strings to facilitate protobuf serialization, but the Loci documentation will reinforce the use of `tonumber()` and `tostring()`. The Wrappers (Phase 6.5.3) will be responsible for type inference, relieving the engine.

### 3. Spawn Command Extensibility (Blueprints)

**Problem:** `Loci.Commands.spawn_entity(entity_id, blueprint, position)` in `command.rs` assigns static components (Default Navigation, Type: Prop) via hardcode, using the blueprint string only as a name.

**Solution:**
- Update the signature (or create an equivalent `spawn_entity_custom`) to allow Lua to inject key attributes at spawn time.
- *Short-Term Proposal:* Change the function to `spawn_entity(blueprint, position_x, position_y, config_table)`.
- *config_table* could include `{ entity_type = "Player", radius = 10, speed = 1.0 }`.
- This ensures we won't depend on hardcoded configurations in the C/Rust Engine for everyday API use.

---

## Acceptance Criteria
- All unit tests (`test_explicit_move_to_pos_intent_and_preemption`, etc.) rewritten to invoke movement through the `on_move_intent` injected by Lua in the tests, ensuring the old coupling is gone.
- The new getters must be operational and typed in the mlua SDK.
- Scripts can block movement by intentionally ignoring an `on_move_intent` if the `stunned = "1"` property is set.
