# Love2D (Lua) Client Example for loci2d

This example demonstrates how a **LÖVE2D / Lua** game client can communicate with the `loci2d` authoritative server using `lua-protobuf` (`pb`) and LuaSocket (`socket`).

## Prerequisites

1. **Love2D**: Installed on your system ([love2d.org](https://love2d.org/)).
2. **lua-protobuf**: A C-module for Lua providing Protobuf encoding/decoding (`pb.lua` / `pb.so` / `pb.dll`).

## Project Structure

- `main.lua`: The Love2D main entry file initializing the UDP socket, loading `.proto` schema at runtime, sending input intents, and rendering server ACK responses.
- `proto/game_packets.proto`: Linked schema file loaded dynamically via `pb.loadfile()`.

## How It Works

`lua-protobuf` loads binary `.pb` descriptors or dynamically parses `.proto` text definitions via `protoc.lua`:

```lua
local pb = require("pb")
local protoc = require("protoc")

-- Load raw text .proto schema at runtime:
local p = protoc.new()
p:loadfile("../../proto/game_packets.proto")

-- Or load precompiled binary .pb descriptor:
-- pb.loadfile("game_packets.pb")
```

## Running the Example

### Player Mode
1. Start the `loci2d` server:
```bash
cargo run
```

2. Run Love2D:
```bash
love examples/love2d
```

### Spectator / Replay Mode
1. Start the replay broadcast server:
```bash
cargo run --bin loci2d -- --replay benchmark.loci
```

2. Run Love2D (it will automatically detect spectator mode, or pass `--spectate`):
```bash
love examples/love2d --spectate
```

#### Spectator Controls
- **WASD / Arrow Keys**: Pan the free camera around the arena
- **Mouse Drag (Left or Right Click)**: Pan the camera
- **Click on Entity**: Lock camera and follow the entity
- **Space / R**: Reset camera back to arena origin `(0, 0)`

