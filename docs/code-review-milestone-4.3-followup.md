# Code Review Follow-Up: Milestone 4.3 Action Items & Determinism Enhancements

**Date:** 2026-08-11  
**Scope:** Replay determinism edge cases, timeout synchronization, and CLI ergonomics  
**Status:** 📋 Proposed Fix Plan  

---

## 1. Executive Summary

During the secondary deep-pass code review of Milestone 4.3 ([`91f7abf`](file:///home/furlan/Documents/workspace/loci2d/src/replay/player.rs) & [`e28360d`](file:///home/furlan/Documents/workspace/loci2d/docs/code-review-milestone-4.3.md)), five specific areas for improvement were identified. 

Addressing these items immediately is **strongly recommended** to ensure zero state desyncs as the project advances to **Milestone 4.4 (Live Spectator Broadcast)** and **Phase 5 (Multi-Instance Scaling)**.

---

## 2. Action Items & Rationale

### Item 1: Fix Re-Join Entity State Reset in `Instance::apply_replay_entry` (Priority: High)
- **File:** `src/world/instance.rs`
- **Issue:** When replaying a `JoinIntent` for an entity that already exists, `apply_replay_entry` creates a new `Entity` initialized with `DeterministicVector2::ZERO` position and velocity. In live gameplay, `handle_join` preserves the entity's current position and velocity and only updates the name.
- **Risk:** Any match where a player rejoins or updates their display name while moving will desync the replay engine at the next checkpoint.
- **Fix:** Update `apply_replay_entry` to check if `self.entities.get_mut(&entry.entity_id)` exists. If present, update `entity.name`; otherwise, insert a new entity.

### Item 2: Synchronize Inactivity Timeouts with Replay Stream (Priority: High)
- **Files:** `src/world/instance.rs`, `src/game_loop/tick.rs`
- **Issue:** In live gameplay, `check_timeouts()` removes inactive sessions and entities inside `instance.tick()`. However, because this does not originate from an incoming `ClientIntent`, no `ReplayIntentEntry` is recorded. During replay verification, `instance.sessions` is empty, so `check_timeouts()` does not remove the entity, causing state hash divergence.
- **Risk:** Any match where a player drops offline without sending an explicit `DisconnectIntent` will fail replay verification.
- **Fix:** Update `check_timeouts()` to return the list of timed-out `(entity_id, player_name)` tuples, and record a synthetic `DisconnectIntent` for each timed-out entity in `GameLoop`.

### Item 3: Order-Independent `end_tick` in `ReplayPlayer::verify_determinism` (Priority: Medium)
- **File:** `src/replay/player.rs`
- **Issue:** `total_ticks` and `max_checkpoint_tick` currently use `.last()` on the raw Protobuf vectors, assuming input frames are strictly sorted.
- **Fix:** Derive `end_tick` directly from the populated `BTreeMap` keys (`keys().next_back()`), guaranteeing robust handling even if input files contain unsorted entries.

### Item 4: Strict CLI Validation for `--verify` and Code Formatting (Priority: Low)
- **File:** `src/main.rs`
- **Issue:** Passing `--verify` without `--replay <FILE>` silently starts the live UDP server instead of displaying an error. Additionally, lines 154–168 have uneven indentation.
- **Fix:** Add a validation check: if `verify_mode` is set without `replay_file`, print an error and exit with code 1. Reformat the block to standard indentation.

### Item 5: Clean Up Duplicate Checkpoint in Test Suite (Priority: Low)
- **File:** `tests/replay_determinism_test.rs`
- **Issue:** `test_desync_diagnostic_report_on_tampered_frame` calls `maybe_record_checkpoint` followed immediately by `record_checkpoint` for tick 10, creating duplicate checkpoints in the recorder.
- **Fix:** Remove the redundant `maybe_record_checkpoint` call so the test explicitly records only the tampered checkpoint.

---

## 3. Proposed Code Diffs

### 3.1. `src/world/instance.rs`

```diff
@@ -190,13 +190,14 @@ impl Instance {
     /// Sweep and remove inactive sessions
-    pub fn check_timeouts(&mut self) {
+    pub fn check_timeouts(&mut self) -> Vec<(u64, String)> {
         let timeout_secs = self.client_timeout_secs;
         let mut timed_out_addrs = Vec::new();

         for (addr, session) in &self.sessions {
             if session.is_timed_out(timeout_secs) {
                 timed_out_addrs.push((*addr, session.entity_id, session.player_name.clone()));
             }
         }

+        let mut timed_out_entities = Vec::new();
         for (addr, entity_id, player_name) in timed_out_addrs {
             self.sessions.remove(&addr);
             self.entity_to_addr.remove(&entity_id);
             self.entities.remove(&entity_id);
+            timed_out_entities.push((entity_id, player_name.clone()));
             println!("[Timeout] Client {} ('{}', EntityId {}) timed out after {}s of inactivity", addr, player_name, entity_id, timeout_secs);
         }
+        timed_out_entities
     }
```

```diff
@@ -267,8 +268,12 @@ impl Instance {
             Intent::Join(join_intent) => {
                 let player_name = if join_intent.player_name.trim().is_empty() {
                     entry.player_name.clone()
                 } else {
                     join_intent.player_name.clone()
                 };
-                let entity = Entity::new(entry.entity_id, player_name, EntityType::Player);
-                self.entities.insert(entry.entity_id, entity);
+                if let Some(existing) = self.entities.get_mut(&entry.entity_id) {
+                    existing.name = player_name;
+                } else {
+                    let entity = Entity::new(entry.entity_id, player_name, EntityType::Player);
+                    self.entities.insert(entry.entity_id, entity);
+                }
             }
```

### 3.2. `src/game_loop/tick.rs`

```diff
@@ -96,3 +96,17 @@ impl GameLoop {
-                // 3. Advance deterministic simulation physics & sweep timeouts
-                instance.tick(tick_count);
+                // 3. Advance deterministic simulation physics & sweep timeouts
+                for entity in instance.entities.values_mut() {
+                    entity.position = entity.position.saturating_add(entity.velocity);
+                }
+                let timed_out = instance.check_timeouts();
+                if let Some(ref mut recorder) = self.recorder {
+                    for (entity_id, player_name) in timed_out {
+                        recorder.record_tick(tick_count, vec![crate::network::packets::ReplayIntentEntry {
+                            entity_id,
+                            player_name,
+                            intent: Some(crate::network::packets::ClientIntent {
+                                intent: Some(crate::network::packets::client_intent::Intent::Disconnect(
+                                    crate::network::packets::DisconnectIntent {
+                                        reason: "Inactivity timeout".to_string(),
+                                    }
+                                )),
+                            }),
+                        }]);
+                    }
+                }
```

### 3.3. `src/replay/player.rs`

```diff
@@ -89,5 +89,0 @@ impl ReplayPlayer {
-        let total_ticks = self.replay.frames.last().map(|f| f.tick).unwrap_or(0);
-        let max_checkpoint_tick = self.replay.checkpoints.last().map(|c| c.tick).unwrap_or(0);
-        let end_tick = total_ticks.max(max_checkpoint_tick);
-
@@ -106,2 +101,8 @@ impl ReplayPlayer {
+        let end_tick = frames_by_tick
+            .keys()
+            .next_back()
+            .copied()
+            .unwrap_or(0)
+            .max(checkpoints_by_tick.keys().next_back().copied().unwrap_or(0));
+
```

### 3.4. `src/main.rs`

```diff
@@ -141,2 +141,7 @@ fn main() {
     // 1. Headless Replay Verification Mode
+    if verify_mode && replay_file.is_none() {
+        eprintln!("Error: --verify requires a replay file specified via --replay <FILE>");
+        process::exit(1);
+    }
+
     if let Some(ref path) = replay_file
```

---

## 4. Verification Plan

1. **Unit & Integration Tests**:
   - `cargo test --all-targets`
   - Add a specific test in `tests/replay_determinism_test.rs` verifying player rejoin name-update determinism.
   - Add a test verifying that live match recording properly handles inactivity timeout disconnects during replay verification.
2. **Clippy Linting**:
   - `cargo clippy --all-targets` with 0 warnings.
3. **CLI Execution**:
   - Verify `cargo run --bin loci2d -- --verify` exits cleanly with the expected error message.
