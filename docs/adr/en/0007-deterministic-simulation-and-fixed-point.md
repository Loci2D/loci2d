# ADR 0007: Deterministic Simulation and Fixed-Point Arithmetic Strategy

## Status

Accepted (Active in Phase 4)

## Context

For `loci2d` to support **deterministic replays** and **event-sourced match recording** (Phase 4), game simulation state must execute identically on any platform (Windows, Linux, macOS) and CPU architecture (x86_64, ARM64).

During Phase 1–3 MVP validation, standard floating-point operations (`f32`) and standard hash maps (`HashMap`) were used for simplicity. However, they exhibit known non-deterministic behaviors:

1. **Floating-Point Non-Determinism (`f32` / `f64`)**:
   - Hardware rounding rules, FMA (Fused Multiply-Accumulate) instructions, subnormal handling, and CPU SIMD extensions (x86 AVX/SSE vs ARM NEON) differ across chipsets.
   - Over hundreds or thousands of ticks, slight rounding differences accumulate and cause state desynchronization (butterfly effect).

2. **Collection Iteration Non-Determinism (`HashMap`)**:
   - Rust's standard `HashMap` uses `RandomState` with a per-process random seed (SipHash) to prevent HashDoS attacks.
   - Iterating over entities in a `HashMap` yields different entity execution orders between server restarts and across machines.

3. **OS Sleep Interrupt Granularity**:
   - `thread::sleep` timing varies across OS kernels (e.g. Windows ~15.6 ms interrupt resolution vs Linux ~1 ms high-resolution timers).

## Decision

We decide to maintain `f32` and `HashMap` during initial MVP phases (Phase 1–3) for developer ergonomics, but enforce strict determinism rules when entering **Phase 4**:

1. **Fixed-Point Arithmetic (`I32F32`) in Phase 4**:
   - Replace `f32` vector coordinates and physics state with fixed-point integers (e.g. `fixed` crate or scaled 32-bit integers).
   - Guarantees bit-exact calculation results across all CPU architectures and compilers.

2. **Ordered Entity Collections (`BTreeMap`)**:
   - Replace `HashMap<u64, Entity>` with `BTreeMap<u64, Entity>` in `Instance`.
   - Guarantees strict, deterministic key-sorted iteration order (`1, 2, 3...`) across all platforms.

3. **Fixed-Timestep Accumulator Loop**:
   - Replace naive `thread::sleep(budget - elapsed)` with a fixed-timestep accumulator loop to eliminate tick timing drift across OS platforms.

4. **Tick-Indexed Input Log**:
   - Record inputs alongside target tick execution indices, ensuring replays consume inputs at the exact frame step regardless of real-time arrival jitter.

## Consequences

**Positive:**
- **100% Bit-Exact Replays**: Match input logs produce identical SHA256 `WorldState` hashes on x86_64, ARM64, and WebAssembly.
- **Cross-Platform Parity**: Server behavior is completely independent of host OS timer resolution.
- **Low Memory Overhead**: Event-sourced replays only need to store client inputs, not full tick snapshot state.

**Negative:**
- **Fixed-Point Ergonomics**: Converting float math to fixed-point requires custom helper utilities or crate dependencies.
- **`BTreeMap` Lookup Cost**: `O(log N)` search cost compared to `O(1)` hash map lookups (negligible for expected instance entity counts).
