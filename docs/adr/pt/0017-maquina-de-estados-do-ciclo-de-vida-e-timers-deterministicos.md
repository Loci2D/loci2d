# ADR 0017: Máquina de Estados do Ciclo de Vida e Timers Determinísticos

## Status
Aceito

## Contexto
Durante a Fase 6.5.0-2, introduzimos a capacidade de pausar a partida (ex: aguardando jogadores conectarem) e de gerenciar a lógica de jogo baseada em tempo (timers e cooldowns).
Anteriormente, a engine do servidor rodava ("tickava") incondicionalmente (ADR-0006). Com a introdução do motor de script Lua embutido (ADR-0014), nos deparamos com escolhas arquiteturais sobre "quem é o dono do tempo" e "como pausamos o jogo?".
Poderíamos ter permitido que o Lua gerenciasse os timers internamente (ex: decrementando variáveis a cada `on_tick`) e ignorasse inputs quando o jogo estivesse "pausado" no Lua, deixando o núcleo em Rust rodando continuamente. No entanto, isso diluiria as fronteiras estritas de determinismo da engine.

## Decisão

### 1. Ciclo de Vida da Partida Gerenciado pelo Núcleo (Rust)
Decidimos implementar um `MatchState` formal (`Paused`, `Running`, `Ended`) diretamente na `Instance` do núcleo em Rust.
- **Espelhamento de Replays:** O arquivo de replay deve espelhar exatamente o estado real da partida. Se a partida está pausada, o replay está pausado (os ticks congelam). Embora isso impeça que recursos assíncronos como chat de texto sejam gravados ou processados durante uma pausa, decidimos adiar essa refatoração para depois do MVP para evitar complexidade desnecessária agora.
- **Estado Inicial (Boot):** O núcleo em Rust *não* iniciará no estado `Paused` por padrão. A engine iniciará como `Running`. É responsabilidade da lógica do jogo (ex: scripts de inicialização) emitir imediatamente um comando `pause_match` caso desejem aguardar a conexão dos jogadores. Isso evita uma refatoração massiva no Rust para lidar com estados de pré-jogo e é um padrão aceitável para este tipo de engine.
- **Disponibilidade de Callbacks:** Mesmo quando `Paused` ou `Ended`, os callbacks de conexão (`on_player_join`, `on_player_leave`) continuarão sendo disparados para permitir o gerenciamento de lobby.

### 2. Timers Determinísticos Gerenciados pelo Rust
Decidimos não permitir que o Lua gerencie timers por conta própria através de contadores de loop. Em vez disso, todos os timers devem ser solicitados via `Loci.Commands.start_timer`.
- O núcleo em Rust gerencia esses timers em um `BTreeMap<String, ActiveTimer>` e os decrementa a cada chamada de `tick()`.
- Isso garante uma execução estrita, alfabética e determinística dos callbacks de timers (`on_timer_complete`), além de transferir o gerenciamento do estado temporal do Lua para o núcleo em Rust, que é altamente otimizado e focado em determinismo.

## Consequências
- **Positivo:** Garantimos determinismo perfeito para eventos baseados em tempo, independentemente da complexidade dos scripts Lua.
- **Positivo:** Arquivos de replay permanecem enxutos e não acumulam "ticks vazios e pausados" desnecessariamente.
- **Negativo:** Funcionalidades futuras que exijam processamento em tempo real independente do estado da simulação (como chat de texto global) exigirão uma refatoração estrutural, já que a arquitetura atual congela todo o pipeline de ticks quando a partida é pausada.
