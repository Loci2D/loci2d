# ADR 0020: Decoupling and Extracting Client SDKs into Dedicated Repositories

## Status
Accepted

## Context
[ADR-0018](0018-client-sdks-and-network-abstraction.md) established the foundational strategy of providing client SDKs to abstract UDP network loops, state replication, and Protocol Buffers serialization. Initially, the official Love2D SDK (`loci_client.lua`) and its assets were hosted directly within the core engine repository at `sdks/love2d/`.

As the ecosystem grew with real game clients (such as `loci-arena`) and ongoing preparation for high-school game developers, this monorepo layout introduced severe engineering and DX (Developer Experience) bottlenecks:
1. **Coupled Versioning:** Minor client SDK adjustments forced artificial patch increments in the Rust engine's `Cargo.toml` (e.g. `0.6.5-sdk.4`).
2. **Local Development Friction (DX):** Game developers were required to manually copy `.lua`, `.proto`, and pre-compiled C binaries into their game directory to test local changes.
3. **Untracked Code Drift:** Enhancements created directly in the game client (such as auxiliary action helpers and flexible argument parsing) remained stranded from the core engine due to manual sync barriers.
4. **Scope Pollution:** Interactive HTML guides, game developer documentation, and native C binaries were cluttering the central Rust server codebase.

## Decision
We decided to **decouple and extract client SDKs out of the `loci2d` repository into dedicated repositories under the `@Loci2D` organization**, starting with **`Loci2D/sdk-love2d`**.

The new architecture is anchored on the following pillars:

### 1. Dedicated Repository (`Loci2D/sdk-love2d`)
* The repository contains canonical source code (`loci_client.lua`), unit tests (`test_sdk.lua`), interactive documentation (`docs/`), agent context guides (`AI_REFERENCE.md`), and clean examples (`examples/`).
* Future engine SDKs (Godot, Web, Unity) will adhere to the same organization naming scheme (`sdk-godot`, `sdk-web`, etc.).

### 2. Independent Versioning & Releases
* The SDK follows its own release cycle via GitHub Releases (`vX.Y.Z`).
* **SemVer Guarantee:** SDK `v0.6.x` is guaranteed to be fully compatible with server `loci2d v0.6.x`.
* **Standardized Pre-built Binaries:** The GitHub Actions release workflow cross-compiles native `lua-protobuf` binaries (`pb.so` for Linux, `pb.dylib` for macOS, `pb.dll` for Windows) into the distributed `.zip`, eliminating C toolchain setup requirements for game creators and students.

### 3. Client Consumption: Local Development vs. Releases
To permanently eliminate manual file copying in consumer projects (such as `loci-arena`):
* The client maintains a pinned `.loci2d-sdk-version` file and a resolution script (`setup_sdk.sh`):
  * **Local Mode (DX):** If a sibling directory `../sdk-love2d` (or `LOCI_SDK_DIR`) exists, the script establishes a symbolic link (`client/loci2d -> ../sdk-love2d`). Code modifications in the SDK are immediately active in the running game without manual file copying.
  * **Release Mode / CI / Students:** If the local repository is absent or `--download` is passed, the script fetches and extracts the official release bundle matching `.loci2d-sdk-version` from GitHub Releases.

### 4. Protocol Contract Management (Protobuf)
* The central `loci2d` repository remains the sole **Canonical Source of Truth** for network packet definitions (`proto/game_packets.proto`).
* Schema propagation to the SDK repository is handled through a controlled script (`tools/sync_sdk_proto.sh`), preventing unreleased engine experimental schema modifications from prematurely breaking active game projects.

### 5. Engine Cleanup
* The `sdks/` directory is removed from the root of `loci2d`.
* `Cargo.toml` returns to standard clean semantic versioning (e.g. `0.6.5`), decoupled from SDK patch cycles.

## Consequences
- **Positive:** Eliminates local manual file copying friction completely via symbolic links.
- **Positive:** Clear separation of concerns: the engine focuses on server determinism, stability, and authority, while the SDK focuses on client ergonomics and developer experience.
- **Positive:** Students and game creators receive stable, pre-built, versioned distribution bundles directly from GitHub Releases.
- **Positive:** Streamlines the addition of future client SDKs for other game engines without cluttering the engine codebase.
- **Negative:** Requires an explicit sync step via script when network contracts in protobuf schemas are modified.
