# Roadmap da Fase 6.5: Validação, DX & Estabilização de API

> **Reference ADRs:** [ADR-0015](adr/pt/0015-estrutura-roadmap-dedicado-fase6.5-validacao-dx.md)
>
> 🌐 *Read this in [English](phase6.5-roadmap.md)*

Este documento detalha os sub-marcos para a Fase 6.5 do projeto **loci2d**. A Fase 6.5 atua como uma "pausa estratégica" (v0.6.x) para consolidar as funcionalidades principais do motor em um ecossistema multiplayer jogável e amigável para desenvolvedores antes de introduzir a infraestrutura de múltiplas salas.

---

## 6.5.0-1: Modelo de Dados de Entidade & Propriedades Globais
> **Reference ADRs:** [ADR-0014](adr/pt/0014-scripting-embutido-lua-e-command-buffer.md) · [ADR-0016](adr/pt/0016-propriedades-de-entidade-orientadas-a-dados-e-agnosticismo.md)
> **Spec:** [phase6.5.0-1-data-model-spec.md](roadmap-specs/phase6.5.0-1-data-model-spec.md)

* [x] **Sistema de Propriedades de Entidade:**
  - Adicionar um mapa chave-valor flexível na struct `Entity` no Rust.
* [x] **Interface de Propriedades no Lua:**
  - Expor `Loci.get_entity_property` e `Loci.Commands.set_property`.
* [x] **Metadados Globais da Partida:**
  - Permitir que o Lua gerencie variáveis globais (placar, etc.) via o sistema de propriedades genéricas (`globals`).

---

## 6.5.0-2: Ciclo de Vida da Partida & Timers
> **Spec:** [phase6.5.0-2-match-lifecycle-spec.md](roadmap-specs/phase6.5.0-2-match-lifecycle-spec.md)

* [x] **Máquina de Estado do Ciclo de Vida da Partida:**
  - Introduzir uma máquina de estados `Pausado`/`Em Execução`/`Encerrado` para a instância.
  - Expor comandos de `start_match`, `pause_match` e `end_match`.
* [x] **Sistema de Timers e Cooldowns:**
  - Expor `Loci.Commands.start_timer` e o callback `on_timer_complete`.

---

## 6.5.0-3: Ações de Gameplay & Configuração de Física
> **Spec:** [phase6.5.0-3-gameplay-actions-spec.md](roadmap-specs/phase6.5.0-3-gameplay-actions-spec.md)

* [x] **Despacho de Ações/Habilidades:**
  - Expandir o `ActionIntent` com campo `target_direction` e rotear para o callback `on_action`.
* [x] **Spawn Dinâmico de Entidades:**
  - Resolver o stub do `SpawnEntity` no CommandBuffer.
