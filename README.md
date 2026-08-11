# loci2d

### English
An authoritative, instance-based UDP game server in Rust using **Protocol Buffers (Protobuf v3)** for cross-language binary packet serialization.

Designed for real-time 2D multiplayer games (e.g. MOBAs) and educational environments where clients can be built using any game engine or programming language, including **Godot (GDScript/C#)**, **Love2D (Lua)**, **Python**, or custom C++/WebAssembly clients.

### Português
Um servidor de jogos UDP autoritativo e baseado em instâncias escrito em Rust, utilizando **Protocol Buffers (Protobuf v3)** para serialização binária de pacotes entre diferentes linguagens.

Projetado para jogos multiplayer 2D em tempo real (ex: MOBAs) e ambientes educacionais, permitindo que clientes sejam desenvolvidos em qualquer engine ou linguagem de programação, incluindo **Godot (GDScript/C#)**, **Love2D (Lua)**, **Python** ou clientes customizados em C++/WebAssembly.

---

## How It Works / Como Funciona

```
[ Client (Godot / Love2D) ]  ---( 1. Send Intent: "Move Left" )---> [ loci2d Server ]
                                                                        | ( 2. Process Physics/Tick )
[ All Clients ]              <---( 3. Broadcast World State )-----------+
```

### English
1. **Client Sends Intent**: The client game engine (Godot, Love2D, etc.) does not modify game state directly. It sends an intent (e.g., *"Move direction X, Y"* or *"Cast ability 1"*).
2. **Server Processes Tick**: `loci2d` runs the authoritative game loop for that instance, validates physics/collisions, and updates entity states.
3. **Server Syncs State**: `loci2d` broadcasts the authoritative world state back to all connected clients for rendering.

### Português
1. **Cliente Envia Intenção**: A engine do cliente (Godot, Love2D, etc.) não altera o estado do jogo diretamente. Ela envia uma intenção (ex: *"Mover na direção X, Y"* ou *"Usar habilidade 1"*).
2. **Servidor Processa o Tick**: O `loci2d` executa o game loop autoritativo daquela instância, valida a física/colisões e atualiza os estados das entidades.
3. **Servidor Sincroniza o Estado**: O `loci2d` envia o estado oficial do mundo de volta para todos os clientes renderizarem na tela.

---

## Architecture & Serialization / Arquitetura & Serialização

### English
- **Protocol Definition (`proto/game_packets.proto`)**: Authoritative `.proto` schema defining envelopes (`GamePacket`, `ServerPacket`), outbound snapshots (`WorldState`, `EntityState`), responses (`ServerResponse`), and extensible client intents (`MoveIntent`, `MoveToPositionIntent`, `ActionIntent`, `PingIntent`, `JoinIntent`, `DisconnectIntent`).
- **Rust Code Generation**: Handled automatically at build time via `build.rs`, `prost-build`, and `protoc-bin-vendored` (no manual `protoc` installation required).
- **Architectural Decision Records**: See [ADR 0005: Cross-Language Binary Serialization](docs/adr/0005-cross-language-binary-serialization.md) and [ADR 0006: Two-Thread Network/Game-Loop Separation](docs/adr/0006-two-thread-network-gameloop-separation.md) for full design rationale.
- **Project Roadmap**: See [docs/roadmap.md](docs/roadmap.md) for planned milestones and development stages.

### Português
- **Definição de Protocolo (`proto/game_packets.proto`)**: Esquema `.proto` autoritativo definindo envelopes (`GamePacket`, `ServerPacket`), snapshots de saída (`WorldState`, `EntityState`), respostas (`ServerResponse`) e intenções extensíveis de cliente (`MoveIntent`, `MoveToPositionIntent`, `ActionIntent`, `PingIntent`, `JoinIntent`, `DisconnectIntent`).
- **Geração de Código Rust**: Executada automaticamente no tempo de compilação via `build.rs`, `prost-build` e `protoc-bin-vendored` (sem necessidade de instalação manual do `protoc`).
- **Registros de Decisão de Arquitetura (ADRs)**: Veja a [ADR 0005: Serialização Binária Multi-linguagem](docs/adr/0005-cross-language-binary-serialization.md) e [ADR 0006: Separação Rede/Game-Loop](docs/adr/0006-two-thread-network-gameloop-separation.md) para a justificativa do design.
- **Roadmap do Projeto**: Veja [docs/roadmap.md](docs/roadmap.md) para os marcos planejados e etapas de desenvolvimento.

---

## Multi-Language Client Examples / Exemplos de Clientes em Várias Linguagens

### English
Cross-language client integration examples are available in the [`examples/`](examples/) directory:

- **[Python Example](examples/python/README.md)**: Python UDP client using standard `protobuf` decoding real-time `WorldState` snapshots.
- **[Love2D Example](examples/love2d/README.md)**: Love2D Lua client using `lua-protobuf` rendering 2D multi-entity movement in real time.
- **[Godot Example](examples/godot/README.md)**: Godot 4 GDScript example using `PacketPeerUDP` and GDScript Protobuf for spatial synchronization.

### Português
Exemplos de integração de clientes em diferentes linguagens estão disponíveis no diretório [`examples/`](examples/):

- **[Exemplo em Python](examples/python/README.md)**: Cliente UDP em Python utilizando a biblioteca padrão `protobuf` decodificando snapshots de `WorldState` em tempo real.
- **[Exemplo em Love2D](examples/love2d/README.md)**: Cliente em Lua para Love2D utilizando `lua-protobuf` renderizando movimentação 2D de múltiplas entidades em tempo real.
- **[Exemplo em Godot](examples/godot/README.md)**: Exemplo em GDScript para Godot 4 utilizando `PacketPeerUDP` e GDScript Protobuf para sincronização espacial.

---

## Quickstart & Testing / Início Rápido e Testes (Rust Server & CLI Client)

### 1. Start the Server / Iniciar o Servidor
```bash
cargo run
```
* **EN**: The server binds to `127.0.0.1:8080` over UDP and deserializes Protobuf `GamePacket` messages.
* **PT**: O servidor vincula-se ao endereço `127.0.0.1:8080` via UDP e desserializa mensagens `GamePacket` do Protobuf.

### 2. Start the Rust CLI Client / Iniciar o Cliente CLI em Rust
In a second terminal / Em um segundo terminal:
```bash
cargo run --bin client
```

### 3. Client Commands / Comandos do Cliente
- `join <name>` — Send join handshake with player name (e.g., `join Alice`) / Envia handshake de entrada com nome do jogador (ex: `join Alice`)
- `status` — Inspect latest world state snapshot and active entities / Inspeciona o snapshot mais recente do estado do mundo e entidades ativas
- `move <x> <y>` — Send 2D movement intent (e.g., `move 1.0 0.5`) / Envia intenção de movimento 2D (ex: `move 1.0 0.5`)
- `stream <on|off>` — Toggle live background snapshot logging (default: off) / Alterna o log contínuo de snapshots em segundo plano (padrão: desativado)
- `leave [reason]` — Send disconnect intent (e.g., `leave quitting`) / Envia intenção de desconexão (ex: `leave quitting`)
- `action <id>` — Send action intent with ability ID (e.g., `action 42`) / Envia intenção de ação com ID de habilidade (ex: `action 42`)
- `ping` — Send ping intent / Envia intenção de ping
- `quit` — Gracefully disconnect and exit client / Desconecta graciosamente e sai do cliente

### Example Session / Sessão de Exemplo
```
Enter command: join Alice
[2026-08-08 17:30:00] Sent 16 bytes to server: sequence_id=0

Enter command: status
--- [World State Snapshot | Tick 30 | Timestamp: 1723140000000] ---
  Active Entities (1):
    - Entity 1 ("Alice", Player) @ (0.0, 0.0), vel=(0.0, 0.0)
---------------------------------------------------------

Enter command: move 1.0 0.5
[2026-08-08 17:30:05] Sent 22 bytes to server: sequence_id=1

Enter command: status
--- [World State Snapshot | Tick 150 | Timestamp: 1723140005000] ---
  Active Entities (1):
    - Entity 1 ("Alice", Player) @ (4.0, 2.0), vel=(1.0, 0.5)
---------------------------------------------------------

Enter command: quit
[2026-08-08 17:30:10] Sending disconnect and shutting down...
[2026-08-08 17:30:10] Sent 22 bytes to server: sequence_id=2
```

---

## Deterministic Replay & Spectator System / Sistema de Replay Determinístico & Espectador

### English
`loci2d` includes an event-sourced deterministic match recording, verification, and live spectator broadcast engine:

```bash
# 1. Start Server with Live Match Recording
cargo run --bin loci2d -- --record match_01.loci

# 2. Custom Checkpoint Frequency (e.g. every 120 ticks)
cargo run --bin loci2d -- --record match_01.loci --checkpoint-interval 120

# 3. Headless Determinism Verification (CI-Ready, max CPU speed)
cargo run --bin loci2d -- --replay match_01.loci --verify

# 4. Live Spectator Broadcast Server (Broadcasts to Godot, Love2D, Python, CLI)
cargo run --bin loci2d -- --replay match_01.loci --broadcast 127.0.0.1:8080 --speed 1.0

# 5. Fast-Forward Replay Broadcast (2x or 4x speed)
cargo run --bin loci2d -- --replay match_01.loci --broadcast 127.0.0.1:8080 --speed 2.0
```

### Português
O `loci2d` inclui um motor de gravação de partidas baseado em event sourcing, verificação determinística e transmissão para espectadores ao vivo:

```bash
# 1. Iniciar Servidor com Gravação de Partida
cargo run --bin loci2d -- --record match_01.loci

# 2. Frequência Personalizada de Checkpoints (ex: a cada 120 ticks)
cargo run --bin loci2d -- --record match_01.loci --checkpoint-interval 120

# 3. Verificação Determinística Headless (Ideal para CI, velocidade máxima)
cargo run --bin loci2d -- --replay match_01.loci --verify

# 4. Servidor de Transmissão para Espectadores (Transmite para Godot, Love2D, Python, CLI)
cargo run --bin loci2d -- --replay match_01.loci --broadcast 127.0.0.1:8080 --speed 1.0

# 5. Transmissão Acelerada de Replay (velocidade 2x ou 4x)
cargo run --bin loci2d -- --replay match_01.loci --broadcast 127.0.0.1:8080 --speed 2.0
```

