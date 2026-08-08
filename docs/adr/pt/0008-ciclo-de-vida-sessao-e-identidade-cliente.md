# ADR 0008: Ciclo de Vida de Sessão e Gerenciamento de Identidade de Cliente

## Status

Aceito — **Temporário (Fase 2–4)**  
Previsto para revisão na **Fase 5**, quando a concorrência multi-instância e autenticação forem introduzidas.

## Contexto

Na Fase 1, o rastreamento de clientes era rudimentar: um simples `HashMap<SocketAddr, u64>` mapeava endereços de rede para IDs de entidades com auto-join no primeiro pacote. Esta abordagem tinha várias limitações críticas:

1. **Nenhum ciclo de vida de sessão**: Entidades eram criadas no primeiro pacote e nunca removidas
2. **Nenhuma detecção de timeout**: Clientes que travaram ou desconectaram deixavam entidades "fantasma" indefinidamente
3. **Nenhum metadado de conexão**: Sem nomes de jogadores, tempos de entrada ou rastreamento de atividade
4. **Nenhuma desconexão graciosa**: Sem protocolo para clientes sinalizarem intenção de sair
5. **Vazamento de memória**: Contagens de entidades e sessões cresciam monotonicamente ao longo do tempo

Para um servidor de jogos pronto para produção, precisamos de gerenciamento de sessão explícito com rastreamento de ciclo de vida, detecção de timeout e limpeza limpa de recursos.

Alternativas consideradas:

| Abordagem | Por que foi rejeitada |
|---|---|
| Sessões com suporte de banco de dados | Excessivo para Fase 2 de instância única; introduz dependências externas e complexidade |
| IDs de sessão baseados em token | Requer infraestrutura de autenticação (adiado para Fase 5) |
| Máquinas de estado complexas | Pesado demais para fase de validação; modelo de 3 estados simples é suficiente |
| Contagem de referências | Não aborda detecção de timeout ou semânticas de desconexão graciosa |

## Decisão

Implementamos um **ciclo de vida de sessão baseado em SocketAddr** com a seguinte arquitetura:

### Modelo de Identidade de Sessão
- **Identificador primário**: `SocketAddr` (combinação IP:porta do cliente)
- **Metadados de sessão**: `session_id: u64`, `entity_id: u64`, `player_name: String`
- **Timestamps**: `connected_at: Instant`, `last_seen: Instant`
- **Máquina de estado**: `Active` → `TimedOut` / `Disconnected`

### Máquina de Estado de Sessão
```
SocketAddr Desconhecido → [JoinIntent] → Sessão Ativa
SocketAddr Desconhecido → [Outro Intent] → Descarte (Sem Sessão)
Sessão Ativa → [DisconnectIntent] → Desconectado → Despawn de Entidade
Sessão Ativa → [Sem pacotes por > CLIENT_TIMEOUT_SECS] → TimedOut → Despawn de Entidade
```

### Estruturas de Dados
```rust
pub struct ClientSession {
    pub session_id: u64,
    pub addr: SocketAddr,
    pub entity_id: u64,
    pub player_name: String,
    pub connected_at: Instant,
    pub last_seen: Instant,
    pub state: SessionState,
}

pub enum SessionState {
    Active,
    TimedOut,
    Disconnected,
}
```

### Mapeamento Bidirecional
- `sessions: HashMap<SocketAddr, ClientSession>` — Endereço → Sessão
- `entity_to_addr: HashMap<u64, SocketAddr>` — EntityID → Endereço (para buscas reversas)

### Detecção de Timeout
- Configurável `CLIENT_TIMEOUT_SECS` (padrão: 10s)
- Verificado a cada tick em `Instance::check_timeouts()`
- Sessões excedendo o limite transicionam para `TimedOut` e acionam despawn de entidade

## Por Que Isso É Temporário

Este design é limitado à operação de instância única (Fase 2–4). Limitações conhecidas para a Fase 5:

1. **Risco de colisão de SocketAddr**: Cenários de NAT traversal ou proxy poderiam causar conflitos de endereço
2. **Sem identidade persistente**: Mudanças de IP do cliente quebram sessões (redes móveis, DHCP)
3. **Escopo de instância única**: Mapas de sessão são por instância; multi-instância requer gerenciamento de sessão distribuído
4. **Sem autenticação**: SocketAddr pode ser falsificado em ambientes não confiáveis
5. **Overhead de memória**: Uma sessão por cliente conectado; necessita otimização para grandes quantidades de jogadores

## Caminho de Migração (Fase 5)

Quando multi-instância e autenticação forem introduzidos:
- Substituir chave primária `SocketAddr` por `session_id: u64` criptográfico ou token JWT
- Adicionar camada de persistência de sessão (Redis/banco de dados) para sessões multi-instância
- Implementar lógica de reconexão de sessão para mudanças de IP
- Adicionar validação de sessão contra serviço de autenticação

## Consequências

**Positivo:**
- **Gerenciamento limpo de recursos**: Entidades são despawnadas na desconexão/timeout
- **Rastreamento de atividade**: `last_seen` permite monitoramento de heartbeat e detecção de inatividade
- **Desconexões graciosas**: Clientes podem sinalizar intenção de sair com razão opcional
- **Visibilidade de debug**: Metadados de sessão auxiliam logging e troubleshooting
- **Implementação simples**: Usa coleções padrão do Rust com semântica clara
- **Limite explícito**: Exige JoinIntent válido antes de processar intents de gameplay

**Negativo:**
- **Fragilidade de SocketAddr**: Mudanças de IP quebram sessões (aceitável para escopo localhost/LAN da Fase 2–4)
- **Sem autenticação**: Falsificação de endereço possível em redes não confiáveis (adiado para Fase 5)
- **Crescimento de memória**: Mapa de sessão ilimitado (aceitável para escala esperada da Fase 2–4)
- **Apenas instância única**: Não generaliza para arquiteturas distribuídas (preocupação da Fase 5)
