# Phase 6.5 Roadmap: Validation, DX & API Stabilization
# Roadmap da Fase 6.5: Validação, DX & Estabilização de API
> **Reference ADRs:** [ADR-0015](adr/en/0015-dedicated-roadmap-structure-phase6.5-validation-dx.md)


### English
This document details the sub-milestones for Phase 6.5 of the **loci2d** project. Phase 6.5 acts as a "strategic pause" (v0.6.x) to consolidate the engine's core features into a playable, developer-friendly multiplayer ecosystem before introducing multi-room infrastructure.

### Português
Este documento detalha os sub-marcos para a Fase 6.5 do projeto **loci2d**. A Fase 6.5 atua como uma "pausa estratégica" (v0.6.x) para consolidar as funcionalidades principais do motor em um ecossistema multiplayer jogável e amigável para desenvolvedores antes de introduzir a infraestrutura de múltiplas salas.

---

## 6.5.0-1: Entity Data Model & Global Properties
## 6.5.0-1: Modelo de Dados de Entidade & Propriedades Globais
> **Reference ADRs:** [ADR-0014](adr/en/0014-embedded-lua-scripting-and-command-buffer.md) · [ADR-0016](adr/en/0016-data-driven-entity-properties-and-engine-agnosticism.md)
> **Spec:** [phase6.5.0-1-data-model-spec.md](roadmap-specs/phase6.5.0-1-data-model-spec.md)

### English
* [x] **Entity Property System (Data-Driven Design):**
  - Add a flexible key-value property map to the `Entity` struct in Rust.
* [x] **Lua Property Interface:**
  - Expose `Loci.get_entity_property` and `Loci.Commands.set_property` to Lua scripts.
* [x] **Global Match Metadata:**
  - Allow Lua to manage global match variables (e.g., match score, player count) via the generic property system (`globals`).

### Português
* [x] **Sistema de Propriedades de Entidade:**
  - Adicionar um mapa chave-valor flexível na struct `Entity` no Rust.
* [x] **Interface de Propriedades no Lua:**
  - Expor `Loci.get_entity_property` e `Loci.Commands.set_property`.
* [x] **Metadados Globais da Partida:**
  - Permitir que o Lua gerencie variáveis globais (placar, etc.) via o sistema de propriedades genéricas (`globals`).

---

## 6.5.0-2: Match Lifecycle & Timers
## 6.5.0-2: Ciclo de Vida da Partida & Timers
> **Spec:** [phase6.5.0-2-match-lifecycle-spec.md](roadmap-specs/phase6.5.0-2-match-lifecycle-spec.md)

### English
* [x] **Match Lifecycle State Machine:**
  - Introduce a `Paused`/`Running`/`Ended` state machine to the server instance.
  - Expose `Loci.Commands.start_match()`, `Loci.Commands.pause_match()`, and `Loci.Commands.end_match()` to Lua scripts.
* [x] **Timer & Cooldown System:**
  - Expose `Loci.Commands.start_timer(timer_id, delay_ticks)` and route expiration to a new Lua callback `on_timer_complete(timer_id)`.

### Português
* [x] **Máquina de Estado do Ciclo de Vida da Partida:**
  - Introduzir uma máquina de estados `Pausado`/`Em Execução`/`Encerrado` para a instância.
  - Expor comandos de `start_match`, `pause_match` e `end_match`.
* [x] **Sistema de Timers e Cooldowns:**
  - Expor `Loci.Commands.start_timer` e o callback `on_timer_complete`.

---

## 6.5.0-3: Gameplay Actions & Physics Configuration
## 6.5.0-3: Ações de Gameplay & Configuração de Física
> **Spec:** [phase6.5.0-3-gameplay-actions-spec.md](roadmap-specs/phase6.5.0-3-gameplay-actions-spec.md)

### English
* [x] **Action/Ability Dispatch:**
  - Expand `ActionIntent` with a `target_direction` field for directional abilities.
  - Route the intent to a new Lua callback `on_action(entity_id, ability_id, dir_x, dir_y)`.
* [x] **Dynamic Entity Spawning:**
  - Resolve the `SpawnEntity` CommandBuffer stub so scripts can dynamically spawn projectiles.
* [x] **Entity Physics Configuration:**
  - Expose `Loci.Commands.set_move_speed(entity_id, speed)` to allow Lua scripts to configure per-entity movement speed at runtime.

### Português
* [x] **Despacho de Ações/Habilidades:**
  - Expandir o `ActionIntent` com campo `target_direction` e rotear para o callback `on_action`.
