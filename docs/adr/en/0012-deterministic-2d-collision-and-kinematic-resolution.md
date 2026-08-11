# ADR 0012: Deterministic 2D Collision Engine and Kinematic Resolution Strategy

## Status

Accepted (Phase 5)

## Context

For `loci2d` to support meaningful gameplay interactions (walls, static obstacles, map perimeters, entity collisions, sensor zones), the engine requires a spatial collision detection and resolution subsystem.

Standard game development practices often reach for external physics libraries (such as Rapier2D, Box2D, or PhysX). However, incorporating these external physics engines into `loci2d` creates major architectural conflicts:

1. **Floating-Point Non-Determinism ([ADR-0007](0007-deterministic-simulation-and-fixed-point.md))**:
   - Most physics libraries rely on IEEE 754 floating-point operations (`f32`/`f64`), SIMD optimizations (AVX, SSE, NEON), and compiler auto-vectorization.
   - Over multi-thousand tick matches, slight floating-point rounding variations between CPUs (x86_64 vs ARM64) cause cumulative divergence, breaking bit-exact `.loci` replay reproducibility ([ADR-0010](0010-event-sourced-replay-format.md)).

2. **Rigid-Body Simulation Overhead ([ADR-0003](0003-2d-map-only.md))**:
   - Rigid-body physics engines solve complex constraint systems: mass moments of inertia, angular velocity, torque, friction impulses, and continuous collision detection (CCD).
   - For 2D top-down, MOBA, RTS, and isometric action games, rigid-body momentum physics often feels imprecise, floaty, and difficult to tune. Players expect crisp, immediate, and responsive kinematic movement.

3. **Dependency Footprint and Complexity ([ADR-0001](0001-authoritative-server-archoitecture.md))**:
   - Heavy physics dependencies significantly increase compilation times, binary size, and the barrier to entry for indie developers.

## Decision

We decide to implement a **built-in, lightweight, 100% deterministic fixed-point 2D collision engine** with **kinematic Minimum Translation Vector (MTV) resolution**:

### 1. Fixed-Point Geometric Primitives (`I16F16`)
Collision detection is restricted to 2D geometric shapes implemented entirely with `I16F16` fixed-point arithmetic:
- **`DeterministicAABB`**: Axis-Aligned Bounding Box for walls, obstacles, rectangular trigger areas, and spatial queries.
- **`DeterministicCircle`**: Circular collider for players, NPCs, projectiles, and radial triggers.
- **Composite Colliders**: Directional skillshots, swinging melee arcs, and rotated attack hitboxes are approximated using clusters of overlapping circles/boxes rather than complex rotated polygon solvers (SAT), adhering to the 2D-only simplicity philosophy of [ADR-0003](0003-2d-map-only.md).

### 2. Deterministic Integer Square Root (`fixed_sqrt`)
Circle distance calculations and normalization avoid standard `f32::sqrt`. Instead, we use a pure integer restoring bitwise square root algorithm operating directly on `I16F16` raw bits, guaranteeing identical results across all CPU architectures and platforms.

### 3. Collision Filtering & Interaction Layers
Entities and static obstacles use a 16-bit bitmask `CollisionFilter` (`layer` and `mask`) to explicitly specify which categories interact (e.g. `SOLID_WALL`, `PLAYER`, `TRIGGER_ZONE`, `PROJECTILE`), preventing unnecessary intersection calculations.

### 4. Kinematic MTV Pushback & Wall Sliding
Rather than simulating forces, mass impulses, or restitution:
- Solid collisions calculate the **Minimum Translation Vector (MTV)** and normal $\vec{n}$.
- The penetrating entity is pushed back out along $\vec{n}$ by the penetration depth.
- The entity's velocity is projected along the surface tangent ($\vec{v}_{\text{slide}} = \vec{v} - (\vec{v} \cdot \vec{n})\vec{n}$), preventing player snagging and producing smooth, responsive sliding against walls.

### 5. Deterministic Collision Resolution & Trigger Event Ordering
- Dynamic collision pairs are normalized with `entity_a.id < entity_b.id` and stored in a `BTreeSet<(u64, u64)>`, guaranteeing identical pair processing order across machines.
- Static obstacles are evaluated in strict ascending order of `obstacle.id` (`BTreeMap<u64, StaticObstacle>`).
- Trigger zone overlaps fire `Enter`, `Stay`, and `Exit` lifecycle events in strict `(trigger_id, entity_id)` tuple order.

### 6. Tunneling Prevention (CCD Mitigation)
To prevent fast entities from passing through thin obstacles:
- Per-tick discrete kinematic displacements are clamped to $\le \frac{1}{2} \min(\text{width}, \text{height})$ of the bounding collider.
- Fast projectiles utilize deterministic fixed-point line-segment raycasting (`fixed_raycast`) rather than discrete multi-tick stepping.

### 7. Deterministic Spatial Broadphase
For scenarios with growing entity counts, broadphase candidate pair pruning uses a uniform spatial hash grid with configurable cell size. Candidate pairs are deduplicated and ordered using strictly sorted collections (`BTreeSet<(u64, u64)>`).

## Consequences

**Positive:**
- **100% Bit-Exact Determinism**: Preserves cross-platform SHA-256 state hash equality for match replays (`.loci`).
- **Responsive Kinematic Controls**: Tight, predictable movement ideal for top-down, MOBA, and RTS control models.
- **Zero External Physics Dependencies**: Keeps loci2d lightweight, fast to compile, and easy to maintain.
- **Direct Scripting Integration**: Provides clean collision and trigger event hooks for Phase 6 Lua scripting.

**Negative:**
- **No Complex Rigid-Body Dynamics**: Does not support joint physics, rotational inertia, stacking physics, or ragdolls (unnecessary for 2D top-down authoritative gameplay).
- **Geometric Simplification**: Restricted to AABBs and Circles; arbitrary rotated polygons are out-of-scope and replaced with composite circular/box clusters.
