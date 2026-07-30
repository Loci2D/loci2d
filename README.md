# loci2d

An authoritative, instance-based UDP game server in Rust using **Protocol Buffers (Protobuf v3)** for cross-language binary packet serialization.

Designed for real-time 2D multiplayer games (e.g. MOBAs) and educational environments where clients can be built using any game engine or programming language, including **Godot (GDScript/C#)**, **Love2D (Lua)**, **Python**, or custom C++/WebAssembly clients.

---

## Architecture & Serialization

- **Protocol Definition (`proto/game_packets.proto`)**: Authoritative `.proto` schema defining envelopes (`GamePacket`, `ServerResponse`) and extensible client intents (`MoveIntent`, `ActionIntent`, `PingIntent`).
- **Rust Code Generation**: Handled automatically at build time via `build.rs`, `prost-build`, and `protoc-bin-vendored` (no manual `protoc` installation required).
- **Architectural Decision Records**: See [ADR 0005: Cross-Language Binary Serialization](docs/adr/0005-cross-language-binary-serialization.md) for full design rationale.

---

## Multi-Language Client Examples

Cross-language client integration examples are available in the [`examples/`](examples/) directory:

- **[Python Example](examples/python/README.md)**: Python UDP client using standard `protobuf`.
- **[Love2D Example](examples/love2d/README.md)**: Love2D Lua client using `lua-protobuf` and `luasocket`.
- **[Godot Example](examples/godot/README.md)**: Godot 4 GDScript example using `PacketPeerUDP` and GDScript Protobuf.

---

## Quickstart & Testing (Rust Server & CLI Client)

### 1. Start the Server
```bash
cargo run
```
The server binds to `127.0.0.1:8080` over UDP and deserializes Protobuf `GamePacket` messages.

### 2. Start the Rust CLI Client
In a second terminal:
```bash
cargo run --bin client
```

### 3. Client Commands
- `ping` — Send ping intent
- `move <x> <y>` — Send 2D movement intent (e.g., `move 1.0 0.5`)
- `action <id>` — Send action intent with ability ID (e.g., `action 42`)
- `quit` — Exit client

### Example Session
```
Enter command (move/action/ping/quit): ping
[2026-07-30 15:30:00] Sent 12 bytes to server: sequence_id=0
[2026-07-30 15:30:00] Received 19 bytes from 127.0.0.1:8080: sequence_id=0, status=ACK: sequence_id=0

Enter command (move/action/ping/quit): move 1.0 0.5
[2026-07-30 15:30:05] Sent 22 bytes to server: sequence_id=1
[2026-07-30 15:30:05] Received 19 bytes from 127.0.0.1:8080: sequence_id=1, status=ACK: sequence_id=1
```
