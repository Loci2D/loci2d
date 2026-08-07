# ADR 0004: Formato de Serialização de Pacotes

## Status

Substituído por [ADR 0005](0005-cross-language-binary-serialization.md)

## Contexto

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

## Decisão

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

## Consequências

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
