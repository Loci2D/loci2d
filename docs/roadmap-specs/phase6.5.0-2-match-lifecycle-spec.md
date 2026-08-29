# Implementation Spec — Phase 6.5.0-2: Match Lifecycle & Timers
> **Status:** Completed  
> **Roadmap Phase:** Phase 6.5.0-2
> **Reference ADRs:** [ADR-0007](../adr/en/0007-deterministic-simulation-and-fixed-point.md) · [ADR-0014](../adr/en/0014-embedded-lua-scripting-and-command-buffer.md)

---

## 1. Context & Technical Motivation

Following the data model establishment (Phase 6.5.0-1), this milestone introduces control over the flow of time and match state. We need to allow Lua to pause the simulation (e.g., waiting for players to connect) and permanently end the match. We also introduce a deterministic timer system for game logic like ability cooldowns.

---

## 2. Technical Implementation Modules

### 2.1 Match Lifecycle State Machine

Currently, the server `Instance` ticks unconditionally. We need to introduce a lifecycle.

```rust
pub enum MatchState {
    Paused,
    Running,
    Ended { winner_data: String },
}

pub struct Instance {
    // ...
    pub state: MatchState,
}
```

* **Logic Update:** In `Instance::tick()`, skip physics updates and `on_tick` Lua callbacks if `state` is not `Running`. If `Ended`, the server should also stop accepting client `ActionIntent` packets and finalize the replay file.
* **Callback Availability During Pause:** Player connection callbacks (`on_player_join`, `on_player_leave`) MUST be invoked regardless of `MatchState`, since they are required for lobby logic.
* **Lua Control:** Expose `Loci.Commands.start_match()`, `Loci.Commands.pause_match()`, and `Loci.Commands.end_match(winner_data: string)`.

---

### 2.2 Timer & Cooldown System

Timers are essential for game logic (e.g., "apply poison damage over 5 seconds", "ability cooldown"). These must be resolved deterministically on the server tick.

#### Storage (ADR-0007 Compliant)
```rust
pub struct ActiveTimer {
    pub timer_id: String,
    pub remaining_ticks: u32,
}

pub struct Instance {
    // ...
    /// Key: timer_id. MUST be BTreeMap to guarantee stable execution order.
    pub active_timers: BTreeMap<String, ActiveTimer>, 
}
```

#### Execution Flow
1. **Lua requests timer:** `Loci.Commands.start_timer("spawn_wave_1", 300)` (300 ticks).
2. **Command Buffer:** Applies `Command::StartTimer` to the `Instance`, inserting into `active_timers`.
3. **Tick Logic:** `Instance::tick()` decrements `remaining_ticks` for all timers. **Crucial:** This decrement is gated by the same `Running` check as `on_tick`. If the match is `Paused` or `Ended`, timers freeze and do not count down.
4. **Trigger:** If a timer reaches `0`, it is removed from the `BTreeMap` and its ID is passed to `ScriptEngine::on_timer_complete(timer_id)`.

---

## 3. Testing & Verification

1. **Determinism Verification:** Ensure `ActiveTimer` decrements precisely on `Instance::tick()` and that multiple timers expiring on the same tick trigger `on_timer_complete` in alphabetical order (guaranteed by `BTreeMap`).
2. **State Machine Lock:** Verify that `on_tick` and physics are skipped when `MatchState::Paused`, but `on_player_join` can still fire.
