# loci2d Project Roadmap / Roadmap do Projeto loci2d

### English
This document outlines the development phases, goals, and technical milestones for **loci2d**, an authoritative 2D game server framework in Rust.

### Português
Este documento descreve as fases de desenvolvimento, metas e marcos técnicos para o **loci2d**, um framework de servidor de jogos 2D autoritativo em Rust.

---

## Current Vision & Approach / Visão Atual & Abordagem

### English
* **Validation Phase**: Focus on getting a fully functional, stable **single instance** running locally (`127.0.0.1`).
* **Authoritative Model**: Receives client intents, processes physics/game loop, and broadcasts authoritative state.
* **Low Barrier**: Minimal complexity for indie devs and students.

### Português
* **Fase de Validação**: Foco em obter uma **única instância** totalmente funcional e estável rodando localmente (`127.0.0.1`).
* **Modelo Autoritativo**: Recebe intenções dos clientes, processa física/game loop e transmite o estado autoritativo.
* **Baixa Barreira de Entrada**: Complexidade mínima para desenvolvedores indies e estudantes.

---

## Roadmap Phases / Fases do Roadmap

### Phase 1: Game Loop & Single-Instance Core (Done)
### Fase 1: Game Loop & Núcleo de Instância Única (Concluído)

#### English
* [x] **Network & Loop Integration**:
  - Connect UDP socket network thread to `GameLoop` via thread-safe channels (`mpsc`).
  - Implement non-blocking intent queue consumption inside fixed tick rate (e.g., 20/30 Hz).
* [x] **Localhost Single Instance**:
  - Ensure a single `Instance` manages entity positions, input application, and spatial ticks on `localhost`.
  - Multi-instance / room management is deferred to future phases.

#### Português
* [x] **Integração de Rede & Game Loop**:
  - Conectar a thread de rede do socket UDP ao `GameLoop` via canais seguros para threads (`mpsc`).
  - Implementar o consumo não-bloqueante da fila de intenções dentro da taxa de tick fixa (ex: 20/30 Hz).
* [x] **Instância Única em Localhost**:
  - Garantir que uma única `Instance` gerencie posições de entidades, aplicação de inputs e ticks espaciais em `localhost`.
  - Gerenciamento de múltiplas instâncias / salas é postergado para fases futuras.

---

### Phase 2: Lightweight Client/Session Mapping (Done)
> **Spec:** [Phase 2 Spec](roadmap-specs/phase2-client-session-mapping-spec.md) · **Reference ADRs:** [ADR-0001](adr/en/0001-authoritative-server-archoitecture.md) · [ADR-0002](adr/en/0002-instance-based-architecture.md) · [ADR-0005](adr/en/0005-cross-language-binary-serialization.md) · [ADR-0006](adr/en/0006-two-thread-network-gameloop-separation.md) · [ADR-0008](adr/en/0008-session-lifecycle-and-client-identity.md) · [ADR-0009](adr/en/0009-simplified-authentication-and-auto-join-strategy.md)
### Fase 2: Mapeamento Leve de Cliente/Sessão (Concluído)

#### English
* [x] **SocketAddr to Entity Mapping**:
  - Map incoming client IP / `SocketAddr` directly to a unique `EntityID` inside the active instance.
  - Handle basic client join / disconnect / timeout detection.
* [x] **Simplified Handshake**:
  - Keep authentication minimal (no token/auth servers for now, optimized for rapid prototype validation).

#### Português
* [x] **Mapeamento de SocketAddr para Entidade**:
  - Mapear o IP / `SocketAddr` do cliente diretamente para um `EntityID` único dentro da instância ativa.
  - Tratar entrada, desconexão básica e detecção de timeout de clientes.
* [x] **Handshake Simplificado**:
  - Manter a autenticação mínima (sem servidores de token/auth por enquanto, otimizado para validação rápida do protótipo).

---

