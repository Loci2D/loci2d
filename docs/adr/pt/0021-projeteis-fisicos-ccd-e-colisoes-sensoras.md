# ADR 0021: Projéteis Físicos, Detecção Contínua de Colisão (CCD) por Varredura e Estratégia de Resolução Sensora

## Status

Aceito

## Contexto

No [ADR-0012](0012-colisao-2d-deterministica-e-resolucao-cinematica.md), o `loci2d` estabeleceu um motor de colisão 2D determinístico em ponto fixo (`I16F16`) com suporte a colisores circulares e AABB, deslizamento contra obstáculos estáticos (100% de pushback), resolução dinâmica entidade-vs-entidade (pushback dividido 50/50) e raycasting sob demanda (`fixed_raycast`).

Embora essa base atenda à movimentação de personagens, combate corpo a corpo e consultas instantâneas (hitscan), jogos de ação em tempo real (como o `loci-arena`) exigem **projéteis físicos persistentes ao longo de múltiplos ticks** (ex.: skillshots, lâminas arremessadas, bolas de fogo, flechas com ricochete). Uma auditoria na engine e no SDK de cliente revelou cinco lacunas arquiteturais ao representar projéteis como entidades dinâmicas comuns:

1. **Ausência de Tipo Nativo de Entidade `Projectile`**:
   - `Command::SpawnEntity` reconhece apenas `"Player"` e `"Enemy" | "NPC"`, caindo em fallback para `EntityType::Prop` com `CollisionFilter::default_player()` e anexando um `NavigationComponent` desnecessário.
   - `CollisionFilter::default_projectile()` existe no Rust (`src/world/physics/map.rs`), mas não é utilizado na criação de entidades, e o `game_packets.proto` não possui `EntityType::PROJECTILE` para identificação nativa no cliente.

2. **Empurrão Cinemático (Pushback 50/50) Indesejado no Impacto**:
   - Em `simulation::tick`, `resolve_dynamic_collision` aplica incondicionalmente um empurrão posicional 50/50 pelo Vetor de Translação Mínima (MTV) e projeção de velocidade em ambas as entidades antes de invocar o callback Lua `on_collision(id_a, id_b)`.
   - Como resultado, um projétil que atinge um jogador empurra fisicamente o alvo para trás antes da execução do script, violando a semântica esperada de impacto de projéteis e conflitando com regras de knockback controladas pelo jogo ([ADR-0016](0016-propriedades-de-entidade-orientadas-a-dados-e-agnosticismo.md)).

3. **Impactos Silenciosos contra Obstáculos Estáticos e Bordas da Arena**:
   - Colisões contra instâncias sólidas de `StaticObstacle` e `MapBounds` apenas deslizam ou limitam (clamp) a posição da entidade silenciosamente no Rust, sem notificar a camada de scripting Lua.
   - Os scripts não possuem callback determinístico nem o vetor normal da superfície $\vec{n}$ para destruir o projétil ao atingir uma parede, acionar uma explosão em área no ponto de impacto ou calcular vetores de reflexão para ricochetes.

4. **Tunneling Discreto em Alta Velocidade**:
   - A integração discreta padrão ($\vec{p}_{t+1} = \vec{p}_t + \vec{v}$) avalia interseção apenas na posição final candidata $\vec{p}_{t+1}$.
   - Projéteis velozes que se deslocam mais que seu diâmetro em um único tick ($\|\vec{v}\| > 2r$) podem atravessar completamente paredes finas ou colisores de jogadores entre um tick e outro (tunneling).

5. **Largura de Banda de Rede vs. Ciclo de Vida Curto no Cliente**:
   - Transmitir eventos brutos de colisão por tick via UDP inflaria os pacotes `ServerPacket` e violaria a arquitetura orientada a snapshots ([ADR-0004](0004-formato-serializacao-pacotes.md), [ADR-0018](0018-sdks-de-cliente-e-abstracao-de-rede.md)).
   - Ao mesmo tempo, projéteis de vida curta (criados e destruídos em poucos ticks) sofrem um congelamento inicial de 50ms no interpolador de snapshots do SDK cliente enquanto aguardam um segundo snapshot no buffer, e `on_entity_despawned(id)` descarta a tabela da entidade antes que callbacks de VFX/SFX no cliente possam ler suas coordenadas finais de impacto.

