# ADR 0013: Server-Authoritative Destination Steering and Navigation (Click-to-Move)

## Status

Accepted (Phase 5)

## Context

In addition to continuous directional inputs (`MoveIntent`, typical in WASD/gamepad control schemes), many 2D multiplayer genres (such as MOBAs, Real-Time Strategy games, and point-and-click action RPGs) rely heavily on mouse destination-based movement:
- The player clicks a location on the 2D plane.
- The client transmits a single target destination packet (`MoveToPositionIntent`).
- The entity moves autonomously toward the destination point at a configured speed, stopping cleanly when reaching the target.

Implementing destination steering presents two major architectural approaches:

| Approach | Where Logic Runs | Network Traffic | Vulnerability / Determinism |
|---|---|---|---|
| **Client-Side Steering Simulation** | Client simulates steering and sends frequent stream of `MoveIntent` direction vectors | **High** (Client must stream continuous directional packets each tick) | Prone to network latency jitter, path desync, and client-side position tampering |
| **Server-Authoritative Steering** | Client sends single `MoveToPositionIntent(x, y)`; Server evaluates steering every tick | **Extremely Low** (Single packet per click) | 100% Authoritative, cheat-proof, and fully deterministic across all match replays |

## Decision

We decide to implement **Server-Authoritative Destination Steering and Waypoint Navigation**:

### 1. Server-Side Navigation Component on Entities
Entities supporting destination navigation gain an optional `NavigationComponent`:
- `target: Option<DeterministicVector2>`
- `arrival_tolerance: I16F16` (distance threshold for stopping)
- `move_speed: I16F16` (fixed-point speed per tick)
- `waypoints: VecDeque<DeterministicVector2>` (ordered queue of navigation waypoints)

### 2. Deterministic Fixed-Point Steering Loop
During `Instance::tick()`:
1. If a target is active, the server calculates displacement $\vec{d} = \text{target} - \text{position}$.
2. Distance is computed using fixed-point integer math: $D = \text{fixed\_sqrt}(d_x^2 + d_y^2)$.
3. **Arrival Tolerance Check**:
   - If $D \le \text{arrival\_tolerance}$:
     - If the `waypoints` queue has remaining nodes, the server pops the next waypoint as the new active `target`.
     - Otherwise, the entity velocity is set to zero and `target` is cleared.
   - If $D > \text{arrival\_tolerance}$:
     - Velocity is set along the unit direction: $\vec{v} = (\vec{d} / D) \times \text{move\_speed}$.

### 3. Jitter Prevention & Arrival Threshold
Without an arrival tolerance ($\epsilon$), discrete fixed-timestep integration can overshoot the target destination on one tick and reverse direction on the next, causing visual oscillation and jitter around the destination point. Enforcing a non-zero arrival tolerance proportional to `move_speed` guarantees clean, one-tick stopping.

### 4. Deterministic Intent Preemption
When a client issues a new intent:
- A new `MoveToPositionIntent` immediately replaces the active target and clears/replaces the waypoint queue.
- A raw `MoveIntent` (e.g. WASD input) immediately cancels any active destination navigation and gives direct velocity control.
- An explicit stop or disconnect immediately resets navigation.

## Consequences

**Positive:**
- **Minimal Bandwidth**: A single packet triggers full navigation toward a destination.
- **Server Authoritative & Cheat-Proof**: Prevents speed-hacking or client-side boundary clipping during path traversal.
- **100% Replay Determinism**: Click-to-move paths are logged as single discrete intent events in `.loci` replay files and reproduce identical traversal paths across all platforms.
- **Multi-Waypoint Support**: Enables queuing waypoints (shift-click movement) for RTS/MOBA mechanics.

**Negative:**
- **Server CPU Work**: The server evaluates steering vector calculations on each tick for moving entities (negligible for 2D fixed-point integer math).
- **Latency Perception**: Under high network latency, players will see a round-trip delay before their character begins moving (client-side visual indicators like click beacons can be rendered instantly client-side without compromising server authority).