### Phase 3: World State Broadcasting & Client Sync (Done)
> **Spec:** [Phase 3 Spec](roadmap-specs/phase3-world-state-broadcasting-spec.md) · **Reference ADRs:** [ADR-0001](adr/en/0001-authoritative-server-archoitecture.md) · [ADR-0002](adr/en/0002-instance-based-architecture.md) · [ADR-0003](adr/en/0003-2d-map-only.md) · [ADR-0005](adr/en/0005-cross-language-binary-serialization.md) · [ADR-0006](adr/en/0006-two-thread-network-gameloop-separation.md) · [ADR-0008](adr/en/0008-session-lifecycle-and-client-identity.md) · [ADR-0009](adr/en/0009-simplified-authentication-and-auto-join-strategy.md)
### Fase 3: Transmissão de Estado do Mundo & Sincronização de Clientes (Concluído)

#### English
* [x] **Snapshot Generation**:
  - Create a `WorldState` snapshot protobuf packet representing all active entity positions and states.
* [x] **Tick Broadcast**:
  - Send state snapshots to all mapped client addresses at regular tick intervals.
* [x] **Client Integration**:
  - Validate snapshot consumption in Love2D, Godot, Python, and Rust CLI examples.

#### Português
* [x] **Geração de Snapshots**:
  - Criar um pacote Protobuf de snapshot (`WorldState`) representando as posições e estados de todas as entidades ativas.
* [x] **Transmissão por Tick**:
  - Enviar snapshots de estado para todos os endereços de clientes mapeados em intervalos regulares de tick.
* [x] **Integração com Clientes**:
  - Validar o consumo de snapshots nos exemplos de Love2D, Godot, Python e CLI Rust.

---

### Phase 4: Event Logging & Deterministic Replay System
> **Spec:** [Phase 4 Spec](roadmap-specs/phase4-deterministic-replay-spec.md) · **Reference ADRs:** [ADR-0001](adr/en/0001-authoritative-server-archoitecture.md) · [ADR-0002](adr/en/0002-instance-based-architecture.md) · [ADR-0003](adr/en/0003-2d-map-only.md) · [ADR-0005](adr/en/0005-cross-language-binary-serialization.md) · [ADR-0006](adr/en/0006-two-thread-network-gameloop-separation.md) · [ADR-0007](adr/en/0007-deterministic-simulation-and-fixed-point.md) · [ADR-0008](adr/en/0008-session-lifecycle-and-client-identity.md) · [ADR-0009](adr/en/0009-simplified-authentication-and-auto-join-strategy.md)
### Fase 4: Registro de Eventos & Sistema de Replay Determinístico

#### English

* [ ] **Deterministic Engine Core & Fixed-Point Refactoring**:
  - Replace `f32` physics simulation with fixed-point math (`I16F16`) to guarantee cross-CPU/OS arithmetic determinism.
  - Convert entity/session storage to strictly ordered collections (`BTreeMap`).
  - Upgrade game loop to a sub-millisecond Fixed-Timestep Accumulator.
* [ ] **Event Sourcing Architecture & Replay Serialization**:
  - Log all incoming client intents sequentially at fixed tick boundaries into `.loci` Protobuf replay files.
  - Implement periodic canonical `WorldState` SHA-256 checkpoints for desync detection.
* [ ] **Headless Replay & Desync Verification Tooling**:
  - Add CLI execution mode to replay match files offline as fast as possible, asserting bit-exact checksum matches across Linux, macOS ARM64, and Windows.
* [ ] **Live Spectator Broadcast & Multi-Client Playback**:
  - Stream replay snapshots over UDP at real-time tick rates (with speed controls: 0.5x, 1x, 2x, 4x) to existing client engines (Godot, Love2D, Python, CLI) with zero client modifications.

#### Português
* [ ] **Núcleo de Motor Determinístico & Refatoração de Ponto Fixo**:
  - Substituir a simulação física em `f32` por matemática de ponto fixo (`I16F16`) para garantir determinismo aritmético entre CPUs/SO.
  - Converter coleções de entidades/sessões para coleções estritamente ordenadas (`BTreeMap`).
  - Atualizar o game loop para um Acumulador de Timestep Fixo com precisão sub-milissegundo.
