# Demand Spec: [CORE][SDK] Extract Client SDK into Dedicated Repository with Versioned Releases

**Issue:** [#35](https://github.com/Loci2D/loci2d/issues/35)  
**Status:** Ready to Implement  
**Reference ADRs:** [ADR-0018](../adr/pt/0018-sdks-de-cliente-e-abstracao-de-rede.md) · [ADR-0020](../adr/pt/0020-desacoplamento-e-extracao-dos-sdks-de-cliente.md)

---

## 1. Overview & Motivation

Currently, the client SDK for Love2D lives directly within the core engine repository at `sdks/love2d/`. Consumer games such as `loci-arena` require manual copying of files (`loci_client.lua`, `.proto`, compiled `pb.so`) into their workspace, introducing multiple operational problems:
1. Version mismatching and pollution of the server's `Cargo.toml` (e.g. `v0.6.5-sdk.4`).
2. High friction in local testing and pair programming.
3. Code divergence: enhancements made in `loci-arena` (e.g., `send_action_at_position`, `send_action_direction`, robust path discovery) are absent from the engine repository.
4. Scope pollution: client documentation and HTML guides are hosted alongside low-level Rust server code.

This specification details the step-by-step extraction of the Love2D SDK into `Loci2D/sdk-love2d`, establishing an independent release pipeline with pre-compiled binaries and automated local development linking.

---

## 2. Target Architecture

```mermaid
flowchart LR
    subgraph Engine ["Engine Repo: Loci2D/loci2d"]
        Proto["proto/game_packets.proto<br>(Single Source of Truth)"]
        SyncTool["tools/sync_sdk_proto.sh"]
        Server["loci2d Rust Binary"]
    end

    subgraph SDK ["SDK Repo: Loci2D/sdk-love2d"]
        LuaSDK["loci_client.lua"]
        ProtoCopy["lib/game_packets.proto<br>+ game_packets.pb"]
        Tests["test_sdk.lua"]
        Docs["docs/ (HTML Guides)"]
        CIRelease["CI: Release Workflow<br>(Compiles pb.so/dylib/dll & zips)"]
    end

    subgraph Game ["Client Repo: Loci2D/loci-arena"]
        LockFile[".loci2d-sdk-version"]
        SetupScript["scripts/setup_sdk.sh"]
        ClientGame["client/loci2d/"]
    end

    SyncTool -.->|Manual Sync| ProtoCopy
    CIRelease -->|Publishes Release .zip| GitHubReleases["GitHub Releases"]
    GitHubReleases -->|Download Fallback / CI| SetupScript
    LuaSDK -.->|Local Symlink (DX)| ClientGame
```

---

## 3. Implementation Plan

### Phase 1: Repository Bootstrap (`Loci2D/sdk-love2d`)
1. Create a public repository under the organization: `Loci2D/sdk-love2d`.
2. Push the initial baseline commit (**v0.6.0**) containing the current snapshot of `sdks/love2d/` from `loci2d`:
   * `loci_client.lua`
   * `test_sdk.lua`
   * `lib/game_packets.proto`, `lib/game_packets.pb`, `lib/protoc.lua`
   * `docs/index.html`, `docs/server_scripts.html`
   * `AI_REFERENCE.md`, `README.md`, `README.pt-BR.md`
   * Initial license (`MIT`) and `.gitignore`.

### Phase 2: Reconciliation & Feature Update (**v0.6.1**)
1. Apply the diff accumulated in `loci-arena/client/loci2d/loci_client.lua`:
   * `loci.send_action(ability_id, dir_x, dir_y)`: add defensive `tonumber` coercion and fallback.
   * `loci.send_action_at_position(ability_id, aim_x, aim_y)`: calculate direction relative to local player entity.
   * `loci.send_action_direction(ability_id, dir_x, dir_y)`: explicit directional wrapper.
   * Dynamic search paths for `package.path` and `package.cpath` to support nested client locations seamlessly.
2. Update `test_sdk.lua` to validate the new action methods and argument tolerances.

### Phase 3: CI/CD & Automated Binary Releases
1. Implement `.github/workflows/test.yml`:
   * Runs `lua test_sdk.lua` on all pushes and pull requests.
2. Implement `.github/workflows/release.yml`:
   * Triggered on release tags (`v*`) or manual `workflow_dispatch`.
   * Cross-compiles `lua-protobuf` binaries:
     * `lib/pb.so` (Linux x86_64)
     * `lib/pb.dylib` (macOS arm64 & x86_64 universal)
     * `lib/pb.dll` (Windows x86_64)
   * Assembles and uploads a standalone `loci2d-sdk-love2d.zip` release bundle containing:
     * `loci_client.lua`
     * `lib/` (including compiled binaries, `.proto`, `.pb`, and `protoc.lua`)
     * `README.md` and `LICENSE`
   * Automatically creates a GitHub Release with sha256 checksums.

### Phase 4: Client Integration in `loci-arena`
1. Add `.loci2d-sdk-version` containing the target SDK version (e.g. `v0.6.1`).
2. Implement `scripts/setup_sdk.sh`:
   * **Local Mode:** If `../sdk-love2d` (or `$LOCI_SDK_DIR`) exists, configure a symbolic link `client/loci2d -> ../sdk-love2d`.
   * **Release Mode:** If local directory does not exist or `--download` is provided, download and extract `loci2d-sdk-love2d.zip` matching `.loci2d-sdk-version` from GitHub Releases.
3. Integrate `setup_sdk.sh` into `setup_client.sh` and `run_client.sh`.
4. Validate running `loci-arena` without any manual copy step.

### Phase 5: Protobuf Sync Tooling in `loci2d`
1. Implement `tools/sync_sdk_proto.sh`:
   * Validates target SDK path (`../sdk-love2d` or specified argument).
   * Copies `proto/game_packets.proto` to `sdk-love2d/lib/game_packets.proto`.
   * Regenerates binary descriptor `sdk-love2d/lib/game_packets.pb` using `protoc` or vendored compiler.
   * Alerts the developer to commit and tag a new release in `sdk-love2d` when schemas change.

### Phase 6: Engine Cleanup & Decoupling
1. Remove `sdks/` directory from `loci2d`.
2. Revert `Cargo.toml` version to clean semantic versioning (`0.6.5`).
3. Update `README.md` and `README.pt-BR.md` with links to `Loci2D/sdk-love2d`.
4. Close issue #35 referencing the new architecture and documentation.

---

## 4. Verification & Acceptance Criteria

* [ ] `docs/adr/pt/0020-desacoplamento-e-extracao-dos-sdks-de-cliente.md` and `docs/adr/en/0020-client-sdks-decoupling-and-extraction.md` committed and reviewed.
* [ ] Repository `Loci2D/sdk-love2d` created with baseline (v0.6.0) and reconciled features (v0.6.1).
* [ ] Automated release workflow publishes `.zip` bundle with pre-compiled `pb` binaries.
* [ ] `loci-arena` runs successfully using automatic symlink in local mode and downloaded release in fallback mode.
* [ ] Zero manual file copies required when developing and testing locally.
