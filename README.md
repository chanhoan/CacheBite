# CacheBite

CacheBite is a cross-platform floating desktop pet that shows Claude and Codex subscription usage without sending account credentials through a CacheBite server.

The pet stays above normal application windows, plays an idle animation at a user-selected position, and hides while a full-screen application is active. It reports provider-calculated five-hour and weekly utilization with reset times.

## Status

CacheBite is currently in the architecture stage. Application scaffolding and distributable packages have not been created yet.

## Goals

- Support Windows, macOS, and Linux from one codebase.
- Display Claude and Codex five-hour and weekly usage.
- Keep provider credentials inside the native application process.
- Run as a transparent, frameless, always-on-top desktop pet.
- Let users drag the pet and restore its last valid position.
- Degrade clearly when an operating system or provider does not expose a capability.

## Architecture

CacheBite uses Tauri 2 with a Svelte and TypeScript renderer plus a Rust local backend.

```text
Svelte pet UI
      |
      | Tauri IPC
      v
Rust local backend
  |-- Claude usage collector
  |-- Codex usage collector
  |-- credential broker
  |-- snapshot and settings store
  `-- platform window adapter
      |
      v
Anthropic / OpenAI
```

There is no CacheBite cloud backend in the initial release. The Rust backend runs locally inside the desktop application and contacts provider services directly.

See [the architecture document](docs/architecture.md) for provider contracts, security boundaries, platform behavior, packaging, and testing strategy.

## Usage collection

- Claude: use the Claude Code OAuth usage contract, with the CLI `/usage` panel as a guarded fallback.
- Codex: prefer the Codex app-server `account/rateLimits/read` RPC, with provider backend and CLI status fallbacks.
- Normalize both providers into five-hour and weekly percentage windows.
- Cache the last successful snapshot and mark old data as stale.

CacheBite treats provider-calculated utilization as authoritative. It does not estimate subscription quota by summing local input and output tokens.

## Security

- Provider access tokens never enter the Svelte renderer.
- Credentials, prompts, source code, and session contents are not uploaded to CacheBite infrastructure.
- Browser cookies are not read.
- Claude and Codex credential files remain read-only.
- Authorization headers, account identifiers, home paths, and response bodies are excluded from logs.

The subscription usage endpoints are internal CLI contracts and may change. Provider integrations remain isolated so they can be updated independently from the rest of the application.

## Planned repository layout

```text
CacheBite/
  src/                  Svelte UI
  src-tauri/            Rust local backend and platform integration
  pets/                 bundled pet packages
  docs/                 product and engineering documentation
  tests/                cross-layer and packaging tests
  .github/workflows/    native builds for all supported operating systems
```

The UI and local backend remain in one repository because they are built, tested, versioned, and released as one desktop application.

## Packaging targets

| Platform | Planned package |
| --- | --- |
| Windows | MSI and NSIS installer |
| macOS | Signed and notarized DMG |
| Linux | AppImage, followed by DEB |

## License

No project license has been selected yet. Until a license is added, all rights are reserved.
