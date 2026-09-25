# Loci2D — Love2D Client SDK

> 🌐 *Read this in [English](README.md)*

O **Loci2D Love2D SDK** (`loci_client.lua`) é uma camada de abstração em alto nível projetada para permitir que desenvolvedores e estudantes construam jogos multiplayer no framework [LÖVE2D](https://love2d.org/) de forma extremamente simples, sem necessidade de lidar diretamente com sockets UDP, buffers binários de Protocol Buffers ou sincronização de ticks.

---

## Estrutura do SDK

```text
sdks/love2d/
├── loci_client.lua       # Módulo principal do cliente (API Facade, State Manager e Netcode)
├── test_sdk.lua          # Suíte de testes unitários para validar o SDK localmente
├── AI_REFERENCE.md       # Referência compacta de API formatada para LLMs/Agentes de IA
├── lib/
│   ├── game_packets.proto # Definição dos pacotes Protobuf do Loci2D
│   ├── game_packets.pb    # Descritor binário pré-compilado (carregamento ultra-rápido)
│   └── protoc.lua         # Parser Protobuf puro em Lua (fallback para parsing dinâmico)
└── docs/
    ├── index.html         # Guia visual interativo para desenvolvimento no Love2D
    └── server_scripts.html# Guia didático para criação de regras de jogo no servidor
```

---

## Como Funciona a Carga de Protocolo

O `loci_client.lua` foi desenvolvido com tolerância máxima a falhas de ambiente:
1. **Parser Puro (`protoc.lua`):** Permite ler e registrar schemas `.proto` diretamente em tempo de execução sem ferramentas de compilação adicionais instaladas.
2. **Descritor Pré-compilado (`game_packets.pb`):** Se o parser dinâmico não estiver disponível, o SDK carrega diretamente o descritor binário via `pb.loadfile()`.

---

## Pré-requisitos

Se você já executou o script de verificação na raiz (`./tools/setup_environment.sh`), o Love2D já deve estar pronto. Caso contrário:

1. **LÖVE 11.x+**: Instalado no seu sistema ([love2d.org](https://love2d.org/)).
2. **lua-protobuf (`pb`)**: Módulo binário C/Lua responsável pelo encode/decode binário rápido dos pacotes Protobuf.
   * **Linux:** `sudo luarocks install lua-protobuf` (ou fornecer o arquivo `pb.so` na pasta `lib/`).
   * **Windows:** Colocar `pb.dll` na pasta do jogo ou na pasta `lib/`.
   * **macOS:** `luarocks install lua-protobuf`.

---

## Testando o SDK Localmente

Antes de rodar seu jogo no Love2D, você pode validar a integridade de todas as abstrações (metatabelas de `Entity`, conversão tipada de propriedades, consultas em raio e despacho de eventos) executando:

```bash
lua sdks/love2d/test_sdk.lua
```

Se o ambiente estiver correto, você verá a mensagem:
```text
All loci_client.lua SDK unit tests passed successfully!
```

---

## Exemplo Rápido de Integração

```lua
-- main.lua
package.path = package.path .. ";sdks/love2d/?.lua"
local loci = require("loci_client")

function love.load()
    -- Conecta ao servidor autoritativo
    loci.connect("127.0.0.1", 8080, "Player1", "sdks/love2d/lib/")

    loci.on_action_cast = function(entity, ability_id, dir_x, dir_y)
        print("Habilidade disparada por entidade:", entity.id)
    end
end

function love.update(dt)
    -- Processa pacotes de rede e interpola entidades
    loci.update(dt)

    -- Envia movimento
    local dx, dy = 0, 0
    if love.keyboard.isDown("w") then dy = -1 end
    if love.keyboard.isDown("s") then dy = 1 end
    if love.keyboard.isDown("a") then dx = -1 end
    if love.keyboard.isDown("d") then dx = 1 end
    loci.send_move(dx, dy)
end

function love.draw()
    for _, entity in ipairs(loci.get_entities()) do
        if entity:is_local_player() then
            love.graphics.setColor(0.3, 0.6, 1.0)
        else
            love.graphics.setColor(0.3, 0.8, 0.4)
        end
        love.graphics.circle("fill", entity.x, entity.y, 16)
    end
end

function love.quit()
    loci.disconnect("Jogo fechado")
end
```

Consulte a documentação completa em `sdks/love2d/docs/index.html` abrindo no seu navegador.
