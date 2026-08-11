# Code Review: Phase 5 Physics & Collision Spec and ADRs

**Reviewed Documents:**
- `docs/roadmap-specs/phase5-physics-and-collision-spec.md`
- `docs/adr/en/0012-deterministic-2d-collision-and-kinematic-resolution.md`
- `docs/adr/en/0013-server-authoritative-destination-steering-and-navigation.md`

**Review Date:** 2026-08-11

---

## Critical Issues (Must Fix Before Implementation)

### 1. Bug in `fixed_sqrt` Implementation
**Location:** Spec lines 152-180

The integer square root algorithm is incomplete:

```rust
if scaled >= root + bit {
    let temp = root + bit;
    root = (root >> 1) + bit;
    // Subtract using intermediate to satisfy integer algorithm
} else {
    root >>= 1;
}
```

The comment states "Subtract using intermediate" but **no subtraction occurs**. The algorithm should subtract `temp` from `scaled`. This will produce incorrect square root results, breaking determinism.

**Fix:** Add `scaled -= temp;` after line 171.

---

### 2. Determinism Risk: `VecDeque` in NavigationComponent
**Location:** Spec line 369

```rust
pub struct NavigationComponent {
    pub waypoints: std::collections::VecDeque<DeterministicVector2>,
}
```

`VecDeque` iteration order is deterministic within a single Rust version, but the spec emphasizes strict determinism across platforms. Consider:
- Document that `VecDeque` iteration order is stable
- Or use `BTreeMap<usize, DeterministicVector2>` for explicitly guaranteed ordering
- Or convert to `Vec<DeterministicVector2>` with an index pointer

---

### 3. Missing Normalization in Circle-Circle Collision
**Location:** Spec line 214

The spec states: "Separation normal is $\Delta / \text{dist}$" but doesn't specify handling when `dist == 0` (identical centers). This would cause division by zero in fixed-point arithmetic.

**Fix:** Add explicit handling for zero-distance case (e.g., use arbitrary axis or skip collision).

---

## ADR-0012 Review Issues

### Strengths
- Clear rationale for avoiding external physics libraries
- Well-argued tradeoffs between determinism and rigid-body complexity
- Good alignment with ADR-0007 (fixed-point determinism)

### Issues

#### 1. Rotated Geometry & Hitboxes (AABB Limitations)
**Location:** ADR-0012 overall, Spec lines 85-144

Since `DeterministicAABB` is strictly axis-aligned, angled hitboxes or directional attacks (e.g., a swinging sword arc or a rotated cone) cannot be natively represented by a single rotated box.

**Mitigation / Pattern:** Document the use of composite shapes—such as a series of overlapping `DeterministicCircle` primitives—to approximate non-axis-aligned skill shots and melee hitboxes. If complex arbitrary polygons are needed in the future, the Separating Axis Theorem (SAT) can be implemented in a later ADR.

---

#### 2. Tunneling Prevention for High-Speed Entities (Lack of CCD)
**Location:** ADR-0012 overall, Spec lines 413-438

Without Continuous Collision Detection (CCD), fast-moving entities or high-velocity projectiles could "teleport" through thin colliders (e.g., walls) if their per-tick displacement exceeds the collider's thickness.

**Mitigation / Pattern:** Add implementation guidance to enforce a maximum speed cap per tick relative to the smallest collider size, or implement deterministic fixed-point raycasting (`fixed_raycast`) specifically for high-speed projectile intents instead of relying on discrete spatial movement ticks.

---

#### 3. SAT for Rotated Polygons Mentioned as Future
**Location:** ADR-0012 line 58

The consequence mentions "arbitrary rotated polygons require... SAT in future revisions". This conflicts with ADR-0003 (2D map only) which suggests keeping geometry simple. Consider whether rotated polygons are actually in scope or should be explicitly out of scope.

---

#### 4. Missing Collision Layer Documentation
**Location:** ADR-0012 overall

ADR mentions collision detection but doesn't address the layer/mask system from the spec. Should reference or define the collision filtering strategy.

---

## ADR-0013 Review Issues

### Strengths
- Excellent comparison table of client-side vs server-side steering
- Clear explanation of jitter prevention with arrival tolerance
- Good coverage of intent preemption logic

### Issues

#### 1. Server CPU Assessment
**Location:** ADR-0013 line 61

States CPU work is "negligible" without quantification. For 1000+ entities with navigation, this could be significant. Consider adding a performance benchmark or target metric.

---

#### 2. Latency Mitigation Vague
**Location:** ADR-0013 line 62

Mentions "client-side visual indicators" but doesn't specify implementation. Should clarify whether this is in Phase 5 scope or deferred.

---

## Cross-Document Consistency Issues

### 1. Trigger Event Ordering
**Location:** Spec line 331, ADR-0012 line 43

Both documents mention `Enter`/`Stay`/`Exit` events but neither specifies the **deterministic ordering** when multiple entities enter/exit the same trigger in the same tick. This could cause replay divergence.

**Recommendation:** Specify that trigger events are ordered by `(trigger_id, entity_id)` tuple using `BTreeSet` ordering.

---

### 2. Spatial Hash Grid Cell Size
**Location:** Spec line 388

Spec suggests "64 × 64 units" with no justification for this value. Should be configurable or derived from map bounds and entity sizes.

---

### 3. Collision Resolution Iteration Order
**Location:** Spec line 51

Spec mentions "Strict BTreeMap Order" but doesn't specify whether collision pairs are processed in `(min_id, max_id)` order. This is critical for determinism when multiple collisions affect the same entity.

---

## Minor Issues

### Spec Issues

#### 1. debug_assert! in Production Code
**Location:** Spec line 98

`debug_assert!` in production code - consider using `assert!` or documenting the invariant.

---

#### 2. Wall Sliding Zero-Length Normal
**Location:** Spec line 318

Wall sliding checks `dot < I16F16::ZERO` but doesn't handle the case where the normal is zero-length.

---

#### 3. Test File Naming Inconsistency
**Location:** Spec line 455

Test file naming inconsistent - uses `physics_primitives_test.rs` instead of `physics_primitives.rs` with `#[cfg(test)]`.

---

### ADR-0012 Issues

#### 1. BTreeSet Ordering Not Specified
**Location:** ADR-0012 line 46

Mentions `BTreeSet<(u64, u64)>` but doesn't specify the ordering (should be `(min_id, max_id)`).

---

### ADR-0013 Issues

#### 1. Arrival Tolerance Description
**Location:** ADR-0013 line 44

Arrival tolerance described as "proportional to move_speed" but spec shows it as a separate field.

---

## Positive Observations

1. **Excellent technical depth** on fixed-point arithmetic and determinism requirements
2. **Clear milestone breakdown** makes implementation tractable
3. **Good integration** with existing ADRs (0001, 0003, 0007, 0010, 0011)
4. **Comprehensive test plan** with cross-platform replay verification
5. **Well-structured diagrams** showing the physics pipeline evolution

---

## Summary

### Must Fix Before Implementation:
1. Complete the `fixed_sqrt` algorithm (add subtraction)
2. Handle zero-distance case in circle-circle collision
3. Specify deterministic trigger event ordering

### Should Address:
4. Document or replace `VecDeque` for determinism guarantees
5. Quantify server CPU overhead for navigation
6. Specify collision pair processing order

### Nice to Have:
7. Make spatial hash grid cell size configurable
8. Add collision layer documentation to ADR-0012
9. Clarify client-side latency mitigation scope
