# Demand Spec: [CORE][Physics] Support for Intangibility (Pass-through) and Raycast for Abilities

**Trello Card:** [YFsEuVe1](https://trello.com/c/YFsEuVe1)
**Status:** Ready to implement
**Reference ADRs:** [ADR-0007](../adr/en/0007-deterministic-simulation-and-fixed-point.md) · [ADR-0010](../adr/en/0010-event-sourced-replay-format.md) · [ADR-0012](../adr/en/0012-deterministic-2d-collision-and-kinematic-resolution.md) · [ADR-0014](../adr/en/0014-embedded-lua-scripting-and-command-buffer.md)

## 1. Overview
To support advanced mobility abilities (such as a dash that passes through enemies without pushing them and without going through environment walls), it is necessary to expand the physics query capabilities and filters exposed to the Lua scripting layer.

Currently:

| Component | Current state | Problem |
|---|---|---|
| `src/world/physics/map.rs` | Defines `CollisionFilter` (`layer`, `mask`) for entities. | Static definition. Cannot be dynamically modified mid-game by scripts. |
| `src/scripting/command.rs` | `CommandBuffer` supports `SetPosition`, `SetVelocity`, etc. | Lacks a command to mutate the `collision_filter` of an entity. |
| `src/scripting/api.rs` | Exports `Loci.Commands` and other utilities to Lua. | Lacks a Lua binding for raycast queries against the physics world. |
| `src/world/physics/collision.rs` | Implements `fixed_raycast` for internal use. | Only checks against `obstacles` (`StaticObstacle`), and is not exposed to Lua. |

**Core problem:** Lua scripts cannot temporarily disable collision (intangibility) for an entity during a dash. Furthermore, they cannot predict where a dash will end up because there is no `raycast` function available in Lua to check for static obstacles (walls) before teleporting or sliding the entity.

---

## 2. Goal

Expand the Rust core and Lua Scripting API to support dynamic collision masks and spatial raycast queries, enabling complex mobility skills (like dash, teleport, and blink).

- **Dynamic Collision Control:** Add a new Command to the `CommandBuffer` to allow Lua to mutate an entity's `collision_filter` (changing its `layer` or `mask` dynamically).
- **Raycasting / Sweep Test Query:** Expose a `Loci.Physics.raycast` function to Lua that queries `fixed_raycast` to find the nearest obstacle, allowing abilities to avoid teleporting players into solid walls.

---

## 3. Changes per File

### 3.1 `src/scripting/command.rs` — Add `SetCollisionFilter`

Add a new command to the `Command` enum to mutate collision filters via the CommandBuffer pattern defined in ADR-0014.

```rust
// In enum Command:
SetCollisionFilter {
    entity_id: u64,
    layer: u16,
    mask: u16,
},
```

In `CommandBuffer::apply` (or where the buffer is drained), handle the new command:

```rust
Command::SetCollisionFilter { entity_id, layer, mask } => {
    if let Some(entity) = instance.entities.get_mut(&entity_id) {
        entity.collision_filter = crate::world::physics::map::CollisionFilter::new(layer, mask);
        if instance.logging_enabled {
            println!("[CommandBuffer] SetCollisionFilter entity={} layer={} mask={}", entity_id, layer, mask);
        }
    }
}
```

---

### 3.2 `src/scripting/api.rs` — Expose API to Lua

**Part A: Expose `Loci.Commands.set_collision_filter`**

Inside `with_scoped_api`, bind the new command:

```rust
let cmd_buf_col_filter = Rc::clone(&cmd_buffer_rc);
let set_collision_filter = scope.create_function(move |_, (id, layer, mask): (u64, u16, u16)| {
    cmd_buf_col_filter.borrow_mut().push(Command::SetCollisionFilter {
        entity_id: id,
        layer,
        mask,
    });
    Ok(())
})?;
commands_table.set("set_collision_filter", set_collision_filter)?;
```

**Part B: Create `Loci.Physics` table and expose `raycast`**

Also inside `with_scoped_api`, expose the deterministic raycast. Note that `fixed_raycast` automatically normalizes the `direction` vector internally, so Lua developers can pass non-normalized vectors safely.

```rust
let physics_table = lua.create_table()?;

let raycast_fn = scope.create_function(|lua, (origin_val, dir_val, max_dist_val): (mlua::Value, mlua::Value, f64)| {
    let origin = extract_vector2(origin_val)?;
    let direction = extract_vector2(dir_val)?;
    let max_distance = fixed::types::I16F16::from_num(max_dist_val);
    
    // Call the deterministic raycast from the physics engine
    if let Some(hit) = crate::world::physics::collision::fixed_raycast(
        origin, 
        direction, 
        max_distance, 
        &instance.static_obstacles
    ) {
        let hit_table = lua.create_table()?;
        hit_table.set("fraction", hit.fraction.to_num::<f64>())?;
        hit_table.set("obstacle_id", hit.obstacle_id)?;
        hit_table.set("point_x", hit.point.x.to_num::<f64>())?;
        hit_table.set("point_y", hit.point.y.to_num::<f64>())?;
        hit_table.set("normal_x", hit.normal.x.to_num::<f64>())?;
        hit_table.set("normal_y", hit.normal.y.to_num::<f64>())?;
        Ok(Some(hit_table))
    } else {
        Ok(None)
    }
})?;
physics_table.set("raycast", raycast_fn)?;
loci_table.set("Physics", physics_table)?;
```

---

## 4. Implementation Checklist

- [ ] **`src/scripting/command.rs`** — Add `SetCollisionFilter` to `Command` enum.
- [ ] **`src/scripting/command.rs`** — Handle `SetCollisionFilter` in the command application loop, mutating `entity.collision_filter`.
- [ ] **`src/scripting/api.rs`** — Add `Loci.Commands.set_collision_filter(entity_id, layer, mask)` bridging to the command buffer.
- [ ] **`src/scripting/api.rs`** — Create `Loci.Physics` table in the Lua global scope.
- [ ] **`src/scripting/api.rs`** — Add `Loci.Physics.raycast(origin, direction, distance)` that calls `fixed_raycast` using `instance.static_obstacles`.
- [ ] **Testing:** Create a Lua script that applies a dash, using `raycast` to limit the dash distance if a wall is in the way, and `set_collision_filter` to temporarily pass through enemies. Verify determinism is maintained.

---

## 5. Files Unchanged in This Phase

| File | Reason |
|---|---|
| `src/world/physics/collision.rs` | `fixed_raycast` is already implemented and deterministic. We only need to call it. |
| `src/world/physics/map.rs` | `CollisionFilter` struct and bitwise logic are already implemented. |
| `src/world/entity.rs` | `Entity` already has `collision_filter`. |
| `sdks/love2d/loci_client.lua` | Client interpolation requires no changes. It simply receives the updated positions. |

---

## 6. Testing & Verification

1. **Unit Tests:** Verify that `Command::SetCollisionFilter` correctly mutates the `CollisionFilter` after `flush_and_apply`.
2. **Raycast Determinism:** Verify that `Loci.Physics.raycast` returns identical results across multiple executions with the same inputs.
3. **Replay Integration:** Verify that using `set_collision_filter` and `raycast` from Lua does not break event-sourced replay determinism (ADR-0010).
4. **Collision Filter Behavior:** Verify that entities with modified collision filters correctly ignore collisions with specified layers while maintaining collisions with others.

---

## 7. Out of Scope for This Phase (Future Work)

| Feature | Phase |
|---|---|
| Entity-against-Entity Raycasting | Future |
| Spherecast / Sweep Test | Future |
| Client-side Prediction of Dash | Future |
