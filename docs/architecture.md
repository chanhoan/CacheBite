# CacheBite Architecture

## Purpose

CacheBite is a cross-platform floating desktop pet that reports Claude and Codex subscription usage. It targets Windows, macOS, and Linux from one codebase while producing a native package for each operating system.

The first release shows provider-calculated five-hour and weekly usage, remains above normal application windows, hides over full-screen applications, and stays where the user drags it.

## Product boundaries

The initial release includes:

- Claude and Codex personal or business subscription usage when the installed CLI exposes compatible authentication.
- Five-hour and weekly utilization percentages and reset times.
- A transparent, frameless, always-on-top pet window.
- Manual dragging with position persistence and monitor-boundary recovery.
- Automatic hiding while a full-screen application is active.
- Local settings and cached usage snapshots.
- Native packages for Windows, macOS, and Linux.

The initial release does not include:

- A CacheBite cloud service or account system.
- Browser-cookie extraction or web-page scraping.
- API billing analytics for Anthropic Console or OpenAI Platform organizations.
- Uploading credentials, prompts, source code, or session contents.
- Movement around the desktop; only the pet animation moves within its fixed window.
- A bundled pet design. Visual assets are supplied independently through the asset contract.

## Technology

- Desktop runtime: Tauri 2
- Core and native integration: Rust
- UI: Svelte with TypeScript
- Local persistence: versioned JSON for settings and the latest usage snapshot
- CI packaging: GitHub Actions with native Windows, macOS, and Linux runners

Tauri keeps the resident process smaller than an Electron application and allows OS-specific window behavior and credential access to remain in Rust.

## Repository strategy

CacheBite starts as one repository under the owner's personal GitHub account:

```text
github.com/<owner>/CacheBite
```

The repository contains both the Svelte UI and the Rust local backend because they are built, versioned, tested, and released as one desktop application. The Rust backend runs inside the Tauri process and communicates with the renderer through Tauri IPC; it is not a separately deployed HTTP service.

The initial repository layout is:

```text
CacheBite/
  src/                  Svelte UI
  src-tauri/            Rust local backend and platform integration
  pets/                 bundled pet packages
  docs/                 product and engineering documentation
  tests/                cross-layer and packaging tests
  .github/workflows/    native builds for the three operating systems
```

The project does not require a GitHub organization for its initial release. Separate `website`, `cloud-api`, or `pets` repositories are created only if those surfaces later gain independent release cycles. The first release has no CacheBite cloud backend.

## System structure

```text
Pet UI
  -> application state
      -> notification policy
      -> normalized usage service
          -> Claude collector
          -> Codex collector
      -> window controller
          -> Windows adapter
          -> macOS adapter
          -> Linux adapter
      -> settings and snapshot store
```

Each component exposes a small interface. Provider-specific response formats never reach the UI, and OS-specific APIs never reach the usage collectors.

## Usage model

Both collectors produce the same normalized snapshot:

```text
ProviderUsageSnapshot
  provider: claude | codex
  plan_type: optional string
  session: optional UsageWindow
  weekly: optional UsageWindow
  captured_at: ISO 8601 timestamp
  source: oauth_api | cli_rpc | cli_pty | cache
  status: fresh | stale | unavailable | error

UsageWindow
  used_percent: number from 0 through 100
  window_minutes: positive integer
  resets_at: optional ISO 8601 timestamp
```

Provider-calculated utilization is authoritative. CacheBite does not derive subscription quota from locally counted input and output tokens because provider plans may weight models, caching, and tasks differently.

## Claude collection

The preferred Claude path calls the Claude Code OAuth usage endpoint directly from the desktop process:

```text
GET https://api.anthropic.com/api/oauth/usage
Authorization: Bearer <Claude Code OAuth access token>
anthropic-beta: oauth-2025-04-20
User-Agent: claude-code/<compatible version>
```

The collector reads credentials through a credential broker:

1. The Claude Code keychain entry on macOS.
2. The active `CLAUDE_CONFIG_DIR` credential file.
3. The default `~/.claude/.credentials.json` credential file.

The response's `five_hour` and `seven_day` windows are mapped to the normalized model. The parser accepts known field variants defensively and rejects malformed percentages or reset timestamps.

If the OAuth request cannot be used, an optional fallback launches Claude in a hidden pseudo-terminal, invokes `/usage`, and parses only the usage panel. It never stores conversation output. The fallback has a strict timeout and cannot trigger an interactive login flow in the background.

## Codex collection

The preferred Codex path delegates authentication and rate-limit retrieval to the installed Codex CLI:

