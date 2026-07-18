# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

CacheBite is a local Tauri 2 desktop pet that surfaces Claude and Codex subscription usage **without sending credentials through any CacheBite server**. Credentials never reach the renderer. The Svelte/TypeScript renderer is a pure presentation layer; all credential access, provider collection, refresh scheduling, persistence, and platform policy live in Rust under `src-tauri/`.

## Commands

Prereqs: Node 22, pnpm 10.15.1 (via `corepack`), Rust toolchain pinned in `rust-toolchain.toml`. Linux native build also needs WebKitGTK 4.1, AppIndicator, librsvg, `patchelf`.

```bash
pnpm install --frozen-lockfile
pnpm dev                      # renderer only (Vite, port 1420)
pnpm tauri dev                # full desktop app
pnpm tauri build --debug      # debug bundles under src-tauri/target/debug/bundle/
pnpm test:ci                  # svelte-check + eslint + prettier + vitest coverage + vite build
pnpm test                     # vitest run (renderer unit tests)
pnpm check                    # svelte-check only
pnpm lint                     # eslint + prettier --check

# single renderer test file / single test
pnpm vitest run src/lib/interaction/petPointer.test.ts
pnpm vitest run -t "clamps inside the selected display"

# native (Rust) tests
cargo test --manifest-path src-tauri/Cargo.toml --all-features
cargo test --manifest-path src-tauri/Cargo.toml window::tests   # single module

# E2E (WebdriverIO): renderer-only vs full native
pnpm test:e2e:renderer        # wdio.browser.conf.ts
pnpm test:e2e                  # wdio.conf.ts (native)

python3 scripts/build-pet-packages.py   # regenerate bundled cat/corgi pet packages from docs/UI-plan/ art
```

Coverage gate is 80% on branches/functions/lines/statements (`vite.config.ts`); `test:ci` enforces it.

## Architecture

**Renderer ↔ native boundary is the spine.** The renderer only ever talks to Rust through one typed gateway:

- `src/lib/api/gateway.ts` — `AppGateway` interface + `tauriGateway` implementation. Every wire DTO (`ProviderBackendStateWire`, `AppSettings`, `HistoryModels`, `PlatformCapabilities`, `CollectorMode`, …) is defined here. This is the contract; changes must stay in sync with the Rust IPC commands.
- Native commands registered in `src-tauri/src/lib.rs` `invoke_handler!`: `get_collector_mode`, `get_provider_states`, `get_settings`, `get_history`, `get_pet_package`, `get_platform_capabilities`, `save_position`, `refresh_provider`, `update_settings`, `show_panel`, `quit`. Implementations live in `src-tauri/src/refresh/ipc.rs`.
- `src/lib/api/fixtureGateway.ts` is the deterministic renderer-side fixture used in tests/E2E — mirror the real gateway shape when adding methods.

**Native layers (`src-tauri/src/`):**

- `collectors/` — provider collection. `claude.rs` (fixed HTTPS Claude Code OAuth usage endpoint), `codex.rs` (`codex -s read-only -a untrusted app-server` + JSON-RPC `initialize` handshake then `account/rateLimits/read`), `broker.rs` (read-only credential selection), `fallback.rs`, `wsl.rs`. Parsers are isolated, size/time-bounded, and fail with typed provider-scoped outcomes rather than propagating raw errors.
- `refresh/` — one independent refresh **actor** per provider (`actor.rs`), the orchestrating `service.rs`, and `ipc.rs` (the Tauri command surface). Providers refresh independently; one failing collector must not stall the other.
- `store/` — persistence: `settings.rs`, `history.rs`, `snapshots.rs`, `pets.rs`. Bundled cat/corgi pet packages install into the app-data pet directory on first launch (`install_bundled_pet_packages` in `lib.rs`), repairing/upgrading incomplete legacy installs.
- `window/` — display geometry & pointer policy (DPI conversion, clamping to nearest remaining display incl. negative coords, panel anchoring/flip).
- `domain.rs` — normalized provider state shared across layers.

**Renderer layers (`src/lib/`):**

- `contracts/domain.ts` + `state/` (`engine.ts`, `presentation.ts`) — normalize gateway DTOs into view models. Pure, heavily unit-tested.
- `interaction/` — pointer/drag, bubble, notification, and event **policies** as pure reducers (`petPointer.ts`, `bubblePolicy.ts`, `notificationPolicy.ts`, `eventPolicy.ts`). Behavior tested here, not in Svelte components.
- `stores/` — Svelte stores wiring gateway → components (`providers.ts`, `settings.ts`, `interaction.ts`).
- `components/` — Svelte views (`PetOverlay`, `PetAnimation`, `UsagePanel`, `SplitUsageRing`, `HistoryGraph`, `SettingsPanel`, …).

**Two windows** (`src-tauri/tauri.conf.json`): `overlay` (transparent, always-on-top, frameless pet) and `panel` (usage UI, hidden until `show_panel`). Both load `index.html` with a `?window=` query param that selects which surface mounts.

## Invariants — do not break

- **Privacy contract:** renderer DTOs and logs must exclude authorization values, raw provider bodies, account identifiers, and credential paths. CacheBite does not read browser cookies, rewrite credential files, or estimate quota from token counts. CI has secret/renderer-endpoint guards (`ci.yml`).
- **Fixture vs production collector modes:** tests set `CACHEBITE_E2E_FIXTURES=1` to swap both collectors for deterministic _unavailable_ fixtures. The separate production-composition smoke runs the real composition with credentials absent and Codex pointed at a nonexistent path — it does not request a manual refresh. Keep these two modes distinct (`collector_mode_distinguishes_...` test in `lib.rs`).
- **Unverified platform capabilities report `unavailable`**, never a provider failure. Fullscreen detection is currently reported unavailable.
- **Pet manifest contract:** packages require a valid `idle` state; asset protocol scope is locked to `$APPDATA/pets/*/frames/*.png` (`tauri.conf.json`). `docs/UI-plan/` art is _source only_ — never bundled into a release directly; it flows through `scripts/build-pet-packages.py`.
- **Codex `initialize` RPC requires `clientInfo{name,version}`** or the provider drops to error (stderr is nulled, so no debug logs surface).

## Authoritative docs

`docs/architecture.md` and `docs/ui-contract.md` are the source of truth for architecture and presentation contracts — consult before changing the boundary or view models.

## CI / release

- `ci.yml`: frontend checks, ≥80% coverage gates, renderer E2E, Rust fmt/clippy/tests, dependency audit, license inventory, secret guards.
- `native-smoke.yml`: Windows, macOS, Linux X11, headless Wayland fixture jobs + credential-free Linux production-composition smoke.
- `release.yml`: MSI, NSIS, validation DMG, AppImage, SHA-256 artifacts. Signed/notarized macOS requires manual opt-in + protected `production-macos-signing` environment.
- The Windows MSI is built on the Windows toolchain via WSL interop inside the `.tmp-msi-build` staging copy.
- Every GitHub Action is pinned to a full commit SHA (Dependabot tracks Actions/npm/Cargo).
