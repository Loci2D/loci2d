# ADR 0001: Authoritative Server Architecture

## Status

Accepted

## Context

For a 2D multiplayer game, we need to decide between different server architectures:
- **Authoritative Server**: Server controls all game state and validates all client actions
- **Client-Side Prediction**: Clients predict their own actions, server validates
- **Peer-to-Peer**: No central server, clients communicate directly

## Decision

We choose an **Authoritative Server** architecture where:
- All game state resides on the server
- Clients send **intentions** (what they want to do) rather than direct state changes
- Server validates and processes all actions
- Server sends authoritative state updates to clients
- Clients are dumb terminals that only send input and render received state

## Consequences

**Positive:**
- **Security**: Prevents cheating since clients can't directly modify game state
- **Consistency**: Single source of truth ensures all clients see the same game state
- **Validation**: Server can enforce game rules and prevent invalid actions
- **Simplified Clients**: Clients don't need complex game logic, just rendering and input

**Negative:**
- **Latency**: Client actions must round-trip to server before taking effect
- **Server Load**: All game logic runs on the server, requiring more computational resources
- **Network Traffic**: More frequent state updates from server to all clients
- **Complexity**: Requires client-side prediction/interpolation to mask latency for good UX
