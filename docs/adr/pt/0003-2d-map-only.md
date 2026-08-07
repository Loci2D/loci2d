# ADR 0003: Arquitetura de Apenas Mapas 2D
 
## Status
 
Aceito
 
## Contexto
 
Servidores de jogos 3D completos introduzem complexidade significativa:
- **Cálculos Espaciais 3D**: Detecção de colisão complexa, raycasting, cálculos volumétricos
- **Navegação**: Pathfinding 3D é computacionalmente caro e difícil de implementar
- **Física**: Motores de física 3D adicionam complexidade substancial e overhead de performance
- **Banda de Rede**: Posições e rotações 3D requerem mais dados para transmitir
- **Curva de Aprendizado**: Matemática 3D (quaternions, matrizes) é mais difícil para iniciantes
 
Muitos jogos de sucesso usam jogabilidade 2D mesmo com visuais 3D (MOBAs, jogos de estratégia, RPGs isométricos).
 
## Decisão
 
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
 
## Consequências
 
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