## Decisão

Decidimos estender a arquitetura de física e scripting do `loci2d` — e alinhar o runtime do cliente `sdk-love2d` — com **projéteis físicos de primeira classe, resolução sensora dinâmica sem pushback, Detecção Contínua de Colisão (CCD) por Soma de Minkowski e callbacks determinísticos de impacto com vetor normal**:

### 1. `EntityType::Projectile` de Primeira Classe e Ciclo de Vida Nativo (`lifetime_ticks`)
- Promover `Projectile` (`EntityType::Projectile` no Rust, `PROJECTILE = 3` em `game_packets.proto`) a uma variante nativa de entidade.
- Ao ser criado via `Loci.Commands.spawn_entity({ entity_type = "Projectile", ... })`:
  - A entidade assume por padrão `CollisionFilter::default_projectile()` (`layer = PROJECTILE`, `mask = SOLID_WALL | PLAYER`), `is_sensor = true` e `navigation = None`.
  - Suportar parâmetros opcionais `velocity = { x, y }` e `lifetime_ticks = N` diretamente no `spawn_entity` (e via `Loci.Commands.set_lifetime`).
  - Entidades com `remaining_lifetime_ticks: Some(n)` têm seu contador decrementado deterministicamente no início de cada tick em execução e são automaticamente removidas ao atingir `0`, evitando projéteis órfãos sem exigir timers manuais em Lua.

### 2. Modo Sensor Dinâmico (`is_sensor`) Sem Pushback 50/50
- Adicionar `pub is_sensor: bool` em `Entity` (padrão `true` para `Projectile`, `false` para `Player`, `NPC`, `Prop`; mutável via `Loci.Commands.set_sensor(entity_id, is_sensor)`).
- Durante a resolução narrowphase dinâmica entre duas entidades:
  - Se qualquer uma das entidades do par colidente possuir `is_sensor == true`, o motor **ignora `resolve_dynamic_collision` (zero pushback MTV e zero alteração de velocidade no alvo)**, mas continua invocando `on_collision(entity_a, entity_b)` em ordem estrita crescente de ID.
  - As regras de gameplay permanecem 100% sob domínio do Lua ([ADR-0016](0016-propriedades-de-entidade-orientadas-a-dados-e-agnosticismo.md)): o script decide se destrói o projétil, atravessa o alvo (piercing) ou aplica knockback explícito.

### 3. Callbacks de Impacto contra Obstáculos Estáticos e Bordas com Vetor Normal
- Expor dois novos callbacks opcionais de ciclo de vida Lua no `ScriptEngine`:
  - `on_static_collision(entity_id, obstacle_id, normal_x, normal_y)`
  - `on_boundary_collision(entity_id, normal_x, normal_y)`
