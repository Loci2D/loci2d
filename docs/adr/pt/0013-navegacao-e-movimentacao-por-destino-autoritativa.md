# ADR 0013: Navegação e Movimentação por Destino Autoritativa (Click-to-Move)

## Status

Aceito (Fase 5)

## Contexto

Além de comandos direcionais contínuos (`MoveIntent`, típicos em esquemas de controle WASD ou direcionais de gamepad), diversos gêneros de jogos multiplayer 2D (como MOBAs, jogos de Estratégia em Tempo Real - RTS e RPGs de ação no estilo point-and-click) dependem fortemente de movimentação por clique no destino:
- O jogador clica em uma posição no plano 2D.
- O cliente transmite um único pacote de destino (`MoveToPositionIntent`).
- A entidade se move autonomamente em direção ao ponto de destino na velocidade configurada, parando perfeitamente ao atingir o alvo.

A implementação dessa navegação apresenta duas abordagens arquiteturais principais:

| Abordagem | Onde a Lógica Executa | Tráfego de Rede | Vulnerabilidade / Determinismo |
|---|---|---|---|
| **Simulação de Navegação no Cliente** | O cliente simula a navegação e envia um fluxo constante de vetores direcionais `MoveIntent` | **Alto** (O cliente precisa enviar pacotes contínuos a cada tick) | Propenso a jitter de latência, dessincronização de caminho e adulteração de posição no cliente |
| **Navegação Autoritativa no Servidor** | O cliente envia um único `MoveToPositionIntent(x, y)`; o Servidor calcula a navegação a cada tick | **Extremamente Baixo** (Um único pacote por clique) | 100% Autoritativo, à prova de trapaças e totalmente determinístico em replays de partidas |

## Decisão

Decidimos implementar **Navegação e Movimentação por Destino Autoritativa no Servidor com Suporte a Waypoints**:

### 1. Componente de Navegação no Servidor em Entidades
Entidades com suporte a movimentação por destino recebem um `NavigationComponent` opcional:
- `target: Option<DeterministicVector2>`
- `arrival_tolerance: I16F16` (limiar de distância para parada)
- `move_speed: I16F16` (velocidade em ponto fixo por tick)
- `waypoints: VecDeque<DeterministicVector2>` (fila ordenada de pontos de navegação / waypoints)

### 2. Loop de Navegação Determinístico em Ponto Fixo
Durante o `Instance::tick()`:
1. Se houver um alvo ativo, o servidor calcula o deslocamento $\vec{d} = \text{target} - \text{position}$.
2. A distância é calculada usando matemática inteira de ponto fixo: $D = \text{fixed\_sqrt}(d_x^2 + d_y^2)$.
3. **Checagem de Tolerância de Chegada**:
   - Se $D \le \text{arrival\_tolerance}$:
     - Se a fila de `waypoints` possuir nós restantes, o servidor extrai o próximo waypoint como o novo `target` ativo.
     - Caso contrário, a velocidade da entidade é zerada e o `target` é limpo.
   - Se $D > \text{arrival\_tolerance}$:
     - A velocidade é definida na direção unitária: $\vec{v} = (\vec{d} / D) \times \text{move\_speed}$.

### 3. Prevenção de Jitter & Limiar de Chegada
Sem uma tolerância de chegada ($\epsilon$), a integração em passos discretos de tempo pode ultrapassar o destino em um tick e inverter a direção no seguinte, causando oscilação visual e jitter ao redor do ponto final. Definir uma tolerância não-nula proporcional à velocidade garante uma parada suave e precisa em um único tick.

### 4. Preempção Determinística de Intenções
Quando um cliente emite uma nova intenção:
- Um novo `MoveToPositionIntent` substitui imediatamente o alvo ativo e atualiza a fila de waypoints.
- Um `MoveIntent` direto (ex: entrada WASD) cancela imediatamente qualquer navegação por destino ativa e assume controle direto de velocidade.
- Uma parada explícita ou desconexão reseta imediatamente o estado de navegação.

## Consequências

**Positivas:**
- **Uso Mínimo de Banda**: Um único pacote desencadeia a trajetória completa em direção ao destino.
- **Autoritativo & Seguro Contra Trapaças**: Impede trapaças de velocidade (speed hack) ou atravessamento indevido de obstáculos durante a navegação.
- **Determinismo 100% em Replays**: Trajetórias de clique são registradas como eventos discretos em arquivos de replay `.loci` e reproduzem caminhos idênticos em todas as plataformas.
- **Suporte a Múltiplos Waypoints**: Permite enfileiramento de waypoints (movimento com Shift-Clique) para mecânicas de RTS/MOBA.

**Negativas:**
- **Carga de CPU no Servidor**: O servidor executa cálculos de vetor de direção a cada tick para entidades em movimento (custo insignificante com aritmética inteira de ponto fixo).
- **Percepção de Latência**: Sob alta latência de rede, os jogadores perceberão um atraso de ida e volta antes de o personagem iniciar o movimento (indicadores visuais como marcadores de clique podem ser renderizados instantaneamente no cliente sem comprometer a autoridade do servidor).
