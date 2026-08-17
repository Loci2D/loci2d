# ADR 0010: Event-Sourced Replay Storage and State Checkpointing Strategy

## Status

Accepted (Phase 4)

## Context

For `loci2d` to support persistent match recording, offline determinism verification, and spectator replays, we must define a storage format for match history.

Two primary architectural patterns exist for recording multiplayer game simulations:

| Approach | Storage Mechanism | Storage Overhead | Playback Overhead | Desync Resilience |
|---|---|---|---|---|
| **Full Snapshot Streaming** | Serialize complete `WorldState` every tick | **High** (~100 KB–1 MB/sec depending on entity count) | **Low** (O(1) frame seeking and scrubbing) | High (self-healing every frame, but masks simulation bugs) |
| **Event Sourcing (Input Logging)** | Record initial seed/config + discrete inputs per tick | **Extremely Low** (~1–5 KB/min for typical matches) | **Moderate** (O(N) sequential simulation from tick 0) | Strict (exposes all non-deterministic simulation bugs) |

Additionally, the storage format must satisfy key project constraints:
1. **Cross-Language Compatibility ([ADR-0005](0005-cross-language-binary-serialization.md))**: Match files should be inspectable by external tools written in Python, Godot (GDScript), or Lua.
2. **Ephemeral Network Decoupling ([ADR-0008](0008-session-lifecycle-and-client-identity.md))**: Live play identifies clients by `SocketAddr`, which does not exist or varies across replay environments.
3. **Automated Desync Detection**: Automated testing on CI needs a mechanism to detect divergence without re-serializing entire world states on every frame.

## Decision

We adopt **Event Sourcing with Protobuf v3 Envelope Serialization and Periodic SHA-256 State Checkpoints** for match recording (`.loci` files):

### 1. Protobuf Replay Container Schema (`proto/replay.proto`)
All replay recordings are stored as structured binary files defined by Protocol Buffers:

```protobuf
syntax = "proto3";
package loci2d;

import "game_packets.proto";

message ReplayHeader {
  string magic = 1;             // "LOCI_REPLAY"
  uint32 version = 2;           // Replay format version (e.g. 1)
  uint32 tick_rate = 3;         // Server tick rate (e.g. 30 Hz)
  uint64 start_timestamp = 4;   // Unix timestamp (ms)
  uint64 instance_id = 5;       // Instance ID
  uint64 random_seed = 6;       // Deterministic PRNG seed
  string map_name = 7;          // Map identifier
  string script_hash = 8;       // SHA-256 hash of the Lua scripts active at Tick 0
}

message ReplayIntentEntry {
  uint64 entity_id = 1;         // Decoupled from SocketAddr
  string player_name = 2;       // Preserved for join events
  ClientIntent intent = 3;      // Concrete intent (Move, Action, Join, Disconnect)
}

message ReplayTickFrame {
  uint64 tick = 1;
  repeated ReplayIntentEntry entries = 2;
}

message ReplayCheckpoint {
  uint64 tick = 1;
  bytes state_sha256 = 2;       // 32-byte SHA-256 digest of canonical WorldState
  uint32 active_entities = 3;
}

message ReplayFile {
  ReplayHeader header = 1;
  repeated ReplayTickFrame frames = 2;
  repeated ReplayCheckpoint checkpoints = 3;
}
```

### 2. Script Versioning
Because `loci2d` embeds Lua scripting (Phase 6), the simulation rules can change based on the active scripts. To guarantee determinism during playback, the `.loci` header stores a `script_hash` (e.g., SHA-256 of the active Lua scripts at Tick 0). 

During replay validation (`--verify`), the engine hashes the local script files. If the hash does not match `script_hash`, the engine aborts loading to prevent a guaranteed desync due to version mismatch. (Note: True cross-version replay support and hot-reloading are deferred to Phase 9).

### 3. Network Decoupling via Entity ID Stamping
During live gameplay, incoming `(SocketAddr, ClientIntent)` tuples are resolved inside the Game Loop thread to their assigned `entity_id`. The recorder stamps events directly with `entity_id` and player name, allowing the replay engine to apply inputs directly to `Instance` entities without creating dummy socket addresses.

### 3. Periodic Canonical State Checkpointing
Every $K$ ticks (e.g., every 60 ticks / 2 seconds), the recorder hashes the canonical binary representation of the `Instance` (sorted by `entity_id` via `BTreeMap` with raw fixed-point bits) using SHA-256. These checkpoints are stored in the `.loci` container.

During replay validation (`--verify`), the player computes its local state hash on checkpoint ticks and asserts bit-exact equality against the recorded hash, pinpointing the exact frame where any desync occurs.

## Consequences

**Positive:**
- **Minimal Disk Footprint**: Event sourcing records only active inputs, resulting in files smaller than 100 KB for typical matches.
- **Cross-Language Inspection**: Any language with Protobuf support (Python, GDScript, Lua) can parse `.loci` replay files for data analytics or UI match statistics.
- **Instant Desync Localization**: SHA-256 checkpoints catch simulation drift immediately on the exact tick it occurs.
- **Network Agnostic**: Eliminates dependencies on host IP addresses or port numbers during replay playback.

**Negative:**
- **Sequential Playback Requirement**: Scrubbing to tick $N$ requires stepping the simulation from tick 0 (acceptable for standard match durations; intermediate state snapshot checkpoints can be added in future phases if instant seeking is needed).
- **Strict Determinism Dependency**: Any divergence in physics calculation breaks all subsequent frames (addressed by [ADR-0007](0007-deterministic-simulation-and-fixed-point.md)).
