# Love2D (Lua) Client Example for loci2d

This example demonstrates how a **LÖVE2D / Lua** game client communicates with the `loci2d` authoritative server using the official [Love2D Client SDK (`loci_client.lua`)](../../sdks/love2d/README.md).

## Prerequisites & Installation

If you ran the root verification script (`./tools/setup_environment.sh`), all dependencies are likely already configured. Otherwise:

### 1. Install Love2D (LÖVE)
- **Ubuntu / Debian / Pop!_OS**:
  ```bash
  sudo apt update && sudo apt install -y love
  ```
- **Arch Linux**:
  ```bash
  sudo pacman -S love
  ```
- **Fedora**:
  ```bash
  sudo dnf install -y love
  ```
- **macOS**:
  ```bash
  brew install --cask love
  ```
- **Windows**:
  Download from [love2d.org](https://love2d.org/) or install with:
  ```powershell
  winget install LOVE.LOVE
  ```

### 2. Protobuf Library (`lua-protobuf`)
The dynamic proto compiler `protoc.lua` and pre-compiled schemas are located in `sdks/love2d/lib/`. For binary Protobuf serialization, ensure `lua-protobuf` is installed (e.g. `luarocks install lua-protobuf`) or place `pb.so` (Linux) / `pb.dll` (Windows) in `lib/`.

---

## Project Structure

- `main.lua`: The Love2D main entry file demonstrating game rendering, input handling, and particle effects using `loci_client.lua`.
- `sdks/love2d/lib/`: Protocol buffer schemas (`game_packets.proto`, `game_packets.pb`) and runtime parser (`protoc.lua`).

## How It Works

The example relies entirely on `loci_client.lua`, which encapsulates UDP networking, protobuf serialization, and fixed-point math:

```lua
local loci = require("loci_client")

function love.load()
    -- Connect to server and load protobuf schema automatically
    loci.connect("127.0.0.1", 8080, "Player1", "../../sdks/love2d/lib/")
end
```

## Running the Example

### Quick Launch (Recommended)
You can start both the server and the client with a single command from the repository root:
```bash
./tools/run_dev.sh
```

### Manual Launch (2 Terminais)

#### 1. Start the Server:
```bash
cargo run
```

#### 2. Run Love2D:
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

