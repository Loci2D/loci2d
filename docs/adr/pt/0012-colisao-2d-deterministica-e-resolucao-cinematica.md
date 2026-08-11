# ADR 0012: Motor de Colisão 2D Determinístico e Estratégia de Resolução Cinemática

## Status

Aceito (Fase 5)

## Contexto

Para que o `loci2d` suporte interações significativas de jogabilidade (paredes, obstáculos estáticos, perímetros de mapa, colisões entre entidades, zonas de sensores), a engine necessita de um subsistema de detecção e resolução de colisões espaciais.

Práticas convencionais de desenvolvimento de jogos frequentemente utilizam bibliotecas externas de física (como Rapier2D, Box2D ou PhysX). No entanto, incorporar essas engines de física externas ao `loci2d` cria conflitos arquiteturais significativos:

1. **Não-Determinismo de Ponto Flutuante ([ADR-0007](0007-simulacao-deterministica-e-ponto-fixo.md))**:
   - A maioria das bibliotecas de física depende de operações de ponto flutuante IEEE 754 (`f32`/`f64`), otimizações SIMD (AVX, SSE, NEON) e auto-vetorização de compiladores.
   - Ao longo de partidas com milhares de ticks, pequenas variações de arredondamento entre CPUs (x86_64 vs ARM64) causam divergência cumulativa, quebrando a reprodutibilidade exata de replays `.loci` ([ADR-0010](0010-formato-replay-event-sourcing.md)).

2. **Sobrecarga de Simulação de Corpos Rígidos ([ADR-0003](0003-arquitetura-apenas-mapas-2d.md))**:
   - Engines de física de corpos rígidos resolvem sistemas complexos de restrições: momentos de inércia de massa, velocidade angular, torque, impulsos de atrito e detecção contínua de colisão (CCD).
   - Para jogos 2D top-down, MOBA, RTS e RPGs de ação isométrica, a física baseada em momento de corpo rígido costuma parecer imprecisa, "escorregadia" e difícil de calibrar. Os jogadores esperam movimentação cinemática nítida, imediata e responsiva.

3. **Tamanho de Dependências e Complexidade ([ADR-0001](0001-arquitetura-servidor-autoritativo.md))**:
   - Dependências pesadas de física aumentam consideravelmente os tempos de compilação, o tamanho do binário e a barreira de entrada para desenvolvedores indie.

## Decisão

Decidimos implementar um **motor de colisão 2D leve, integrado e 100% determinístico em ponto fixo**, com **resolução cinemática por Vetor de Translação Mínima (MTV)**:

### 1. Primitivas Geométricas de Ponto Fixo (`I16F16`)
A detecção de colisão é restrita a formas geométricas 2D implementadas inteiramente com aritmética de ponto fixo `I16F16`:
- **`DeterministicAABB`**: Caixa Delimitadora Alinhada aos Eixos para paredes, obstáculos, áreas retangulares de gatilho e consultas espaciais.
- **`DeterministicCircle`**: Colisor circular para jogadores, NPCs, projéteis e gatilhos radiais.

### 2. Raiz Quadrada Inteira Determinística (`fixed_sqrt`)
Cálculos de distância circular e normalização evitam o uso do `f32::sqrt` padrão. Em vez disso, utilizamos um algoritmo de raiz quadrada inteira bitwise operando diretamente sobre os bits brutos de `I16F16`, garantindo resultados idênticos em todas as arquiteturas de CPU e plataformas.

### 3. Empurrão Cinemático por MTV & Deslizamento em Paredes
Em vez de simular forças, impulsos de massa ou restituição:
- Colisões sólidas calculam o **Vetor de Translação Mínima (MTV)** e o vetor normal $\vec{n}$.
- A entidade penetrante é empurrada para fora ao longo de $\vec{n}$ pela profundidade de penetração.
- A velocidade da entidade é projetada ao longo da tangente da superfície ($\vec{v}_{\text{slide}} = \vec{v} - (\vec{v} \cdot \vec{n})\vec{n}$), evitando que o jogador fique preso em cantos e produzindo um deslizamento suave e responsivo contra paredes.

### 4. Volumes de Gatilho / Sensores Não-Sólidos
Zonas de gatilho usam as mesmas primitivas de colisão em ponto fixo, mas não aplicam empurrão físico. Elas mantêm o rastreamento determinístico de entidades sobrepostas e disparam eventos de ciclo de vida `Enter`, `Stay` e `Exit` (que alimentarão diretamente os hooks de scripting em Lua na Fase 6).

### 5. Fase Ampla (Broadphase) Espacial Determinística
Para cenários com maior contagem de entidades, a poda de pares candidatos na fase ampla utiliza um spatial hash grid uniforme. Para garantir o determinismo, os pares candidatos são desduplicados e ordenados usando coleções estritamente ordenadas (`BTreeSet<(u64, u64)>`).

## Consequências

**Positivas:**
- **Determinismo 100% Exato Bit-a-Bit**: Preserva a igualdade de hash SHA-256 de estado entre plataformas para replays de partidas (`.loci`).
- **Controles Cinemáticos Responsivos**: Movimentação precisa e previsível, ideal para modelos de controle top-down, MOBA e RTS.
- **Zero Dependências Externas de Física**: Mantém o loci2d leve, rápido de compilar e fácil de manter.
- **Integração Direta com Scripting**: Fornece ganchos de eventos de colisão e gatilho claros para scripts Lua na Fase 6.

**Negativas:**
- **Sem Dinâmica Complexa de Corpos Rígidos**: Não suporta física de juntas/articulações, inércia rotacional, empilhamento físico ou ragdolls (desnecessários para jogabilidade autoritativa 2D top-down).
- **Simplificação Geométrica**: Restrito a AABBs e Círculos (polígonos rotacionados arbitrários exigiriam decomposição em caixas/círculos ou adição do Teorema dos Eixos Separadores - SAT em revisões futuras, se necessário).
