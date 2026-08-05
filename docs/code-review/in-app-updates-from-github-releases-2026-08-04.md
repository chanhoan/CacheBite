# Code Review: In-App Updates From GitHub Releases

**Reviewed**: 2026-08-04  
**Scope**: Local uncommitted changes, including tracked diffs and untracked updater files  
**Decision**: REQUEST CHANGES

## Findings

### CRITICAL

None.

### HIGH

**[HIGH]** `src/App.svelte:714`

Issue: The update banner always wires its primary action to `gateway.installUpdate()`, but the failed-state view model labels that same button `Try again` (`src/lib/state/updatePresentation.ts:109` and `src/lib/state/updatePresentation.ts:114`). Clicking `Try again` therefore invokes `install_update`, not `check_for_update`. Native `UpdateService::install` intentionally returns immediately unless the state is `Available` (`src-tauri/src/update/service.rs:174` and `src-tauri/src/update/service.rs:178`), so the visible recovery action after an update failure is a no-op.

Fix: Make the banner action explicit, for example `primaryAction: 'install' | 'check'`, and dispatch `failed` to `gateway.checkForUpdate()` while keeping `available` on `gateway.installUpdate()`. Add an App-level regression test that emits a failed update state, clicks `Try again`, and asserts `checkForUpdate` was called and `installUpdate` was not.

### MEDIUM

**[MEDIUM]** `src/App.svelte:281`

Issue: The failed-state view model marks the notice as dismissible (`src/lib/state/updatePresentation.ts:116`), so `UpdateNotice` renders a `Later` button. However, `dismissUpdate` only records a dismissal when the current status is `available`; for `failed` it does nothing. The failed-state component test explicitly expects `Later`, but no App-level test clicks it. Users therefore see a second enabled recovery control that has no effect.

Fix: Resolve the failure-state contract explicitly. Either make `failed.dismissible` false and remove `Later`, or track a session-scoped failed-notice dismissal separately and test that clicking `Later` hides the failed notice without hiding Settings status.

**[MEDIUM]** `tests/e2e/native.spec.ts:357`

Issue: The native E2E suite has a failed-update branch, but the workflows only run the fixture suite with `CACHEBITE_E2E_UPDATE: available` (`.github/workflows/native-smoke.yml:22` and `.github/workflows/native-smoke.yml:50`). That means the failed-state UI path is currently dead in CI, and the existing failed test only checks that `Try again` is enabled (`tests/e2e/native.spec.ts:364`), not that it triggers a new check. This gap allowed the no-op retry regression above to pass the full current validation set.

Fix: Add a failed-update fixture matrix entry, or a separate native smoke job with `CACHEBITE_E2E_UPDATE: failed`, and make the failed test click `Try again` and verify the state moves through another check or that the fixture probe count increments.

### LOW

None.

## Summary

The updater architecture is mostly well-contained: the renderer does not get direct updater plugin permissions, failure details are typed, release notes are bounded, and signed manifest generation has a self-test. The blocking issue is the failed-update retry path: the UI presents a recovery action that cannot recover.

## Validation Results

| Check | Result |
|---|---|
| `cargo test --manifest-path src-tauri/Cargo.toml --all-features` | Pass, 142 tests |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-features -- -D warnings` | Pass |
| `python scripts/build_updater_manifest.py --self-test` | Pass |
| `corepack pnpm test:ci` | Pass: type check, lint, formatting, 25 files / 281 tests, coverage gates, production build |
| `corepack pnpm test:e2e:renderer` | Pass, 2 specs / 8 tests |
| `corepack pnpm audit:ci` | Pass at high-severity gate; 1 low and 11 moderate advisories reported |
| `git diff --check HEAD` | Pass |
| `pnpm check` / `pnpm test` | Not run directly; `pnpm` is not on PATH in this shell, so the equivalent `corepack pnpm ...` commands were used |

## Files Reviewed

### Source / Config

- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock` (dependency lockfile; validated through Cargo build/test/clippy)
- `src-tauri/tauri.conf.json`
- `src-tauri/src/lib.rs`
- `src-tauri/src/refresh/ipc.rs`
- `src-tauri/src/window/mod.rs`
- `src-tauri/src/update/channel.rs`
- `src-tauri/src/update/feed.rs`
- `src-tauri/src/update/ipc.rs`
- `src-tauri/src/update/mod.rs`
- `src-tauri/src/update/service.rs`
- `src-tauri/src/update/state.rs`
- `src/App.svelte`
- `src/lib/api/fixtureGateway.ts`
- `src/lib/api/gateway.ts`
- `src/lib/components/SettingsPanel.svelte`
- `src/lib/components/UpdateNotice.svelte`
- `src/lib/state/updatePresentation.ts`
- `scripts/build_updater_manifest.py`

### Workflows

- `.github/workflows/ci.yml`
- `.github/workflows/native-smoke.yml`
- `.github/workflows/release.yml`
- `.github/workflows/updater-manifest.yml`

### Tests

- `src-tauri/src/window/tests.rs`
- `src-tauri/src/update/tests.rs`
- `src/App.test.ts`
- `src/lib/components/SettingsPanel.test.ts`
- `src/lib/components/UpdateNotice.test.ts`
- `src/lib/state/updatePresentation.test.ts`
- `src/nativeWorkflow.test.ts`
- `src/securityConfig.test.ts`
- `tests/e2e/native.spec.ts`

### Documentation / Planning

- `CLAUDE.md`
- `docs/architecture.md`
- `docs/beta-testing.md`
- `docs/ui-contract.md`
- `.claude/PRPs/plans/completed/in-app-updates-from-github-releases.plan.md`
- `.claude/PRPs/reports/in-app-updates-from-github-releases-report.md`

## Residual Risk

I did not run native WebDriver smoke tests or real updater install flows. The review relies on unit coverage and static workflow inspection for native update behavior; macOS/Linux replacement is also explicitly called out as unconfirmed on real hardware. The dependency audit passed the configured high-severity gate but still reported 12 lower-severity advisories.
