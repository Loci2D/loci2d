# Phase 6.5 Roadmap: Validation, DX & API Stabilization
# Roadmap da Fase 6.5: Validação, DX & Estabilização de API
> **Reference ADRs:** [ADR-0015](adr/en/0015-dedicated-roadmap-structure-phase6.5-validation-dx.md)


### English
This document details the sub-milestones for Phase 6.5 of the **loci2d** project. Phase 6.5 acts as a "strategic pause" (v0.6.x) to consolidate the engine's core features into a playable, developer-friendly multiplayer ecosystem before introducing multi-room infrastructure.

### Português
Este documento detalha os sub-marcos para a Fase 6.5 do projeto **loci2d**. A Fase 6.5 atua como uma "pausa estratégica" (v0.6.x) para consolidar as funcionalidades principais do motor em um ecossistema multiplayer jogável e amigável para desenvolvedores antes de introduzir a infraestrutura de múltiplas salas.

---

## 6.5.0: Entity Data Model & Game Rules API
## 6.5.0: Modelo de Dados de Entidade & API de Regras de Jogo
> **Reference ADRs:** [ADR-0014](adr/en/0014-embedded-lua-scripting-and-command-buffer.md) · [ADR-0016](adr/en/0016-data-driven-entity-properties-and-engine-agnosticism.md)

### English
* [ ] **Entity Property System (Data-Driven Design):**
  - Add a flexible key-value property map (e.g., `health`, `team`) to the `Entity` struct in Rust.
  - **Crucial Rule:** The Rust core must remain agnostic to game rules. Hardcoded fields like `health` or `team` must NOT be added directly to the Rust structs. The engine only provides generic property storage; Lua scripts define their meaning and game logic.
* [ ] **Lua Property Interface:**
  - Expose `Loci.get_entity_property` and `Loci.Commands.set_property` to Lua scripts.
* [ ] **Action/Ability Dispatch:**
  - Expand `ActionIntent` with a `target_direction` field for directional abilities (e.g., projectile aim).
  - Route the intent to a new Lua callback `on_action(entity_id, ability_id, dir_x, dir_y)`.
* [ ] **Global Match Metadata:**
  - Allow Lua to manage global match variables (e.g., match score, player count) via the generic property system.
* [ ] **Match Lifecycle State Machine:**
  - Introduce a `Paused`/`Running`/`Ended` state machine to the server instance.
  - Expose `Loci.Commands.start_match()`, `Loci.Commands.pause_match()`, and `Loci.Commands.end_match()` to Lua scripts so developers can dictate when the game loop actually begins advancing ticks based on player connections or readiness.
* [ ] **Timer & Cooldown System:**
  - Expose `Loci.Commands.start_timer(timer_id, delay_ticks)` and route expiration to a new Lua callback `on_timer_complete(timer_id)`.
* [ ] **Dynamic Entity Spawning:**
  - Resolve the `SpawnEntity` CommandBuffer stub so scripts can dynamically spawn projectiles and pickups at runtime.
