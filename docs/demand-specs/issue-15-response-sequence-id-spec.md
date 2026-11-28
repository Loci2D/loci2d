# Demand Spec: [CORE] Retain and Transmit Intent `sequence_id` in `ServerResponse` Packets

**Issue:** [`Loci2D/loci2d#15`](https://github.com/Loci2D/loci2d/issues/15)  
**Status:** Ready to Implement  
**Reference ADRs:** [ADR-0004](../adr/en/0004-packet-serialization-format.md) · [ADR-0006](../adr/en/0006-two-thread-network-gameloop-separation.md) · [ADR-0010](../adr/en/0010-event-sourced-replay-format.md) · [ADR-0019](../adr/en/0019-authoritative-intent-rejection-feedback.md) · [ADR-0020](../adr/en/0020-client-sdks-decoupling-and-extraction.md)

---

## 1. Overview & Problem Statement

Currently, `loci2d` decodes `GamePacket.sequence_id` on the network thread and immediately discards it before sending the `ClientIntent` to the game loop. As a result, all outbound `ServerResponse` packets (and their enclosing `ServerPacket` envelopes for intent rejections) carry a hardcoded `sequence_id: 0`.

While this was sufficient when clients acted as stateless terminals, `Loci2D/sdk-love2d` now implements client-side prediction, in-flight RTT tracking (`loci/network/protocol.lua` checking `(packet.response and packet.response.sequence_id) or packet.sequence_id`), and `on_intent_rejected`. Echoing the originating `GamePacket.sequence_id` lets clients pair server acknowledgments and rejections with the exact intent that triggered them.

| Component | Current State | Problem |
|---|---|---|
| `src/network/server.rs` (`run_server`, lines 17–21) | Decodes `GamePacket` (`sequence_id`, `timestamp`, `intent`) and sends `(src_addr, intent)` over `mpsc::Sender`. | `packet.sequence_id` is dropped at the network boundary. |
| `src/world/instance.rs` (`apply_intent`, lines 372–383) | Pushes `ServerResponse { sequence_id: 0, status: "joined" \| "resumed", ... }` into `pending_responses`. | Join and Session Resume responses always report `sequence_id: 0`. |
| `src/game_loop/tick.rs` (`GameLoop::start`, lines 108–122) | On `ApplyIntentResult::Rejected(reason)`, sends `ServerResponse { sequence_id: 0, ... }` inside `ServerPacket { sequence_id: 0, ... }`. | Rejected intents cannot be correlated to in-flight client packets or measured for RTT. |
| `Loci2D/sdk-love2d` (`loci/network/protocol.lua`) | Tracks `_inflight_intents[sequence_id]` for RTT and rejection handling. | Receiving `0` prevents matching responses/rejections to the originating intent. |

---

## 2. Target Architecture & Packet Flow

```mermaid
sequenceDiagram
    autonumber
    participant Client as "Client SDK (sdk-love2d)"
    participant Net as "Network Thread (src/network/server.rs)"
    participant Loop as "GameLoop (src/game_loop/tick.rs)"
    participant Inst as "Instance (src/world/instance.rs)"

    Client->>Net: "GamePacket (sequence_id = 42, intent)"
    Net->>Loop: "InboundClientPacket (addr, sequence_id = 42, intent)"
    Loop->>Inst: "apply_intent_with_seq(addr, 42, intent)"

    alt "Join or Resume Accepted"
        Inst-->>Loop: "pending_responses <- ServerResponse (sequence_id = 42)"
        Loop-->>Client: "ServerPacket (sequence_id = 42, Response(42, joined or resumed))"
    else "Schema Mismatch or Lua Rejection"
        Inst-->>Loop: "ApplyIntentResult::Rejected(reason)"
        Loop-->>Client: "ServerPacket (sequence_id = 42, Response(42, rejected))"
    end
```

---

## 3. Detailed Changes per File

### 3.1 `src/network/server.rs` & `src/network/mod.rs` — `InboundClientPacket`

1. Define and re-export a transport envelope carrying the packet's `sequence_id` across the `mpsc` channel:
   ```rust
   #[derive(Debug, Clone)]
   pub struct InboundClientPacket {
       pub addr: SocketAddr,
       pub sequence_id: u64,
       pub intent: ClientIntent,
   }
   ```
2. Update `run_server` signature and send path:
   ```rust
   pub fn run_server(socket: Arc<UdpSocket>, intent_tx: mpsc::Sender<InboundClientPacket>)
   ```
   When `GamePacket::decode` succeeds and `packet.intent` is `Some(intent)`, send:
   ```rust
   let _ = intent_tx.send(InboundClientPacket {
       addr: src_addr,
       sequence_id: packet.sequence_id,
       intent,
   });
   ```
3. Re-export `InboundClientPacket` from `src/network/mod.rs` alongside `run_server`.

### 3.2 `src/world/instance.rs` — `apply_intent_with_seq`

1. Add `pub fn apply_intent_with_seq(&mut self, addr: SocketAddr, sequence_id: u64, intent: ClientIntent) -> ApplyIntentResult` containing the canonical `apply_intent` logic.
2. When pushing `"joined"` or `"resumed"` responses to `self.pending_responses`, populate `sequence_id` with the passed-in `sequence_id` instead of `0`.
3. Keep `pub fn apply_intent(&mut self, addr: SocketAddr, intent: ClientIntent) -> ApplyIntentResult` as a thin wrapper:
   ```rust
   pub fn apply_intent(&mut self, addr: SocketAddr, intent: ClientIntent) -> ApplyIntentResult {
       self.apply_intent_with_seq(addr, 0, intent)
   }
   ```
   This preserves 100% source compatibility for all ~40 existing unit tests, integration tests, and benchmarks.

### 3.3 `src/game_loop/tick.rs` — Propagate `sequence_id` to Responses

1. Update `GameLoop::start` to accept `intent_rx: mpsc::Receiver<InboundClientPacket>`.
2. In the intent-drain loop, call `instance.apply_intent_with_seq(pkt.addr, pkt.sequence_id, pkt.intent)`.
3. On `ApplyIntentResult::Rejected(reason)`:
   - Set `sequence_id: pkt.sequence_id` on **both** the inner `ServerResponse` and the outer `ServerPacket` (matching `pending_responses` dispatch at line 137).

### 3.4 `src/replay/player.rs` — Spectator Broadcast Channel Type

1. Update `ReplayPlayer::broadcast_live` to accept `intent_rx: mpsc::Receiver<InboundClientPacket>`.
2. Read `pkt.addr` and `pkt.intent` when registering/disconnecting spectators.

### 3.5 Callers of `run_server` in `src/` and `tests/`

Update existing `mpsc::channel()` + `run_server(...)` call sites in `src/main.rs` and integration tests (`tests/broadcasting_integration_test.rs`, `tests/logging_and_csv_test.rs`, `tests/recording_integration_test.rs`, `tests/replay_determinism_test.rs`, `tests/session_integration_test.rs`, `tests/session_resume_test.rs`, `tests/spectator_integration_test.rs`) only if type inference requires `InboundClientPacket` annotations (most `mpsc::channel()` sites infer the type automatically from `run_server` and `GameLoop::start`).

---

## 4. Non-Goals & Determinism Compatibility

- **Zero `.proto` Changes:** `GamePacket.sequence_id`, `ServerResponse.sequence_id`, and `ServerPacket.sequence_id` already exist in `proto/game_packets.proto`.
- **`PingIntent` Behavior Unchanged:** `PingIntent` remains a silent session-keepalive heartbeat; no new `pong` response packets are introduced.
- **100% Replay Bit-Exactness:** `sequence_id` is transport metadata excluded from `ReplayIntentEntry` and `compute_canonical_state_hash` (`src/replay/hash.rs`). Existing `.loci` replays remain unaffected.

---

## 5. Implementation Checklist

- [ ] Add `InboundClientPacket` in `src/network/server.rs` and re-export in `src/network/mod.rs`.
- [ ] Update `run_server` to forward `packet.sequence_id`.
- [ ] Add `Instance::apply_intent_with_seq` in `src/world/instance.rs` and delegate `apply_intent` to it.
- [ ] Update `GameLoop::start` in `src/game_loop/tick.rs` to use `InboundClientPacket` and echo `pkt.sequence_id` on `Rejected` responses.
- [ ] Update `ReplayPlayer::broadcast_live` in `src/replay/player.rs` to accept `mpsc::Receiver<InboundClientPacket>`.
- [ ] Add unit and UDP integration tests.

---

## 6. Verification Plan

1. **Unit Tests (`src/world/instance.rs`):**
   - Verify `apply_intent_with_seq` sets `ServerResponse.sequence_id` in `pending_responses` for both initial join and token-based session resume.
   - Verify `ApplyIntentResult::Rejected` is returned for schema mismatch, `on_player_join`, `on_move_intent`, `on_nav_intent`, and `on_action` rejections.
2. **UDP Integration Test:**
   - Send `GamePacket { sequence_id: 42, ... }` over UDP and assert the received `ServerPacket` has `sequence_id == 42` and `ServerResponse.sequence_id == 42` for both accepted joins and rejected actions/moves.
3. **Full Suite:**
   - Run `cargo test` and verify 0 failures and 0 replay determinism regressions.