* [x] **Spawn Dinâmico de Entidades:**
  - Resolver o stub do `SpawnEntity` no CommandBuffer.
* [x] **Configuração de Física por Entidade:**
  - Expor `Loci.Commands.set_move_speed(entity_id, speed)` (resolve [Issue #4](https://github.com/lamfsantos/loci2d/issues/4)).

---

## 6.5.1: Architecture, Safety & Determinism Validation
## 6.5.1: Arquitetura, Segurança & Validação de Determinismo

### English
* [x] **Strict State vs. Scripting Separation:**
  - Decouple canonical match state (pure deterministic physics, spatial data) from Lua script execution.
* [x] **Safe Dispatch Layer:**
  - Implement a safe command/intent dispatch layer for Lua scripts to prevent unauthorized state corruption and ensure deterministic replay integrity.
* [x] **Determinism CI Suite & Benchmarks:**
  - Establish a rigorous cross-platform continuous integration (CI) test suite to mathematically prove `I16F16` fixed-point determinism across ARM64 and x86_64 architectures.
  - Update `benchmark.rs` and core tests to stress-test Lua Gameplay Actions and ensure they maintain determinism.

### Português
* [x] **Separação Estrita entre Estado e Scripting:**
  - Desacoplar o estado canônico da partida (física determinística pura, dados espaciais) da execução dos scripts Lua.
* [x] **Camada Segura de Despacho:**
  - Implementar uma camada segura de envio de comandos/intenções para scripts Lua, evitando corrupção direta de estado e preservando a integridade dos replays.
* [x] **Suíte CI de Determinismo & Benchmarks:**
  - Estabelecer uma suíte de testes de Integração Contínua (CI) rigorosa multiplataforma para provar matematicamente o determinismo do ponto-fixo `I16F16` entre arquiteturas ARM64 e x86_64.
  - Atualizar o `benchmark.rs` e os testes base para estressar as Ações de Gameplay em Lua e garantir que o determinismo seja mantido.

---

## 6.5.2: API Refinement & SDK Preparation
## 6.5.2: Refinamento da API & Preparação para SDKs

### English
* [ ] **Strict Intent Interception (Safe Layer Fix):**
  - Refactor `intent_handler.rs` so `Intent::Move` and `Intent::MoveToPos` do not directly mutate Entity velocity and navigation.
  - Expose new Lua hooks (`on_move_intent(entity_id, dir_x, dir_y)`) allowing scripts to validate and apply movement via `Loci.Commands`.
* [ ] **Lua API Consistency & Getters:**
  - Standardize Vector2 arguments across the Lua API (e.g., `on_action`, `set_position`) so they accept consistent formats.
  - Implement basic Getters (`get_velocity`, `get_move_speed`, `get_entity_name`) to prevent state duplication in Lua.
* [ ] **Blueprint Extensibility:**
  - Enhance `SpawnEntity` to accept configuration parameters (or integrate a registry) rather than hardcoding EntityType and Components.

### Português
* [ ] **Interceptação Estrita de Intents (Correção da Camada Segura):**
  - Refatorar `intent_handler.rs` para que `Intent::Move` e `Intent::MoveToPos` não mutem diretamente a velocidade e navegação da Entidade.
  - Expor novos hooks Lua (`on_move_intent(entity_id, dir_x, dir_y)`) permitindo que scripts validem e apliquem movimento via `Loci.Commands`.
* [ ] **Consistência da API Lua & Getters:**
  - Padronizar argumentos Vector2 na API Lua (ex: `on_action`, `set_position`) para que aceitem formatos consistentes.
  - Implementar Getters básicos (`get_velocity`, `get_move_speed`, `get_entity_name`) para evitar duplicação de estado no Lua.
* [ ] **Extensibilidade de Blueprints:**
  - Melhorar `SpawnEntity` para aceitar parâmetros de configuração (ou integrar um registry) em vez de fixar (hardcode) EntityType e Componentes.

---

## 6.5.3: Client SDKs & Wrappers
## 6.5.3: SDKs & Wrappers para Clientes

### English
* [ ] **[BLOCKING] API Review & Stabilization:**
  - Audit and stabilize the Protobuf schema (`game_packets.proto`) and Lua Engine API (`Loci.Commands`) before building client wrappers to ensure ergonomic consumption.
* [ ] **Godot 4 Wrapper (`LociClient.gd`):**
  - Build an ergonomic GDScript client module to eliminate boilerplate UDP socket/Protobuf parsing for end users.
* [ ] **Love2D Wrapper (`loci_client.lua`):**
  - Create a lightweight Lua module for Love2D developers to connect to the engine effortlessly.
* [ ] **Python SDK:**
  - Expose a Python SDK for AI training, data analysis, or rapid prototyping.
* [ ] **Signals & Callbacks API:**
  - Expose intuitive, high-level signals/callbacks for state updates (e.g., `on_entity_updated`, `on_match_event`).

### Português
* [ ] **[BLOQUEANTE] Revisão & Estabilização da API:**
  - Auditar e estabilizar o esquema do Protobuf (`game_packets.proto`) e a API do Motor Lua (`Loci.Commands`) antes de construir os wrappers de cliente, garantindo um consumo ergonômico.
* [ ] **Wrapper para Godot 4 (`LociClient.gd`):**
  - Construir um módulo cliente ergonômico em GDScript para eliminar o boilerplate de sockets UDP e decodificação Protobuf para os usuários finais.
* [ ] **Wrapper para Love2D (`loci_client.lua`):**
  - Criar um módulo Lua leve para desenvolvedores Love2D conectarem-se ao motor sem esforço.
* [ ] **SDK para Python:**
  - Expor um SDK Python para treinamento de IA, análise de dados ou prototipagem rápida.
* [ ] **API de Sinais & Callbacks:**
  - Expor sinais/callbacks intuitivos de alto nível para atualizações de estado (ex: `on_entity_updated`, `on_match_event`).

---

## 6.5.4: Reference Examples & Templates
## 6.5.4: Exemplos de Referência & Templates

### English
* [ ] **Standard Lua Game Template:** Create a boilerplate `main.lua` file with heavily documented callbacks (`on_init`, `on_player_join`, etc.) establishing the official structure for students to build game rules.
* [ ] **Refactor CLI Example:** Update the headless rust client example to reflect recent architectural changes.
* [ ] **Refactor Love2D Example:** Rebuild the Love2D demo using the new `loci_client.lua` SDK.
* [ ] **Refactor Godot Example:** Rebuild the Godot demo using the new `LociClient.gd` SDK.
* [ ] **Refactor Python Example:** Showcase basic interactions using the new Python SDK.

### Português
* [ ] **Template Padrão de Jogo em Lua:** Criar um arquivo `main.lua` modelo com callbacks amplamente documentados (`on_init`, `on_player_join`, etc.), estabelecendo a estrutura oficial para os alunos criarem regras de jogo.
* [ ] **Refatorar Exemplo CLI:** Atualizar o exemplo de cliente headless em Rust para refletir as mudanças arquiteturais recentes.
* [ ] **Refatorar Exemplo Love2D:** Reconstruir a demo em Love2D usando o novo SDK `loci_client.lua`.
* [ ] **Refatorar Exemplo Godot:** Reconstruir a demo em Godot usando o novo SDK `LociClient.gd`.
* [ ] **Refatorar Exemplo Python:** Demonstrar interações básicas usando o novo SDK Python.

---

## 6.5.5: User-Friendly Documentation
## 6.5.5: Documentação Acessível

### English
* [ ] **Quickstart Tutorial:**
  - Author a "15-Minute First Multiplayer Game" tutorial (e.g., a simple 2D Arena or Tag game).
* [ ] **Lua API Reference:**
  - Create a comprehensive, copy-paste-ready Lua scripting API reference guide with clear use cases for every engine callback.

### Português
* [ ] **Tutorial de Início Rápido (Quickstart):**
  - Criar um tutorial prático "Seu Primeiro Jogo Multiplayer em 15 Minutos" (ex: um jogo simples de Arena 2D ou Pega-Pega).
* [ ] **Referência da API Lua:**
  - Criar uma referência completa e didática da API de scripting em Lua com exemplos prontos para uso em todos os callbacks do motor.

---

## 6.5.6: LAN Playtesting & Empirical Feedback
## 6.5.6: Playtesting em LAN & Feedback Empírico

### English
* [ ] **Multi-Device Sessions:**
  - Run physical LAN playtest sessions with high school and university students to identify UX/DX friction points and API ergonomics issues.
* [ ] **Network Stability Metrics:**
  - Measure tick stability, desync resilience, and packet latency under real local network conditions.

### Português
* [ ] **Sessões Multi-Dispositivo:**
  - Realizar sessões físicas de playtest em LAN com estudantes de ensino médio e graduação para identificar pontos de atrito de UX/DX e testar a ergonomia da API.
* [ ] **Métricas de Estabilidade de Rede:**
  - Medir a estabilidade dos ticks, resiliência a dessincronização e latência de pacotes sob condições reais de rede local.
