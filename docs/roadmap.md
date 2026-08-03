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

### Phase 1: Game Loop & Single-Instance Core (Active / Next Step)
### Fase 1: Game Loop & Núcleo de Instância Única (Ativo / Próximo Passo)

#### English
* [ ] **Network & Loop Integration**:
  - Connect UDP socket network thread to `GameLoop` via thread-safe channels (`mpsc`).
  - Implement non-blocking intent queue consumption inside fixed tick rate (e.g., 20/30 Hz).
* [ ] **Localhost Single Instance**:
  - Ensure a single `Instance` manages entity positions, input application, and spatial ticks on `localhost`.
  - Multi-instance / room management is deferred to future phases.

#### Português
* [ ] **Integração de Rede & Game Loop**:
  - Conectar a thread de rede do socket UDP ao `GameLoop` via canais seguros para threads (`mpsc`).
  - Implementar o consumo não-bloqueante da fila de intenções dentro da taxa de tick fixa (ex: 20/30 Hz).
* [ ] **Instância Única em Localhost**:
  - Garantir que uma única `Instance` gerencie posições de entidades, aplicação de inputs e ticks espaciais em `localhost`.
  - Gerenciamento de múltiplas instâncias / salas é postergado para fases futuras.

---

### Phase 2: Lightweight Client/Session Mapping
### Fase 2: Mapeamento Leve de Cliente/Sessão

#### English
* [ ] **SocketAddr to Entity Mapping**:
  - Map incoming client IP / `SocketAddr` directly to a unique `EntityID` inside the active instance.
  - Handle basic client join / disconnect / timeout detection.
* [ ] **Simplified Handshake**:
  - Keep authentication minimal (no token/auth servers for now, optimized for rapid prototype validation).

#### Português
* [ ] **Mapeamento de SocketAddr para Entidade**:
  - Mapear o IP / `SocketAddr` do cliente diretamente para um `EntityID` único dentro da instância ativa.
  - Tratar entrada, desconexão básica e detecção de timeout de clientes.
* [ ] **Handshake Simplificado**:
  - Manter a autenticação mínima (sem servidores de token/auth por enquanto, otimizado para validação rápida do protótipo).

---

### Phase 3: World State Broadcasting & Client Sync
### Fase 3: Transmissão de Estado do Mundo & Sincronização de Clientes

#### English
* [ ] **Snapshot Generation**:
  - Create a `WorldState` snapshot protobuf packet representing all active entity positions and states.
* [ ] **Tick Broadcast**:
  - Send state snapshots to all mapped client addresses at regular tick intervals.
* [ ] **Client Integration**:
  - Validate snapshot consumption in Love2D, Godot, Python, and Rust CLI examples.

#### Português
* [ ] **Geração de Snapshots**:
  - Criar um pacote Protobuf de snapshot (`WorldState`) representando as posições e estados de todas as entidades ativas.
* [ ] **Transmissão por Tick**:
  - Enviar snapshots de estado para todos os endereços de clientes mapeados em intervalos regulares de tick.
* [ ] **Integração com Clientes**:
  - Validar o consumo de snapshots nos exemplos de Love2D, Godot, Python e CLI Rust.

---

### Phase 4: Event Logging & Deterministic Replay System
### Fase 4: Registro de Eventos & Sistema de Replay Determinístico

#### English
* [ ] **Event Sourcing Architecture**:
  - Log all incoming client intents, timestamp/tick numbers, and system inputs sequentially into an event log buffer.
* [ ] **Deterministic Execution Guarantee**:
  - Ensure fixed-point math / deterministic tick updates so that replaying an intent log reproduces exact world state frame-by-frame.
* [ ] **Replay Storage & Playback Tooling**:
  - Implement serialization for input logs (e.g., binary file or Protobuf log).
  - Add CLI / server replay execution mode for debugging, testing, and spectator replays.

#### Português
* [ ] **Arquitetura de Event Sourcing**:
  - Registrar sequencialmente todas as intenções de clientes recebidas, números de tick/timestamp e inputs de sistema em um buffer de log de eventos.
* [ ] **Garantia de Execução Determinística**:
  - Garantir matemática/atualizações de tick determinísticas para que a reexecução de um log de intenções reproduza o estado exato do mundo quadro a quadro.
* [ ] **Armazenamento e Ferramental de Replay**:
  - Implementar serialização para logs de input (ex: arquivo binário ou log Protobuf).
  - Adicionar modo de execução de replay via CLI / servidor para depuração, testes e replays de espectadores.

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
