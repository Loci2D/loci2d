# Roadmap do Projeto loci2d

> 🌐 *Read this in [English](roadmap.md)*

Este documento descreve as fases de desenvolvimento, metas e marcos técnicos para o **loci2d**, um framework de servidor de jogos 2D autoritativo em Rust.

---

## Visão Atual & Abordagem

* **Fase de Validação**: Foco em obter uma **única instância** totalmente funcional e estável rodando localmente (`127.0.0.1`).
* **Modelo Autoritativo**: Recebe intenções dos clientes, processa física/game loop e transmite o estado autoritativo.
* **Baixa Barreira de Entrada**: Complexidade mínima para desenvolvedores indies e estudantes.

---

## Fases do Roadmap

### Fase 1: Game Loop & Núcleo de Instância Única (Concluído)
* [x] **Integração de Rede & Game Loop**:
  - Conectar a thread de rede do socket UDP ao `GameLoop` via canais seguros para threads (`mpsc`).
  - Implementar o consumo não-bloqueante da fila de intenções dentro da taxa de tick fixa (ex: 20/30 Hz).
* [x] **Instância Única em Localhost**:
  - Garantir que uma única `Instance` gerencie posições de entidades, aplicação de inputs e ticks espaciais em `localhost`.
  - Gerenciamento de múltiplas instâncias / salas é postergado para fases futuras.

---

### Fase 2: Mapeamento Leve de Cliente/Sessão (Concluído)
> **Spec:** [Phase 2 Spec](roadmap-specs/phase2-client-session-mapping-spec.md) · **Reference ADRs:** [ADR-0001](adr/pt/0001-arquitetura-servidor-autoritativo.md) · [ADR-0002](adr/pt/0002-arquitetura-baseada-instancias.md) · [ADR-0005](adr/pt/0005-serializacao-binaria-multilinguagem.md) · [ADR-0006](adr/pt/0006-separacao-duas-threads-rede-gameloop.md) · [ADR-0008](adr/pt/0008-ciclo-de-vida-sessao-e-identidade-cliente.md) · [ADR-0009](adr/pt/0009-estrategia-autenticacao-simplificada-auto-join.md)

* [x] **Mapeamento de SocketAddr para Entidade**:
  - Mapear o IP / `SocketAddr` do cliente diretamente para um `EntityID` único dentro da instância ativa.
  - Tratar entrada, desconexão básica e detecção de timeout de clientes.
* [x] **Handshake Simplificado**:
  - Manter a autenticação mínima (sem servidores de token/auth por enquanto, otimizado para validação rápida do protótipo).

---

### Fase 3: Transmissão de Estado do Mundo & Sincronização de Clientes (Concluído)
> **Spec:** [Phase 3 Spec](roadmap-specs/phase3-world-state-broadcasting-spec.md) · **Reference ADRs:** [ADR-0001](adr/pt/0001-arquitetura-servidor-autoritativo.md) · [ADR-0002](adr/pt/0002-arquitetura-baseada-instancias.md) · [ADR-0003](adr/pt/0003-arquitetura-apenas-mapas-2d.md) · [ADR-0005](adr/pt/0005-serializacao-binaria-multilinguagem.md) · [ADR-0006](adr/pt/0006-separacao-duas-threads-rede-gameloop.md) · [ADR-0008](adr/pt/0008-ciclo-de-vida-sessao-e-identidade-cliente.md) · [ADR-0009](adr/pt/0009-estrategia-autenticacao-simplificada-auto-join.md)

* [x] **Geração de Snapshots**:
  - Criar um pacote Protobuf de snapshot (`WorldState`) representando as posições e estados de todas as entidades ativas.
* [x] **Transmissão por Tick**:
  - Enviar snapshots de estado para todos os endereços de clientes mapeados em intervalos regulares de tick.
* [x] **Integração com Clientes**:
  - Validar o consumo de snapshots nos exemplos de Love2D, Godot, Python e CLI Rust.

---

### Fase 4: Registro de Eventos & Sistema de Replay Determinístico
> **Spec:** [Phase 4 Spec](roadmap-specs/phase4-deterministic-replay-spec.md) · **Reference ADRs:** [ADR-0001](adr/pt/0001-arquitetura-servidor-autoritativo.md) · [ADR-0002](adr/pt/0002-arquitetura-baseada-instancias.md) · [ADR-0003](adr/pt/0003-arquitetura-apenas-mapas-2d.md) · [ADR-0005](adr/pt/0005-serializacao-binaria-multilinguagem.md) · [ADR-0006](adr/pt/0006-separacao-duas-threads-rede-gameloop.md) · [ADR-0007](adr/pt/0007-simulacao-deterministica-e-ponto-fixo.md) · [ADR-0008](adr/pt/0008-ciclo-de-vida-sessao-e-identidade-cliente.md) · [ADR-0009](adr/pt/0009-estrategia-autenticacao-simplificada-auto-join.md) · [ADR-0010](adr/pt/0010-formato-replay-event-sourcing.md) · [ADR-0011](adr/pt/0011-transmissao-espectador-replay-autoritativo.md)

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

