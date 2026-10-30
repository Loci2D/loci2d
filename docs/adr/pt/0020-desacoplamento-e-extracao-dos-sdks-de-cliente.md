# ADR 0020: Desacoplamento e Extração dos SDKs de Cliente para Repositórios Dedicados

## Status
Aceito

## Contexto
O [ADR-0018](0018-sdks-de-cliente-e-abstracao-de-rede.md) definiu a estratégia fundamental de fornecer SDKs de cliente para abstrair loops de rede UDP, gerenciamento de estado e serialização Protocol Buffers. Inicialmente, o SDK oficial de Love2D (`loci_client.lua`) e seus recursos foram mantidos dentro do repositório principal da engine em `sdks/love2d/`.

Conforme o ecossistema evoluiu com clientes de jogos reais (como `loci-arena`) e a preparação para o público escolar, essa abordagem monolítica apresentou gargalos críticos de Engenharia e DX (Developer Experience):
1. **Acoplamento de Versionamento:** Qualquer ajuste pontual no SDK forçava versões artificiais no `Cargo.toml` da engine do servidor (ex.: `0.6.5-sdk.4`).
2. **Fricção no Desenvolvimento Local (DX):** Desenvolvedores de jogos precisavam copiar manualmente arquivos `.lua`, `.proto` e bibliotecas compiladas C para dentro de seus projetos para testar mudanças locais.
3. **Divergência de Código não Rastreada:** Melhorias e utilitários criados diretamente no cliente de jogo (como novos métodos de despacho de ações e tolerância a tipos) deixavam de ser integrados à engine principal devido à barreira manual de sincronização.
4. **Poluição de Escopo:** Documentação interativa em HTML, guias para criadores de jogos e bibliotecas C nativas estavam sobrecarregando o repositório central da engine Rust.

## Decisão
Decidimos **desacoplar e extrair os SDKs de cliente da engine `loci2d` para repositórios dedicados dentro da organização `@Loci2D`**, iniciando com o **`Loci2D/sdk-love2d`**.

A nova arquitetura adota os seguintes pilares:

### 1. Repositório Dedicado (`Loci2D/sdk-love2d`)
* O repositório conterá o código-fonte canônico (`loci_client.lua`), testes unitários (`test_sdk.lua`), documentação interativa (`docs/`), referência para agentes (`AI_REFERENCE.md`) e exemplos mínimos (`examples/`).
* Futuros SDKs (Godot, Web, Unity) seguirão a mesma convenção de nomenclatura na organização (`sdk-godot`, `sdk-web`, etc.).

### 2. Versionamento e Releases Independentes
* O SDK terá ciclo de release próprio via GitHub Releases (`vX.Y.Z`).
* **Garantia SemVer:** O SDK `v0.6.x` é garantido compatível com o servidor `loci2d v0.6.x`.
* **Binários Nativos Padronizados:** O workflow de release no GitHub Actions compilará e empacotará antecipadamente as extensões nativas do `lua-protobuf` (`pb.so` para Linux, `pb.dylib` para macOS e `pb.dll` para Windows), poupando os desenvolvedores de jogos e alunos da necessidade de compilação C local.

### 3. Consumo no Cliente: Desenvolvimento Local vs. Releases
Para eliminar a cópia manual de arquivos nos projetos consumidores (como `loci-arena`):
* O cliente possuirá um arquivo de trava `.loci2d-sdk-version` e um script de resolução (`setup_sdk.sh`):
  * **Modo Local (DX):** Se o repositório irmão `../sdk-love2d` (ou o caminho configurado via `LOCI_SDK_DIR`) existir, o script cria um link simbólico (`client/loci2d -> ../sdk-love2d`). Quaisquer modificações no SDK surtem efeito imediato no cliente em execução, sem cópias manuais.
  * **Modo Release / CI / Alunos:** Se o repositório local não estiver presente ou for passado `--download`, o script baixa o bundle compactado da release oficial especificada no `.loci2d-sdk-version`.

### 4. Gestão do Contrato do Protocolo (Protobuf)
* O repositório central `loci2d` permanece como a única **Fonte Canônica da Verdade** das definições de pacote (`proto/game_packets.proto`).
* A propagação de mudanças de schema para o repositório do SDK será realizada através de um script controlado (`tools/sync_sdk_proto.sh`), evitando atualizações prematuras ou disruptivas no desenvolvimento de jogos antes da homologação de novas releases da engine.

### 5. Limpeza da Engine
* A pasta `sdks/` é removida da raiz do repositório `loci2d`.
* A versão no `Cargo.toml` volta a seguir o versionamento semântico limpo do servidor (ex.: `0.6.5`), desacoplada de iterações do SDK.

## Consequências
- **Positivo:** Elimina definitivamente o atrito de cópia manual no desenvolvimento local através de links simbólicos.
- **Positivo:** Separação estrita de responsabilidades: a engine foca em estabilidade, determinismo e autoridade de servidor, enquanto o SDK foca em ergonomia de consumo no cliente.
- **Positivo:** Alunos e criadores de jogos recebem bundles estáveis, versionados e pré-compilados diretamente do GitHub Releases.
- **Positivo:** Facilita a expansão modular de novos SDKs para outras engines de jogos sem impacto no codebase central.
- **Negativo:** Introduz um passo explícito de sincronização de schemas via script quando houver mudanças no contrato protobuf do servidor.
