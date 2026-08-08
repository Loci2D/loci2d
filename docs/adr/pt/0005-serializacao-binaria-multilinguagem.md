# ADR 0005: Serialização Binária Multi-linguagem

## Status

Aceito (Substitui [ADR 0004](0004-formato-serializacao-pacotes.md))

## Contexto

No ADR 0004, Serde + Bincode foi selecionado para serialização. Embora compacto e de custo zero para aplicações puramente em Rust, o formato de codificação do Bincode é fortemente acoplado à representação interna de dados do Rust e à ordem do Serde.

Para suportar um **servidor autoritativo baseado em instâncias** utilizável por estudantes em diversos motores de jogos e ambientes cliente (como **Godot (GDScript/C#)**, **Love2D (Lua)**, **Unity (C#)** ou **WebSockets (JS/TS)**):
- O protocolo de rede deve ser **independente de linguagem** (language-agnostic) e definido por um contrato de schema explícito.
- Formatos em texto puro como **JSON ou XML são inviáveis** para jogos multiplayer de alta frequência (ex.: MOBA / jogos de ação em tempo real) devido ao overhead de banda e custo de parse de strings.
- O formato deve preservar baixa latência, pacotes binários compactos e segurança de tipos em todas as linguagens suportadas.

## Decisão

Escolhemos **Protocol Buffers (Protobuf v3)** utilizando **`prost`** em Rust como o formato padrão de serialização de rede para o `loci2d`, mantendo o **Padrão Envelope**:

1. **Definição de Schema (`.proto`):**
   - Todos os pacotes de rede são definidos em um arquivo `.proto` único e autoritativo (`proto/game_packets.proto`).
   - Campos `oneof` padronizados mapeiam diretamente para as intenções do cliente (`MoveIntent`, `ActionIntent`, `PingIntent`).

2. **Geração de Código Multi-linguagem:**
   - **Rust (Servidor):** Crate `prost` gera código Rust type-safe a partir de schemas `.proto` no momento do build.
   - **Godot (GDScript/C#):** Utiliza `godot-protobuf` ou `Google.Protobuf` nativo do C#.
   - **Love2D (Lua):** Utiliza `lua-protobuf` ou `pb.lua` para codificar/decodificar buffers binários diretamente compatíveis.

3. **Estrutura de Envelope (`GamePacket`):**
   (Ver definição Protobuf acima)

## Consequências

**Positivo:**
- **Compatibilidade Real Multi-linguagem:** Qualquer linguagem ou engine com suporte a Protobuf (Godot, Love2D, Unity, Web, Python, C++) pode se comunicar com o servidor.
- **Contrato de Schema Formal:** Fonte única de verdade em `proto/game_packets.proto` torna a API de rede explícita para os alunos.
- **Alto Desempenho e Baixa Banda:** Codificação varint e tags compactas tornam o Protobuf ideal para taxas de atualização (tick rate) de MOBAs em tempo real (20-60 Hz).
- **Compatibilidade Retroativa e Futura:** Números de campo permitem evolução do schema sem quebrar clientes antigos.

**Negativo:**
- **Dependência Externa:** Requer ferramentas Protobuf (`protoc` ou `prost-build`) durante o build.
- **Pequeno Overhead vs Zero-Copy:** A decodificação realiza pequenas alocações de memória se comparada a buffers zero-copy, embora irrelevante para a escala esperada.
