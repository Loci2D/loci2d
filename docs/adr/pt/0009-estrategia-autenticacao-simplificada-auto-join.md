# ADR 0009: Estratégia de Autenticação Simplificada e Handshake Explícito de Sessão

## Status

Aceito — **Temporário (Fase 2–4)**  
Previsto para ser substituído na **Fase 5** pela implementação completa de autenticação e segurança.

## Contexto

Para um servidor de jogos multiplayer de produção, autenticação tipicamente envolve:
- Validação de token OAuth/JWT
- Contas de usuário com suporte de banco de dados
- Gerenciamento de tokens de sessão
- Assinatura criptográfica de pacotes
- Medidas anti-cheat e anti-tampering

No entanto, durante a fase de validação (Fase 2–4), as prioridades do projeto são:
1. **Baixa barreira de entrada**: Estudantes e desenvolvedores indies devem executar o servidor localmente sem dependências externas
2. **Prototipagem rápida**: Focar no game loop e mecânicas de sessão, não em infraestrutura de autenticação
3. **Ambiente confiável**: Deploy inicial tem como alvo localhost e salas de jogos LAN privadas
4. **Validação de arquitetura**: Provar que o modelo de servidor autoritativo funciona antes de adicionar camadas de segurança

Introduzir autenticação completa na Fase 2 iria:
- Requerer configuração de banco de dados (PostgreSQL, Redis, etc.)
- Necessitar integração de provedor OAuth ou implementação JWT customizada
- Adicionar complexidade significativa à base de código
- Aumentar tempo de onboarding para novos contribuidores
- Distrair da validação de rede e simulação principais

Alternativas consideradas:

| Abordagem | Por que foi rejeitada |
|---|---|
| OAuth/JWT completo na Fase 2 | Complexo demais para fase de validação; bloqueia prototipagem rápida |
| Contas de usuário com banco de dados | Requer infraestrutura externa; viola simplicidade de instância única |
| Chaves/senhas pré-compartilhadas | Ainda requer infraestrutura de distribuição de chaves; UX ruim |
| Auto-join implícito em intent cru | Burlar limites de sessão; cria entidades fantasmas em pacotes aleatórios |

## Decisão

Adotamos um **modelo de autenticação baseado em confiança mínima com handshake explícito de sessão** otimizado para prototipagem rápida:

### Mecanismo de Identidade
- **Identificador primário**: `SocketAddr` (IP:porta do cliente)
- **Identidade de exibição**: Nome legível `player_name` fornecido via `JoinIntent`
- **Sem tokens criptográficos**: Confia no endereço de rede para vinculação de sessão
- **Sem banco de dados**: Todo estado de sessão está em memória dentro da `Instance`

### Protocolo de Handshake Explícito
- **Comportamento**: Um cliente DEVE enviar um `JoinIntent` explícito contendo seu `player_name` para registrar uma sessão e criar uma entidade na instância.
- **Política para Intents sem Sessão**: Se um `SocketAddr` desconhecido envia intents de gameplay (`MoveIntent`, `ActionIntent`, `PingIntent`) sem `JoinIntent` prévio, o pacote é descartado e ignorado.
- **Saída Graciosa**: Um cliente pode enviar `DisconnectIntent` com uma razão opcional para terminar sua sessão e destruir sua entidade de forma limpa.

### Trade-offs de Segurança (Explicitamente Aceitos)
- **Falsificação de pacotes**: Clientes maliciosos poderiam impersonar qualquer IP em ambientes LAN
  - *Mitigação*: Fase 2–4 tem como alvo redes privadas confiáveis; segurança de produção adiada para Fase 5
- **Sem proteção contra replay**: Pacotes poderiam ser capturados e retransmitidos
  - *Mitigação*: IDs de sequência existem no protobuf mas não são validados; validação adiada para Fase 5
- **Sem criptografia**: Pacotes UDP são enviados em texto plano
  - *Mitigação*: Aceitável para desenvolvimento localhost; DTLS/TLS considerado para Fase 5
- **Identidade baseada em IP**: NAT traversal, redes móveis ou mudanças de IP quebram sessões
  - *Mitigação*: Aceitável para LAN/localhost estável; identidade baseada em token adiada para Fase 5

### Extensões de Protocolo
- **`JoinIntent`**: Handshake explícito com campo `player_name`
- **`DisconnectIntent`**: Terminação graciosa de sessão com razão opcional

## Por Que Isso É Temporário

Esta abordagem é explicitamente um **compromisso de fase de validação** com lacunas de segurança conhecidas:

1. **Sem autenticação**: Qualquer um que consiga alcançar a porta UDP pode entrar
2. **Sem autorização**: Todos os jogadores têm permissões iguais; sem sistema de admin/papel
3. **Sem criptografia**: Tráfego de rede é visível para sniffers de pacotes
4. **Sem integridade**: Pacotes podem ser modificados em trânsito sem detecção
5. **Sem proteção contra replay**: Pacotes capturados podem ser retransmitidos

Isso é aceitável para:
- Desenvolvimento localhost (`127.0.0.1`)
- Salas de jogos LAN privadas com participantes confiáveis
- Ambientes educacionais onde segurança não é o objetivo de aprendizado
- Validação de prova de conceito antes de investimento em produção

## Caminho de Migração (Fase 5)

A Fase 5 introduzirá autenticação e segurança completas:
- Substituir identidade `SocketAddr` por tokens JWT ou IDs de sessão
- Adicionar servidor de autenticação (OAuth 2.0 ou implementação customizada)
- Implementar assinatura de pacotes (HMAC ou similar) para integridade
- Adicionar criptografia (DTLS ou criptografia em camada de aplicação)
- Implementar proteção contra ataques de replay (validação nonce/timestamp)
- Adicionar autorização baseada em papéis (admin, jogador, espectador)
- Integrar com banco de dados para contas de usuário persistentes

## Consequências

**Positivo:**
- **Zero dependências externas**: Sem banco de dados, servidor de autenticação ou serviços de terceiros necessários
- **Onboarding instantâneo**: Novos contribuidores podem executar o servidor imediatamente após `cargo run`
- **Iteração rápida**: Focar em mecânicas de jogo, não infraestrutura de autenticação
- **Debug simples**: Sem expiração de token, migrações de banco de dados ou fluxos de autenticação para depurar
- **Clareza educacional**: Mecânicas de sessão são visíveis e compreensíveis sem complexidade criptográfica
- **Limites limpos de sessão**: Exigência de join explícito garante que apenas conexões intencionais gerem entidades

**Negativo:**
- **Sem segurança**: Completamente inadequado para deploy em internet pública
- **Vulnerabilidade de falsificação**: Atores maliciosos podem impersonar clientes em ambientes LAN
- **Sem identidade persistente**: Mudanças de IP quebram sessões (redes móveis, DHCP)
- **Sem trilha de auditoria**: Sem logging de qual usuário humano está por trás de qual conexão
- **Assunção de modelo de confiança**: Requer ambiente de rede confiável (localhost/LAN privado)
- **Bloqueador de produção**: Deve ser substituído antes de qualquer deploy público
