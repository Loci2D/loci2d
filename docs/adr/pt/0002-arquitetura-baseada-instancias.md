# ADR 0002: Arquitetura Baseada em Instâncias

## Status

Aceito

## Contexto

Desenvolvedores indie e estudantes frequentemente lutam com arquiteturas de rede complexas ao construir jogos multiplayer. Abordagens comuns incluem:
- **Servidor de Mundo Único**: Todos os jogadores em um mundo compartilhado (estilo MMORPG)
- **Baseado em Lobby**: Jogadores entram em sessões temporárias
- **Peer-to-Peer**: Problemas complexos de NAT traversal e segurança
- **Particionamento Espacial**: Complexo de implementar e debugar

Essas abordagens frequentemente introduzem complexidade significativa que distrai do desenvolvimento core do jogo.

## Decisão

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

## Consequências

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
- **Persistência de Estado**: Requer sistemaeparado se estado global é necessário
