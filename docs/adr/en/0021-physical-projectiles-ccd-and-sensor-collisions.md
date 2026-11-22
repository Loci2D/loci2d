# ADR 0021: Physical Projectiles, Swept Continuous Collision Detection (CCD), and Sensor Resolution Strategy

## Status

Accepted

## Context

In [ADR-0012](0012-deterministic-2d-collision-and-kinematic-resolution.md), `loci2d` established a deterministic 2D fixed-point collision engine (`I16F16`) supporting circular and AABB colliders, static obstacle wall-sliding (100% pushback), dynamic entity-vs-entity resolution (50/50 split pushback), and on-demand point raycasting (`fixed_raycast`).

While this foundation supports character movement, melee combat, and instant hitscan queries, real-time action games (such as `loci-arena`) require **persistent, multi-tick physical projectiles** (e.g., skillshots, thrown blades, fireballs, ricocheting arrows). Auditing the engine and client SDK revealed five architectural gaps when representing projectiles as standard dynamic entities:

1. **Lack of First-Class Projectile Entity Type**:
   - `Command::SpawnEntity` only recognizes `"Player"` and `"Enemy" | "NPC"`, falling back to `EntityType::Prop` with `CollisionFilter::default_player()` and attaching a `NavigationComponent`.
   - `CollisionFilter::default_projectile()` exists in Rust (`src/world/physics/map.rs`), but is unused during entity creation, and `game_packets.proto` lacks `EntityType::PROJECTILE` for native client identification.

2. **Unwanted 50/50 Kinematic Pushback on Impact**:
   - In `simulation::tick`, `resolve_dynamic_collision` unconditionally applies a 50/50 Minimum Translation Vector (MTV) positional pushback and velocity projection to both colliding entities before invoking Lua's `on_collision(id_a, id_b)`.
   - Consequently, a projectile striking a player physically shoves the player backward prior to script execution, violating expected sensor-like projectile impact semantics and conflicting with game-specific knockback rules ([ADR-0016](0016-data-driven-entity-properties-and-engine-agnosticism.md)).

3. **Silent Static Obstacle and Arena Boundary Impacts**:
   - Collisions against solid `StaticObstacle` instances and `MapBounds` silently clamp or slide the entity's position in Rust without notifying the Lua scripting layer.
   - Scripts have no deterministic callback or surface normal $\vec{n}$ to destroy a projectile upon hitting a wall, trigger an area-of-effect explosion at the impact point, or compute reflection vectors for ricocheting projectiles.

4. **High-Velocity Discrete Tunneling**:
   - Standard discrete integration ($\vec{p}_{t+1} = \vec{p}_t + \vec{v}$) evaluates overlap only at the candidate end position $\vec{p}_{t+1}$.
   - Fast projectiles moving further than their diameter in a single tick ($\|\vec{v}\| > 2r$) can tunnel completely through thin walls or player colliders between ticks.

5. **Network Bandwidth vs. Short-Lived Client Lifecycle**:
   - Broadcasting raw per-tick collision events over UDP would bloat `ServerPacket` payloads and violate the snapshot-centric design ([ADR-0004](0004-packet-serialization-format.md), [ADR-0018](0018-client-sdks-and-network-abstraction.md)).
   - At the same time, short-lived projectiles (spawned and destroyed within a few ticks) suffer from a 50ms initial freeze in the client SDK's snapshot interpolator while waiting for a second buffered snapshot, and `on_entity_despawned(id)` discards the entity table before client VFX/SFX callbacks can read its final impact coordinates.

## Decision

We decide to extend the `loci2d` physics and scripting architecture—and align the `sdk-love2d` client runtime—with **first-class physical projectiles, non-pushback dynamic sensor resolution, swept Minkowski Continuous Collision Detection (CCD), and deterministic surface-normal impact callbacks**:

### 1. First-Class `EntityType::Projectile` & Native Lifecycle (`lifetime_ticks`)
- Promote `Projectile` (`EntityType::Projectile` in Rust, `PROJECTILE = 3` in `game_packets.proto`) to a native entity variant.
- When spawned via `Loci.Commands.spawn_entity({ entity_type = "Projectile", ... })`:
  - The entity defaults to `CollisionFilter::default_projectile()` (`layer = PROJECTILE`, `mask = SOLID_WALL | PLAYER`), `is_sensor = true`, and `navigation = None`.
  - Support optional `velocity = { x, y }` and `lifetime_ticks = N` directly at spawn time (and via `Loci.Commands.set_lifetime`).
  - Entities with `remaining_lifetime_ticks: Some(n)` decrement deterministically at the start of each running tick and automatically despawn when reaching `0`, preventing leaked projectiles without requiring boilerplate Lua timers.

### 2. Dynamic Sensor Mode (`is_sensor`) Without 50/50 Pushback
- Add `pub is_sensor: bool` to `Entity` (default `true` for `Projectile`, `false` for `Player`, `NPC`, `Prop`; mutable via `Loci.Commands.set_sensor(entity_id, is_sensor)`).
- During dynamic entity-vs-entity narrowphase resolution:
  - If either entity in a colliding pair has `is_sensor == true`, the engine **skips `resolve_dynamic_collision` (zero MTV pushback and zero velocity projection on the target)** while still invoking `on_collision(entity_a, entity_b)` in strict ascending ID order.
  - Game rules remain 100% owned by Lua ([ADR-0016](0016-data-driven-entity-properties-and-engine-agnosticism.md)): the script decides whether to destroy the projectile, pierce through the target, or apply explicit knockback.

