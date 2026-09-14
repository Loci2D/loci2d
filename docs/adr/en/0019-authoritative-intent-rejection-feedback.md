# ADR 0019: Authoritative Intent Rejection Feedback

## Status
Accepted

## Context
During Phase 6.5.2, we introduced safe intent dispatch, allowing Lua scripts to intercept and authorize intents before any state mutation occurs. However, if a Lua script rejected an intent (e.g. preventing movement because the player is stunned), it either dropped it silently or threw a Lua error (which causes a fatal server panic in the Rust core). 
This silent rejection severely impacts the Developer Experience (DX) for client developers (especially the target demographic for Phase 6.5.3), who cannot distinguish between a dropped UDP packet and a deliberate gameplay rejection by the server.

## Decision
We will implement a controlled rejection mechanism in the Rust core using the existing `ServerResponse` protobuf packet.

1. **Lua Hooks:** All intent-authorizing Lua hooks (`on_move_intent`, `on_action`, and `on_nav_intent`) can return an explicit rejection tuple (e.g., `return false, "Stunned"` or `return false, "Not enough mana"`).
2. **Intent Handler:** The Rust intent handler (`intent_handler.rs`) will parse this tuple and return a controlled rejection state (e.g., `IntentResult::Rejected(String)`), separating it from fatal script execution errors.
3. **Game Loop Dispatch:** The game loop (`tick.rs`) will intercept `Rejected(String)`, construct a `ServerPacket` containing a `ServerResponse` with the rejection reason, and send it immediately back to the client's UDP address.

## Consequences
- **Positive:** Client SDKs can listen to `ServerResponse` packets and trigger dedicated callbacks (like `on_intent_rejected`), vastly improving DX and debugging.
- **Positive:** Server stability is maintained, as gameplay rejections are no longer treated as fatal script panics.
- **Negative:** Requires minor refactoring of the internal `apply_intent` logic and `mlua` binding signatures in the core engine.
