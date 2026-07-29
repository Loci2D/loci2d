# ADR 0001: Authoritative Server Architecture

## Status

Accepted

## Context / Contexto

### English
For a 2D multiplayer game, we need to decide between different server architectures:
- **Authoritative Server**: Server controls all game state and validates all client actions
- **Client-Side Prediction**: Clients predict their own actions, server validates
- **Peer-to-Peer**: No central server, clients communicate directly

### Português
Para um jogo multiplayer 2D, precisamos decidir entre diferentes arquiteturas de servidor:
- **Servidor Autoritativo**: Servidor controla todo o estado do jogo e valida todas as ações do cliente
- **Predição no Lado do Cliente**: Clientes predizem suas próprias ações, servidor valida
- **Peer-to-Peer**: Sem servidor central, clientes se comunicam diretamente

## Decision / Decisão

### English
We choose an **Authoritative Server** architecture where:
- All game state resides on the server
- Clients send **intentions** (what they want to do) rather than direct state changes
- Server validates and processes all actions
- Server sends authoritative state updates to clients
- Clients are dumb terminals that only send input and render received state

### Português
Escolhemos uma arquitetura de **Servidor Autoritativo** onde:
- Todo o estado do jogo reside no servidor
- Clientes enviam **intenções** (o que querem fazer) em vez de mudanças diretas de estado
- Servidor valida e processa todas as ações
- Servidor envia atualizações de estado autoritativas para os clientes
- Clientes são terminais burros que apenas enviam input e renderizam o estado recebido

## Consequences / Consequências

### English

**Positive:**
- **Security**: Prevents cheating since clients can't directly modify game state
- **Consistency**: Single source of truth ensures all clients see the same game state
- **Validation**: Server can enforce game rules and prevent invalid actions
- **Simplified Clients**: Clients don't need complex game logic, just rendering and input

**Negative:**
- **Latency**: Client actions must round-trip to server before taking effect
- **Server Load**: All game logic runs on the server, requiring more computational resources
- **Network Traffic**: More frequent state updates from server to all clients
- **Complexity**: Requires client-side prediction/interpolation to mask latency for good UX

### Português

**Positivo:**
- **Segurança**: Previne trapaças já que clientes não podem modificar diretamente o estado do jogo
- **Consistência**: Fonte única de verdade garante que todos os clientes vejam o mesmo estado do jogo
- **Validação**: Servidor pode impor regras do jogo e prevenir ações inválidas
- **Clientes Simplificados**: Clientes não precisam de lógica complexa de jogo, apenas renderização e input

**Negativo:**
- **Latência**: Ações do cliente precisam fazer round-trip ao servidor antes de ter efeito
- **Carga no Servidor**: Toda a lógica do jogo roda no servidor, exigindo mais recursos computacionais
- **Tráfego de Rede**: Atualizações de estado mais frequentes do servidor para todos os clientes
- **Complexidade**: Requer predição/interpolação no lado do cliente para mascarar latência para boa UX
