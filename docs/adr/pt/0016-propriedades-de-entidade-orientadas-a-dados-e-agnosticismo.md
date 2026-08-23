# ADR 0016: Propriedades de Entidade Orientadas a Dados e Agnosticismo do Motor

## Status

Aceito (Fase 6.5.0)

## Contexto

Como parte da Fase 6.5.0, a API de scripting do `loci2d` está sendo expandida para suportar regras de jogo concretas (por exemplo, determinar se uma entidade pertence a um "time" específico para implementar fogo amigo, ou checar a "vida" para aplicar dano).

A abordagem mais natural e fácil no Rust seria adicionar esses campos diretamente na struct `Entity` (`pub health: i32`, `pub team_id: u8`). Contudo, o `loci2d` foi projetado para ser um framework genérico de servidor 2D autoritativo, e não atrelado a um gênero específico (como um MOBA 3v3 ou um Battle Royale). Adicionar atributos específicos de um jogo ("hardcoding") no núcleo do motor em Rust polui a arquitetura, torna o motor menos flexível e borra a linha entre infraestrutura de engine e lógica de jogo.

Além disso, se a engine ditasse quais propriedades existem, os desenvolvedores precisariam modificar o código-fonte em Rust e recompilar o binário do servidor toda vez que quisessem adicionar uma nova mecânica (ex: "mana", "stamina", "armadura").

## Decisão

Adotaremos um **Design Orientado a Dados (Data-Driven)** estrito para atributos de entidades implementando um Sistema Genérico de Propriedades.

1. **Armazenamento Genérico:** A struct `Entity` no Rust possuirá apenas um mapa chave-valor flexível (ex: `properties: HashMap<String, String>` ou armazenamento genérico similar).
2. **Núcleo Agnóstico:** O núcleo em Rust deve permanecer completamente ignorante em relação às regras do jogo. Campos de gameplay fixos (como `health`, `team`, `mana`) são estritamente proibidos nas structs do núcleo em Rust.
3. **Domínio do Lua:** A camada de scripting em Lua é a única responsável por definir, modificar, consultar e interpretar o significado dessas propriedades via `Loci.get_entity_property` e `Loci.Commands.set_property`.
4. **Sincronização de Estado:** O mapa genérico de propriedades será serializado e transmitido para os clientes como parte do snapshot Protobuf `WorldState`, permitindo que os clientes renderizem visuais (como contornos de time vermelho/azul) com base nas propriedades dinâmicas.

## Consequências

**Positivas:**
- **Flexibilidade Absoluta:** Desenvolvedores podem construir qualquer tipo de jogo 2D (FFA, Team Deathmatch, PvE) apenas através de scripts Lua, sem nunca tocar no código Rust.
- **Fronteira Arquitetural Clara:** A engine (Rust) foca exclusivamente em performance, rede e física determinística. A lógica do jogo (Lua) foca inteiramente nas regras e interpretação do estado.
- **À Prova do Futuro:** Previne o inchaço estrutural na struct `Entity` conforme o framework evolui.

**Negativas:**
- **Sobrecarga de Serialização:** Serializar um mapa de hash dinâmico em Protobuf para transmissão em rede consome mais banda e ciclos de CPU se comparado a serializar campos estáticos e tipados fortemente.
- **Perda de Segurança de Tipagem (Type Safety):** Propriedades resgatadas no Lua ou no Rust são valores genéricos (ex: Strings ou enums básicos), significando que os desenvolvedores devem lidar com conversões e validações de tipo manualmente em seus scripts.
