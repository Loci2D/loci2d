# ADR 0006: Two-Thread Network/Game-Loop Separation via `mpsc`

## Status

Accepted — **Temporary (Phase 1–3)**  
Expected to be revisited in **Phase 5** when multi-instance concurrency is introduced.

## Context

In Phase 1, two responsibilities that were previously coupled on a single thread need to be decoupled:

- **Network I/O**: `UdpSocket::recv_from` is a blocking call. While it is waiting for a packet, no other work can run on the same thread.
- **Game loop**: The fixed-rate tick (30 Hz) must advance independently of whether packets are arriving.

Keeping both on one thread creates a fundamental conflict: the server cannot simultaneously block on `recv_from` and advance the simulation. Any incoming packet is only processed when the server is not ticking — and the server never ticks because it is always blocked.

Alternatives considered:

| Approach | Why rejected |
|---|---|
| `Arc<Mutex<VecDeque<Intent>>>` shared queue | Requires explicit locking; `mpsc` already provides the same semantics without a user-managed lock |
| Single thread with non-blocking `recv_from` + spin-loop | Wastes CPU; adds complexity without benefit at this scale |
| Async runtime (`tokio`) | Correct long-term direction for multi-instance scale, but introduces significant complexity for a single-instance Phase 1; deferred to Phase 5 |
| Single thread with `select!` / poll-based I/O | Non-trivial to implement with std-only UDP; async runtime is better suited |

## Decision

Use **two OS threads** connected by a **`std::sync::mpsc` channel** carrying `(SocketAddr, ClientIntent)` tuples:

```
┌──────────────────────────────────────────────┐
│  Network Thread                               │
│  UdpSocket::recv_from (blocking)              │
│  → decodes GamePacket (prost)                 │
│  → sends (src_addr, ClientIntent) → intent_tx │
└──────────────────┬───────────────────────────┘
                   │  mpsc::channel<(SocketAddr, ClientIntent)>
                   ▼
┌──────────────────────────────────────────────┐
│  Game Loop Thread                             │
│  Fixed tick: 30 Hz (~33 ms/tick)              │
│  1. Drain intent_rx (try_recv, non-blocking)  │
│  2. Apply intents to Instance                 │
│  3. Advance simulation (update positions)     │
│  4. Sleep remaining tick budget               │
└──────────────────────────────────────────────┘
```

`mpsc` is chosen because:
- It is the standard Rust primitive for producer→consumer inter-thread communication
- Ownership of each intent is transferred (no clone needed), enforced at compile time
- No explicit lock required; the channel itself serializes access
- Clean shutdown: when the sender is dropped, the receiver observes a `RecvError`

## Why This Is Temporary

This design is scoped to single-instance operation (Phase 1–3). It has known limitations that will become blockers in Phase 5:

1. **One network thread is a bottleneck for multiple instances.** With one blocking `recv_from` thread, all instances share a single inbound socket. Adding more instances requires either routing packets from one network thread to N game-loop threads (N `mpsc` senders, a routing table) or switching to a non-blocking async socket.

2. **One OS thread per instance game loop does not scale.** OS threads carry ~8 MB stack overhead each. Dozens of concurrent instances are manageable; hundreds are not. Async tasks (`tokio::task::spawn`) are lightweight by comparison.

3. **The channel is unbounded.** `mpsc::channel()` has no capacity limit. Under a packet flood, the intent queue grows without bound. A bounded `sync_channel(N)` would add back-pressure but blocks the network thread when the game loop falls behind — acceptable for Phase 1, but not the right primitive for production scale.

4. **No outbound path.** The current design has no mechanism for the game loop to send `WorldState` back to clients. Phase 3 will require an explicit outbound path (e.g., sharing `Arc<UdpSocket>` with the game loop, or a third send-thread with its own channel).

## Migration Path (Phase 5)

The conceptual model — "network receiver → queue → game loop" — survives the migration. The machinery changes:

| Now (Phase 1–3) | Phase 5 target |
|---|---|
| `std::thread::spawn` | `tokio::task::spawn` |
| `UdpSocket::recv_from` (blocking) | `tokio::net::UdpSocket::recv_from` (async) |
| `std::sync::mpsc::channel` | `tokio::sync::mpsc::channel` |
| `thread::sleep` for tick timing | `tokio::time::interval` |

The refactor is mechanical, not a redesign. The Phase 1 implementation intentionally uses only `std`-primitives to remain dependency-free and maximally transparent for learners.

## Consequences

**Positive:**
- **Correctness**: Blocking `recv_from` is isolated to its own thread — the correct and natural idiom
- **Simplicity**: Two threads with one channel are easy to reason about, test, and debug
- **Zero locking**: `mpsc` eliminates the need for `Mutex` in the hot path
- **Idiomatic Rust**: Compile-time ownership transfer of each intent; channel disconnect is automatically observable
- **Pedagogical clarity**: The data flow is explicit and linear — appropriate for a learning-oriented codebase

**Negative:**
- **Single-instance only**: Does not generalize to N instances without architectural changes
- **No outbound path**: Game loop cannot respond to clients without further design work (Phase 3)
- **Unbounded queue**: No built-in protection against intent flooding
- **OS thread overhead**: Not suitable for high instance counts (Phase 5 blocker)
