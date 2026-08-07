# ADR 0002: Instance-Based Architecture

## Status

Accepted

## Context

Indie developers and students often struggle with complex networking architectures when building multiplayer games. Common approaches include:
- **Single World Server**: All players in one shared world (MMORPG style)
- **Lobby-Based**: Players matchmake into temporary sessions
- **Peer-to-Peer**: Complex NAT traversal and security issues
- **Spatial Partitioning**: Complex to implement and debug

These approaches often introduce significant complexity that distracts from core game development.

## Decision

We choose an **Instance-Based Architecture** where:
- The server manages multiple independent **instances** (rooms/sessions)
- Each instance has its own game state, entity list, and tick rate
- Players connect to specific instances rather than a global world
- Instances can be created/destroyed dynamically based on player demand
- Each instance runs its own game loop independently

This architecture supports multiple game genres:
- **Co-op games**: Small groups in private instances
- **Battle Royale**: Instances with limited player counts
- **Party-based RPGs**: Groups adventuring together
- **Session-based games**: Quick match sessions

## Consequences

**Positive:**
- **Simplified Networking**: Clear boundaries between instances reduce complexity
- **Scalability**: Easy to horizontally scale by distributing instances across servers
- **Genre Flexibility**: Supports many game types without architectural changes
- **Isolation**: Issues in one instance don't affect others
- **Learning Curve**: Easier for beginners to understand than spatial partitioning
- **Predictable Performance**: Each instance has known player limits

**Negative:**
- **No Global World**: Can't support games requiring a single persistent world (MMORPG)
- **Instance Management**: Need logic for creating/destroying instances
- **Cross-Instance Communication**: Complex if players need to interact across instances
- **Load Balancing**: Need to distribute players across instances efficiently
- **State Persistence**: Requires separate system if global state is needed
