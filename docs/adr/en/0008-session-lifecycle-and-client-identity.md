# ADR 0008: Session Lifecycle & Client Identity Management

## Status

Accepted — **Temporary (Phase 2–4)**  
Expected to be revisited in **Phase 5** when multi-instance concurrency and authentication are introduced.

## Context

In Phase 1, client tracking was rudimentary: a simple `HashMap<SocketAddr, u64>` mapped network addresses to entity IDs with auto-join on first packet. This approach had several critical limitations:

1. **No session lifecycle**: Entities were created on first packet and never removed
2. **No timeout detection**: Crashed or disconnected clients left "ghost" entities indefinitely
3. **No connection metadata**: No player names, join times, or activity tracking
4. **No graceful disconnect**: No protocol for clients to signal intent to leave
5. **Memory leaks**: Entity and session counts grew monotonically over time

For a production-ready game server, we need explicit session management with lifecycle tracking, timeout detection, and clean resource cleanup.

Alternatives considered:

| Approach | Why rejected |
|---|---|
| Database-backed sessions | Overkill for single-instance Phase 2; introduces external dependencies and complexity |
| Token-based session IDs | Requires authentication infrastructure (deferred to Phase 5) |
| Complex state machines | Too heavy for validation phase; simple 3-state model suffices |
| Reference counting | Doesn't address timeout detection or graceful disconnect semantics |

## Decision

We implement a **SocketAddr-based session lifecycle** with the following architecture:

### Session Identity Model
- **Primary identifier**: `SocketAddr` (client IP:port combination)
- **Session metadata**: `session_id: u64`, `entity_id: u64`, `player_name: String`
- **Timestamps**: `connected_at: Instant`, `last_seen: Instant`
- **State machine**: `Active` → `TimedOut` / `Disconnected`

### Session State Machine
```
Unknown SocketAddr → [JoinIntent] → Active Session
Unknown SocketAddr → [Other Intent] → Drop (No Session)
Active Session → [DisconnectIntent] → Disconnected → Despawn Entity
Active Session → [No packets for > CLIENT_TIMEOUT_SECS] → TimedOut → Despawn Entity
```

### Data Structures
```rust
pub struct ClientSession {
    pub session_id: u64,
    pub addr: SocketAddr,
    pub entity_id: u64,
    pub player_name: String,
    pub connected_at: Instant,
    pub last_seen: Instant,
    pub state: SessionState,
}

pub enum SessionState {
    Active,
    TimedOut,
    Disconnected,
}
```

### Bidirectional Mapping
- `sessions: HashMap<SocketAddr, ClientSession>` — Address → Session
- `entity_to_addr: HashMap<u64, SocketAddr>` — EntityID → Address (for reverse lookups)

### Timeout Detection
- Configurable `CLIENT_TIMEOUT_SECS` (default: 10s)
- Checked each tick in `Instance::check_timeouts()`
- Sessions exceeding threshold transition to `TimedOut` and trigger entity despawn

## Why This Is Temporary

This design is scoped to single-instance operation (Phase 2–4). Known limitations for Phase 5:

1. **SocketAddr collision risk**: NAT traversal or proxy scenarios could cause address conflicts
2. **No persistent identity**: Client IP changes break sessions (mobile networks, DHCP)
3. **Single-instance scope**: Session maps are per-instance; multi-instance requires distributed session management
4. **No authentication**: SocketAddr can be spoofed in untrusted environments
5. **Memory overhead**: One session per connected client; needs optimization for high player counts

## Migration Path (Phase 5)

When multi-instance and authentication are introduced:
- Replace `SocketAddr` primary key with cryptographic `session_id: u64` or JWT token
- Add session persistence layer (Redis/database) for cross-instance sessions
- Implement session reconnection logic for IP changes
- Add session validation against authentication service

## Consequences

**Positive:**
- **Clean resource management**: Entities are despawned on disconnect/timeout
- **Activity tracking**: `last_seen` enables heartbeat monitoring and idle detection
- **Graceful disconnects**: Clients can signal intent to leave with optional reason
- **Debug visibility**: Session metadata aids logging and troubleshooting
- **Simple implementation**: Uses standard Rust collections with clear semantics
- **Explicit boundary**: Enforces valid JoinIntent before processing gameplay intents

**Negative:**
- **SocketAddr fragility**: IP changes break sessions (acceptable for Phase 2–4 localhost/LAN scope)
- **No authentication**: Address spoofing possible in untrusted networks (deferred to Phase 5)
- **Memory growth**: Unbounded session map (acceptable for expected Phase 2–4 scale)
- **Single-instance only**: Doesn't generalize to distributed architectures (Phase 5 concern)
