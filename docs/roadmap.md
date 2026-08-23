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
> **Spec:** [Phase 4 Spec](roadmap-specs/phase4-deterministic-replay-spec.md) · **Reference ADRs:** [ADR-0001](adr/en/0001-authoritative-server-archoitecture.md) · [ADR-0002](adr/en/0002-instance-based-architecture.md) · [ADR-0003](adr/en/0003-2d-map-only.md) · [ADR-0005](adr/en/0005-cross-language-binary-serialization.md) · [ADR-0006](adr/en/0006-two-thread-network-gameloop-separation.md) · [ADR-0007](adr/en/0007-deterministic-simulation-and-fixed-point.md) · [ADR-0008](adr/en/0008-session-lifecycle-and-client-identity.md) · [ADR-0009](adr/en/0009-simplified-authentication-and-auto-join-strategy.md) · [ADR-0010](adr/en/0010-event-sourced-replay-format.md) · [ADR-0011](adr/en/0011-authoritative-spectator-replay-broadcasting.md)
### Fase 4: Registro de Eventos & Sistema de Replay Determinístico

#### English

* [x] **Deterministic Engine Core & Fixed-Point Refactoring**:
  - Replace `f32` physics simulation with fixed-point math (`I16F16`) to guarantee cross-CPU/OS arithmetic determinism.
  - Convert entity/session storage to strictly ordered collections (`BTreeMap`).
  - Upgrade game loop to a sub-millisecond Fixed-Timestep Accumulator.
* [x] **Event Sourcing Architecture & Replay Serialization**:
  - Log all incoming client intents sequentially at fixed tick boundaries into `.loci` Protobuf replay files.
  - Implement periodic canonical `WorldState` SHA-256 checkpoints for desync detection.
* [x] **Headless Replay & Desync Verification Tooling**:
  - Add CLI execution mode to replay match files offline as fast as possible, asserting bit-exact checksum matches across Linux, macOS ARM64, and Windows.
* [x] **Live Spectator Broadcast & Multi-Client Playback**:
  - Stream replay snapshots over UDP at real-time tick rates (with speed controls: 0.5x, 1x, 2x, 4x) to existing client engines (Godot, Love2D, Python, CLI) with zero client modifications.

#### Português
* [x] **Núcleo de Motor Determinístico & Refatoração de Ponto Fixo**:
  - Substituir a simulação física em `f32` por matemática de ponto fixo (`I16F16`) para garantir determinismo aritmético entre CPUs/SO.
  - Converter coleções de entidades/sessões para coleções estritamente ordenadas (`BTreeMap`).
  - Atualizar o game loop para um Acumulador de Timestep Fixo com precisão sub-milissegundo.
* [x] **Arquitetura de Event Sourcing & Serialização de Replay**:
  - Registrar sequencialmente todas as intenções de clientes nos limites de tick fixos em arquivos de replay Protobuf `.loci`.
  - Implementar checkpoints periódicos de hash SHA-256 do `WorldState` canônico para detecção de dessincronização.
* [x] **Replay Headless & Ferramental de Verificação de Dessincronização**:
  - Adicionar modo de execução CLI para reproduzir arquivos de partida offline na velocidade máxima, garantindo correspondência exata de checksums no Linux, macOS ARM64 e Windows.
* [x] **Transmissão para Espectadores & Reprodução Multi-Cliente**:
  - Transmitir snapshots de replay via UDP em taxa de tick em tempo real (com controles de velocidade: 0.5x, 1x, 2x, 4x) para os motores de cliente existentes (Godot, Love2D, Python, CLI) sem alterações no código do cliente.

---

### Phase 5: Deterministic Physics & Collision Engine
> **Spec:** [Phase 5 Spec](roadmap-specs/phase5-physics-and-collision-spec.md) · **Reference ADRs:** [ADR-0001](adr/en/0001-authoritative-server-archoitecture.md) · [ADR-0002](adr/en/0002-instance-based-architecture.md) · [ADR-0003](adr/en/0003-2d-map-only.md) · [ADR-0005](adr/en/0005-cross-language-binary-serialization.md) · [ADR-0006](adr/en/0006-two-thread-network-gameloop-separation.md) · [ADR-0007](adr/en/0007-deterministic-simulation-and-fixed-point.md) · [ADR-0008](adr/en/0008-session-lifecycle-and-client-identity.md) · [ADR-0010](adr/en/0010-event-sourced-replay-format.md) · [ADR-0011](adr/en/0011-authoritative-spectator-replay-broadcasting.md) · [ADR-0012](adr/en/0012-deterministic-2d-collision-and-kinematic-resolution.md) · [ADR-0013](adr/en/0013-server-authoritative-destination-steering-and-navigation.md)
### Fase 5: Motor de Física e Colisão Determinístico

#### English
* [x] **Deterministic Collision Detection**:
  - Implement fixed-point AABB (Axis-Aligned Bounding Box) and Circle colliders using `I16F16`.
