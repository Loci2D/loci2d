# ADR 0005: Cross-Language Binary Serialization

## Status

Accepted (Supersedes [ADR 0004](0004-packet-serialization-format.md))

## Context / Contexto

### English
In ADR 0004, Serde + Bincode was selected for serialization. While compact and zero-cost for pure Rust applications, Bincode's encoding format is tightly coupled to Rust's internal data representation and Serde derive order. 

To support an **authoritative instance-based server** usable by students across diverse game engines and client environments (such as **Godot (GDScript/C#)**, **Love2D (Lua)**, **Unity (C#)**, or **WebSockets (JS/TS)**):
- The network protocol must be **language-agnostic** and defined by an explicit schema contract.
- Plain text formats like **JSON or XML are unviable** for high-frequency multiplayer games (e.g., MOBA / real-time action games) due to bandwidth overhead and string parsing performance costs.
- The format must preserve low latency, compact binary payloads, and type safety across all supported language bindings.

### Português
No ADR 0004, Serde + Bincode foi selecionado para serialização. Embora compacto e de custo zero para aplicações puramente em Rust, o formato de codificação do Bincode é fortemente acoplado à representação interna de dados do Rust e à ordem do Serde.

Para suportar um **servidor autoritativo baseado em instâncias** utilizável por estudantes em diversos motores de jogos e ambientes cliente (como **Godot (GDScript/C#)**, **Love2D (Lua)**, **Unity (C#)** ou **WebSockets (JS/TS)**):
- O protocolo de rede deve ser **independente de linguagem** (language-agnostic) e definido por um contrato de schema explícito.
- Formatos em texto puro como **JSON ou XML são inviáveis** para jogos multiplayer de alta frequência (ex.: MOBA / jogos de ação em tempo real) devido ao overhead de banda e custo de parse de strings.
- O formato deve preservar baixa latência, pacotes binários compactos e segurança de tipos em todas as linguagens suportadas.

## Decision / Decisão

### English
We select **Protocol Buffers (Protobuf v3)** using **`prost`** in Rust as the standard network serialization format for `loci2d`, maintaining the **Envelope Pattern**:

1. **Schema Definition (`.proto`):**
   - All network packets are defined in a single, authoritative `.proto` file (`proto/game_packets.proto`).
   - Standardized `oneof` fields map directly to client intents (`MoveIntent`, `ActionIntent`, `PingIntent`).

2. **Cross-Language Code Generation:**
   - **Rust (Server):** Standard `prost` crate generates type-safe Rust code from `.proto` schemas during build time.
   - **Godot (GDScript/C#):** Uses `godot-protobuf` or native C# `Google.Protobuf`.
   - **Love2D (Lua):** Uses `lua-protobuf` or `pb.lua` to encode/decode binary buffers directly matching `.proto` fields.

3. **Envelope Structure (`GamePacket`):**
   ```protobuf
   syntax = "proto3";
   package loci2d;

   message Vector2 {
     float x = 1;
     float y = 2;
   }

   message MoveIntent {
     Vector2 direction = 1;
   }

   message ActionIntent {
     uint32 ability_id = 1;
   }

   message PingIntent {}

   message ClientIntent {
     oneof intent {
       MoveIntent move = 1;
       ActionIntent action = 2;
       PingIntent ping = 3;
     }
   }

   message GamePacket {
     uint64 sequence_id = 1;
     uint64 timestamp = 2;
     ClientIntent intent = 3;
   }

   message ServerResponse {
     uint64 sequence_id = 1;
     string status = 2;
   }
   ```

### Português
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

## Consequences / Consequências

### English

**Positive:**
- **True Cross-Language Compatibility:** Any language/engine with Protobuf support (Godot, Love2D, Unity, Web, Python, C++) can communicate with the server.
- **Formal Schema Contract:** Single source of truth in `proto/game_packets.proto` makes network API explicit for students.
- **High Performance & Low Bandwidth:** Varint encoding and compact field tags make Protobuf suitable for real-time MOBA tick rates (20-60 Hz).
- **Forward & Backward Compatibility:** Field numbers allow schema additions without breaking legacy client binaries.

**Negative:**
- **External Dependency:** Requires Protobuf tools (`protoc` or `prost-build`) during build pipeline.
- **Slight Overhead vs Zero-Copy:** Decoding incurs minimal memory allocation compared to zero-copy raw buffers, though negligible for expected game scale.

### Português

**Positivo:**
- **Compatibilidade Real Multi-linguagem:** Qualquer linguagem ou engine com suporte a Protobuf (Godot, Love2D, Unity, Web, Python, C++) pode se comunicar com o servidor.
- **Contrato de Schema Formal:** Fonte única de verdade em `proto/game_packets.proto` torna a API de rede explícita para os alunos.
- **Alto Desempenho e Baixa Banda:** Codificação varint e tags compactas tornam o Protobuf ideal para taxas de atualização (tick rate) de MOBAs em tempo real (20-60 Hz).
- **Compatibilidade Retroativa e Futura:** Números de campo permitem evolução do schema sem quebrar clientes antigos.

**Negativo:**
- **Dependência Externa:** Requer ferramentas Protobuf (`protoc` ou `prost-build`) durante o build.
- **Pequeno Overhead vs Zero-Copy:** A decodificação realiza pequenas alocações de memória se comparada a buffers zero-copy, embora irrelevante para a escala esperada.
