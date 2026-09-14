# ADR 0018: Client SDKs and Network Abstraction

## Status
Accepted

## Context
The loci2d engine relies on a strict server-authoritative architecture utilizing Protocol Buffers over UDP (ADR-0004, ADR-0005). It requires explicit connection handshakes (Join/Disconnect intents - ADR-0008) and deterministic state synchronization.
As we prepare for the 3v3 Arena MVP (Phase 6), the target audience for building the game client includes high school students using Love2D. Expecting game developers (especially students) to manually handle UDP socket non-blocking loops, binary serialization, state interpolation, and timeout management is an unreasonable barrier to entry.

## Decision
We decided to completely abstract the engine's network complexity by providing official, ergonomic **Client SDKs** for supported engines (starting with Love2D via `loci_client.lua`).

The Client SDKs are responsible for:
1. **Network & Protocol Encapsulation:** Managing the underlying UDP socket, tracking sequence IDs, and serializing/deserializing Protobuf payloads internally.
2. **State Management:** Maintaining a local, canonical representation of the `WorldState` sent by the server, including automatic type casting of entity properties.
3. **High-Level API Facade:** Exposing a simple, event-driven interface to the developer. The developer will only interact with game state (e.g., `loci.get_entities()`) and high-level intents (e.g., `loci.send_move()`, `loci.send_action()`), completely isolated from the "netcode".
4. **Callbacks:** Triggering specific hooks (e.g., `on_entity_spawned`, `on_property_changed`) when the server broadcasts state changes, allowing the client to easily hook up UI and visual effects.

## Consequences
- **Positive:** Drastically lowers the barrier to entry for building clients. Developers can treat the multiplayer environment almost as a local single-player game.
- **Positive:** Standardizes critical network behavior (e.g., how disconnects, timeouts, and pings are handled) across all future clients (Love2D, Godot, Python).
- **Negative:** Introduces a significant maintenance burden. Any change to the server's protocol or core mechanics now requires synchronous updates across multiple language-specific SDKs.
