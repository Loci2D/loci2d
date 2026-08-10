# Phase 4.1 Implementation Review — Findings & Recommendations

> **Document Type:** Temporary Review Notes & Recommendations  
> **Target Milestone:** Milestone 4.1 (Deterministic Engine Core & Fixed-Point Refactoring)  
> **Reference Specs:** [Phase 4 Spec](roadmap-specs/phase4-deterministic-replay-spec.md) · [ADR-0007](adr/en/0007-deterministic-simulation-and-fixed-point.md) · [ADR-0006](adr/en/0006-two-thread-network-gameloop-separation.md)

---

## 1. Executive Summary

Milestone 4.1 refactoring is structurally complete and fully complies with ADR-0007:
- Simulation arithmetic uses `I16F16` integer ALU math across [Entity](file:///home/furlan/Documents/workspace/loci2d/src/world/entity.rs) and [DeterministicVector2](file:///home/furlan/Documents/workspace/loci2d/src/world/fixed_point.rs).
- All entity and session collections in [Instance](file:///home/furlan/Documents/workspace/loci2d/src/world/instance.rs) have migrated from `HashMap` to `BTreeMap` for strict key ordering.
- Sub-millisecond fixed-timestep accumulator game loop is implemented in [GameLoop](file:///home/furlan/Documents/workspace/loci2d/src/game_loop/tick.rs).
- All 16 unit tests and 5 integration tests pass cleanly with zero compiler or clippy warnings.

The items below document **three defensive improvements** identified during review to ensure panic-resilience and robust bounds enforcement before proceeding to Milestone 4.2.

---

## 2. Findings & Recommendations

### Finding 1: Input Sanitization & Panic Resilience on Float Quantization (`from_f32`)

* **Location:** [src/world/fixed_point.rs](file:///home/furlan/Documents/workspace/loci2d/src/world/fixed_point.rs#L43-L50)
* **Risk Level:** **High (Denial of Service / Panic Vulnerability)**
* **Description:**
  In `DeterministicVector2::from_f32`, `I16F16::from_num(x)` is called directly:
  ```rust
  pub fn from_f32(x: f32, y: f32) -> Self {
      Self {
          x: I16F16::from_num(x),
          y: I16F16::from_num(y),
      }
  }
  ```
  In the `fixed` crate, `from_num` panics if the input `f32` is `NaN`, `+Infinity`, `-Infinity`, or outside the range `[-32768.0, 32767.99998]`.
  Because incoming `MoveIntent` payloads arrive from untrusted UDP network clients, a malformed or malicious packet containing `NaN` or huge float values (e.g., `1e10`) will panic the authoritative Game Loop thread and terminate server simulation.

* **Recommended Fix:**
  Use `I16F16::saturating_from_num(x)`. In the `fixed` crate, `saturating_from_num` converts `NaN` to `0` and clamps `±Infinity` or extreme float values to `I16F16::MIN` / `I16F16::MAX` without panicking:
  ```rust
  #[inline]
  pub fn from_f32(x: f32, y: f32) -> Self {
      Self {
          x: I16F16::saturating_from_num(x),
          y: I16F16::saturating_from_num(y),
      }
  }
  ```
* **Additional Test Case to Add:**
  ```rust
  #[test]
  fn test_from_f32_malformed_inputs() {
      let nan_vec = DeterministicVector2::from_f32(f32::NAN, f32::INFINITY);
      assert_eq!(nan_vec.x, I16F16::ZERO);
      assert_eq!(nan_vec.y, I16F16::MAX);

      let neg_inf_vec = DeterministicVector2::from_f32(f32::NEG_INFINITY, 100000.0);
      assert_eq!(neg_inf_vec.x, I16F16::MIN);
      assert_eq!(neg_inf_vec.y, I16F16::MAX);
  }
  ```

---

### Finding 2: Physics Step Coordinate Overflow Protection (`Instance::tick`)

* **Location:** [src/world/instance.rs](file:///home/furlan/Documents/workspace/loci2d/src/world/instance.rs#L156-L160)
* **Risk Level:** **Medium (Debug Build Panic)**
* **Description:**
  In `Instance::tick`, physics updates use standard `AddAssign`:
  ```rust
  for entity in self.entities.values_mut() {
      entity.position += entity.velocity;
  }
  ```
  `DeterministicVector2::add_assign` executes standard integer addition (`self.x += rhs.x`). If an entity moves continuously outside map boundaries without collision stops (or before Phase 6 collision boundaries are implemented), the coordinate will eventually exceed `32767` units and trigger an integer overflow panic in debug builds.

* **Recommended Fix:**
  Use the already implemented `saturating_add` method:
  ```rust
  for entity in self.entities.values_mut() {
      entity.position = entity.position.saturating_add(entity.velocity);
  }
  ```

---

### Finding 3: Direct Bound on Accumulator Clamping (`GameLoop::start`)

* **Location:** [src/game_loop/tick.rs](file:///home/furlan/Documents/workspace/loci2d/src/game_loop/tick.rs#L39-L46)
* **Risk Level:** **Low (Defensive Timing Optimization)**
* **Description:**
  Currently, `delta` is clamped before adding to `accumulator`:
  ```rust
  let now = Instant::now();
  let mut delta = now.duration_since(last_time);
  last_time = now;

  if delta > max_accumulator {
      delta = max_accumulator;
  }
  accumulator += delta;
  ```
  If previous iterations had a small residual duration left in `accumulator`, `accumulator` could momentarily reach `max_accumulator + old_accumulator`.

* **Recommended Fix:**
  Clamp `accumulator` directly to guarantee the inner while loop strictly never executes more than 5 fixed ticks per frame pass:
  ```rust
  let now = Instant::now();
  let delta = now.duration_since(last_time);
  last_time = now;

  accumulator = (accumulator + delta).min(max_accumulator);
  ```

---

## 3. Forward-Looking Notes for Upcoming Milestones

1. **Milestone 4.2 (Event Sourcing & Protobuf Replay Serialization):**
   - Ensure `ReplayRecorder` stamps events with `entity_id` rather than ephemeral `SocketAddr`.
   - `EntityType::as_u8()` in `entity.rs` is already prepared for `compute_canonical_state_hash`.

2. **Milestone 4.3 (Headless Replay & Hash Verification):**
   - Checkpoint hash calculation will hash raw integer bits (`position.x.to_bits().to_be_bytes()`) directly from `I16F16`.