* [ ] **Entity Physics Configuration:**
  - Expose `Loci.Commands.set_move_speed(entity_id, speed)` to allow Lua scripts to configure per-entity movement speed at runtime (resolves [Issue #4](https://github.com/lamfsantos/loci2d/issues/4)).

### Português
* [ ] **Sistema de Propriedades de Entidade (Design Orientado a Dados):**
  - Adicionar um mapa chave-valor flexível (ex: `health`, `team`) na struct `Entity` no Rust.
  - **Regra Crucial:** O núcleo em Rust deve permanecer agnóstico às regras do jogo. Campos "hardcoded" como `health` ou `team_id` NÃO devem ser adicionados nas structs Rust. O motor apenas provê o armazenamento genérico das propriedades; os scripts Lua definem seu significado e a lógica.
* [ ] **Interface de Propriedades no Lua:**
  - Expor `Loci.get_entity_property` e `Loci.Commands.set_property` para os scripts Lua.
* [ ] **Despacho de Ações/Habilidades:**
  - Expandir o `ActionIntent` com campo `target_direction` para habilidades direcionais (ex: mira de projéteis).
  - Roteamento do intent para um novo callback Lua `on_action(entity_id, ability_id, dir_x, dir_y)`.
* [ ] **Metadados Globais da Partida:**
  - Permitir que o Lua gerencie variáveis globais da partida (ex: placar, contagem de jogadores) via o sistema genérico de propriedades.
* [ ] **Máquina de Estado do Ciclo de Vida da Partida:**
  - Introduzir uma máquina de estados `Pausado`/`Em Execução`/`Encerrado` para a instância do servidor.
  - Expor `Loci.Commands.start_match()`, `Loci.Commands.pause_match()` e `Loci.Commands.end_match()` para os scripts Lua, permitindo que os desenvolvedores ditem quando o loop do jogo realmente começa a avançar os ticks com base nas conexões ou prontidão dos jogadores.
* [ ] **Sistema de Timers e Cooldowns:**
  - Expor `Loci.Commands.start_timer(timer_id, delay_ticks)` e rotear o fim da contagem para um novo callback Lua `on_timer_complete(timer_id)`.
* [ ] **Spawn Dinâmico de Entidades:**
  - Resolver o stub do `SpawnEntity` no CommandBuffer para que os scripts possam criar projéteis e itens dinamicamente durante a partida.
* [ ] **Configuração de Física por Entidade:**
  - Expor `Loci.Commands.set_move_speed(entity_id, speed)` para permitir que scripts Lua configurem a velocidade de movimento por entidade em tempo de execução (resolve [Issue #4](https://github.com/lamfsantos/loci2d/issues/4)).

---

## 6.5.1: Architecture, Safety & Determinism Validation
## 6.5.1: Arquitetura, Segurança & Validação de Determinismo

### English
* [ ] **Strict State vs. Scripting Separation:**
  - Decouple canonical match state (pure deterministic physics, spatial data) from Lua script execution.
* [ ] **Safe Dispatch Layer:**
  - Implement a safe command/intent dispatch layer for Lua scripts to prevent unauthorized state corruption and ensure deterministic replay integrity.
* [ ] **Determinism CI Suite:**
  - Establish a rigorous cross-platform continuous integration (CI) test suite to mathematically prove `I16F16` fixed-point determinism across ARM64 and x86_64 architectures.

### Português
* [ ] **Separação Estrita entre Estado e Scripting:**
  - Desacoplar o estado canônico da partida (física determinística pura, dados espaciais) da execução dos scripts Lua.
* [ ] **Camada Segura de Despacho:**
  - Implementar uma camada segura de envio de comandos/intenções para scripts Lua, evitando corrupção direta de estado e preservando a integridade dos replays.
* [ ] **Suíte CI de Determinismo:**
  - Estabelecer uma suíte de testes de Integração Contínua (CI) rigorosa multiplataforma para provar matematicamente o determinismo do ponto-fixo `I16F16` entre arquiteturas ARM64 e x86_64.

---

## 6.5.2: Client SDKs & Wrappers
## 6.5.2: SDKs & Wrappers para Clientes

### English
* [ ] **Godot 4 Wrapper (`LociClient.gd`):**
  - Build an ergonomic GDScript client module to eliminate boilerplate UDP socket/Protobuf parsing for end users.
* [ ] **Love2D Wrapper (`loci_client.lua`):**
  - Create a lightweight Lua module for Love2D developers to connect to the engine effortlessly.
* [ ] **Python SDK:**
  - Expose a Python SDK for AI training, data analysis, or rapid prototyping.
* [ ] **Signals & Callbacks API:**
  - Expose intuitive, high-level signals/callbacks for state updates (e.g., `on_entity_updated`, `on_match_event`).

### Português
* [ ] **Wrapper para Godot 4 (`LociClient.gd`):**
  - Construir um módulo cliente ergonômico em GDScript para eliminar o boilerplate de sockets UDP e decodificação Protobuf para os usuários finais.
* [ ] **Wrapper para Love2D (`loci_client.lua`):**
  - Criar um módulo Lua leve para desenvolvedores Love2D conectarem-se ao motor sem esforço.
* [ ] **SDK para Python:**
  - Expor um SDK Python para treinamento de IA, análise de dados ou prototipagem rápida.
* [ ] **API de Sinais & Callbacks:**
  - Expor sinais/callbacks intuitivos de alto nível para atualizações de estado (ex: `on_entity_updated`, `on_match_event`).

---

## 6.5.3: Reference Examples & Templates
## 6.5.3: Exemplos de Referência & Templates

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

## 6.5.4: User-Friendly Documentation
## 6.5.4: Documentação Acessível

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

## 6.5.5: LAN Playtesting & Empirical Feedback
## 6.5.5: Playtesting em LAN & Feedback Empírico

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
