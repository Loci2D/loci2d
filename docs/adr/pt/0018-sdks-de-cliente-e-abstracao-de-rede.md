# ADR 0018: SDKs de Cliente e Abstração de Rede

## Status
Aceito

## Contexto
A engine loci2d baseia-se em uma arquitetura estrita e autoritativa de servidor utilizando Protocol Buffers sobre UDP (ADR-0004, ADR-0005). Ela requer handshakes de conexão explícitos (Intents de Join/Disconnect - ADR-0008) e sincronização de estado determinística.
Enquanto nos preparamos para o MVP do 3v3 Arena (Fase 6), o público-alvo para construir o cliente do jogo inclui estudantes do ensino médio usando Love2D. Esperar que os desenvolvedores do jogo (especialmente estudantes) gerenciem manualmente loops não-bloqueantes de sockets UDP, serialização binária, interpolação de estado e gerenciamento de timeouts é uma barreira de entrada irrazoável.

## Decisão
Decidimos abstrair completamente a complexidade de rede da engine fornecendo **SDKs de Cliente** oficiais e ergonômicos para as engines suportadas (começando com Love2D via `loci_client.lua`).

Os SDKs de Cliente são responsáveis por:
1. **Encapsulamento de Rede e Protocolo:** Gerenciar o socket UDP subjacente, rastrear IDs de sequência e serializar/deserializar os payloads Protobuf internamente.
2. **Gerenciamento de Estado:** Manter uma representação local e canônica do `WorldState` enviado pelo servidor, incluindo a conversão automática de tipos das propriedades das entidades.
3. **Fachada de API de Alto Nível:** Expor uma interface simples e orientada a eventos para o desenvolvedor. O desenvolvedor interagirá apenas com o estado do jogo (ex: `loci.get_entities()`) e intents de alto nível (ex: `loci.send_move()`, `loci.send_action()`), completamente isolado do "netcode".
4. **Callbacks:** Disparar hooks específicos (ex: `on_entity_spawned`, `on_property_changed`) quando o servidor transmite mudanças de estado, permitindo que o cliente conecte facilmente a UI e os efeitos visuais.

## Consequências
- **Positivo:** Reduz drasticamente a barreira de entrada para a construção de clientes. Os desenvolvedores podem tratar o ambiente multiplayer quase como um jogo local single-player.
- **Positivo:** Padroniza o comportamento crítico de rede (ex: como desconexões, timeouts e pings são tratados) em todos os futuros clientes (Love2D, Godot, Python).
- **Negativo:** Introduz um fardo de manutenção significativo. Qualquer mudança no protocolo do servidor ou mecânicas centrais agora exige atualizações síncronas em vários SDKs de linguagens específicas.
