# ADR 0009: Simplified Authentication & Explicit Session Handshake

## Status

Accepted — **Temporary (Phase 2–4)**  
Expected to be superseded in **Phase 5** by full authentication & security implementation.

## Context

For a production multiplayer game server, authentication typically involves:
- OAuth/JWT token validation
- Database-backed user accounts
- Session token management
- Cryptographic packet signing
- Anti-cheat and anti-tamper measures

However, during the validation phase (Phase 2–4), the project priorities are:
1. **Low barrier to entry**: Students and indie devs should run the server locally without external dependencies
2. **Rapid prototyping**: Focus on core game loop and session mechanics, not auth infrastructure
3. **Trusted environment**: Initial deployment targets localhost and private LAN game rooms
4. **Architecture validation**: Prove the authoritative server model works before adding security layers

Introducing full authentication in Phase 2 would:
- Require database setup (PostgreSQL, Redis, etc.)
- Need OAuth provider integration or custom JWT implementation
- Add significant complexity to the codebase
- Increase onboarding time for new contributors
- Distract from core networking and simulation validation

Alternatives considered:

| Approach | Why rejected |
|---|---|
| Full OAuth/JWT in Phase 2 | Too complex for validation phase; blocks rapid prototyping |
| Database-backed user accounts | Requires external infrastructure; violates single-instance simplicity |
| Pre-shared keys/passwords | Still requires key distribution infrastructure; poor UX |
| Implicit auto-join on raw intent | Bypasses session boundaries; spawns phantom entities on random packets |

## Decision

We adopt a **minimal trust-based authentication model with explicit session handshake** optimized for rapid prototyping:

### Identity Mechanism
- **Primary identifier**: `SocketAddr` (client IP:port)
- **Display identity**: Human-readable `player_name` supplied via `JoinIntent`
- **No cryptographic tokens**: Trust the network address for session binding
- **No database**: All session state is in-memory within the `Instance`

### Explicit Handshake Protocol
- **Behavior**: A client MUST send an explicit `JoinIntent` containing their `player_name` to register a session and spawn an entity in the instance.
- **Unjoined Intent Policy**: If an unknown `SocketAddr` sends gameplay intents (`MoveIntent`, `ActionIntent`, `PingIntent`) without a prior `JoinIntent`, the packet is dropped and ignored.
- **Graceful Leave**: A client can send `DisconnectIntent` with an optional reason to cleanly terminate their session and despawn their entity.

### Security Trade-offs (Explicitly Accepted)
- **Packet spoofing**: Malicious clients could impersonate any IP in LAN environments
  - *Mitigation*: Phase 2–4 targets trusted private networks; production security deferred to Phase 5
- **No replay protection**: Packets could be captured and retransmitted
  - *Mitigation*: Sequence IDs exist in protobuf but are not validated; validation deferred to Phase 5
- **No encryption**: UDP packets are sent in plaintext
  - *Mitigation*: Acceptable for localhost development; DTLS/TLS considered for Phase 5
- **IP-based identity**: NAT traversal, mobile networks, or IP changes break sessions
  - *Mitigation*: Acceptable for stable LAN/localhost; token-based identity deferred to Phase 5

### Protocol Extensions
- **`JoinIntent`**: Explicit handshake with `player_name` field
- **`DisconnectIntent`**: Graceful session termination with optional reason

## Why This Is Temporary

This approach is explicitly a **validation-phase compromise** with known security gaps:

1. **No authentication**: Anyone who can reach the UDP port can join
2. **No authorization**: All players have equal permissions; no admin/role system
3. **No encryption**: Network traffic is visible to packet sniffers
4. **No integrity**: Packets can be modified in transit without detection
5. **No replay protection**: Captured packets can be retransmitted

These are acceptable for:
- Localhost development (`127.0.0.1`)
- Private LAN game rooms with trusted participants
- Educational environments where security is not the learning objective
- Proof-of-concept validation before production investment

## Migration Path (Phase 5)

Phase 5 will introduce full authentication & security:
- Replace `SocketAddr` identity with JWT tokens or session IDs
- Add authentication server (OAuth 2.0 or custom implementation)
- Implement packet signing (HMAC or similar) for integrity
- Add encryption (DTLS or application-layer encryption)
- Implement replay attack protection (nonce/timestamp validation)
- Add role-based authorization (admin, player, spectator)
- Integrate with database for persistent user accounts

## Consequences

**Positive:**
- **Zero external dependencies**: No database, auth server, or third-party services required
- **Instant onboarding**: New contributors can run the server immediately after `cargo run`
- **Rapid iteration**: Focus on game mechanics, not auth infrastructure
- **Simple debugging**: No token expiration, database migrations, or auth flows to debug
- **Educational clarity**: Session mechanics are visible and understandable without crypto complexity
- **Clean session boundaries**: Explicit join requirement ensures only intentional connections spawn entities

**Negative:**
- **No security**: Completely unsuitable for public internet deployment
- **Spoofing vulnerability**: Malicious actors can impersonate clients in LAN environments
- **No persistent identity**: IP changes break sessions (mobile networks, DHCP)
- **No audit trail**: No logging of which human user is behind which connection
- **Trust model assumption**: Requires trusted network environment (localhost/private LAN)
- **Production blocker**: Must be replaced before any public deployment
