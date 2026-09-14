# ADR 0019: Feedback de Rejeição de Intents Autoritativo

## Status
Aceito

## Contexto
Durante a Fase 6.5.2, introduzimos o despacho seguro de intents, permitindo que scripts Lua interceptem e autorizem intents antes que qualquer mutação de estado ocorra. Porém, se um script Lua rejeitasse um intent (ex: impedindo movimento porque o jogador está atordoado), ele o descartava silenciosamente ou lançava um erro Lua (causando um pânico fatal no núcleo Rust).
Essa rejeição silenciosa prejudica severamente a Experiência do Desenvolvedor (DX) no cliente (especialmente para o público-alvo da Fase 6.5.3), que fica impossibilitado de distinguir entre um pacote UDP perdido na rede e uma rejeição intencional do servidor.

## Decisão
Implementaremos um mecanismo de rejeição controlada no núcleo Rust utilizando o pacote protobuf `ServerResponse` já existente.

1. **Hooks Lua:** Todos os hooks de autorização de intents (`on_move_intent`, `on_action`, e `on_nav_intent`) poderão retornar uma tupla de rejeição explícita (ex: `return false, "Atordoado"` ou `return false, "Sem mana suficiente"`).
2. **Intent Handler:** O handler no Rust (`intent_handler.rs`) interpretará essa tupla e retornará um estado de rejeição controlado (ex: `IntentResult::Rejected(String)`), separando de erros fatais de execução do script.
3. **Game Loop:** O game loop (`tick.rs`) interceptará esse `Rejected(String)`, construirá um `ServerPacket` contendo um `ServerResponse` com o motivo da rejeição, e enviará imediatamente de volta ao endereço UDP do cliente.

## Consequências
- **Positivo:** SDKs de cliente podem interceptar pacotes `ServerResponse` e disparar callbacks dedicadas (como `on_intent_rejected`), melhorando drasticamente a DX e o debug.
- **Positivo:** A estabilidade do servidor é mantida, já que rejeições de gameplay deixam de ser tratadas como pânicos fatais de script.
- **Negativo:** Exige uma leve refatoração da lógica interna do `apply_intent` e das assinaturas do `mlua` na engine principal.
