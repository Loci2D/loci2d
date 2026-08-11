# Code Review: Milestone 4.2 - Event Sourcing & Protobuf Replay Serialization

**Date:** 2026-08-11
**Commit:** Implementation of Event Sourcing & Protobuf Replay Serialization
**Spec:** `docs/roadmap-specs/phase4-deterministic-replay-spec.md` (Milestone 4.2)

---

## Overview
The implementation correctly implements the event-sourced replay system with Protobuf serialization. The code follows the spec closely with some thoughtful enhancements.

---

## ✅ Strengths

### Schema Compliance
- `proto/replay.proto` matches the spec exactly with all required messages
- Proper import of `game_packets.proto` for `ClientIntent` reuse

### ReplayRecorder Implementation (`src/replay/recorder.rs`)
- Clean API with `record_tick()` skipping empty entries (wire-efficient)
- Added `maybe_record_checkpoint()` for automatic interval-based checkpoints
- `to_bytes()` helper method for better testability
- Comprehensive unit test for roundtrip encoding/decoding

### Game Loop Integration (`src/game_loop/tick.rs`)
- Correctly drains intents at tick boundaries (decoupled from network jitter per ADR-0006)
- Collects `ReplayIntentEntry` from `apply_intent()` return value
- Records checkpoints after simulation tick
- Flushes replay file on shutdown with success/error logging

### Instance Changes (`src/world/instance.rs`)
- `apply_intent()` returns `Option<ReplayIntentEntry>` for event sourcing
- Player name only populated on `JoinIntent`, empty string on other intents (wire-efficient per spec)
- Added `apply_replay_entry()` for replay playback (prepares for Milestone 4.3)

### Canonical Hashing (`src/replay/hash.rs`)
- Uses BTreeMap iteration for deterministic ordering
- Hashes raw fixed-point bits for cross-platform bit-exactness
- Includes entity count in hash (good for detecting entity desyncs)
- Tests verify determinism across insertion orders

### Build System
- `build.rs` correctly compiles both proto files

### Testing
- Integration test validates end-to-end recording flow
- Unit tests cover roundtrip encoding and hash determinism

---

## ⚠️ Issues & Recommendations

### 1. Missing `map_name` Configuration
**Location:** `src/replay/recorder.rs:40`
```rust
map_name: "default_arena".to_string(),
```
**Issue:** Hardcoded to `"default_arena"` but spec indicates this should be configurable.
**Recommendation:** Accept `map_name` as parameter in `ReplayRecorder::new()` or derive from instance configuration.
**Priority:** Medium

---

### 2. Checkpoint Interval Logic Edge Case 🔴 CRITICAL
**Location:** `src/replay/recorder.rs:57`
```rust
if tick > 0 && tick.is_multiple_of(self.checkpoint_interval_ticks)
```
**Issue:** `tick.is_multiple_of()` is not a standard Rust method - this won't compile.
**Recommendation:** Use `tick % self.checkpoint_interval_ticks == 0` instead.
**Priority:** High (compilation error)

---

### 3. Integration Test Race Condition 🔴 CRITICAL
**Location:** `tests/recording_integration_test.rs:35-40`
```rust
let _loop_handle = thread::spawn(move || {
    let instance = Instance::new(1, 60, 5);
    let mut game_loop = GameLoop::new(60);
    game_loop.enable_recording(1, 42, 10, loop_path_str);
    game_loop.start(instance, intent_rx, loop_socket);
});
```
**Issue:** The game loop runs forever with no shutdown mechanism. The test relies on `thread::sleep()` to collect data before the test ends, but the loop thread continues running.
**Recommendation:** Add a timeout or explicit shutdown signal. Consider using a channel to signal the game loop to stop after collecting test data.
**Priority:** High (test reliability)

---

### 4. Potential Data Loss on Crash
**Location:** `src/game_loop/tick.rs:110-117`
**Issue:** Replay file only saved on graceful shutdown. If the server crashes, all recorded data is lost.
**Recommendation:** Consider periodic flushes (e.g., every N checkpoints) or implement a write-ahead log for crash recovery. This may be out of scope for Milestone 4.2 but worth documenting.
**Priority:** Low (future enhancement)

---

### 5. Missing Error Handling in `to_bytes()`
**Location:** `src/replay/recorder.rs:85`
```rust
replay_file.encode(&mut buf).expect("Failed to encode ReplayFile");
```
**Issue:** Uses `expect()` which will panic on encoding failure.
**Recommendation:** Return `Result<Vec<u8>, prost::EncodeError>` for better error propagation, or document why encoding cannot fail.
**Priority:** Low

---

### 6. Unused `record_path` Field
**Location:** `src/game_loop/tick.rs:15`
**Issue:** `record_path` is stored but only used at shutdown. Could be passed directly to `save_to_file()` at shutdown time.
**Recommendation:** This is fine for clarity, but consider if storing is necessary.
**Priority:** Trivial (code style)

---

## ✅ Spec Compliance Summary

| Spec Requirement | Implementation | Status |
|---|---|---|
| ReplayHeader with all fields | `proto/replay.proto` | ✅ |
| ReplayIntentEntry with entity_id, player_name, intent | `proto/replay.proto` | ✅ |
| ReplayTickFrame with tick, entries | `proto/replay.proto` | ✅ |
| ReplayCheckpoint with tick, state_sha256, active_entities | `proto/replay.proto` | ✅ |
| ReplayFile container | `proto/replay.proto` | ✅ |
| DEFAULT_CHECKPOINT_INTERVAL_TICKS = 60 | `src/replay/recorder.rs:11` | ✅ |
| ReplayRecorder::new() | `src/replay/recorder.rs:22` | ✅ |
| record_tick() method | `src/replay/recorder.rs:49` | ✅ |
| record_checkpoint() method | `src/replay/recorder.rs:69` | ✅ |
| save_to_file() method | `src/replay/recorder.rs:90` | ✅ |
| Game loop integration at tick boundaries | `src/game_loop/tick.rs:61-84` | ✅ |
| build.rs updates | `build.rs:8` | ✅ |

---

## 🔧 Required Fixes Before Merge

1. **Fix checkpoint interval check** - Replace `tick.is_multiple_of()` with modulo operator
2. **Fix integration test** - Add proper shutdown mechanism for game loop thread

---

## 📝 Optional Improvements (Can Defer)

1. Make `map_name` configurable in `ReplayRecorder::new()`
2. Add periodic flush for crash resilience
3. Return `Result` from `to_bytes()` instead of panicking
4. Consider removing unused `record_path` field or document its purpose

---

## Overall Assessment

The implementation is **solid and well-structured**, following the spec closely with good test coverage. The two critical issues (compilation error and test race condition) must be addressed before merging. The optional improvements are quality-of-life enhancements that can be deferred to future milestones.

**Recommendation:** Fix the two critical issues and approve merge.