### Fase 5: Motor de Física e Colisão Determinístico
> **Spec:** [Phase 5 Spec](roadmap-specs/phase5-physics-and-collision-spec.md) · **Reference ADRs:** [ADR-0001](adr/pt/0001-arquitetura-servidor-autoritativo.md) · [ADR-0002](adr/pt/0002-arquitetura-baseada-instancias.md) · [ADR-0003](adr/pt/0003-arquitetura-apenas-mapas-2d.md) · [ADR-0005](adr/pt/0005-serializacao-binaria-multilinguagem.md) · [ADR-0006](adr/pt/0006-separacao-duas-threads-rede-gameloop.md) · [ADR-0007](adr/pt/0007-simulacao-deterministica-e-ponto-fixo.md) · [ADR-0008](adr/pt/0008-ciclo-de-vida-sessao-e-identidade-cliente.md) · [ADR-0010](adr/pt/0010-formato-replay-event-sourcing.md) · [ADR-0011](adr/pt/0011-transmissao-espectador-replay-autoritativo.md) · [ADR-0012](adr/pt/0012-colisao-2d-deterministica-e-resolucao-cinematica.md) · [ADR-0013](adr/pt/0013-navegacao-e-movimentacao-por-destino-autoritativa.md)

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

### Fase 6: Scripting Embutido & Lógica de Jogo (Motor Lua)

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

### Fase 6.5: Validação, Experiência do Desenvolvedor (DX) & Estabilização de API
> **Foco do Marco:** Pausa temporária na adição de novas funcionalidades estruturais para focar na estabilidade da versão v0.6.x, DX, abstração de clientes e documentação amigável antes de avançar para a infraestrutura de múltiplas salas.

> [!NOTE]
> **Pausa Estratégica (Consolidação da v0.6.x):** Ao atingir a Fase 6, o `loci2d` alcança uma pilha multiplayer em LAN totalmente jogável e determinística. O desenvolvimento de novas funcionalidades será pausado temporariamente para validar o motor, coletar feedback e refinar a API pública.
> 
> Devido à natureza heterogênea deste marco (abrangendo SDKs de clientes, documentação e arquitetura da engine), o roadmap detalhado e as sub-especificações para a Fase 6.5 foram movidos para um documento dedicado.
> 
> **[Veja o Roadmap completo da Fase 6.5 aqui](phase6.5-roadmap.pt-BR.md)** (Veja a [ADR 0015](adr/pt/0015-estrutura-roadmap-dedicado-fase6.5-validacao-dx.md) para detalhes).

---

### Fase 7: Gerenciador de Múltiplas Instâncias e Salas

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

### Fase 8: Endurecimento de Produção, Segurança & Autenticação

* [ ] **Autenticação de Sessão & Tokens**:
  - Implementar validação de handshake seguro, tokens de sessão e webhooks opcionais de autenticação externa.
* [ ] **Validação de Intenções & Anti-Adulteração**:
  - Aplicar limitação de taxa (rate-limiting) de intenções, integridade de sequência de pacotes e checagens de sanidade de velocidade/teletransporte.
* [ ] **Blindagem de Segurança do Sandbox Lua**:
  - Implementar proteção OOM (limites de memória por instância), desabilitar globais perigosas nativas (`dofile`, `load`, `getmetatable`) e estabelecer testes de penetração na CI para evitar escapes do Sandbox.
* [ ] **Observabilidade de Produção & Implantação**:
  - Logs estruturados, métricas Prometheus e conteinerização para produção.

---

### Fase 9: Versionamento de Scripts & Recarregamento Dinâmico

* [ ] **Suporte a Replays de Múltiplas Versões**:
  - Empacotar ou versionar scripts Lua para garantir que clientes possam assistir a replays `.loci` antigos com a lógica exata usada naquela partida.
* [ ] **Instâncias Ao Vivo Imutáveis**:
  - Garantir que recarregar scripts no servidor afete apenas novas instâncias; partidas em andamento devem terminar em suas versões de script originais para manter o determinismo.
