# Local Changes Code Review

**Reviewed:** 2026-07-17
**Scope:** All local uncommitted changes, including untracked files
**Decision:** REQUEST CHANGES

## Summary

The frontend validation suite passes, but the changes contain correctness,
security, and test-integrity issues that should be addressed before commit. In
particular, reset events can leave stale usage visible, startup can hang after
an IPC failure, executable discovery permits binary substitution, and native
smoke tests do not exercise the production collector composition they claim to
cover.

## Findings

### CRITICAL

None.

### HIGH

1. **Reset-pending state is ignored by the renderer**
   Location: `src/App.svelte:121`
   The refresh actor retains the previous snapshot when a quota reset becomes
   pending, but the renderer reapplies that old snapshot without consuming
   `reset_pending`. If the subsequent provider refresh fails, pre-reset usage
   and severity can remain visible indefinitely. Handle `reset_pending`
   explicitly or invoke the reset-transition logic before applying the retained
   snapshot.

2. **Startup IPC rejection leaves the application permanently loading**
   Location: `src/App.svelte:176`
   `getProviderStates()` is awaited inside a detached async startup task without
   rejection handling. A transient IPC or service failure leaves `ready` false,
   produces an unhandled rejection, and keeps the UI on “CacheBite is starting.”
   Add a visible error/retry state and a test covering this failure path.

3. **Codex executable discovery permits binary substitution**
   Locations: `src-tauri/src/lib.rs:50`,
   `src-tauri/src/collectors/codex.rs:82`
   The production collector launches the bare name `codex`, delegating selection
   to operating-system path lookup. A substituted executable earlier in the
   search path could run with the application's privileges. Resolve and validate
   an absolute executable path before spawning it, and test rejection of bare or
   relative paths.

4. **Native smoke tests replace the production collectors with fixtures**
   Locations: `.github/workflows/native-smoke.yml:18`,
   `tests/e2e/native.spec.ts:2`, `src-tauri/src/lib.rs:38`
   Every native smoke job sets `CACHEBITE_E2E_FIXTURES=1`, while the test claims
   to start the production composition root without fixture routing. Its
   assertion only excludes the renderer fixture text and cannot detect the
   backend fixture collectors. Add a smoke path using real collector composition
   with absent credentials treated as an expected state, or expose and assert
   the selected backend collector mode.

### MEDIUM

1. **Concurrent settings writes can revert newer choices**
   Location: `src/App.svelte:317`
   Multiple `changeSettings` calls can overlap and resolve out of order. An older
   response can overwrite a newer selection in both `appSettings` and the store.
   Serialize settings writes or discard stale responses using request versions.

2. **Persistence failures are silently discarded**
   Location: `src-tauri/src/refresh/actor.rs:230`
   Snapshot and history write errors are ignored, so disk-full, permission, or
   corrupt-store failures can lose data while the UI reports success. Log errors
   with provider context and expose or retry degraded persistence where
   appropriate.

3. **Coverage thresholds exclude the application composition root**
   Location: `vite.config.ts:21`
   The 80% gate includes only `src/lib/**/*`, excluding `App.svelte`, `main.ts`,
   and other application-level orchestration. Include the complete application
   source set and narrowly document any necessary exclusions.

4. **Window-position persistence is not tested**
   Location: `src/lib/api/gateway.ts:164`
   The production path performs DPI conversion, debouncing, timer replacement,
   and cleanup flushing, but tests never exercise it. Add fake-timer coverage for
   repeated moves, scale factors, a single debounced save, and unlisten flushing.

5. **Native E2E does not exercise registered Tauri commands**
   Location: `tests/e2e/native.spec.ts:2`
   The test checks shell presence only. It does not validate command
   registration, authorization, error mapping, or renderer/Rust serialization.
   Exercise overlay hydration and representative panel settings/history flows
   through native IPC.

6. **GitHub Actions use mutable references**
   Locations: `.github/workflows/ci.yml`,
   `.github/workflows/native-smoke.yml`, `.github/workflows/release.yml`
   Several actions use release tags rather than full commit SHAs, including jobs
   associated with macOS signing. Pin actions to reviewed commit SHAs and use
   Dependabot to update the pins.

### LOW

None.

## Validation Results

| Check | Result |
| --- | --- |
| Svelte diagnostics | Pass — 0 errors and 0 warnings |
| ESLint and Prettier | Pass |
| Frontend tests | Pass — 98 tests |
| Configured frontend coverage | Pass — 87.42% statements |
| Renderer production build | Pass |
| Rust formatting | Pass |
| Rust tests | Blocked — local `pkg-config`/GLib prerequisites missing |
| Rust Clippy | Blocked — local `pkg-config`/GLib prerequisites missing |

No hardcoded secrets, IPC authorization bypasses, or path-traversal
vulnerabilities were found.

## Recommendation

Block commit until the HIGH findings are fixed and covered by regression tests.
Re-run the Rust test and Clippy checks in an environment with the required Tauri
native prerequisites before merging.
