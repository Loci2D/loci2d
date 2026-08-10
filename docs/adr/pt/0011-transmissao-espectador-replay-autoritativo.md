# ADR 0011: Transmissão Autoritativa de Replay para Espectadores

## Status

Aceito (Fase 4)

## Contexto

O `loci2d` foi projetado para suportar diversos motores de jogo clientes, incluindo **Godot 4 (GDScript/C#)**, **Love2D (Lua)**, **Python** e a **CLI em Rust**.

Quando partidas gravadas são reproduzidas para observação visual ou depuração, precisamos de um mecanismo para que esses clientes renderizem o replay na tela.

Duas alternativas arquiteturais foram avaliadas:

| Abordagem | Detalhes de Implementação | Complexidade nos Clientes | Sobrecarga de Manutenção |
|---|---|---|---|
| **Replay por Simulação no Cliente** | Cada linguagem cliente (Lua, GDScript, Python) implementa seu próprio motor de ponto fixo, carrega o arquivo `.loci` e executa o loop de simulação localmente. | **Muito Alta** (Exige portar a matemática determinística, ordenação `BTreeMap` e física em Rust para mais de 3 linguagens diferentes). | **Alta** (Qualquer ajuste na física do servidor precisaria ser replicado identicamente em todos os clientes). |
| **Transmissão UDP Autoritativa para Espectadores** | O servidor loci2d executa o motor de replay localmente na taxa de tick desejada e transmite snapshots padrão de `WorldState` via UDP ([ADR-0005](0005-serializacao-binaria-multilinguagem.md)). | **Zero** (Os clientes reutilizam os mesmos renderizadores de rede da Fase 3 sem alterar nenhuma linha de código). | **Zero** (Toda a lógica de simulação permanece centralizada no servidor em Rust). |

## Decisão

Adotamos a **Transmissão Autoritativa de Replay para Espectadores** como o mecanismo primário de visualização para os clientes multi-linguagem:

```
┌──────────────────────────────────────────────────────────────────────────┐
│ Servidor loci2d (Modo Replay)                                            │
│ loci2d --replay match.loci --broadcast 127.0.0.1:4000 --speed 1.0       │
│                                                                          │
│ 1. Carrega match.loci (Inputs de Event Sourcing)                         │
│ 2. Executa a Instance com física de ponto fixo na taxa fixa (30 Hz)      │
│ 3. Gera snapshot padrão de WorldState em Protobuf                        │
│ 4. Transmite via socket UDP ─────────────────────────────────────────┐   │
└──────────────────────────────────────────────────────────────────────┼───┘
                                                                       │
                                      Transmissão UDP (WorldState)     │
                                                                       ▼
                        ┌──────────────────────────────────────────────────────────┐
                        │             Motores de Jogo dos Clientes                 │
                        ├────────────────────┬────────────────────┬────────────────┤
                        │ Godot 4 (GDScript) │    Love2D (Lua)    │  Python / CLI  │
                        │ Visual Node View   │  Canvas 2D Render  │  State Monitor │
                        └────────────────────┴────────────────────┴────────────────┘
```

### 1. Protocolo Unificado de Clientes
Como o servidor de replay emite envelopes padrão `ServerPacket` contendo snapshots `WorldState`, os clientes conectados não precisam diferenciar se a partida está ocorrendo ao vivo ou sendo reproduzida de uma gravação.

### 2. Controle de Velocidade de Reprodução
O executor de replay oferece suporte a multiplicadores de velocidade configuráveis:
- `0.5x`: Câmera lenta para inspeção detalhada de física e colisões.
- `1.0x`: Reprodução da partida em tempo real.
- `2.0x` / `4.0x`: Avanço rápido para revisão da partida.
- `--verify` (Velocidade Máxima): Execução headless sem transmissão UDP para validação em pipelines de CI/CD.

## Consequências

**Positivas:**
- **Zero Modificações nos Clientes**: Clientes em Godot, Love2D, Python e Rust CLI conseguem reproduzir partidas gravadas imediatamente sem alterar uma única linha de código.
- **Fonte Única da Verdade**: Regras de física e cálculos de simulação residem exclusivamente no código Rust, eliminando divergências sutis de ponto flutuante entre linguagens.
- **Eficiência de Rede**: O replay se beneficia da mesma compactação binária Protobuf utilizada nas partidas ao vivo.
- **Simplicidade Pedagógica**: Desenvolvedores indies e estudantes podem assistir seus replays utilizando o motor de jogo que preferirem.

**Negativas:**
- **Necessidade do Processo do Servidor**: Assistir ao replay requer que o binário do servidor `loci2d` esteja em execução no `localhost` em modo de replay.