* [x] **Static Map Geometry & Boundaries**:
  - Define world boundaries and static colliders (walls, obstacles) loaded from map files/configuration.
* [x] **Collision Resolution & Trigger Zones**:
  - Implement solid obstacle pushback and trigger/sensor zones (`on_overlap_enter` / `on_overlap_exit`).
* [x] **Click-to-Move Steering & Navigation**:
  - Implement destination-based steering (`MoveToPositionIntent`) with basic obstacle navigation for RTS/MOBA controls.
* [-] **[Deferred to Phase 8] Spatial Partitioning (Optional Optimization)**:
  - Add a fixed spatial hash grid for efficient broadphase collision queries.

#### Português
* [x] **Detecção de Colisão Determinística**:
  - Implementar colisores AABB (Caixa Delimitadora) e de Círculos usando ponto fixo `I16F16`.
* [x] **Geometria Estática do Mapa & Limites**:
  - Definir limites do mundo e colisores estáticos (paredes, obstáculos) carregados de arquivos de mapa/configuração.
* [x] **Resolução de Colisão & Zonas de Gatilho (Triggers)**:
  - Implementar desvio/bloqueio de obstáculos sólidos e zonas de sensores/gatilhos (`on_overlap_enter` / `on_overlap_exit`).
* [x] **Navegação & Movimento por Clique (Click-to-Move)**:
  - Implementar movimentação baseada em destino (`MoveToPositionIntent`) com navegação básica por obstáculos para controles RTS/MOBA.
* [-] **[Adiado para a Fase 8] Particionamento Espacial (Otimização Opcional)**:
  - Adicionar um spatial hash grid fixo para consultas eficientes de colisão em fase ampla (broadphase).

---

### Phase 6: Embedded Scripting & Game Logic (Lua Engine)
### Fase 6: Scripting Embutido & Lógica de Jogo (Motor Lua)

#### English
* [x] **Lua VM Integration (`mlua`)**:
  - Embed a sandboxed, deterministic Lua runtime into the game instance.
* [x] **Rust-to-Lua Engine API**:
  - Expose entity manipulation, fixed-point vectors, intent hooks, and collision/trigger callbacks to Lua.
* [x] **Event-Driven Gameplay Callbacks**:
  - Implement lifecycle hooks: `on_init`, `on_tick`, `on_player_join`, `on_player_leave`, `on_collision`.
* [-] **[Deferred to Phase 9] Hot-Reloadable Game Rules**:
  - Support reloading script files at runtime without restarting the server binary.
* [x] **Deterministic Script Execution**:
  - Ensure Lua callbacks execute strictly at deterministic tick boundaries to preserve `.loci` replay parity.

#### Português
* [x] **Integração com Máquina Virtual Lua (`mlua`)**:
  - Embutir um ambiente de execução Lua seguro e determinístico dentro da instância do jogo.
* [x] **API de Engine Rust-para-Lua**:
  - Expor manipulação de entidades, vetores de ponto fixo, ganchos de intenção e callbacks de colisão/gatilho para Lua.
* [x] **Callbacks de Jogabilidade Orientados a Eventos**:
  - Implementar ganchos de ciclo de vida: `on_init`, `on_tick`, `on_player_join`, `on_player_leave`, `on_collision`.
* [-] **[Adiado para a Fase 9] Regras de Jogo com Recarregamento Dinâmico (Hot-Reload)**:
  - Suportar recarregamento de scripts em tempo de execução sem reiniciar o executável do servidor.
* [x] **Execução Determinística de Scripts**:
  - Garantir que callbacks Lua sejam executados estritamente nos limites determinísticos de ticks para preservar a paridade de replays `.loci`.

---

### Phase 6.5: Validation, Developer Experience (DX) & API Stabilization
> **Milestone Focus:** Feature freeze on new engine features to focus on v0.6.x stability, DX, client abstraction, and user-friendly documentation before moving to multi-room infrastructure.

### Fase 6.5: Validação, Experiência do Desenvolvedor (DX) & Estabilização de API
> **Foco do Marco:** Pausa temporária na adição de novas funcionalidades estruturais para focar na estabilidade da versão v0.6.x, DX, abstração de clientes e documentação amigável antes de avançar para a infraestrutura de múltiplas salas.

#### English
> [!NOTE]
> **Strategic Pause (v0.6.x Consolidation):** Once Phase 6 is reached, `loci2d` achieves a fully playable, deterministic LAN multiplayer stack. New feature development will be temporarily delayed to validate the engine, gather playtester feedback, and refine the public API.
> 
> Due to the heterogeneous nature of this milestone (spanning client SDKs, documentation, and engine architecture), the detailed roadmap and sub-specifications for Phase 6.5 have been moved to a dedicated document.
> 
> **[View the full Phase 6.5 Roadmap here](phase6.5-roadmap.md)** (See [ADR 0015](adr/en/0015-dedicated-roadmap-structure-phase6.5-validation-dx.md) for details).

#### Português