* [ ] **Arquitetura de Event Sourcing & Serialização de Replay**:
  - Registrar sequencialmente todas as intenções de clientes nos limites de tick fixos em arquivos de replay Protobuf `.loci`.
  - Implementar checkpoints periódicos de hash SHA-256 do `WorldState` canônico para detecção de dessincronização.
* [ ] **Replay Headless & Ferramental de Verificação de Dessincronização**:
  - Adicionar modo de execução CLI para reproduzir arquivos de partida offline na velocidade máxima, garantindo correspondência exata de checksums no Linux, macOS ARM64 e Windows.
* [ ] **Transmissão para Espectadores & Reprodução Multi-Cliente**:
  - Transmitir snapshots de replay via UDP em taxa de tick em tempo real (com controles de velocidade: 0.5x, 1x, 2x, 4x) para os motores de cliente existentes (Godot, Love2D, Python, CLI) sem alterações no código do cliente.

---

### Phase 5: Future Enhancements (Post-Validation)
### Fase 5: Melhorias Futuras (Pós-Validação)

#### English
* [ ] **Embedded Scripting (Lua Engine)**:
  - Integrate `mlua` for hot-swappable custom server logic and event callbacks.
* [ ] **Multi-Instance Room Manager**:
  - Dynamic allocation, creation, and teardown of game instances / rooms.
* [ ] **Authentication & Security**:
  - Add session tokens, security validation, and anti-tamper intent validation.
* [ ] **Client-Side Helpers**:
  - Client-side prediction and interpolation utilities for smoother rendering under latency.

#### Português
* [ ] **Scripting Embutido (Motor Lua)**:
  - Integrar `mlua` para lógica de servidor customizada e callbacks de eventos recarregáveis em tempo de execução.
* [ ] **Gerenciador de Múltiplas Instâncias / Salas**:
  - Alocação, criação e destruição dinâmica de instâncias de jogo / salas.
* [ ] **Autenticação & Segurança**:
  - Adicionar tokens de sessão, validação de segurança e validação de intenções contra adulteração.
* [ ] **Utilitários para Clientes**:
  - Predição no lado do cliente e utilitários de interpolação para renderização suave sob latência.

---

### Phase 6: Physics & Collision Engine
### Fase 6: Motor de Física e Colisão

#### English
* [ ] **Deterministic Collision Detection**:
  - Implement AABB (Axis-Aligned Bounding Box) and circle collision using fixed-point math.
* [ ] **Map Boundaries & Static Geometry**:
  - Define map boundaries and static colliders (e.g., walls, obstacles) loaded from configuration or Lua scripts.
* [ ] **Click-to-Move Steering & Navigation**:
  - Implement destination-based steering (`MoveToPositionIntent`) and obstacle pathfinding for RTS/MOBA mouse controls.
* [ ] **Spatial Partitioning (Optional)**:
  - Add a simple grid or QuadTree for efficient collision queries if entity count grows.

#### Português
* [ ] **Detecção de Colisão Determinística**:
  - Implementar colisão AABB (Caixa Delimitadora Alinhada aos Eixos) e de círculos usando matemática de ponto fixo.
* [ ] **Limites do Mapa e Geometria Estática**:
  - Definir limites do mapa e colisores estáticos (ex: paredes, obstáculos) carregados a partir de configurações ou scripts Lua.
* [ ] **Navegação & Movimento por Clique (Click-to-Move)**:
  - Implementar movimentação baseada em destino (`MoveToPositionIntent`) e desvio de obstáculos para controles de mouse estilo RTS/MOBA.
* [ ] **Particionamento Espacial (Opcional)**:
  - Adicionar um grid simples ou QuadTree para consultas de colisão eficientes caso o número de entidades cresça.

