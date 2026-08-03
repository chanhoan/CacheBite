# Implementation Report: Pet Double-Click Panel Toggle

## Summary

Implemented issue #48 so a pet double-click toggles the usage panel from native
visibility state. Hidden or absent-from-screen panels begin the existing
layout-gated reveal; visible or pending panels are hidden without exiting
CacheBite. The renderer sends one toggle request and keeps no visibility copy.

## Assessment vs Reality

| Metric | Predicted (Plan) | Actual |
| --- | --- | --- |
| Complexity | Medium | Medium |
| Confidence | Not stated | High after full automated validation |
| Files Changed | 12 code/test + 3 docs | 15 implementation files |

## Tasks Completed

| # | Task | Status | Notes |
| --- | --- | --- | --- |
| 1-4 | Native toggle policy and authorization | Complete | Compile-time RED was staged before `PanelToggle` existed; GREEN covers all four visibility/pending combinations and overlay-only authorization. |
| 5-7 | IPC toggle, layout gate, and hide-path integration | Complete | Pending reveals are disarmed for toggle, close control, hotkey, and fullscreen hides. |
| 8-11 | Gateway, fixture, component, and app wiring | Complete | Renderer discards `PanelVisibility` and stores no panel state. |
| 12-15 | Component, gateway, app, and native E2E tests | Complete | Added keyboard parity, three-toggle cycle, and panel-window rejection coverage. |
| 16-17 | UI contract, beta guide, and invariant docs | Complete | Existing install/security/reporting guidance preserved. |
| 18 | Full validation and diff audit | Complete | No active stale references or capability changes. |

## Validation Results

| Level | Status | Notes |
| --- | --- | --- |
| Static Analysis | Pass | `pnpm check`, `cargo fmt --check`, clippy with all targets/features and `-D warnings`, ESLint, Prettier |
| Unit Tests | Pass | 116 Rust tests and 247 renderer tests in full suites; 68 targeted renderer tests |
| Build | Pass | Vite production build and Tauri debug WebDriver smoke build |
| Integration / E2E | Pass | Renderer: 2 specs / 8 tests; native fixture suite passed after CI-equivalent WebDriver build |
| Edge Cases | Pass | Pending reveal cancellation, overlay-only authorization, close/Quit separation, and three-toggle alternation covered |

Coverage: 95.79% statements, 90.4% branches, 93.61% functions, and
95.79% lines.

## Files Changed

| File | Action | Lines |
| --- | --- | --- |
| `src-tauri/src/window/mod.rs` | Updated | +21 / -20 |
| `src-tauri/src/window/tests.rs` | Updated | +11 / -5 |
| `src-tauri/src/refresh/ipc.rs` | Updated | +81 / -28 |
| `src-tauri/src/lib.rs` | Updated | +10 / -2 |
| `src/lib/api/gateway.ts` | Updated | +10 / -2 |
| `src/lib/api/fixtureGateway.ts` | Updated | +1 / -1 |
| `src/App.svelte` | Updated | +1 / -1 |
| `src/lib/components/PetOverlay.svelte` | Updated | +5 / -5 |
| `src/lib/components/PetOverlay.test.ts` | Updated | +28 / -2 |
| `src/App.test.ts` | Updated | +9 / -5 |
| `src/lib/api/gateway.test.ts` | Updated | +2 / -2 |
| `tests/e2e/native.spec.ts` | Updated | +37 |
| `docs/ui-contract.md` | Updated | +4 / -3 |
| `docs/beta-testing.md` | Updated | +7 / -4 |
| `CLAUDE.md` | Updated | +3 / -3 |

## Deviations from Plan

- Validation used `corepack pnpm` because `pnpm` was not directly available on
  PowerShell `PATH`; Corepack selected the pinned `pnpm@10.15.1`.
- Native E2E required the repository's CI environment variables and a debug
  build with `--features webdriver`; running `pnpm test:e2e` without those did
  not start tests.
- During review, an accidental `WindowCommand` rename and retained
  `PanelReveal` policy were corrected to match the plan. An over-broad beta-guide
  rewrite was restored to a scoped three-hunk documentation change.

## Issues Encountered

- Delegated child workers inherited a read-only shell. The primary coder made
  the changes, and the parent performed the final corrections and validation.
- The first native E2E attempt used a non-WebDriver debug binary and timed out
  during service startup. Rebuilding with the repository's CI command resolved
  it; the suite then passed.

## Tests Written

| Test File | Tests | Coverage |
| --- | --- | --- |
| `src-tauri/src/window/tests.rs` | 1 policy test replaced + authorization assertions | Four visible/pending combinations and overlay-only permission |
| `src/lib/components/PetOverlay.test.ts` | 1 added | Double-click and Enter route to the same callback |
| `src/App.test.ts` | 2 existing flows expanded | Three-toggle cycle and close/Quit separation |
| `tests/e2e/native.spec.ts` | 2 added | Native alternating cycle and panel-window rejection |

## Next Steps

- [ ] Review via `$code-review`
- [ ] Commit and open a pull request when ready
