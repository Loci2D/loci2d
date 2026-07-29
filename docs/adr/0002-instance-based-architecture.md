# ADR 0002: Instance-Based Architecture

## Status

Accepted

## Context / Contexto

### English
Indie developers and students often struggle with complex networking architectures when building multiplayer games. Common approaches include:
- **Single World Server**: All players in one shared world (MMORPG style)
- **Lobby-Based**: Players matchmake into temporary sessions
- **Peer-to-Peer**: Complex NAT traversal and security issues
- **Spatial Partitioning**: Complex to implement and debug

These approaches often introduce significant complexity that distracts from core game development.

### Português
Desenvolvedores indie e estudantes frequentemente lutam com arquiteturas de rede complexas ao construir jogos multiplayer. Abordagens comuns incluem:
- **Servidor de Mundo Único**: Todos os jogadores em um mundo compartilhado (estilo MMORPG)
- **Baseado em Lobby**: Jogadores entram em sessões temporárias
- **Peer-to-Peer**: Problemas complexos de NAT traversal e segurança
- **Particionamento Espacial**: Complexo de implementar e debugar

Essas abordagens frequentemente introduzem complexidade significativa que distrai do desenvolvimento core do jogo.

## Decision / Decisão

### English
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

### Português
Escolhemos uma **Arquitetura Baseada em Instâncias** onde:
- O servidor gerencia múltiplas **instâncias** independentes (salas/sessões)
- Cada instância tem seu próprio estado de jogo, lista de entidades e tick rate
- Jogadores se conectam a instâncias específicas em vez de um mundo global
- Instâncias podem ser criadas/destruídas dinamicamente baseado na demanda de jogadores
- Cada instância roda seu próprio game loop independentemente

Esta arquitetura suporta múltiplos gêneros de jogos:
- **Jogos cooperativos**: Grupos pequenos em instâncias privadas
- **Battle Royale**: Instâncias com contagem limitada de jogadores
- **RPGs baseados em party**: Grupos aventurando juntos
- **Jogos baseados em sessão**: Sessões de match rápido

## Consequences / Consequências

### English

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

### Português

**Positivo:**
- **Rede Simplificada**: Limites claros entre instâncias reduzem complexidade
- **Escalabilidade**: Fácil escalar horizontalmente distribuindo instâncias entre servidores
- **Flexibilidade de Gênero**: Suporta muitos tipos de jogos sem mudanças arquiteturais
- **Isolamento**: Problemas em uma instância não afetam outras
- **Curva de Aprendizado**: Mais fácil para iniciantes entenderem do que particionamento espacial
- **Performance Preditível**: Cada instância tem limites conhecidos de jogadores

**Negativo:**
- **Sem Mundo Global**: Não suporta jogos que requerem um mundo persistente único (MMORPG)
- **Gerenciamento de Instâncias**: Precisa de lógica para criar/destruir instâncias
- **Comunicação Cross-Instance**: Complexo se jogadores precisam interagir entre instâncias
- **Balanceamento de Carga**: Precisa distribuir jogadores entre instâncias eficientemente
- **Persistência de Estado**: Requer sistema separado se estado global é necessário
