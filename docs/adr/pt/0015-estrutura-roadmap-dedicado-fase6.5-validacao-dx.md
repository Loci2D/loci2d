# ADR 0015: Estrutura de Sub-Roadmap Dedicado e Governança para a Fase 6.5

## Status

Aceito (Fase 6.5)

## Contexto

Das Fases 1 a 6, o desenvolvimento do `loci2d` seguiu uma progressão estritamente linear focada no motor core em Rust (loop de jogo, netcode, replays, física e scripting Lua embutido). Cada uma dessas fases possui um único documento de especificação técnica correspondente em `docs/roadmap-specs/phaseX-*-spec.md`.

A **Fase 6.5 (Validação, Experiência do Desenvolvedor & Estabilização de API)** representa uma pausa estratégica no desenvolvimento de novas funcionalidades estruturais para consolidar a pilha v0.6.x como um ecossistema multiplayer jogável e amigável. Diferente das fases anteriores, a Fase 6.5 abrange múltiplos domínios tecnologicamente heterogêneos:
- **Arquitetura da Engine (Rust):** Desacoplamento estrito entre o estado determinístico canônico e os scripts Lua, com fila segura de intenções.
- **SDKs Clientes Multilinguagem:** Desenvolvimento de abstrações idiomáticas e wrappers leves para **Godot 4** (`GDScript`), **LÖVE2D** (`Lua`) e **Python**.
- **Documentação e Templates:** Elaboração do guia "Primeiro Jogo em 15 Minutos", documentação de referência da API Lua e modelos iniciais.
- **Migração de Exemplos:** Atualização dos exemplos do projeto (CLI, Love2D, Godot, Python) utilizando a nova camada de SDKs.
- **Playtesting Empírico:** Testes multi-dispositivo em rede LAN física com estudantes para validar estabilidade de ticks, resiliência a desincronizações e ergonomia da API.

Tentar agrupar todos esses entregáveis em uma única especificação ou em um único bloco no `roadmap.md` principal geraria documentos prolixos, difíceis de manter e que quebrariam o foco de desenvolvimento e revisão de código.

## Decisão

Adotamos a criação de um **Sub-Roadmap Dedicado (`docs/phase6.5-roadmap.md`)** e a **Divisão em Especificações Modulares por Sub-Marco** para a Fase 6.5.

1. **Arquivo de Sub-Roadmap Exclusivo:** A Fase 6.5 terá seu próprio arquivo `docs/phase6.5-roadmap.md`, que dividirá a pausa estratégica em sub-marcos bem delimitados (ex: 6.5.1 a 6.5.5).
2. **Especificações Modulares por Sub-Marco:** Cada sub-marco da Fase 6.5 terá um documento de especificação próprio em `docs/roadmap-specs/phase6.5.X-*-spec.md` (por exemplo, `phase6.5.1-state-scripting-decoupling-spec.md` e `phase6.5.2-client-sdks-spec.md`).
3. **Visão Macro no Roadmap Principal:** O arquivo [roadmap.md](../../roadmap.md) manterá apenas a visão macro da Fase 6.5 com um link direcionando para o sub-roadmap dedicado.

## Consequências

**Positivas:**
- **Clareza de Escopo e Modularidade:** Permite que o desenvolvimento dos SDKs em Godot/Lua/Python e as alterações na engine Rust ocorram de forma isolada, com especificações focadas e PRs menores.
- **Organização do `roadmap.md` Macro:** Preserva a legibilidade do roteiro geral do projeto sem inflacioná-lo com detalhes de integração de múltiplos SDKs e guias de playtesting.
- **Rastreabilidade de Governança:** Documenta explicitamente por que a Fase 6.5 diverge da estrutura linear das fases anteriores, evitando confusão para futuros mantenedores.

**Negativas:**
- **Pequena Divergência de Padrão:** Introduz uma exceção ao padrão de documento único de especificação por fase usado nas Fases 1 a 6. Esta exceção fica justificada e formalizada por esta ADR.
