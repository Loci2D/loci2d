# ADR 0001: Arquitetura de Servidor Autoritativo

## Status

Aceito

## Contexto

Para um jogo multiplayer 2D, precisamos decidir entre diferentes arquiteturas de servidor:
- **Servidor Autoritativo**: Servidor controla todo o estado do jogo e valida todas as ações do cliente
- **Predição no Lado do Cliente**: Clientes predizem suas próprias ações, servidor valida
- **Peer-to-Peer**: Sem servidor central, clientes se comunicam diretamente

## Decisão

Escolhemos uma arquitetura de **Servidor Autoritativo** onde:
- Todo o estado do jogo reside no servidor
- Clientes enviam **intenções** (o que querem fazer) em vez de mudanças diretas de estado
- Servidor valida e processa todas as ações
- Servidor envia atualizações de estado autoritativas para os clientes
- Clientes são terminais burros que apenas enviam input e renderizam o estado recebido

## Consequências

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
