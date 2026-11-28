# Demand Spec: [Docs / DX] Post-SDK-Extraction Cleanup and Authoritative Server Lua API Reference

**Issue:** [`Loci2D/loci2d#54`](https://github.com/Loci2D/loci2d/issues/54) (superseding server-side scope of `#17`)  
**Status:** Ready to Implement  
**Reference ADRs:** [ADR-0014](../adr/en/0014-embedded-lua-scripting-and-command-buffer.md) · [ADR-0015](../adr/en/0015-dedicated-roadmap-structure-phase6.5-validation-dx.md) · [ADR-0016](../adr/en/0016-data-driven-entity-properties-and-engine-agnosticism.md) · [ADR-0020](../adr/en/0020-client-sdks-decoupling-and-extraction.md) · [ADR-0021](../adr/en/0021-physical-projectiles-ccd-and-sensor-collisions.md)

---

## 1. Overview & Motivation

Following [ADR-0020](../adr/en/0020-client-sdks-decoupling-and-extraction.md), the official Love2D client SDK (`loci_client.lua`, pre-built `lua-protobuf` binaries, and client HTML guides including `docs/server_scripts.html`) was extracted from `sdks/love2d/` into [`Loci2D/sdk-love2d`](https://github.com/Loci2D/sdk-love2d).

Several files in `loci2d` still assume the old monorepo layout or lack a canonical server-side Lua reference, creating friction for newcomers and students during Phase 6.5 validation:
1. `examples/love2d/README.md` still points to the deleted `sdks/love2d/lib/` directory and instructs manual `luarocks install lua-protobuf`.
2. `README.md`, `README.pt-BR.md`, `tools/setup_environment.sh`, and `tools/run_dev.sh` instruct fresh clones to run `love examples/love2d` directly without checking or documenting the `Loci2D/sdk-love2d` dependency (`../sdk-love2d`, `$LOCI_SDK_DIR`, or `examples/love2d/loci2d`).
3. `README.md` and `README.pt-BR.md` still state *"No client-side prediction or reconciliation to debug"*, which contradicts `sdk-love2d`'s transparent client-side prediction and snapshot interpolation.
4. `loci2d` currently has no authoritative Markdown reference for its server-side Lua scripting API (`Loci.*`, `Loci.Commands.*`, `Loci.Physics.*`, and lifecycle hooks), leaving Phase 6.5.5's Lua API Reference item incomplete.

---

## 2. File-by-File Manifest & Rationale

| File | Action | Rationale |
|---|---|---|
| `examples/love2d/README.md` | **Edit** | Remove stale `sdks/love2d/lib/` and `luarocks install lua-protobuf` instructions (line 33); document `Loci2D/sdk-love2d` sibling clone (`../sdk-love2d`), `$LOCI_SDK_DIR`, or release bundle extraction (`examples/love2d/loci2d`). |
| `examples/love2d/main.lua` | **Edit** | Prepend `os.getenv("LOCI_SDK_DIR")` (when set) to `package.path` and `package.cpath` alongside `../sdk-love2d` and `./loci2d`, matching `tools/sync_sdk_proto.sh` and ADR-0020. |
| `tools/setup_environment.sh` | **Edit** | Add step `[5/5] Checking Love2D Client SDK (sdk-love2d)` checking `$LOCI_SDK_DIR`, `../sdk-love2d/loci_client.lua`, or `examples/love2d/loci2d/loci_client.lua`. Emit a non-blocking `[WARNING]` with `git clone https://github.com/Loci2D/sdk-love2d.git ../sdk-love2d` if absent so Rust-only contributors still pass. |
| `tools/run_dev.sh` | **Edit** | Verify `sdk-love2d` presence (`$LOCI_SDK_DIR`, `../sdk-love2d`, or `examples/love2d/loci2d`) **before** starting the background server and launching `love examples/love2d`, exiting with a clear actionable error instead of a raw Lua `module 'loci_client' not found` traceback. |
| `README.md` & `README.pt-BR.md` | **Edit** | (1) Update line 11 pitch to clarify that the server remains 100% authoritative while client-side prediction and snapshot interpolation are handled transparently by official client SDKs (`Loci2D/sdk-love2d`); (2) add cloning `Loci2D/sdk-love2d` to *"How to Run the Project"*; (3) link to the new Server Lua Scripting API Reference. |
| `docs/lua-scripting-api.md` & `docs/lua-scripting-api.pt-BR.md` | **Create** | Authoritative bilingual (`EN` + `PT-BR`) server-side Lua scripting reference matching the exact `src/scripting/api.rs` and `src/scripting/engine.rs` surface. |
| `docs/phase6.5-roadmap.md` & `docs/phase6.5-roadmap.pt-BR.md` | **Edit** | Mark `[x] Lua API Reference` under 6.5.5 and link to `docs/lua-scripting-api.md` / `docs/lua-scripting-api.pt-BR.md`. |

---

## 3. Server Lua Scripting API Reference Scope (`docs/lua-scripting-api*.md`)

Both `docs/lua-scripting-api.md` (English) and `docs/lua-scripting-api.pt-BR.md` (Portuguese) must document the exact current runtime surface in `src/scripting/api.rs` and `src/scripting/engine.rs`:

1. **Execution Model & Determinism Guarantees:**
   - Deferred mutation model via `CommandBuffer` (mutations apply at deterministic phase boundaries).
   - Sandboxed standard libraries (`table`, `string`, `math` only; `io`, `os`, `package`, `debug` excluded).
   - `pairs` disabled in favor of deterministic `dpairs(tbl)`; `math.random([min, [max]])` backed by seeded LCG PRNG; 100,000 instruction limit per callback; multi-file sandboxed `require("...")` VFS.
2. **Core Types & Logging:**
   - `Loci.Vector2(x, y)` (`x_float()`, `y_float()`, `distance_to(other)`, `length()`, `normalize()`, `+`, `-`), plus `{ x = ..., y = ... }` table acceptance.
   - `Loci.Log.info(msg)`, `Loci.Log.warn(msg)`, `Loci.Log.error(msg)`.
3. **Read Queries (`Loci.*`):**
   - Entities: `get_entity`, `get_entity_by_name`, `get_entity_name`, `get_entity_type`, `get_entity_position`, `get_velocity`, `get_move_speed`, `get_entity_property`, `get_entities_in_radius`.
   - Globals & Obstacles: `get_global`, `get_obstacle`, `get_obstacle_position`, `get_obstacle_property`, `get_obstacles_by_blueprint`.
4. **Commands (`Loci.Commands.*`):**
   - Entities & Projectiles: `spawn_entity(opts) -> entity_id` (documenting `"Projectile"`, `velocity`, `is_sensor`, `lifetime_ticks`, `collision_layer`, `collision_mask`, `properties`), `destroy_entity`, `set_position`, `set_velocity`, `set_navigation_target`, `set_move_speed`, `set_property`, `set_collision_filter`, `set_sensor`, `set_lifetime`.
   - Match, Globals & Timers: `set_global`, `send_event`, `start_match`, `pause_match`, `end_match`, `start_timer`.
   - Map & Obstacles: `spawn_static_obstacle(opts) -> obstacle_id`, `destroy_static_obstacle`, `set_obstacle_property`, `set_map_bounds`.
5. **Physics Queries & Collision Layers (`Loci.Physics.*`):**
   - `raycast(origin, dir, max_distance)` (explicitly noting current static-obstacle-only scope).
   - `get_obstacles_in_radius(pos, radius)`.
   - `Loci.Physics.Layer.{NONE, SOLID_WALL, PLAYER, TRIGGER_ZONE, PROJECTILE}`.
6. **Lifecycle & Intent Hooks (all 14 callbacks in `src/scripting/engine.rs`):**
   - Match & Tick: `on_init()`, `on_tick(tick)`, `on_timer_complete(timer_id)`.
   - Players & Intents (documenting `return false, "reason"` rejection support): `on_player_join(entity_id)`, `on_player_leave(entity_id)`, `on_move_intent(entity_id, dir_x, dir_y)`, `on_nav_intent(entity_id, target_x, target_y)`, `on_action(entity_id, ability_id, dir_x, dir_y)`.
   - Collisions & Triggers: `on_collision(entity_a, entity_b)`, `on_static_collision(entity_id, obstacle_id, nx, ny)`, `on_boundary_collision(entity_id, nx, ny)`, `on_trigger_enter(entity_id, trigger_id)`, `on_trigger_stay(entity_id, trigger_id)`, `on_trigger_exit(entity_id, trigger_id)`.

---

## 4. Implementation Checklist

- [ ] Update `examples/love2d/README.md` and `examples/love2d/main.lua`.
- [ ] Update `tools/setup_environment.sh` and `tools/run_dev.sh` with `sdk-love2d` checks.
- [ ] Update `README.md` and `README.pt-BR.md`.
- [ ] Author `docs/lua-scripting-api.md` and `docs/lua-scripting-api.pt-BR.md`.
- [ ] Update `docs/phase6.5-roadmap.md` and `docs/phase6.5-roadmap.pt-BR.md`.

---

## 5. Verification Plan

1. **Stale Reference Check:** Search `README*`, `examples/`, `tools/`, and `docs/lua-scripting-api*` for `sdks/love2d` or `luarocks` and confirm zero stale instructions remain.
2. **Script Checks:**
   - Run `bash -n tools/setup_environment.sh tools/run_dev.sh`.
   - Test `./tools/setup_environment.sh` and `./tools/run_dev.sh` behavior both without and with `../sdk-love2d` (or `LOCI_SDK_DIR`).
3. **API Parity Audit:** Cross-check every `Loci.*`, `Loci.Commands.*`, `Loci.Physics.*`, and `on_*` signature in `docs/lua-scripting-api.md` and `docs/lua-scripting-api.pt-BR.md` against `src/scripting/api.rs` and `src/scripting/engine.rs`.
