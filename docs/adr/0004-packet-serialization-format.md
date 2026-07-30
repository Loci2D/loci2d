# ADR 0004: Packet Serialization Format

## Status

Superseded by [ADR 0005](0005-cross-language-binary-serialization.md)

## Context / Contexto

### English
Networked games need a reliable way to serialize data for transmission. Common approaches include:
- **JSON/Text**: Human-readable but verbose and slow to parse
- **Protocol Buffers**: Fast and compact but requires external tooling
- **Custom Binary**: Fast but error-prone and hard to maintain
- **XML**: Too verbose and slow for real-time games

Additionally, the packet format should support:
- Easy addition of new client intents as the game evolves
- Version compatibility for protocol changes
- Efficient bandwidth usage
- Type safety to prevent serialization errors

### Português
Jogos em rede precisam de uma forma confiável de serializar dados para transmissão. Abordagens comuns incluem:
- **JSON/Texto**: Legível por humanos mas verboso e lento para parsear
- **Protocol Buffers**: Rápido e compacto mas requer ferramentas externas
- **Binário Customizado**: Rápido mas propenso a erros e difícil de manter
- **XML**: Muito verboso e lento para jogos em tempo real

Além disso, o formato de pacote deve suportar:
- Adição fácil de novas intenções de cliente conforme o jogo evolui
- Compatibilidade de versão para mudanças de protocolo
- Uso eficiente de banda
- Segurança de tipos para prevenir erros de serialização

## Decision / Decisão

### English
We choose **Serde + Bincode** for serialization with an **Envelope Pattern**:

**Serialization Format:**
- **Serde**: Rust serialization framework for type-safe (de)serialization
- **Bincode**: Compact binary format for efficient network transmission
- **Derive Macros**: Automatic implementation of Serialize/Deserialize traits

**Packet Structure:**
- **GamePacket (Envelope)**: Wrapper containing sequence_id and ClientIntent
- **ClientIntent (Extensible Enum)**: All possible client actions as enum variants
- **Vector2**: Reusable 2D position type used across multiple intents
- **ServerResponse**: Standardized response format with sequence_id

**Extensibility:**
Adding new client intents is as simple as adding a new enum variant:
```rust
pub enum ClientIntent {
    Move { direction: Vector2 },
    Action { ability_id: u32 },
    Ping,
    // New intent can be added here without breaking existing code
    CastSpell { spell_id: u32, target: Vector2 },
}
```

### Português
Escolhemos **Serde + Bincode** para serialização com um **Padrão Envelope**:

**Formato de Serialização:**
- **Serde**: Framework de serialização Rust para (de)serialização type-safe
- **Bincode**: Formato binário compacto para transmissão de rede eficiente
- **Macros Derive**: Implementação automática de traits Serialize/Deserialize

**Estrutura de Pacote:**
- **GamePacket (Envelope)**: Wrapper contendo sequence_id e ClientIntent
- **ClientIntent (Enum Extensível)**: Todas as ações possíveis do cliente como variantes de enum
- **Vector2**: Tipo de posição 2D reutilizável usado em múltiplas intenções
- **ServerResponse**: Formato de resposta padronizado com sequence_id

**Extensibilidade:**
Adicionar novas intenções de cliente é tão simples quanto adicionar uma nova variante de enum:
```rust
pub enum ClientIntent {
    Move { direction: Vector2 },
    Action { ability_id: u32 },
    Ping,
    // Nova intenção pode ser adicionada aqui sem quebrar código existente
    CastSpell { spell_id: u32, target: Vector2 },
}
```

## Consequences / Consequências

### English

**Positive:**
- **Easy Extension**: Adding new intents requires only adding enum variants
- **Type Safety**: Compile-time guarantees prevent serialization errors
- **Efficiency**: Binary format minimizes bandwidth usage
- **No External Tools**: Pure Rust solution, no code generation steps
- **Versioning**: sequence_id enables basic version compatibility
- **Pattern Matching**: Rust's exhaustive matching ensures all intents are handled
- **Zero-Cost Abstractions**: Serde has minimal runtime overhead

**Negative:**
- **Rust-Only**: Other languages need manual implementation or serde-compatible libraries
- **Binary Format**: Not human-readable for debugging (need logging)
- **Breaking Changes**: Adding fields to existing variants can break old clients
- **No Schema Validation**: Unlike protobuf, no formal schema definition
- **Forward Compatibility**: Old clients can't deserialize new enum variants they don't know

### Português

**Positivo:**
- **Extensão Fácil**: Adicionar novas intenções requer apenas adicionar variantes de enum
- **Segurança de Tipos**: Garantias em tempo de compilação previnem erros de serialização
- **Eficiência**: Formato binário minimiza uso de banda
- **Sem Ferramentas Externas**: Solução pura em Rust, sem passos de geração de código
- **Versionamento**: sequence_id permite compatibilidade básica de versões
- **Pattern Matching**: Matching exhaustivo do Rust garante que todas as intenções são tratadas
- **Abstrações Zero-Cost**: Serde tem overhead mínimo em runtime

**Negativo:**
- **Apenas Rust**: Outras linguagens precisam de implementação manual ou bibliotecas compatíveis com serde
- **Formato Binário**: Não legível por humanos para debug (precisa de logging)
- **Mudanças Quebrantes**: Adicionar campos a variantes existentes pode quebrar clientes antigos
- **Sem Validação de Schema**: Ao contrário de protobuf, sem definição formal de schema
- **Compatibilidade Forward**: Clientes antigos não podem deserializar novas variantes de enum que não conhecem
