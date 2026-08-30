# Implementation Spec — Phase 6.5.1: Architecture, Safety & Determinism Validation
> **Status:** Pending
> **Roadmap Phase:** Phase 6.5.1
> **Reference ADRs:** [ADR-0007](../adr/en/0007-deterministic-simulation-and-fixed-point.md) · [ADR-0014](../adr/en/0014-embedded-lua-scripting-and-command-buffer.md) · [ADR-0015](../adr/en/0015-dedicated-roadmap-structure-phase6.5-validation-dx.md)

---

## 1. Context & Technical Motivation

Phase 6.5.1 focuses on hardening the engine architecture by fully decoupling the canonical game state (deterministic physics, spatial data) from Lua script execution. As outlined in [ADR-0014](../adr/en/0014-embedded-lua-scripting-and-command-buffer.md), allowing user scripts to mutate state directly mid-tick poses stability risks, such as iterator invalidation during physics resolution.

Additionally, to guarantee event-sourced replay integrity across different operating systems and hardware architectures (x86_64, ARM64), we must mathematically prove that our `I16F16` fixed-point arithmetic ([ADR-0007](../adr/en/0007-deterministic-simulation-and-fixed-point.md)) is deterministic through an automated Continuous Integration (CI) suite.

---

## 2. Technical Implementation Modules

### 2.1 Strict State vs. Scripting Separation

The canonical match state, managed by the Rust core, must be strictly isolated from the embedded Lua runtime. 
* **Immutable Access:** During a tick (e.g., in `on_tick` or `on_collision` callbacks), Lua scripts should only have read-only access to the current physics state or spatial data.
* **Decoupling Validation:** Ensure that no Rust iterators over entities or physics components can be invalidated by Lua script execution (e.g., deleting an entity during collision resolution).

### 2.2 Safe Dispatch Layer (Command Buffer)

Implement a safe command/intent dispatch layer (Command Buffer) to enforce the separation defined above.
* **Deferred Execution:** Any state mutations requested by Lua (such as destroying entities, spawning entities, or altering physics parameters) must be pushed into a deferred Command Buffer.
* **End-of-Tick Application:** The Rust engine must flush and apply these queued commands sequentially at the end of the tick cycle, ensuring all iterators are dropped before state mutations occur.
* **API Ergonomics:** To maintain a good Developer Experience (DX), entity creation commands should pre-allocate and return an ID synchronously, allowing scripts to continue referencing the new entity within the same tick.

### 2.3 Determinism CI Suite & Benchmarks

Establish a rigorous testing environment to guarantee deterministic execution across architectures.
* **Cross-Platform CI:** Configure a GitHub Actions (or similar) CI matrix that compiles and runs the physics/scripting tests on both `x86_64` (e.g., standard runners) and `ARM64` (e.g., macOS ARM runners or QEMU) architectures.
* **Mathematical Proof:** The CI must run simulations utilizing the `I16F16` fixed-point math and hash the final world states. The hashes *must* match exactly across all architectures to pass.
* **Stress Testing:** Update `benchmark.rs` and core tests to aggressively stress-test Lua Gameplay Actions (spawning, collisions, movement adjustments) to ensure they do not introduce non-determinism or desyncs under heavy load.

---

## 3. Testing & Verification

1. **Iterator Safety:** Write a test that attempts to destroy an entity from Lua during an `on_collision` callback. Verify that the command is deferred and does not cause a Rust panic/iterator invalidation.
2. **Command Application:** Verify that queued commands from the Command Buffer are applied in the correct deterministic order at the end of the tick.
3. **Cross-Architecture Hash Matching:** Run the core deterministic tests on an `x86_64` machine and an `ARM64` machine. Verify that for identical inputs and seeds, the final state hashes are mathematically identical.
4. **Benchmark Stability:** Ensure `benchmark.rs` runs without crashing and maintains consistent state resolution when executing a large volume of Lua intents over multiple ticks.
