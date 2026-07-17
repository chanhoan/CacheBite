# CacheBite

CacheBite is a local Tauri 2 desktop pet that presents Claude and Codex subscription usage without sending credentials through a CacheBite server. The renderer is Svelte/TypeScript; credential access, provider collection, refresh state, persistence, and platform policy live in Rust.

## Current status

The repository now contains the MVP application code and automated release definitions. The renderer build and isolated native-core tests pass locally; a full Tauri build still requires the platform prerequisites listed below. The implementation includes normalized provider state, read-only credential handling, Claude OAuth usage collection, Codex app-server RPC collection, independent refresh actors, an idle pet overlay, split usage ring, settings/snapshot persistence, pointer and display geometry policies, and fixture-only test modes.

The release workflow is configured to produce these native validation artifacts when its platform jobs pass:

| Platform | Artifact |
| --- | --- |
| Windows | MSI and NSIS |
| macOS | unsigned/ad-hoc validation DMG; protected signed/notarized DMG on explicit approval |
| Linux | AppImage |

Public macOS distribution remains blocked until the protected signing environment is configured and the notarization job passes.

## Development setup

Requirements:

- Node.js 22 and pnpm 10.15.1
- the Rust toolchain pinned in `rust-toolchain.toml`
- Tauri 2 native prerequisites for your operating system
- Linux: WebKitGTK 4.1, AppIndicator, librsvg, and `patchelf`

```bash
pnpm install --frozen-lockfile
pnpm dev                 # renderer only
pnpm tauri dev           # desktop application
pnpm test:ci             # check, lint, coverage, renderer build
cargo test --manifest-path src-tauri/Cargo.toml --all-features
python3 scripts/build-pet-packages.py # rebuild bundled cat/corgi packages from docs/UI-plan assets
```

CacheBite bundles generated cat and corgi packages with a valid `idle` state and installs them into the application-data pet directory on first launch. Additional user-supplied packages can follow the same manifest contract; source artwork under `docs/UI-plan/` is only used by the package build script.

## Provider collection and privacy

MVP collection uses only these primary paths:

- Claude: the fixed HTTPS Claude Code OAuth usage endpoint, using read-only credentials selected by the native credential broker.
- Codex: `codex -s read-only -a untrusted app-server`, followed by the JSON-RPC initialize handshake and `account/rateLimits/read`.

The deferred Claude PTY and Codex direct-backend/status fallbacks are not present in the MVP. CacheBite does not read browser cookies, rewrite credential files, estimate quota from token counts, or expose provider tokens to the renderer. Logs and renderer DTOs exclude authorization values, raw provider bodies, account identifiers, and credential paths.

Fixture tests set `CACHEBITE_E2E_FIXTURES=1`, which replaces both collectors with deterministic unavailable fixtures. The separate production-composition smoke starts the real collector composition with credentials absent and Codex pointed at a nonexistent absolute path; it does not request a manual refresh.

## Platform behavior and limits

- Saved logical positions are DPI-converted and clamped to the nearest remaining display, including displays with negative coordinates.
- Position listener cleanup starts a best-effort final native save synchronously; an immediate process termination can still lose the last pending move because cleanup cannot await IPC completion.
- A release below 4 px toggles the panel; movement at or above 4 px is a drag.
- Fullscreen detection is currently reported unavailable; the tested policy reducer can hide presentation windows without stopping refresh once a platform adapter is connected.
- Panel anchoring flips and clamps inside the selected display.
- X11 and Wayland smoke jobs are defined separately. Unverified platform capabilities are reported as unavailable rather than treated as provider failure.
- Internal provider contracts may drift; parsers are isolated, size/time bounded, and fail with typed provider-scoped outcomes.

## Validation and releases

- `ci.yml`: frontend checks, ≥80% configured coverage gates, renderer E2E, Rust format/clippy/tests, dependency audit, license inventory, and secret/renderer-endpoint guards.
- `native-smoke.yml`: Windows, macOS, Linux X11, and headless Wayland fixture native jobs, plus a credential-free Linux production-composition smoke.
- `release.yml`: MSI, NSIS, validation DMG, AppImage, and SHA-256 artifacts. Signed/notarized macOS builds require manual opt-in plus the protected `production-macos-signing` environment.
- Dependabot tracks GitHub Actions, npm, and Cargo updates. Every GitHub Action is pinned to a reviewed full commit SHA with its human-readable release or selector retained in a comment.

See [docs/architecture.md](docs/architecture.md) and [docs/ui-contract.md](docs/ui-contract.md) for the authoritative architecture and presentation contracts.

## License

No project license has been selected. Until one is added, all rights are reserved.
