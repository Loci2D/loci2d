# ADR 0003: 2D Map Only Architecture
 
## Status
 
Accepted
 
## Context / Contexto
 
### English
Full 3D game servers introduce significant complexity:
- **3D Spatial Calculations**: Complex collision detection, raycasting, volumetric calculations
- **Navigation**: 3D pathfinding is computationally expensive and difficult to implement
- **Physics**: 3D physics engines add substantial complexity and performance overhead
- **Network Bandwidth**: 3D positions and rotations require more data to transmit
- **Learning Curve**: 3D math (quaternions, matrices) is harder for beginners
 
Many successful games use 2D gameplay even with 3D visuals (MOBAs, strategy games, isometric RPGs).
 
### Português
Servidores de jogos 3D completos introduzem complexidade significativa:
- **Cálculos Espaciais 3D**: Detecção de colisão complexa, raycasting, cálculos volumétricos
- **Navegação**: Pathfinding 3D é computacionalmente caro e difícil de implementar
- **Física**: Motores de física 3D adicionam complexidade substancial e overhead de performance
- **Banda de Rede**: Posições e rotações 3D requerem mais dados para transmitir
- **Curva de Aprendizado**: Matemática 3D (quaternions, matrizes) é mais difícil para iniciantes
 
Muitos jogos de sucesso usam jogabilidade 2D mesmo com visuais 3D (MOBAs, jogos de estratégia, RPGs isométricos).
 
## Decision / Decisão
 
### English
We choose to support **2D maps only** where:
- All game logic operates on a 2D plane (X, Y coordinates)
- Vector2 is the primary position type throughout the codebase
- Games can still have 3D visuals/rendering on the client side
- Server never deals with Z-axis, height, or 3D rotations
- Simplified collision detection using 2D bounding boxes/circles
- 2D pathfinding algorithms (A*, Dijkstra) are sufficient
 
This supports game types like:
- **MOBA games**: 3D characters on 2D map
- **Strategy games**: Top-down or isometric views
- **2D platformers**: Side-scrolling gameplay
- **Isometric RPGs**: 2D logic with 3D presentation
 
### Português
Escolhemos suportar apenas **mapas 2D** onde:
- Toda a lógica do jogo opera em um plano 2D (coordenadas X, Y)
- Vector2 é o tipo de posição primário em toda a codebase
- Jogos ainda podem ter visuais/renderização 3D no lado do cliente
- Servidor nunca lida com eixo Z, altura ou rotações 3D
- Detecção de colisão simplificada usando bounding boxes/círculos 2D
- Algoritmos de pathfinding 2D (A*, Dijkstra) são suficientes
 
Isso suporta tipos de jogos como:
- **Jogos MOBA**: Personagens 3D em mapa 2D
- **Jogos de Estratégia**: Visões top-down ou isométrica
- **Platformers 2D**: Jogabilidade side-scrolling
- **RPGs Isométricos**: Lógica 2D com apresentação 3D
 
## Consequences / Consequências
 
### English
 
**Positive:**
- **Reduced Complexity**: 2D math is simpler and easier to debug
- **Better Performance**: 2D calculations are faster than 3D
- **Lower Bandwidth**: 2D positions require less network data
- **Easier Learning**: Beginners can focus on game logic instead of 3D math
- **Simpler Pathfinding**: 2D pathfinding is well-understood and efficient
- **Faster Development**: Less time spent on 3D spatial problems
- **Genre Coverage**: Still supports many popular game genres
 
**Negative:**
- **Limited Gameplay**: Can't support true 3D gameplay (flight, underwater, multi-level)
- **Visual Restrictions**: 3D visuals must conform to 2D gameplay constraints
- **Height Limitations**: No true vertical gameplay elements
- **Camera Constraints**: Camera angles limited by 2D gameplay
 
### Português
 
**Positivo:**
- **Complexidade Reduzida**: Matemática 2D é mais simples e fácil de debugar
- **Melhor Performance**: Cálculos 2D são mais rápidos que 3D
- **Menor Banda**: Posições 2D requerem menos dados de rede
- **Aprendizado Mais Fácil**: Iniciantes podem focar na lógica do jogo em vez de matemática 3D
- **Pathfinding Simplificado**: Pathfinding 2D é bem entendido e eficiente
- **Desenvolvimento Mais Rápido**: Menos tempo gasto em problemas espaciais 3D
- **Cobertura de Gêneros**: Ainda suporta muitos gêneros de jogos populares
 
**Negativo:**
- **Jogabilidade Limitada**: Não suporta jogabilidade 3D verdadeira (voo, subaquático, multi-nível)
- **Restrições Visuais**: Visuais 3D devem conformar às restrições de jogabilidade 2D
- **Limitações de Altura**: Sem elementos de jogabilidade vertical verdadeiros
- **Restrições de Câmera**: Ângulos de câmera limitados pela jogabilidade 2D