### 3. Static Obstacle & Boundary Impact Callbacks with Surface Normals
- Expose two new optional Lua lifecycle callbacks in `ScriptEngine`:
  - `on_static_collision(entity_id, obstacle_id, normal_x, normal_y)`
  - `on_boundary_collision(entity_id, normal_x, normal_y)`
- Whenever an entity collides with a solid `StaticObstacle` (evaluated in ascending `obstacle.id` order) or is clamped by `MapBounds`, the engine passes the outward unit contact normal $\vec{n} = (\text{normal\_x}, \text{normal\_y})$ to Lua.
- For sensor entities (`is_sensor == true`), the entity's position is resolved to the exact contact surface without projecting/zeroing its velocity prior to the callback, allowing Lua scripts to either call `destroy_entity(entity_id)` or compute deterministic bounce reflections ($\vec{v}' = \vec{v} - 2(\vec{v} \cdot \vec{n})\vec{n}$).

### 4. Deterministic Swept-Circle CCD (Minkowski Sum Raycasting)
- To prevent tunneling without introducing floating-point operations or iterative sub-stepping overhead (while preserving standard discrete MTV wall-sliding for `Player`, `NPC`, and `Prop` entities):
  - During velocity integration in `simulation::tick`, entities with `EntityType::Projectile` perform a **swept circle test** along the segment $\vec{p}_0 \to \vec{p}_0 + \vec{v}$ (ignoring receding or tangential surface contacts at $t = 0$ so ricocheting projectiles move freely away).
  - Sweeping a circle of radius $r_p$ against target shapes is computed via 100% deterministic `I16F16` / 64-bit integer ray intersection (`ray_intersect_aabb` and `ray_intersect_circle`) against the **Minkowski-expanded shapes**:
    - Target `DeterministicCircle(c, r_t)` is expanded to radius $R = r_p + r_t$.
    - Target `DeterministicAABB(min, max)` is expanded by $r_p$ on each axis (`min - r_p`, `max + r_p`).
  - Solid `StaticObstacle` intersections clamp the travel segment to the earliest time-of-impact $t_{\min}$ (breaking ties deterministically by ascending `obstacle.id`) and fire `on_static_collision`; the wall-clamped segment is then swept against dynamic entities without snagging on teammates or piercing targets, firing `on_collision` for every swept target in front of any occluding wall.

### 5. Snapshot-Centric Client Replication & SDK Ergonomics (No Raw Collision Packets)
- We explicitly **reject** adding a raw `repeated CollisionEvent` field to `ServerPacket` / `WorldState`. Per-tick collision arrays create network spam and duplicate state transitions already observable via entity lifecycle and properties.
- Instead, in `Loci2D/sdk-love2d`:
  - Ingest `EntityType::PROJECTILE` (`3`) into `entity.entity_type = "projectile"` and expose `Entity:is_projectile()` and `loci.get_projectiles()`.
  - Eliminate the 50ms initial spawn freeze for newly spawned projectiles in `loci/netcode/interpolation.lua` by forward-integrating along authoritative velocity `(server_vx, server_vy)` when the entity exists only in the newer snapshot of the interpolation window.
  - Pass the full `entity` table as the second argument to `loci.on_entity_despawned(entity_id, entity)` (with `entity.x, entity.y` snapped to its final authoritative coordinates `server_x, server_y`), allowing clients to spawn impact VFX/SFX at the exact point of destruction while maintaining 100% backward compatibility with existing `function(id)` handlers.

### 6. Backwards-Compatible Canonical Replay Hashing
- To preserve bit-exact verification for existing `.loci` replays (such as `benchmark.loci`, [ADR-0010](0010-event-sourced-replay-format.md)), `compute_canonical_state_hash` in `src/replay/hash.rs` only appends tag-prefixed bytes for `is_sensor` (`0x5E`) when `entity.is_sensor == true` and `remaining_lifetime_ticks` (`0x4C` + `u32` big-endian) when `Some(_)`. Standard `Player` entities produce identical SHA-256 digests.

## Consequences

**Positive:**
- **100% Bit-Exact Determinism Preserved**: Swept Minkowski raycasting reuses 64-bit integer square-root math (`integer_sqrt_u64`), guaranteeing identical projectile trajectories and impact ticks across x86_64 and ARM64.
- **Crisp Combat Feel**: Projectiles no longer shove players via 50/50 rigid pushback, never tunnel through walls or targets at high speeds, and cleanly trigger impact or ricochet logic on obstacles and arena borders.
- **Engine Agnosticism Maintained ([ADR-0016](0016-data-driven-entity-properties-and-engine-agnosticism.md))**: Zero combat rules (damage, stun, piercing, bounce limits) are hardcoded in Rust; Lua scripts compose all projectile behaviors via callbacks and properties.
- **Zero Protocol Bloat**: Replicates projectiles cleanly through existing `EntityState` snapshots and lifecycle callbacks without per-tick collision packet spam.

**Negative:**
- **Slight Per-Tick CPU Cost for High-Speed Entities**: Swept Minkowski raycasting evaluates line-segment intersections against collidable obstacles and entities for active projectiles, though bounded to $O(N_{\text{projectiles}} \times (N_{\text{obstacles}} + N_{\text{entities}}))$ with fast bitmask filtering.