- Sempre que uma entidade colidir com um `StaticObstacle` sólido (avaliado em ordem crescente de `obstacle.id`) ou sofrer clamp pelos limites `MapBounds`, o motor passa a normal unitária de contato $\vec{n} = (\text{normal\_x}, \text{normal\_y})$ para o Lua.
- Para entidades sensoras (`is_sensor == true`), a posição é resolvida até a superfície exata de contato sem projetar/zerar sua velocidade antes do callback, permitindo que o script Lua chame `destroy_entity(entity_id)` ou calcule reflexões determinísticas de ricochete ($\vec{v}' = \vec{v} - 2(\vec{v} \cdot \vec{n})\vec{n}$).

### 4. CCD Determinístico por Círculo Varredor (Raycasting com Soma de Minkowski)
- Para impedir tunneling sem introduzir ponto flutuante nem custo de sub-stepping iterativo (preservando o deslizamento discreto MTV em paredes para entidades `Player`, `NPC` e `Prop`):
  - Durante a integração de velocidade em `simulation::tick`, entidades do tipo `EntityType::Projectile` executam um **teste de círculo varredor (swept circle)** ao longo do segmento $\vec{p}_0 \to \vec{p}_0 + \vec{v}$ (ignorando contatos tangenciais ou de afastamento em $t = 0$ para permitir ricochetes livres).
  - A varredura de um círculo de raio $r_p$ contra formas alvo é calculada via interseção de raio 100% determinística em `I16F16` / inteiros de 64 bits (`ray_intersect_aabb` e `ray_intersect_circle`) contra as **formas expandidas por Soma de Minkowski**:
    - Um alvo `DeterministicCircle(c, r_t)` é expandido para o raio $R = r_p + r_t$.
    - Um alvo `DeterministicAABB(min, max)` é expandido por $r_p$ em cada eixo (`min - r_p`, `max + r_p`).
  - Interseções com `StaticObstacle` sólidos limitam o segmento de deslocamento ao menor tempo de impacto $t_{\min}$ (desempate determinístico por `obstacle.id` crescente) e disparam `on_static_collision`; o segmento já limitado por paredes é então varrido contra entidades dinâmicas sem prender o projétil em aliados ou alvos perfuráveis, disparando `on_collision` para todos os alvos não ocluídos no trajeto.

### 5. Replicação Orientada a Snapshots e Ergonomia no SDK (Sem Pacotes Brutos de Colisão)
- Rejeitamos explicitamente a adição de um campo `repeated CollisionEvent` no `ServerPacket` / `WorldState`. Arrays de colisão por tick geram spam de rede e duplicam transições de estado já observáveis via ciclo de vida e propriedades.
- Em vez disso, no `Loci2D/sdk-love2d`:
  - Mapear `EntityType::PROJECTILE` (`3`) para `entity.entity_type = "projectile"` e expor `Entity:is_projectile()` e `loci.get_projectiles()`.
  - Eliminar o congelamento inicial de 50ms para projéteis recém-criados em `loci/netcode/interpolation.lua`, integrando a posição a partir da velocidade autoritativa `(server_vx, server_vy)` quando a entidade existir apenas no snapshot mais recente da janela de interpolação.
  - Passar a tabela completa `entity` como segundo argumento em `loci.on_entity_despawned(entity_id, entity)` (com `entity.x, entity.y` ajustados para as coordenadas autoritativas finais `server_x, server_y`), permitindo ao cliente instanciar VFX/SFX de impacto no ponto exato da destruição e mantendo 100% de retrocompatibilidade com handlers `function(id)` existentes.

### 6. Hashing Canônico de Replay Retrocompatível
- Para preservar a verificação bit-a-bit de replays `.loci` existentes (como `benchmark.loci`, [ADR-0010](0010-formato-replay-event-sourcing.md)), `compute_canonical_state_hash` em `src/replay/hash.rs` só acrescenta bytes prefixados por tag para `is_sensor` (`0x5E`) quando `entity.is_sensor == true` e `remaining_lifetime_ticks` (`0x4C` + `u32` big-endian) quando `Some(_)`. Entidades `Player` padrão produzem digests SHA-256 idênticos aos anteriores.

## Consequências

**Positivas:**
- **Determinismo Bit-a-Bit 100% Preservado**: O raycasting varredor por Minkowski reutiliza a matemática de raiz quadrada inteira de 64 bits (`integer_sqrt_u64`), garantindo trajetórias e ticks de impacto idênticos entre x86_64 e ARM64.
- **Combate Preciso e Responsivo**: Projéteis não empurram jogadores com pushback rígido 50/50, nunca sofrem tunneling através de paredes ou alvos em alta velocidade e acionam de forma limpa lógica de impacto ou ricochete em obstáculos e bordas da arena.
- **Agnosticismo da Engine Mantido ([ADR-0016](0016-propriedades-de-entidade-orientadas-a-dados-e-agnosticismo.md))**: Nenhuma regra de combate (dano, stun, perfuração, limite de ricochetes) fica hardcoded no Rust; os scripts Lua compõem todos os comportamentos via callbacks e propriedades.
- **Zero Inflação no Protocolo**: Replica projéteis de forma limpa através dos snapshots `EntityState` existentes e callbacks de ciclo de vida, sem spam de pacotes de colisão por tick.

**Negativas:**
- **Custo Leve de CPU por Tick para Entidades de Alta Velocidade**: O raycasting de Minkowski avalia interseções de segmento contra obstáculos e entidades colidíveis para projéteis ativos, embora limitado a $O(N_{\text{projéteis}} \times (N_{\text{obstáculos}} + N_{\text{entidades}}))$ com filtragem rápida por bitmask.