* [x] **Configuração de Física por Entidade:**
  - Expor `Loci.Commands.set_move_speed(entity_id, speed)` (resolve [Issue #4](https://github.com/lamfsantos/loci2d/issues/4)).

---

## 6.5.1: Arquitetura, Segurança & Validação de Determinismo

* [x] **Separação Estrita entre Estado e Scripting:**
  - Desacoplar o estado canônico da partida (física determinística pura, dados espaciais) da execução dos scripts Lua.
* [x] **Camada Segura de Despacho:**
  - Implementar uma camada segura de envio de comandos/intenções para scripts Lua, evitando corrupção direta de estado e preservando a integridade dos replays.
* [x] **Suíte CI de Determinismo & Benchmarks:**
  - Estabelecer uma suíte de testes de Integração Contínua (CI) rigorosa multiplataforma para provar matematicamente o determinismo do ponto-fixo `I16F16` entre arquiteturas ARM64 e x86_64.
  - Atualizar o `benchmark.rs` e os testes base para estressar as Ações de Gameplay em Lua e garantir que o determinismo seja mantido.

---

## 6.5.2: Refinamento da API & Preparação para SDKs

* [x] **Interceptação Estrita de Intents (Correção da Camada Segura):**
  - Refatorar `intent_handler.rs` para que `Intent::Move` e `Intent::MoveToPos` não mutem diretamente a velocidade e navegação da Entidade.
  - Expor novos hooks Lua (`on_move_intent(entity_id, dir_x, dir_y)`) permitindo que scripts validem e apliquem movimento via `Loci.Commands`.
* [x] **Consistência da API Lua & Getters:**
  - Padronizar argumentos Vector2 na API Lua (ex: `on_action`, `set_position`) para que aceitem formatos consistentes.
  - Implementar Getters básicos (`get_velocity`, `get_move_speed`, `get_entity_name`) para evitar duplicação de estado no Lua.
* [x] **Extensibilidade de Blueprints:**
  - Melhorar `SpawnEntity` para aceitar parâmetros de configuração (ou integrar um registry) em vez de fixar (hardcode) EntityType e Componentes.
* [x] **Ergonomia da API para Skills (Suporte a MOBA):**
  - Adicionar `Loci.get_entities_in_radius` para Spatial Queries (skills em área).
  - Expor métodos matemáticos em fixed-point (`distance_to`, `normalize`, `length`) no `DeterministicVector2` para evitar perda de determinismo com ponto flutuante.

---

## 6.5.3: SDKs & Wrappers para Clientes
> **Specs:** [phase6.5.3-love2d-sdk-spec.md](roadmap-specs/phase6.5.3-love2d-sdk-spec.md) · [phase6.5.3-1-client-sdk-ergonomics-and-events-spec.md](roadmap-specs/phase6.5.3-1-client-sdk-ergonomics-and-events-spec.md)

* [x] **Spec do SDK Love2D:** Escrever `phase6.5.3-love2d-sdk-spec.md` detalhando a arquitetura do cliente, garantindo que o netcode fique totalmente abstraído dos devs.
* [x] **Wrapper para Love2D (`loci_client.lua`):** (PRIORIDADE)
  - Criar um módulo Lua leve para desenvolvedores Love2D conectarem-se ao motor sem esforço, abstraindo sockets UDP e Protobuf completamente.
* [x] **Fase 6.5.3-1: Abstração de Entidades em Alto Nível & Cast Automático:**
  - Criar objetos `Entity` em memória com metatabelas no wrapper para conversão automática de propriedades para números/booleanos e acesso direto (`entity.hp`).
* [x] **Fase 6.5.3-1: Broadcast de Ações & Estado da Partida:**
  - Estender Protobuf com `ActionBroadcast` transiente e `MatchLifecycleState`, atualizando servidor e SDK para disparar `on_action_cast` e `on_match_state_changed`.

---

## 6.5.4: Exemplos de Referência & Templates

* [ ] **MVP Arena 3v3 (Love2D):** Construir um jogo Arena 3v3 totalmente funcional usando o SDK do Love2D para validar a engine.
* [ ] **Template Padrão de Jogo em Lua:** Criar um arquivo `main.lua` modelo com callbacks amplamente documentados (`on_init`, `on_player_join`, etc.), estabelecendo a estrutura oficial para os alunos criarem regras de jogo.
* [ ] **Refatorar Exemplo CLI:** Atualizar o exemplo de cliente headless em Rust para refletir as mudanças arquiteturais recentes.

---

## 6.5.5: Documentação Acessível

* [ ] **Tutorial de Início Rápido (Quickstart):**
  - Criar um tutorial prático "Seu Primeiro Jogo Multiplayer em 15 Minutos" (ex: um jogo simples de Arena 2D ou Pega-Pega).
* [ ] **Referência da API Lua:**
  - Criar uma referência completa e didática da API de scripting em Lua com exemplos prontos para uso em todos os callbacks do motor.

---

## 6.5.6: Playtesting em LAN & Feedback Empírico

* [ ] **Sessões Multi-Dispositivo:**
  - Realizar sessões físicas de playtest em LAN com estudantes de ensino médio e graduação para identificar pontos de atrito de UX/DX e testar a ergonomia da API.
* [ ] **Métricas de Estabilidade de Rede:**
  - Medir a estabilidade dos ticks, resiliência a dessincronização e latência de pacotes sob condições reais de rede local.

---

## Trabalhos Futuros (Fora do Escopo do MVP da Fase 6.5)

* [ ] **Wrapper para Godot 4 (`LociClient.gd`):** Construir um módulo cliente ergonômico em GDScript para eliminar o boilerplate de sockets UDP e decodificação Protobuf para os usuários finais.
* [ ] **SDK para Python:** Expor um SDK Python para treinamento de IA, análise de dados ou prototipagem rápida.
* [ ] **Refatorar Exemplo Godot:** Reconstruir a demo em Godot usando o novo SDK `LociClient.gd`.
* [ ] **Refatorar Exemplo Python:** Demonstrar interações básicas usando o novo SDK Python.
