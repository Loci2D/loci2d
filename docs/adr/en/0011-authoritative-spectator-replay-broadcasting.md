# ADR 0011: Server-Authoritative Spectator Replay Broadcasting

## Status

Accepted (Phase 4)

## Context

`loci2d` is designed to support diverse client engines including **Godot 4 (GDScript/C#)**, **Love2D (Lua)**, **Python**, and **Rust CLI**.

When match replays are played back for human observation or debugging, we need a mechanism for clients to render the replay visually.

Two architectural alternatives were evaluated:

| Approach | Implementation Details | Cross-Client Complexity | Maintenance Overhead |
|---|---|---|---|
| **Client-Side Simulation Replay** | Each client language (Lua, GDScript, Python) implements its own fixed-point math engine, loads the `.loci` file, and runs the simulation loop locally. | **Very High** (Requires porting Rust's deterministic math, `BTreeMap` sorting, and physics logic into 3+ different programming languages). | **High** (Any tweak to Rust server physics must be replicated identically across all client SDKs). |
| **Server-Authoritative UDP Spectator Broadcast** | The loci2d server runs the deterministic replay engine locally, stepping `Instance` at the target tick rate, and broadcasts standard `WorldState` snapshots over UDP via [ADR-0005](0005-cross-language-binary-serialization.md). | **Zero** (Clients reuse their existing Phase 3 network renderers with no code changes). | **Zero** (All simulation logic remains centralized in the Rust server). |

## Decision

We adopt **Server-Authoritative Spectator Replay Broadcasting** as the primary playback mechanism for multi-language client engines:

```
┌──────────────────────────────────────────────────────────────────────────┐
│ loci2d Server (Replay Mode)                                              │
│ loci2d --replay match.loci --broadcast 127.0.0.1:4000 --speed 1.0       │
│                                                                          │
│ 1. Loads match.loci (Event Sourcing Inputs)                              │
│ 2. Steps Instance with fixed-point physics at fixed tick rate (30 Hz)    │
│ 3. Generates standard WorldState Protobuf snapshot                       │
│ 4. Broadcasts over UDP socket ───────────────────────────────────────┐   │
└──────────────────────────────────────────────────────────────────────┼───┘
                                                                       │
                                      UDP Broadcast (WorldState)       │
                                                                       ▼
                        ┌──────────────────────────────────────────────────────────┐
                        │                Spectator Client Engines                  │
                        ├────────────────────┬────────────────────┬────────────────┤
                        │ Godot 4 (GDScript) │    Love2D (Lua)    │  Python / CLI  │
                        │ Visual Node View   │  Canvas 2D Render  │  State Monitor │
                        └────────────────────┴────────────────────┴────────────────┘
```

### 1. Unified Client Protocol
Because the replay server emits standard `ServerPacket` envelopes containing `WorldState` snapshots, connected clients do not know or care whether the game is a live multiplayer match or a replayed recording.

### 2. Variable Playback Speed Controls
The replay runner supports configurable simulation speed multipliers:
- `0.5x`: Half-speed slow motion for detailed physics inspection.
- `1.0x`: Real-time match playback.
- `2.0x` / `4.0x`: Fast-forward spectator review.
- `--verify` (Max Speed): Headless execution without UDP broadcast for CI/CD desync validation.

## Consequences

**Positive:**
- **Zero Client Modification**: Godot, Love2D, Python, and CLI client examples immediately support watching replays without modifying a single line of client code.
- **Single Source of Truth**: Physics rules, collision geometry, and state simulation logic are written once in Rust, eliminating subtle cross-language floating-point divergence.
- **Bandwidth Control**: Replay broadcasting shares the same efficient Protobuf serialization format as live matches.
- **Pedagogical Simplicity**: Indie developers and students learning game development can inspect replays visually using their favorite game engine.

**Negative:**
- **Server Process Requirement**: Viewing a replay requires running the `loci2d` server executable in replay mode on `localhost` rather than opening a standalone file viewer directly inside Godot or Love2D.
