# Code Review Remediation Design

## Goal

Resolve every finding in `docs/code-review/local-changes-2026-07-17.md` without weakening CacheBite's local-only security boundary or overstating native test coverage.

## Design

The renderer will consume backend reset-pending state before retained snapshots, and startup will terminate in one of three explicit states: ready, recoverable error, or retrying. Settings writes will use a single serialized queue so responses cannot commit out of order.

Codex discovery will occur once during native startup. A resolver will search the inherited PATH, canonicalize the selected executable, and return an absolute file path. `CodexCollector` and `collect_app_server` will reject bare and relative paths so later process launches cannot repeat PATH lookup. This narrows binary selection to startup; it does not claim platform signature verification.

Native tests will distinguish fixture collector composition from production collector composition through an allowlisted diagnostic DTO. Fixture smoke remains deterministic and network-free. A separate production-composition smoke starts without the fixture environment and treats absent credentials or CLI installation as valid provider outcomes. Representative hydration, settings, and history IPC flows will run through the native WebView boundary.

Persistence failures will be recorded with provider and repository category but without paths, payloads, accounts, or credentials. Failed writes remain eligible for later retry. Frontend coverage will include `App.svelte`; bootstrap-only and generated/test fixture files may be narrowly excluded with comments. Window-position tests will use fake timers and mocked Tauri window events to prove mixed-DPI conversion, one debounced save, timer replacement, and cleanup flushing.

GitHub Actions will use reviewed immutable commit SHAs. Dependabot remains responsible for update proposals.

## Validation

Every behavior change follows RED/GREEN TDD. The final gate is frontend check/lint/test/coverage/build, isolated Rust tests and Clippy, workflow syntax inspection, secret scan, and full native tests where host prerequisites permit. Environment-blocked native checks must remain explicitly reported.

## Scope Boundaries

- No user-configurable Codex executable setting.
- No provider network calls in fixture tests.
- No platform code-signature verification in this remediation.
- No unrelated UI or architecture refactor.
