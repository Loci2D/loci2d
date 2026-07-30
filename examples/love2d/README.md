# Love2D (Lua) Client Example for loci2d

This example demonstrates how a **LÖVE2D / Lua** game client can communicate with the `loci2d` authoritative server using `lua-protobuf` (`pb`) and LuaSocket (`socket`).

## Prerequisites

1. **Love2D**: Installed on your system ([love2d.org](https://love2d.org/)).
2. **lua-protobuf**: A C-module for Lua providing Protobuf encoding/decoding (`pb.lua` / `pb.so` / `pb.dll`).

## Project Structure

- `main.lua`: The Love2D main entry file initializing the UDP socket, loading `.proto` schema at runtime, sending input intents, and rendering server ACK responses.
- `proto/game_packets.proto`: Linked schema file loaded dynamically via `pb.loadfile()`.

## How It Works

`lua-protobuf` can load `.pb` binary descriptors or raw `.proto` schema definitions at runtime:

```lua
local pb = require("pb")
local socket = require("socket")

-- Load schema definition dynamically
pb.loadfile("../../proto/game_packets.proto")

-- Prepare and send binary packet
local packet = {
    sequence_id = seq_id,
    timestamp = os.time(),
    intent = {
        move = {
            direction = { x = 1.0, y = 0.5 }
        }
    }
}

local data = pb.encode("loci2d.GamePacket", packet)
udp:send(data)
```

## Running the Example

1. Start the `loci2d` server:
```bash
cargo run
```

2. Run Love2D on this folder:
```bash
love .
```
