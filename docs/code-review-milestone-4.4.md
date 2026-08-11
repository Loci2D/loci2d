# Code Review: Milestone 4.4 - Live Spectator Broadcast & Multi-Client Playback

**Commit:** e6bf7f8c51b1ab8631813430c1db27e6a1ffaccf  
**Date:** Aug 11, 2026  
**Status:** ✅ Implementation Complete - Meets Spec Requirements

## Summary
The implementation successfully delivers the core requirements for Milestone 4.4. The code is well-structured and follows the spec closely.

## ✅ Spec Compliance

### 1. Live Spectator Broadcast Mode
- **src/replay/player.rs:171-293** - `broadcast_live()` method implements real-time replay streaming
- Variable speed support with clamping (0.1x to 10.0x)
- Fixed-timestep accumulator loop for deterministic timing
- WorldState snapshot generation and UDP broadcasting

### 2. Spectator Connection Lifecycle
- **src/replay/player.rs:200-242** - Session tracking with `BTreeMap<SocketAddr, Instant>`
- Registration on any incoming intent (line 224)
- Disconnect handling (lines 227-230)
- Inactivity sweeping with 10s timeout (lines 234-242)
- **Read-only observer**: Spectators are NOT spawned as entities - only recorded match entities are in the Instance

### 3. CLI Interface
- **src/main.rs:146-154** - `--broadcast` flag parsing
- **src/main.rs:131-145** - `--speed` flag parsing
- **src/main.rs:200-244** - Spectator broadcast mode entry point with graceful shutdown

## ✅ Integration Test Coverage
- **tests/spectator_integration_test.rs** - Comprehensive multi-client test
- Tests 2 simultaneous spectators (JoinIntent and PingIntent)
- Verifies spectators receive correct WorldState (2 entities: Alice, Bob)
- Confirms spectator is NOT in entity list (read-only observer)
- Tests disconnect handling

## 🔍 Code Quality Observations

### Strengths
1. **Clean separation of concerns** - Network thread handles intents, broadcast loop handles replay
2. **Proper resource management** - Arc cloning for socket sharing, AtomicBool for shutdown
3. **Graceful shutdown** - Ctrl+C handler + console listener
4. **Terminal frame broadcasting** (lines 276-290) - Ensures clients receive final state reliably
5. **Deterministic timing** - Accumulator loop with max_accumulator clamp prevents spiral-of-death

### Minor Suggestions

#### 1. Speed Clamping Feedback
**Location:** `src/replay/player.rs:181`
```rust
let speed = if speed <= 0.0 { 1.0 } else { speed.clamp(0.1, 10.0) };
```
**Suggestion:** Consider logging when speed is clamped for user feedback:
```rust
let original_speed = speed;
let speed = if speed <= 0.0 { 1.0 } else { speed.clamp(0.1, 10.0) };
if speed != original_speed {
    println!("[Spectator] Speed clamped from {:.1}x to {:.1}x", original_speed, speed);
}
```

#### 2. Default Broadcast Address Behavior
**Location:** `src/main.rs:203`
```rust
let bind_target = broadcast_addr.unwrap_or(cfg.bind_addr);
```
**Suggestion:** This defaults to server config, which might be confusing. Consider requiring explicit `--broadcast` for spectator mode or logging the default:
```rust
let bind_target = if let Some(addr) = broadcast_addr {
    addr
} else {
    println!("[Spectator] No --broadcast specified, using server default: {}", cfg.bind_addr);
    cfg.bind_addr
};
```

#### 3. Test Speed Configurability
**Location:** `tests/spectator_integration_test.rs:103`
```rust
player.broadcast_live(socket, intent_rx, 2.0, running_clone);
```
**Suggestion:** Hardcoded 2.0x speed is fine for tests, but consider making it a test constant for easier adjustment:
```rust
const TEST_REPLAY_SPEED: f32 = 2.0;
player.broadcast_live(socket, intent_rx, TEST_REPLAY_SPEED, running_clone);
```

## 📋 Potential Issues (Not Blocking)

### 1. Race Condition in Integration Test
**Location:** `tests/spectator_integration_test.rs:141-155`
**Issue:** The test relies on timing (20 recv attempts with 500ms timeout). This could be flaky on slow systems or under load.
**Suggestion:** 
- Increase timeout to 1000ms for more robustness
- Add a retry loop with exponential backoff
- Consider using a synchronization mechanism (e.g., a channel signal when first WorldState is sent)

### 2. No Spectator Limit
**Location:** `src/replay/player.rs:201`
**Issue:** The implementation doesn't limit the number of spectators. In production, this could lead to resource exhaustion.
**Suggestion:** Add a max spectator count constant:
```rust
const MAX_SPECTATORS: usize = 100;
// When registering:
if spectators.len() >= MAX_SPECTATORS {
    println!("[Spectator] Rejected spectator {}: max spectators reached", addr);
    continue;
}
```

### 3. Packet Loss Handling
**Location:** `src/replay/player.rs:262-267`
**Issue:** No acknowledgment or retransmission for lost WorldState packets. Spectators may miss frames if UDP packets are dropped.
**Note:** This is acceptable for the spec (spectators are observers, not participants), but worth documenting for production use.
**Suggestion:** Add a comment noting this limitation:
```rust
// 6. Encode and broadcast to all active spectators
// Note: UDP is lossy - spectators may miss frames. For production, consider
// adding sequence numbers and client-side interpolation for smooth playback.
```

### 4. Terminal Frame Duplication
**Location:** `src/replay/player.rs:284-289`
**Issue:** The terminal frame is sent 3 times with 15ms delay. This is good for reliability, but the hardcoded values might not be optimal for all network conditions.
**Suggestion:** Consider making these configurable or documenting the rationale:
```rust
// Send terminal frame 3 times to ensure reliable delivery across lossy networks
const TERMINAL_FRAME_RETRIES: u32 = 3;
const TERMINAL_FRAME_DELAY_MS: u64 = 15;
```

## ✅ ADR Compliance
- **ADR-0011** (Spectator Broadcasting): Fully compliant - authoritative WorldState streaming, speed controls, observer pattern
- **ADR-0006** (Thread Separation): Network thread handles intents, broadcast loop handles replay - proper decoupling
- **ADR-0010** (Event-Sourced Replay): Uses existing replay format correctly for playback
- **ADR-0007** (Deterministic Simulation): Fixed-timestep accumulator ensures deterministic timing

## Conclusion
**The implementation is solid and meets the spec requirements.** The code is clean, well-documented, and includes comprehensive integration tests. The minor suggestions above are optional enhancements and don't block the implementation. The potential issues are noted for future consideration but are not critical for the current milestone.

**Recommendation:** ✅ **APPROVED** - Ready for testing and deployment.
