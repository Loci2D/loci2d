# ADR 0014: Scripting Embutido em Lua e Command Buffer Determinístico

## Status

Aceito (Fase 6)

## Contexto

Para permitir a prototipagem rápida, criação de mods e personalização da lógica de jogo sem a necessidade de recompilar o servidor em Rust, o `loci2d` precisa de uma linguagem de script embutida.

A linguagem deve ser leve, fácil para desenvolvedores indies aprenderem, e capaz de rodar em um ambiente seguro (sandbox) e determinístico, preservando os replays baseados em eventos ([ADR-0010](../en/0010-event-sourced-replay-format.md)) e a simulação determinística ([ADR-0007](../en/0007-deterministic-simulation-and-fixed-point.md)).

Além disso, permitir que scripts fornecidos pelos usuários mutem o estado canônico do jogo (entidades, árvore de física) diretamente no meio de um *tick* representa sérios riscos de estabilidade. Por exemplo, se um script Lua destruir uma entidade durante um callback `on_collision` enquanto o motor de física do Rust estiver iterando ativamente sobre as colisões, isso causará uma invalidação de iterador (derrubando o motor) ou corromperá o estado no meio do tick. Para que o motor suporte jogos complexos e de alta interação como MOBAs, a arquitetura deve ser indestrutível contra erros de script do usuário.

## Decisão

Adotamos **Lua (via `mlua`) com Sandboxing Estrito e uma Arquitetura de Command Buffer Diferido**.

### 1. A Escolha de Lua
Lua é o padrão da indústria para scripting de jogos. Sua VM é extremamente leve, sua compatibilidade com C-FFI (`mlua`) a torna rápida, e possui uma curva de aprendizado muito baixa. Isso se alinha perfeitamente com o objetivo do projeto de abstrair a complexidade de rede para desenvolvedores indies e estudantes. Alternativas como WASM ou Rhai foram descartadas devido à maior complexidade ou falta de adoção generalizada na indústria de jogos.

### 2. Sandboxing Determinístico
Para prevenir exploits no servidor e garantir que os replays permaneçam 100% determinísticos em diferentes sistemas operacionais:
- Bibliotecas padrão perigosas (`io`, `os`, `package`, `debug`) são estritamente desabilitadas.
- Iterações não-determinísticas (`pairs()`) são removidas do ambiente global, forçando o uso de arrays determinísticos (`ipairs()`).
- A função padrão `math.random` é sobrescrita com um PRNG determinístico alimentado pela seed da `Instance`.
- Um limite estrito de instruções (proteção contra DoS) é aplicado por callback. Se um script entrar em loop infinito, a Instância aborta graciosamente e desconecta os clientes sem derrubar o binário principal do servidor.

### 3. O Padrão de Command Buffer (Execução Diferida)
Para prevenir a invalidação de iteradores e garantir um forte desacoplamento de estado:
- Scripts Lua **não podem** mutar o estado canônico do jogo diretamente durante um tick (ex: dentro de `on_collision` ou `on_tick`).
- Em vez disso, a API Rust-para-Lua é apoiada por um `CommandBuffer`. Funções como `spawn_entity`, `destroy_entity` ou `set_position` colocam intenções nesta fila.
- O Rust descarrega e aplica o Command Buffer sequencialmente no final do tick, garantindo segurança absoluta para os iteradores internos do motor.
- **Mitigação de DX (Experiência do Desenvolvedor):** Para evitar que o Command Buffer seja frustrante para desenvolvedores indies, comandos de criação de entidades pré-alocam e retornam um `EntityID` imediatamente. Isso permite que o script Lua enfileire comandos subsequentes referenciando a nova entidade dentro do mesmo tick, escondendo a complexidade diferida por trás de uma API que parece síncrona.

## Consequências

**Positivas:**
- **Baixa Barreira de Entrada:** A lógica do jogo pode ser escrita rapidamente em Lua sem conhecimento de Rust.
- **Estabilidade Sólida:** O Command Buffer protege o motor contra corrupção de estado no meio do tick e invalidação de iterador, escalando com segurança para a complexidade de um MOBA.
- **Determinismo Mantido:** O sandboxing e a sobrescrita do PRNG preservam a arquitetura de replay via event-sourcing.
- **Segurança do Servidor:** Desabilitar `io`/`os` e impor limites de instrução impede que uma sala maliciosa ou com bug derrube o servidor inteiro.

**Negativas:**
- **Complexidade Diferida:** A execução diferida introduz uma leve complexidade conceitual para os desenvolvedores. A API Rust-para-Lua deve ser cuidadosamente projetada para abstrair isso (ex: pré-alocando IDs) para mitigar o atrito de DX.
- **Sem Acesso Direto a Arquivos:** Desenvolvedores de jogos não podem salvar/carregar arquivos externos diretamente do Lua, exigindo que o motor Rust gerencie todo o carregamento de mapas/assets antes de passar os dados para o contexto Lua.
