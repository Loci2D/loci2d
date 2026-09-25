# Loci2D — Love2D Client SDK

> 🌐 *Read this in [Português (Brasil)](README.pt-BR.md)*

The **Loci2D Love2D SDK** (`loci_client.lua`) is a high-level abstraction layer designed to allow developers and students to build multiplayer games on the [LÖVE2D](https://love2d.org/) framework with minimal effort, without needing to interact directly with UDP sockets, raw Protocol Buffer buffers, or tick synchronization.

---

## SDK Directory Structure

```text
sdks/love2d/
├── loci_client.lua        # Main client module (API Facade, State Manager, and Netcode)
├── test_sdk.lua           # Unit test suite to validate the SDK locally
├── AI_REFERENCE.md        # Compact API reference formatted for LLMs / AI Agents
├── lib/
│   ├── game_packets.proto # Loci2D Protobuf packet definitions
│   ├── game_packets.pb    # Pre-compiled binary descriptor (ultra-fast loading)
│   └── protoc.lua         # Pure-Lua Protobuf parser (fallback for runtime dynamic parsing)
└── docs/
    ├── index.html         # Interactive visual guide for Love2D game development
    └── server_scripts.html# Guide for authoring server-side game rules
```

---

## Schema Loading Mechanism

`loci_client.lua` is designed for maximum fault tolerance across different environments:
1. **Pure-Lua Parser (`protoc.lua`):** Parses and registers `.proto` text schemas at runtime without requiring external compiler binaries.
2. **Pre-compiled Descriptor (`game_packets.pb`):** If runtime parsing is bypassed or unnecessary, the SDK loads the binary descriptor directly via `pb.loadfile()`.

---

## Prerequisites

If you have already verified your environment via the root script (`./tools/setup_environment.sh`), Love2D is ready to go. Otherwise:

1. **LÖVE 11.x+**: Installed on your system ([love2d.org](https://love2d.org/)).
2. **lua-protobuf (`pb`)**: Fast C/Lua binary module for Protobuf encoding and decoding.
   * **Linux:** `sudo luarocks install lua-protobuf` (or place `pb.so` inside `lib/`).
   * **Windows:** Place `pb.dll` in your project root or inside `lib/`.
   * **macOS:** `luarocks install lua-protobuf`.

---

## Testing the SDK Locally

Before launching your game in Love2D, you can verify that all abstractions (such as `Entity` metatables, auto-typed property casting, spatial radius queries, and event dispatching) are functional on your system by running:

```bash
lua sdks/love2d/test_sdk.lua
```

If the environment is configured correctly, the test suite will output:
```text
All loci_client.lua SDK unit tests passed successfully!
```

---

## Quickstart Integration Example

```lua
-- main.lua
package.path = package.path .. ";sdks/love2d/?.lua"
local loci = require("loci_client")

function love.load()
    -- Connect to the authoritative server
    loci.connect("127.0.0.1", 8080, "Player1", "sdks/love2d/lib/")

    loci.on_action_cast = function(entity, ability_id, dir_x, dir_y)
        print("Ability cast by entity:", entity.id)
    end
end

function love.update(dt)
    -- Process incoming network packets and interpolate entities
    loci.update(dt)

    -- Send movement input
    local dx, dy = 0, 0
    if love.keyboard.isDown("w") then dy = -1 end
    if love.keyboard.isDown("s") then dy = 1 end
    if love.keyboard.isDown("a") then dx = -1 end
    if love.keyboard.isDown("d") then dx = 1 end
    loci.send_move(dx, dy)
end

function love.draw()
    for _, entity in ipairs(loci.get_entities()) do
        if entity:is_local_player() then
            love.graphics.setColor(0.3, 0.6, 1.0)
        else
            love.graphics.setColor(0.3, 0.8, 0.4)
        end
        love.graphics.circle("fill", entity.x, entity.y, 16)
    end
end

function love.quit()
    loci.disconnect("Game closed")
end
```

For the complete interactive guide, open `sdks/love2d/docs/index.html` in your browser.
