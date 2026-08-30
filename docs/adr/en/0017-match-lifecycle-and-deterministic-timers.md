# ADR 0017: Match Lifecycle State Machine and Deterministic Timers

## Status
Accepted

## Context
During Phase 6.5.0-2, we introduced the ability to pause the match (e.g., waiting for players to connect) and manage time-based game logic (timers and cooldowns). 
Previously, the server engine ticked unconditionally (ADR-0006). With the introduction of an embedded Lua script engine (ADR-0014), we faced architectural choices regarding "who owns time" and "how do we pause?". 
We could have allowed Lua to handle timers internally (e.g., decrementing variables on every `on_tick`) and ignore inputs when the game is "paused" in Lua, leaving the Rust core running continuously. However, this would dilute the strict deterministic boundaries of the engine.

## Decision

### 1. Match Lifecycle Managed by Rust Core
We decided to implement a formal `MatchState` (`Paused`, `Running`, `Ended`) directly in the Rust core `Instance`.
- **Replay Mirroring:** The replay file must strictly mirror the live match state. If the match is paused, the replay is paused (ticks are frozen). While this prevents out-of-band features like text-chat from being recorded or processed during a pause, we deliberately defer this limitation to post-MVP to avoid over-engineering.
- **Boot State:** The Rust core will *not* boot into a `Paused` state by default. The engine will start as `Running`. It is the responsibility of the game logic (e.g., initialization scripts) to immediately issue a `pause_match` command if they wish to wait for player connections. This avoids a massive Rust refactor to handle pre-game states and is an acceptable pattern for this type of engine.
- **Callback Availability:** Even when `Paused` or `Ended`, connection callbacks (`on_player_join`, `on_player_leave`) will still fire to allow lobby management.

### 2. Deterministic Timers Managed by Rust
We decided to prohibit Lua from self-managing timers via standard loop counters. Instead, all timers must be requested via `Loci.Commands.start_timer`.
- The Rust core manages these in a `BTreeMap<String, ActiveTimer>` and decrements them on every `tick()`.
- This enforces strict, alphabetical, deterministic execution of timer callbacks (`on_timer_complete`) and offloads the temporal state management from Lua to the highly optimized, determinism-proven Rust core.

## Consequences
- **Positive:** We guarantee perfect determinism for time-based events regardless of how complex the Lua scripts get.
- **Positive:** Replay files remain clean and do not accumulate empty "paused" ticks unnecessarily.
- **Negative:** Future features that require real-time processing regardless of the simulation state (like global text chat) will require a structural refactor, as the current architecture freezes the entire tick pipeline when paused.
