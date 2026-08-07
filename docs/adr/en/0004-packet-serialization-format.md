# ADR 0004: Packet Serialization Format

## Status

Superseded by [ADR 0005](0005-cross-language-binary-serialization.md)

## Context

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

## Decision

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

## Consequences

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
