# ADR 0010: Armazenamento de Replay via Event Sourcing e Estratégia de Checkpoints

## Status

Aceito (Fase 4)

## Contexto

Para que o `loci2d` suporte gravação persistente de partidas, verificação offline de determinismo e replays para espectadores, é necessário definir um formato de armazenamento para o histórico de partidas.

Existem dois padrões arquiteturais principais para gravação de simulações em jogos multiplayer:

| Abordagem | Mecanismo de Armazenamento | Sobrecarga de Espaço | Sobrecarga de Execução | Resiliência a Dessincronização |
|---|---|---|---|---|
| **Streaming de Snapshots Completos** | Serializar o `WorldState` completo a cada tick | **Alta** (~100 KB–1 MB/s dependendo da quantidade de entidades) | **Baixa** (Busca O(1) de qualquer quadro) | Alta (autocorreção a cada quadro, mas mascara bugs de simulação) |
| **Event Sourcing (Log de Inputs)** | Gravar semente/configuração inicial + inputs discretos por tick | **Extremamente Baixa** (~1–5 KB/min para partidas típicas) | **Moderada** (Simulação sequencial O(N) a partir do tick 0) | Estrita (expõe qualquer bug ou não-determinismo na simulação) |

Além disso, o formato precisa respeitar os requisitos do projeto:
1. **Compatibilidade Multi-Linguagem ([ADR-0005](0005-serializacao-binaria-multilinguagem.md))**: Arquivos de partida devem ser inspecionáveis por ferramentas externas em Python, Godot (GDScript) ou Lua.
2. **Desacoplamento de Rede Efêmera ([ADR-0008](0008-ciclo-de-vida-sessao-e-identidade-cliente.md))**: O jogo ao vivo identifica clientes por `SocketAddr`, que não existe ou muda em ambientes de replay.
3. **Detecção Automatizada de Dessincronização**: Testes automatizados em CI precisam de um mecanismo para validar a paridade de execução sem serializar o estado completo a cada quadro.

## Decisão

Adotamos **Event Sourcing com Serialização Protobuf v3 e Checkpoints Periódicos de Hash SHA-256** para gravação de partidas (arquivos `.loci`):

### 1. Esquema do Contêiner Protobuf (`proto/replay.proto`)
Todas as gravações de replay são salvas como arquivos binários estruturados definidos via Protocol Buffers:

```protobuf
syntax = "proto3";
package loci2d;

import "game_packets.proto";

message ReplayHeader {
  string magic = 1;             // "LOCI_REPLAY"
  uint32 version = 2;           // Versão do formato de replay (ex: 1)
  uint32 tick_rate = 3;         // Taxa de tick do servidor (ex: 30 Hz)
  uint64 start_timestamp = 4;   // Timestamp Unix (ms)
  uint64 instance_id = 5;       // ID da Instância
  uint64 random_seed = 6;       // Semente determinística do PRNG
  string map_name = 7;          // Identificador do mapa
  string script_hash = 8;       // Hash SHA-256 dos scripts Lua ativos no Tick 0
}

message ReplayIntentEntry {
  uint64 entity_id = 1;         // Desacoplado de SocketAddr
  string player_name = 2;       // Preservado para eventos de join
  ClientIntent intent = 3;      // Intenção concreta (Move, Action, Join, Disconnect)
}

message ReplayTickFrame {
  uint64 tick = 1;
  repeated ReplayIntentEntry entries = 2;
}

message ReplayCheckpoint {
  uint64 tick = 1;
  bytes state_sha256 = 2;       // Digest SHA-256 de 32 bytes do WorldState canônico
  uint32 active_entities = 3;
}

message ReplayFile {
  ReplayHeader header = 1;
  repeated ReplayTickFrame frames = 2;
  repeated ReplayCheckpoint checkpoints = 3;
}
```

### 2. Versionamento de Scripts
Como o `loci2d` incorpora scripting em Lua (Fase 6), as regras da simulação podem mudar com base nos scripts ativos. Para garantir o determinismo durante a reprodução, o cabeçalho do `.loci` armazena um `script_hash` (ex: hash SHA-256 dos scripts Lua ativos no Tick 0).

Durante a validação de replays (`--verify`), o motor calcula o hash dos arquivos de script locais. Se o hash não corresponder a `script_hash`, o motor aborta o carregamento para evitar uma dessincronização garantida devido à incompatibilidade de versão. (Nota: O suporte real a replays de múltiplas versões e recarregamento dinâmico estão adiados para a Fase 9).

### 3. Desacoplamento de Rede através do `entity_id`
Durante o jogo ao vivo, as tuplas `(SocketAddr, ClientIntent)` recebidas são resolvidas dentro da thread do Game Loop para o `entity_id` correspondente. O gravador carimba os eventos diretamente com `entity_id` e nome do jogador, permitindo que o motor de replay aplique os inputs diretamente às entidades da `Instance` sem criar endereços de socket falsos.

### 4. Checkpoints Periódicos de Estado Canônico
A cada $K$ ticks (ex: a cada 60 ticks / 2 segundos), o gravador calcula o hash SHA-256 da representação binária canônica da `Instance` (ordenada por `entity_id` via `BTreeMap` com os bits inteiros brutos de ponto fixo). Esses checkpoints são gravados no contêiner `.loci`.

Durante a validação do replay (`--verify`), o player calcula seu hash local nos ticks de checkpoint e valida a igualdade exata com o hash gravado, identificando o quadro exato em que ocorreu qualquer divergência.

## Consequências

**Positivas:**
- **Consumo Mínimo de Disco**: O Event Sourcing grava apenas inputs ativos, gerando arquivos menores que 100 KB para partidas comuns.
- **Inspeção Multi-Linguagem**: Qualquer linguagem compatível com Protobuf (Python, GDScript, Lua) pode inspecionar arquivos `.loci` para análise de dados ou estatísticas de partida.
- **Localização Instantânea de Dessincronização**: Checkpoints SHA-256 detectam divergências de simulação no tick exato em que ocorrem.
- **Agnóstico à Rede**: Elimina dependências de endereços IP ou portas durante a reprodução.

**Negativas:**
- **Necessidade de Execução Sequencial**: Avançar para o tick $N$ requer simular a partir do tick 0 (aceitável para durações típicas de partida; snapshots intermediários podem ser adicionados em fases futuras se busca instantânea for necessária).
- **Dependência Estrita de Determinismo**: Qualquer divergência nos cálculos de física invalida os quadros subsequentes (endereçado pelo [ADR-0007](0007-simulacao-deterministica-e-ponto-fixo.md)).