1. Start `codex -s read-only -a untrusted app-server` as a hidden child process.
2. Complete the JSON-RPC initialization handshake.
3. Call `account/rateLimits/read`.
4. Map `primary` to the five-hour window and `secondary` to the weekly window.
5. Stop the child process after receiving the response or reaching the timeout.

This path lets Codex own token refresh, proxy behavior, and certificate handling.

For environments where app-server is unavailable, the collector may call the backend contract used by Codex:

```text
GET https://chatgpt.com/backend-api/wham/usage
Authorization: Bearer <Codex access token>
ChatGPT-Account-Id: <account ID when present>
OpenAI-Beta: codex-1
User-Agent: codex-cli
originator: Codex Desktop
```

The credential broker reads the selected `CODEX_HOME/auth.json` without modifying it. A final optional fallback launches a hidden Codex status command and parses only rate-limit output.

## Credential and network security

- Requests go directly from CacheBite to Anthropic or OpenAI. There is no CacheBite relay server.
- Access and refresh tokens are never written to CacheBite settings, caches, or logs.
- Tokens live only for the duration of a request and are not exposed to the Svelte renderer.
- The renderer receives normalized percentages, reset times, source, and status only.
- CacheBite never reads browser cookies.
- Credential files are read-only. CacheBite does not refresh or rewrite provider credentials in the first release.
- Logs redact authorization headers, account identifiers, home paths, and response bodies.
- Provider responses have size limits, timeouts, schema validation, and HTTPS-only endpoints.
- Custom endpoints are not user-configurable in the first release.

Direct provider endpoints are internal CLI contracts rather than stable public subscription APIs. Each provider implementation is isolated behind a collector interface so contract changes can be shipped without changing the rest of the application.

## Refresh and failure policy

- Refresh immediately after startup when credentials are present.
- Poll every 15 minutes by default.
- Debounce focus, resume, and manual refresh requests to protect provider endpoint budgets.
- Apply exponential backoff after consecutive failures.
- Retain the last successful snapshot for up to 30 minutes.
- Mark retained data as stale and show its capture time.
- Drop expired data after the stale window instead of presenting it as current.
- Preserve independent state for Claude and Codex so one provider's failure does not hide the other.
- Treat missing CLI installation or sign-in as unavailable, not as an application error.

## Window behavior

The pet is a transparent, frameless window sized to its visual asset. It stays above normal application windows, does not appear as a normal taskbar or Dock window, and accepts pointer input over its visible content.

Dragging pauses interaction animations, moves the window, and persists the final logical position. At startup and after display changes, the window controller clamps the saved position into the nearest available display.

Platform adapters provide:

```text
set_always_on_top
set_taskbar_visibility
detect_fullscreen_application
show_pet
hide_pet
get_display_bounds
register_start_at_login
```

Full-screen detection hides the pet over games, video, and other exclusive full-screen applications. The first release does not attempt to overlay exclusive full-screen content.

Linux support targets X11 and Wayland where the compositor permits the required behavior. Unsupported Wayland capabilities degrade visibly: CacheBite reports the unavailable feature rather than silently pretending it is active.

## Asset contract

The visual package is independent of the application core. A pet package contains a manifest plus GIF, WebP, or sprite-sheet assets. The manifest declares the pet identifier, display name, default size, animation source, frame timing when required, and optional states.

The first implementation must render one idle animation. Additional usage-driven animations can be added without changing collector or window interfaces.

## Packaging

- Windows: MSI and NSIS installer
- macOS: signed and notarized application in a DMG
- Linux: AppImage first, with DEB support after the portable build is stable

Each target is built on its native GitHub Actions runner. Release automation produces checksums. Code signing credentials are supplied only through CI secrets and are never stored in the repository.

## Testing

Unit tests cover response normalization, field variants, invalid payloads, reset timestamps, stale-state transitions, backoff, position clamping, and settings migration.

Integration tests use local mock HTTP servers and fake Claude/Codex executables. They verify headers without recording secret values, RPC handshakes, PTY timeouts, malformed output, missing credentials, and provider isolation.

UI tests verify dragging, persisted position, stale and unavailable states, and animation loading. Platform smoke tests run on Windows, macOS, Linux X11, and at least one Wayland compositor. Packaging tests install, launch, and uninstall each produced artifact.

No test contacts production Anthropic or OpenAI endpoints by default.

## Source precedent

The provider strategy is based on the MIT-licensed Orca implementation at `stablyai/orca`, inspected at commit `319ae4e9eafe8505212851709d70e3c663368a59`. If substantial Orca code is copied rather than independently implemented, its copyright notice and MIT license must accompany that code.
