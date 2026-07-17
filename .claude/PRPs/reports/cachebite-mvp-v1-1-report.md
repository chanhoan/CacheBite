# CacheBite MVP + v1.1 Implementation Report

## Outcome

Implemented the Tauri 2/Rust/Svelte application foundation and the MVP/v1.1 product layers: typed provider state, Claude OAuth and Codex app-server collectors, independent refresh actors, versioned atomic persistence, narrow per-window IPC, overlay/panel UI, notifications, bounded history, pet manifests, CI/release definitions, and platform geometry policy.

The implementation is ready for CI/native-host validation, with the explicit exceptions under **Deferred or environment-blocked**.

## Delivered

- Svelte/TypeScript renderer with split usage ring, provider panel, settings, speech bubbles, accessible history graph, and revision-safe stores.
- Rust domain normalization, strict provider parsing, credential broker, timeout/size bounds, actor scheduling, backoff, TTL, reset handling, and cached startup hydration.
- Versioned settings/snapshot/history repositories with path-scoped locking, validation, migration, quarantine, atomic replacement, retention, and secret-exclusion tests.
- Tauri IPC DTOs and explicit overlay/panel authorization allowlists; debounced position persistence and live provider/history events.
- Saved-position recovery/clamping, current-position panel anchoring, mixed-DPI position sampling, and visible unavailable capability diagnostics.
- Optional animation resolution, opt-in native notifications, secondary-provider notification policy, and bounded provider history.
- Renderer/native E2E separation, native platform matrices, release artifact definitions, dependency automation, and security scans.

## Validation evidence

- Frontend CI gate: 16 files / 98 tests passed; Svelte check, ESLint, Prettier, and production Vite build passed.
- Coverage: 87.42% statements, 86.45% branches, 87.77% functions overall; domain/state/policy thresholds exceed 80%.
- Isolated Rust core harness: 31/31 tests passed in 0.19s; isolated Clippy previously passed with `-D warnings`.
- Rust formatting, Cargo metadata, JSON/config parsing, secret scan, and `git diff --check` passed.
- No production provider calls are used by fixture test modes.

## Deferred or environment-blocked

- Full Tauri `cargo test`/native packaging could not run on this Linux host because `pkg-config` and GTK/WebKitGTK development libraries are absent. The checked-in native CI matrix is the remaining acceptance gate.
- Browser WebdriverIO E2E could not launch `/usr/bin/google-chrome` in the sandbox (`EPERM`). Renderer integration behavior is covered by Vitest; CI must run the browser/native suites.
- Position listener cleanup starts its final native write synchronously as a best-effort flush. The cleanup contract cannot await IPC, so an immediate process exit may still lose the last pending move.
- Dependency license inventory is currently blocked by a missing pnpm store index file; `pnpm audit` also encountered the registry's retired/410 endpoint. CI retains both gates.
- Autostart and fullscreen detection are reported unavailable in this build rather than silently claimed. Their policy/core seams are tested, but production platform adapters remain future work.
- Claude PTY and Codex CLI/direct fallbacks are intentionally absent: no pinned, realistic noninteractive protocol and process-tree cleanup fixture was available. Primary collectors remain functional and isolated.
- Release workflows are defined but were not executed here. Every GitHub Action reference is pinned to a reviewed full commit SHA, with its human-readable release or selector retained in a comment and Dependabot configured to propose updates.
- The release intentionally contains no unlicensed prototype art. A valid user-supplied app-data pet package with an `idle` asset is required, per the plan's locked asset decision.

## Review result

Two focused code/security reviews were completed. IPC privilege, settings races, notification restart behavior, live history, fixture routing, fallback safety, position debouncing, recovery, and panel anchoring findings were remediated. No critical security finding remains. Native OS acceptance and the deferred items above must not be represented as locally verified.
