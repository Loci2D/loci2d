# ADR 0015: Dedicated Sub-Roadmap Structure and Governance for Phase 6.5

## Status

Accepted (Phase 6.5)

## Context

From Phase 1 through Phase 6, `loci2d` development followed a strictly linear progression centered on the Rust core engine (game loop, netcode, replays, physics, and embedded Lua scripting). Each of these phases is mapped to a single corresponding technical specification document in `docs/roadmap-specs/phaseX-*-spec.md`.

**Phase 6.5 (Validation, Developer Experience & API Stabilization)** represents a strategic pause in adding new engine features in order to consolidate the v0.6.x stack as a playable, developer-friendly multiplayer ecosystem. Unlike previous technical phases, Phase 6.5 spans multiple technologically heterogeneous domains:
- **Engine Architecture (Rust):** Strict decoupling of canonical deterministic state from Lua script execution via a safe intent queue.
- **Multi-Language Client SDKs:** Creation of idiomatic client wrappers for **Godot 4** (`GDScript`), **LÖVE2D** (`Lua`), and **Python**.
- **Documentation & Starter Templates:** Authoring a "15-Minute First Game" guide, Lua API reference, and copy-paste templates.
- **Examples Refactoring:** Updating project examples (CLI, Love2D, Godot, Python) to consume the new SDK layer.
- **Empirical LAN Playtesting:** Multi-device physical network sessions with students to test tick stability, desync resilience, and API ergonomics.

Attempting to consolidate all these deliverables into a single specification document or a single section within the main `roadmap.md` would yield overly bloated, unmaintainable documents that impair focused development and code reviews.

## Decision

We adopt a **Dedicated Sub-Roadmap (`docs/phase6.5-roadmap.md`)** and **Modular Specifications per Sub-Milestone** for Phase 6.5.

1. **Dedicated Sub-Roadmap File:** Phase 6.5 will maintain its own dedicated file at `docs/phase6.5-roadmap.md`, organizing the strategic pause into clear sub-milestones (e.g., 6.5.1 through 6.5.5).
2. **Modular Specification Files:** Each sub-milestone within Phase 6.5 will have a dedicated specification document in `docs/roadmap-specs/phase6.5.X-*-spec.md` (e.g., `phase6.5.1-state-scripting-decoupling-spec.md` and `phase6.5.2-client-sdks-spec.md`).
3. **Macro Overview in Main Roadmap:** The primary [roadmap.md](../../roadmap.md) will retain a high-level summary of Phase 6.5 and link directly to the dedicated sub-roadmap document.

## Consequences

**Positive:**
- **Scope Clarity & Modularity:** Allows development of client SDKs (Godot/Lua/Python) and Rust engine modifications to proceed with isolated, focused specifications and manageable pull requests.
- **Clean Main Roadmap:** Preserves readability of the high-level project roadmap without cluttering it with multi-SDK integration details and playtest guides.
- **Governance Traceability:** Explicitly documents why Phase 6.5 diverges from the single-spec pattern of earlier phases, preventing confusion for future maintainers.

**Negative:**
- **Minor Process Exception:** Introduces a structural exception to the single specification file pattern used in Phases 1–6. This exception is officially documented and justified by this ADR.
