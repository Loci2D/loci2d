# ADR 0006: Separação em Duas Threads Rede/Game Loop via `mpsc`

## Status

Aceito — **Temporário (Fase 1–3)**  
Previsto para revisão na **Fase 5**, quando a concorrência multi-instância for introduzida.

## Contexto

Na Fase 1, duas responsabilidades que estavam acopladas em uma única thread precisam ser desacopladas:

- **I/O de rede**: `UdpSocket::recv_from` é uma chamada bloqueante. Enquanto aguarda um pacote, nenhum outro trabalho pode ser executado na mesma thread.
- **Game loop**: O tick de frequência fixa (30 Hz) deve avançar independentemente de chegarem ou não pacotes de rede.

Manter ambas na mesma thread cria um conflito fundamental: o servidor não pode simultaneamente bloquear no `recv_from` e avançar a simulação. Qualquer pacote recebido só é processado quando o servidor não está executando o tick — e o servidor nunca executa o tick porque está sempre bloqueado.

Alternativas consideradas:

| Abordagem | Por que foi rejeitada |
|---|---|
| Fila compartilhada `Arc<Mutex<VecDeque<Intent>>>` | Requer lock explícito; `mpsc` já oferece a mesma semântica sem um lock gerenciado pelo usuário |
| Thread única com `recv_from` não-bloqueante + spin-loop | Desperdiça CPU; adiciona complexidade sem benefício nesta escala |
| Runtime assíncrono (`tokio`) | Direção correta para escala multi-instância a longo prazo, mas introduz complexidade significativa para uma Fase 1 com instância única; adiado para a Fase 5 |
| Thread única com `select!` / I/O baseado em poll | Não trivial de implementar com UDP padrão da `std`; runtime async é mais adequado |

## Decisão

Usar **duas threads do SO** conectadas por um **canal `std::sync::mpsc`** transportando tuplas `(SocketAddr, ClientIntent)`:

```
┌──────────────────────────────────────────────┐
│  Thread de Rede                               │
│  UdpSocket::recv_from (bloqueante)            │
│  → decodifica GamePacket (prost)              │
│  → envia (src_addr, ClientIntent) → intent_tx │
└──────────────────┬───────────────────────────┘
                   │  mpsc::channel<(SocketAddr, ClientIntent)>
                   ▼
┌──────────────────────────────────────────────┐
│  Thread do Game Loop                          │
│  Tick fixo: 30 Hz (~33 ms/tick)               │
│  1. Esvaziar intent_rx (try_recv, não-bloquea)│
│  2. Aplicar intents à Instance                │
│  3. Avançar simulação (atualizar posições)    │
│  4. Dormir o restante do orçamento do tick    │
└──────────────────────────────────────────────┘
```

`mpsc` foi escolhido porque:
- É o primitivo padrão do Rust para comunicação inter-thread produtor→consumidor
- A propriedade de cada intent é transferida (sem clone necessário), garantida em tempo de compilação
- Nenhum lock explícito é necessário; o canal serializa o acesso por si só
- Desligamento limpo: quando o remetente é descartado, o receptor observa um `RecvError`

## Por Que Isso É Temporário

Este design é limitado à operação de instância única (Fase 1–3). Ele tem limitações conhecidas que se tornarão bloqueadores na Fase 5:

1. **Uma thread de rede é um gargalo para múltiplas instâncias.** Com uma thread bloqueante de `recv_from`, todas as instâncias compartilham um único socket de entrada. Adicionar mais instâncias requer roteamento de pacotes de uma thread de rede para N threads de game loop (N remetentes `mpsc`, uma tabela de roteamento) ou migração para um socket assíncrono não-bloqueante.

2. **Uma thread do SO por game loop de instância não escala.** Threads do SO carregam ~8 MB de overhead de pilha cada. Dezenas de instâncias concorrentes são gerenciáveis; centenas não são. Tasks assíncronas (`tokio::task::spawn`) são leves em comparação.

3. **O canal é ilimitado.** `mpsc::channel()` não tem limite de capacidade. Sob um flood de pacotes, a fila de intents cresce sem limite. Um `sync_channel(N)` delimitado adicionaria back-pressure, mas bloqueia a thread de rede quando o game loop fica para trás — aceitável na Fase 1, mas não o primitivo correto para escala de produção.

4. **Nenhum caminho de saída.** O design atual não tem mecanismo para o game loop enviar `WorldState` de volta aos clientes. A Fase 3 exigirá um caminho de saída explícito (ex.: compartilhar `Arc<UdpSocket>` com o game loop, ou uma terceira thread de envio com seu próprio canal).

## Caminho de Migração (Fase 5)

O modelo conceitual — "receptor de rede → fila → game loop" — sobrevive à migração. A maquinaria muda:

| Agora (Fase 1–3) | Alvo na Fase 5 |
|---|---|
| `std::thread::spawn` | `tokio::task::spawn` |
| `UdpSocket::recv_from` (bloqueante) | `tokio::net::UdpSocket::recv_from` (async) |
| `std::sync::mpsc::channel` | `tokio::sync::mpsc::channel` |
| `thread::sleep` para temporização do tick | `tokio::time::interval` |

A refatoração é mecânica, não uma reestruturação. A implementação da Fase 1 usa intencionalmente apenas primitivos da `std` para permanecer livre de dependências externas e maximamente transparente para quem está aprendendo.

## Consequências

**Positivo:**
- **Correção**: `recv_from` bloqueante está isolado em sua própria thread — o idioma correto e natural
- **Simplicidade**: Duas threads com um canal são fáceis de raciocinar, testar e depurar
- **Zero de locking**: `mpsc` elimina a necessidade de `Mutex` no caminho crítico
- **Rust idiomático**: Transferência de propriedade de cada intent em tempo de compilação; desconexão do canal é automaticamente observável
- **Clareza pedagógica**: O fluxo de dados é explícito e linear — adequado para uma base de código orientada ao aprendizado

**Negativo:**
- **Instância única apenas**: Não generaliza para N instâncias sem mudanças arquiteturais
- **Nenhum caminho de saída**: O game loop não pode responder aos clientes sem trabalho de design adicional (Fase 3)
- **Fila ilimitada**: Sem proteção nativa contra flood de intents
- **Overhead de thread do SO**: Não adequado para grandes quantidades de instâncias (bloqueador da Fase 5)
