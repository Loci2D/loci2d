# Demand Spec: [CORE][Physics] Support for Intangibility (Pass-through) and Raycast for Abilities

**Trello Card:** [YFsEuVe1](https://trello.com/c/YFsEuVe1)
**Status:** Draft

## 1. Overview
To support advanced mobility abilities (such as a dash that passes through enemies without pushing them and without going through environment walls), it is necessary to expand the physics query capabilities and filters exposed to the Lua scripting layer.

## 2. Motivation
This demand arises from the need for gameplay mechanics where characters use dash or teleportation without getting stuck on the collision of other players, while still respecting the solid architecture of the map (such as walls and boundaries). Because this requires low-level access to the collision and physics system in Rust, it becomes an essential core change that will enable more complex mechanics in the arenas.

## 3. Technical Requirements
1. **Dynamic Collision Control:**
   - Add methods to `Loci.Commands` that allow temporarily disabling/modifying body-to-body collision (e.g., `set_intangible(entity_id, bool)` or direct manipulation of the `collision_mask`).
2. **Raycasting / Sweep Test Query:**
   - Expose a method in the Scripting API (Lua) to cast rays and check for static collisions before applying sudden position displacements (e.g., `Loci.Physics.raycast(origin, direction, distance, layer_mask)`).

## 4. Acceptance Criteria
- [ ] A player in a "dash" (intangible) state must not suffer collision *pushback* against other players.
- [ ] Dash or teleportation abilities must not allow passing through solid walls or map boundaries (thanks to the prior check using raycast/sweep).

## 5. Core Impact (Loci2D)
- **Engine / Core (Rust):** Access to the internal collision/physics API (Rapier or similar engine) to implement layer filters dynamically and spatial queries (raycast).
- **Scripting API:** New functions must be exposed and documented, extending `Loci.Commands` (state/collision modifiers) and the Physics interface (pure spatial queries).
- **Client (LÖVE 2D):** In principle, no deep changes to the client are required, only the support to correctly interpret abrupt position updates or handle position interpolation during the dash.