> **Pausa Estratégica (Consolidação da v0.6.x):** Ao atingir a Fase 6, o `loci2d` alcança uma pilha multiplayer em LAN totalmente jogável e determinística. O desenvolvimento de novas funcionalidades será pausado temporariamente para validar o motor, coletar feedback e refinar a API pública.
> 
> Devido à natureza heterogênea deste marco (abrangendo SDKs de clientes, documentação e arquitetura da engine), o roadmap detalhado e as sub-especificações para a Fase 6.5 foram movidos para um documento dedicado.
> 
> **[Veja o Roadmap completo da Fase 6.5 aqui](phase6.5-roadmap.md)** (Veja a [ADR 0015](adr/pt/0015-estrutura-roadmap-dedicado-fase6.5-validacao-dx.md) para detalhes).
---

### Phase 7: Multi-Instance & Room Management
### Fase 7: Gerenciador de Múltiplas Instâncias e Salas

#### English
* [ ] **Dynamic Room Lifecycle**:
  - Allocate, configure (map, scripts, tick rate, max players), and teardown game rooms dynamically.
* [ ] **Packet Demultiplexing & Room Routing**:
  - Route incoming client UDP packets to target instances via Room ID headers or port allocation.
* [ ] **Concurrent Room Execution**:
  - Execute multiple room loops in parallel across a threadpool / async worker tasks with memory isolation.
* [ ] **Per-Room Replay Logging**:
  - Isolate `.loci` match replay recordings per room instance.
* [ ] **Server Room Administration & Metrics**:
  - CLI commands and metrics to monitor active rooms, player counts, and tick health.

#### Português
* [ ] **Ciclo de Vida Dinâmico de Salas**:
  - Alocar, configurar (mapa, scripts, tick rate, limite de jogadores) e destruir salas de jogo dinamicamente.
* [ ] **Demultiplexação de Pacotes & Roteamento de Salas**:
  - Encaminhar pacotes UDP dos clientes para as instâncias corretas via cabeçalho de ID de Sala ou alocação de portas.
* [ ] **Execução Concorrente de Salas**:
  - Executar múltiplos loops de salas em paralelo através de um threadpool / workers assíncronos com isolamento de memória.
* [ ] **Gravação de Replays por Sala**:
  - Isolar gravações de replay de partidas `.loci` por instância de sala.
* [ ] **Administração & Métricas de Salas do Servidor**:
  - Comandos CLI e métricas para monitorar salas ativas, contagem de jogadores e integridade dos ticks.

---

### Phase 8: Production Hardening, Security & Authentication
### Fase 8: Endurecimento de Produção, Segurança & Autenticação

#### English
* [ ] **Session Authentication & Tokens**:
  - Implement secure handshake validation, session tokens, and optional external auth webhooks.
* [ ] **Intent Validation & Anti-Tamper**:
  - Enforce server-side intent rate-limiting, packet sequence integrity, and velocity/teleportation sanity checks.
* [ ] **Lua Sandbox Security Hardening**:
  - Implement OOM protection (memory limits per instance), disable dangerous base globals (`dofile`, `load`, `getmetatable`), and establish CI penetration tests to prevent Sandbox escapes.
* [ ] **Production Observability & Deployment**:
  - Structured logging, Prometheus metrics, and production containerization.

#### Português
* [ ] **Autenticação de Sessão & Tokens**:
  - Implementar validação de handshake seguro, tokens de sessão e webhooks opcionais de autenticação externa.
* [ ] **Validação de Intenções & Anti-Adulteração**:
  - Aplicar limitação de taxa (rate-limiting) de intenções, integridade de sequência de pacotes e checagens de sanidade de velocidade/teletransporte.
* [ ] **Blindagem de Segurança do Sandbox Lua**:
  - Implementar proteção OOM (limites de memória por instância), desabilitar globais perigosas nativas (`dofile`, `load`, `getmetatable`) e estabelecer testes de penetração na CI para evitar escapes do Sandbox.
* [ ] **Observabilidade de Produção & Implantação**:
  - Logs estruturados, métricas Prometheus e conteinerização para produção.

---

### Phase 9: Script Versioning & Hot-Reloading
### Fase 9: Versionamento de Scripts & Recarregamento Dinâmico

#### English
* [ ] **Cross-Version Replay Support**:
  - Bundle or version Lua scripts to ensure clients can watch old `.loci` replays with the exact logic used during that match.
* [ ] **Immutable Live Instances**:
  - Guarantee that hot-reloading scripts on the server only affects new instances; running matches must finish on their original script versions to maintain determinism.

#### Português
* [ ] **Suporte a Replays de Múltiplas Versões**:
  - Empacotar ou versionar scripts Lua para garantir que clientes possam assistir a replays `.loci` antigos com a lógica exata usada naquela partida.
* [ ] **Instâncias Ao Vivo Imutáveis**:
  - Garantir que recarregar scripts no servidor afete apenas novas instâncias; partidas em andamento devem terminar em suas versões de script originais para manter o determinismo.
