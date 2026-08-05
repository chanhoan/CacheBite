# Plan: Secure in-app updates from GitHub Releases (Issue #49)

## Summary

CacheBite gains a native update service that reads a signed channel manifest hosted as a GitHub
Release asset, compares it against the running version, and offers an in-panel **Install / Later**
notice. Installing downloads the correct platform artifact, verifies its minisign signature
(fail-closed, enforced by `tauri-plugin-updater`), installs over the current build, and relaunches —
no browser, no separate CacheBite server, no credentials anywhere in the path.

## User Story

As a CacheBite beta tester,
I want the app to tell me a newer build exists and install it for me,
So that I stop having to watch GitHub Releases and hand-install every beta.

## Problem → Solution

**Current state:** the tester must notice a release, open GitHub, pick the right installer, and
install it over the old build. `docs/beta-testing.md` documents this as "There is no auto-updater."

**Desired state:** the app checks a channel manifest on a throttle, shows the target version in the
panel with explicit Install / Later, and — on Install — downloads, verifies, installs and restarts
itself. Every failure mode (offline, rate limit, bad metadata, missing artifact, download failure,
signature failure, install failure) is a recoverable status that never blocks normal use.

## Metadata

- **Complexity**: Large (≈24 files, ≈1200 lines including tests)
- **Source PRD**: N/A — GitHub Issue #49
- **PRD Phase**: N/A — standalone
- **Estimated Files**: 24 (11 created, 13 updated)

---

## Feasibility verdict (verified, not assumed)

Source-verified against `tauri-plugin-updater` v2 (`src/updater.rs`, 1742 lines) and
`tauri-bundler` (`crates/tauri-bundler/src/bundle/windows/{msi,nsis}/mod.rs`).

| Question | Verdict | Evidence |
|---|---|---|
| Can the app install and relaunch itself, without opening a browser? | **Yes** | `UpdaterContext.restart_after_install` defaults to `true` (`updater.rs:197`). Windows: NSIS gets `/UPDATE` + restart args + `/ARGS`, then `std::process::exit(0)` (`updater.rs:876-905`); the installer relaunches the app. macOS: the `.app.tar.gz` is extracted and the bundle replaced in-process, then `app.restart()`. Linux: the AppImage is rewritten in place, then `app.restart()`. |
| Is verification fail-closed? | **Yes** | minisign verification is unconditional in `Update::download`; there is no bypass flag. |
| Can the client pick the release at runtime? | **Yes** | `UpdaterBuilder::endpoints(Vec<String>)` overrides config endpoints. Config endpoints must still be non-empty (`Error::EmptyEndpoints`). |
| Does `darwin-universal` work as a platform key? | **No** | `Updater::get_urls` searches only `{os}-{arch}-{installer}` then `{os}-{arch}` (`updater.rs:~610-640`). There is no universal fallback. **The manifest must list `darwin-x86_64` and `darwin-aarch64` pointing at the same universal artifact.** |
| Does `releases/latest/download/...` work for our releases? | **No** | GitHub excludes pre-releases from `/releases/latest`, and every CacheBite release so far is `v0.1.0-beta.N`. Resolved by the fixed-tag manifest release below. |
| How is "a newer release exists" actually decided? | **Plain semver `>` on one fetched file** | `Updater::check()` GETs the manifest, parses `RemoteRelease` (the `version` field, `name` alias, leading `v` stripped), and with no custom comparator evaluates `release.version > self.current_version`. `self.current_version` is `app.package_info().version` (`UpdaterBuilder::new`, `updater.rs:199`) — i.e. `tauri.conf.json`'s `version`, which is why Task 9 derives it from the tag. A `204 No Content` response is `Ok(None)`; a non-2xx body is logged and the next endpoint is tried. |
| Does a missing platform key read as "up to date"? | **No — it is an error** | `get_urls` is called *before* the `should_update` branch is consumed and propagates with `?`, so an omitted key returns `Err(TargetsNotFound)` on **every** check, including when the client already matches the manifest version. See the Task 3 note. |
| Can `tauri.conf.json` carry a pre-release version? | **NSIS yes, MSI no** | `nsis::try_add_numeric_build_number` only rewrites *build* metadata; the pre-release is preserved and `VIProductVersion` becomes `major.minor.patch.0`. `msi::convert_version` **bails** on a non-numeric pre-release ("optional pre-release identifier in app version must be numeric-only"). Resolved by injecting `bundle.windows.wix.version` (`msi/mod.rs:474-485` reads `settings.windows().wix.version` and skips `convert_version` when present). |

### Blocker found and resolved: the version is currently wrong for updates

`src-tauri/tauri.conf.json` has `"version": "0.1.0"` while tags are `v0.1.0-beta.3`. The updater's
`current_version` is `app.package_info().version`, i.e. the config version. Semver says
`0.1.0 > 0.1.0-beta.4`, so **a shipped beta would never be offered an update**. This plan makes the
git tag the single source of truth: CI derives `version` (and the numeric `wix.version`) from the
tag and merges them via `tauri build --config`.

### Bootstrap limitation (must be documented, cannot be engineered away)

Builds `v0.1.0-beta.1` … `v0.1.0-beta.3` contain no updater plugin. They will never auto-update.
The first updater-enabled release must be installed by hand; only the release *after* it updates
automatically. This goes in the release notes and `docs/beta-testing.md`.

---

## Decisions taken (confirmed with the requester)

1. **Metadata hosting** — a fixed-tag GitHub Release named `updater` holds `stable.json` and
   `beta.json`. A `release: published` workflow regenerates and `--clobber`-uploads them. Endpoint
   URLs are therefore constant, CDN-backed, rate-limit-free, and only ever reflect **published**
   releases (drafts publish nothing).
2. **Channel policy** — derived from the installed version, no new setting. A pre-release version
   (`0.1.0-beta.4`) uses `beta.json`; a release version (`0.1.0`) uses `stable.json`.
   `stable.json` only ever lists release versions, so a stable user can never be pulled onto beta.
3. **Platform scope** — artifacts, signatures and manifest entries are produced for all three
   platforms and all three attempt installation with identical UI and no platform branching in the
   client. macOS and Linux in-place replacement is labelled "not yet verified on real hardware" in
   `docs/beta-testing.md` and the release notes until someone confirms it.

---

## UX Design

### Before

```
┌──────────────────────────────────────┐
│  CacheBite panel                     │
│  ┌────────────────────────────────┐  │
│  │ Claude   Codex                 │  │
│  │ 5-hour   ████████░░  81%       │  │
│  │ Weekly   ███░░░░░░░  31%       │  │
│  │ ● Fresh · captured 2m ago      │  │
│  ├────────────────────────────────┤  │
│  │ [Refresh now] [Set as primary] │  │
│  │ [Settings]            [Quit]   │  │
│  └────────────────────────────────┘  │
└──────────────────────────────────────┘
   A newer release exists. Nothing here says so.
   Tester → browser → Releases → download → reinstall.
```

### After

```
┌──────────────────────────────────────┐
│  CacheBite panel                     │
│  ┌────────────────────────────────┐  │
│  │ ▲ Update available  0.1.0-b5   │  │  ← UpdateNotice.svelte, above the tabs
│  │   [Install and restart] [Later]│  │
│  ├────────────────────────────────┤  │
│  │ Claude   Codex                 │  │
│  │ 5-hour   ████████░░  81%       │  │
│  │ ...                            │  │
│  └────────────────────────────────┘  │
└──────────────────────────────────────┘

  Install pressed →
  │ ▲ Downloading 0.1.0-beta.5  ▓▓▓▓░░ 64%   │   (Later hidden, Install disabled)
  │ ▲ Installing 0.1.0-beta.5…               │   → app exits and relaunches

  Failure →
  │ ▲ Update failed — check your connection   │
  │   [Try again]                    [Later]  │   panel stays fully usable

Settings view (always reachable, even after "Later"):
  ┌────────────────────────────────┐
  │ Version           0.1.0-beta.4 │
  │ Updates    Up to date · 3m ago │
  │ [Check for updates]            │
  └────────────────────────────────┘
```

### Interaction Changes

| Touchpoint | Before | After | Notes |
|---|---|---|---|
| Panel open, update available | nothing | `UpdateNotice` banner above `ProviderTabs` | Panel height changes → existing `ResizeObserver` → `resize_panel`. No new resize plumbing. |
| Opening the panel | usage refresh only | also nudges an update check, at most every 15 min | The reveal is the only moment the notice is visible, so it is the only moment worth a check. Never blocks or fails the reveal. |
| Install action | n/a | download → verify → install → relaunch | Windows relaunches via NSIS; macOS/Linux via `app.restart()`. |
| Later action | n/a | banner hidden for that version, this session only | Reappears next launch. Deliberately not persisted — see NOT Building. |
| Settings | no version shown | version row + status + `Check for updates` | Satisfies "provide a manual check in Settings". Bypasses the throttle. |
| Offline / failed | n/a | typed failure line + `Try again`; panel fully usable | Never blocks usage collection or any other panel action. |
| Overlay (pet) | unchanged | unchanged | Deliberately untouched. |

---

## Mandatory Reading

| Priority | File | Lines | Why |
|---|---|---|---|
| P0 | `src-tauri/src/window/mod.rs` | 62-109 | `NativeCommand` + `command_allowed`. Three new commands must be added here or they are silently denied at runtime. |
| P0 | `src-tauri/src/lib.rs` | 12-51 | **Both** `apply_invoke_handler` arms (`webdriver` / `not(webdriver)`) must list every new command. |
| P0 | `src-tauri/src/lib.rs` | 77-178 | `setup` closure — where the plugin, `UpdateService` and the startup check are wired; shows the `app.manage(...)` ordering discipline. |
| P0 | `src/lib/api/gateway.ts` | 79-110, 166-269 | `AppGateway` contract + `tauriGateway`. Every DTO is declared here; `invokeNative` is the only renderer→native boundary. |
| P0 | `src-tauri/src/refresh/ipc.rs` | 133-192, 557-595 | `authorize()` helper, `IpcError` enum, and `emit_provider_states` — the exact event-emission pattern the update service mirrors. |
| P0 | `src-tauri/src/refresh/ipc.rs` | 355-441 | `reveal_panel` / `conceal_panel` / `begin_reveal` / `toggle_panel`. `begin_reveal` is the **only** path that puts the panel on screen, which is why the panel-open update check hooks in there. |
| P1 | `src-tauri/src/refresh/actor.rs` | 131-151 | `SchedulerConfig::default()` — `poll_interval` is 15 min per provider. This is the traffic baseline the update cadence is calibrated against. |
| P1 | `src-tauri/src/collectors/claude.rs` | 149-175 | `secure_client()` + `MAX_RESPONSE_BYTES` guard — the network discipline any new HTTP work must match. |
| P1 | `src-tauri/src/store/settings.rs` | 12-53, 121-160 | Repository shape, `SETTINGS_SCHEMA_VERSION`, atomic write. Read to confirm **no schema bump is needed**. |
| P1 | `src/App.svelte` | 630-690 | Panel render block — exact insertion point for `UpdateNotice` and the `SettingsPanel` props. |
| P1 | `src/lib/components/SettingsPanel.svelte` | 1-36, 109-127 | Props typedef style (JSDoc `$props()`), and the read-only `.field` + `.field-help` markup the version row mirrors. |
| P1 | `.github/workflows/release.yml` | 20-70, 76-125 | `package` matrix and the `publish` job's `Collect installers` step — both change. |
| P2 | `src/lib/interaction/notificationPolicy.ts` | 1-60 | The pure-reducer + typed-diagnostic style the update state machine mirrors. |
| P2 | `src/nativeWorkflow.test.ts` | 1-68 | `extractJob` / `extractNamedStep` / `extractRunCommands` helpers for asserting on workflow YAML. |
| P2 | `src/securityConfig.test.ts` | 1-60 | How `tauri.conf.json` invariants are asserted from the renderer test suite. |
| P2 | `src/lib/api/fixtureGateway.ts` | all | Must mirror every new `AppGateway` method or `svelte-check` fails. |

## External Documentation

| Topic | Source | Key Takeaway |
|---|---|---|
| Updater plugin config & API | https://v2.tauri.app/plugin/updater/ | `bundle.createUpdaterArtifacts: true`; `plugins.updater.{pubkey,endpoints,windows.installMode}`; artifacts are `*-setup.exe(+.sig)`, `*.msi(+.sig)`, `*.app.tar.gz(+.sig)`, `*.AppImage(+.sig)`. Signature validation is mandatory and cannot be disabled. |
| Target resolution | `tauri-plugin-updater` `src/updater.rs` `Updater::get_urls` | Searches `{os}-{arch}-{installer}` then `{os}-{arch}` only. Installer names: `nsis`, `msi`, `appimage`, `deb`, `rpm`. **No `darwin-universal` fallback.** |
| Windows restart | `updater.rs:876-935` | `restart_after_install` defaults `true`; NSIS receives `/UPDATE` + `nsis_restart_after_install_args()` + `/ARGS <escaped current args>`; process then `exit(0)`. |
| Linux constraint | `updater.rs` `install_appimage` | Requires the temp dir and the AppImage to be on the **same mount point** (`dev()` equality) and the AppImage to be writable, else `Error::TempDirNotOnSameMountPoint`. |
| MSI version constraint | `tauri-bundler` `msi/mod.rs:342-370, 474-485` | `convert_version` bails on non-numeric pre-release; `bundle.windows.wix.version` bypasses it entirely. |
| NSIS version handling | `tauri-bundler` `nsis/mod.rs:150-170` | Only *build metadata* is rewritten; the full semver string (pre-release included) is used as `version`. |
| Signing keys | https://v2.tauri.app/plugin/updater/ | `tauri signer generate`; `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` must be **real environment variables** — `.env` files are ignored. |

---

## Patterns to Mirror

### NAMING_CONVENTION — native module layout
```rust
// SOURCE: src-tauri/src/window/mod.rs:62-78 and src-tauri/src/refresh/mod.rs
// Domain-named modules under src-tauri/src/, each with a `mod.rs`, pure policy
// functions at module level, and `#[cfg(test)] mod tests;` at the bottom.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeCommand {
    GetCollectorMode,
    GetProviderStates,
    // ...
}
```

### ERROR_HANDLING — typed, serialised, renderer-safe
```rust
// SOURCE: src-tauri/src/refresh/ipc.rs:142-152
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IpcError {
    Forbidden,
    ServiceUnavailable,
    InvalidSettings,
    // ...
}
```
Every command returns `Result<T, IpcError>`; raw `reqwest`/`io` errors are never propagated.

### AUTHORIZATION — per-window allowlist, checked first in every command
```rust
// SOURCE: src-tauri/src/refresh/ipc.rs:188-205
fn authorize(window: &tauri::WebviewWindow, command: NativeCommand) -> Result<(), IpcError> {
    command_allowed(window.label(), command)
        .then_some(())
        .ok_or(IpcError::Forbidden)
}

#[tauri::command]
pub fn get_provider_states(
    window: tauri::WebviewWindow,
    service: State<'_, RefreshService>,
) -> Result<ProviderStatesDto, IpcError> {
    authorize(&window, NativeCommand::GetProviderStates)?;
    // ...
}
```

### LOGGING_PATTERN — `eprintln!`, debug-gated for chatter, never secrets
```rust
// SOURCE: src-tauri/src/lib.rs:93-95 and src-tauri/src/refresh/ipc.rs:349-351
if let Err(error) = install_bundled_pet_packages(&app.path().resource_dir()?, &app_data) {
    eprintln!("failed to install bundled pet packages: {error}");
}
// SOURCE: src-tauri/src/refresh/ipc.rs:565-588 — verbose diagnostics behind cfg
#[cfg(debug_assertions)]
eprintln!("[CacheBite:{:?}] emit has_snapshot={} ...", state.provider, ...);
```

### EVENT_EMISSION — background task, break when the app is gone
```rust
// SOURCE: src-tauri/src/refresh/ipc.rs:557-595
pub fn emit_provider_states(app: &AppHandle, service: &RefreshService) {
    let app = app.clone();
    let mut states = service.subscribe(provider);
    tauri::async_runtime::spawn(async move {
        while states.changed().await.is_ok() {
            let state: ProviderStateDto = states.borrow_and_update().clone().into();
            if app.emit(PROVIDER_STATE_EVENT, state).is_err() {
                break;
            }
        }
    });
}
```

### NETWORK_DISCIPLINE — https-only, timeout, bounded body
```rust
// SOURCE: src-tauri/src/collectors/claude.rs:151-158 and collectors/mod.rs:10
pub const MAX_RESPONSE_BYTES: usize = 1024 * 1024;

fn secure_client() -> Result<reqwest::Client, CollectorError> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| CollectorError::Internal)
}
```
> The updater plugin owns its own client and **does** follow redirects (required: GitHub release
> asset URLs 302 to `objects.githubusercontent.com`). Do not try to reuse `secure_client()` for it.

### PURE_POLICY — decision extracted from the side effect so both branches are testable
```rust
// SOURCE: src-tauri/src/window/mod.rs:130-136
pub fn panel_toggle(visible: bool, reveal_pending: bool) -> PanelToggle {
    if visible || reveal_pending {
        PanelToggle::Hide
    } else {
        PanelToggle::Show
    }
}
// SOURCE: src-tauri/src/window/mod.rs:165-167
pub fn should_restore_overlay_after_fullscreen(user_hidden: bool) -> bool {
    !user_hidden
}
```

### RUST_TEST_STRUCTURE
```rust
// SOURCE: src-tauri/src/lib.rs:528-556
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collector_mode_distinguishes_fixture_from_production_composition() {
        assert_eq!(
            CollectorModeDto::for_fixture_gate(true),
            CollectorModeDto { claude: CollectorMode::Fixture, codex: CollectorMode::Fixture }
        );
    }
}
```

### GATEWAY_WIRE_MAPPING — snake_case wire, camelCase model
```ts
// SOURCE: src/lib/api/gateway.ts:112-152
type SettingsWire = { schema_version: number; primary_provider: Provider; /* ... */ };
const fromSettings = (wire: SettingsWire): AppSettings => ({
  schemaVersion: wire.schema_version,
  primaryProvider: wire.primary_provider,
  // ...
});
export const tauriGateway: AppGateway = {
  getCollectorMode: () => invokeNative('get_collector_mode'),
  async listenSettings(next) {
    return listen<SettingsWire>('settings-updated', (event) => next(fromSettings(event.payload)));
  },
};
```
> `PetSummary` uses `#[serde(rename_all = "camelCase")]` on the Rust side and therefore needs no
> mapping (`gateway.ts:189-191`). Do the same for the update DTO and skip the mapper entirely.

### TS_DIAGNOSTIC_UNION — the shape used for every "might not work here" report
```ts
// SOURCE: src/lib/api/gateway.ts:56-58
export type CapabilityDiagnostic =
  | { readonly status: 'available' }
  | { readonly status: 'unavailable'; readonly reason: string };
```

### SVELTE_COMPONENT — JSDoc-typed `$props()` in a `.svelte` file
```svelte
<!-- SOURCE: src/lib/components/SettingsPanel.svelte:1-12 -->
<script>
  /** @type {{ settings: import('../state/presentation').SettingsStoreState; onChange?: (s: ...) => void }} */
  let {
    settings,
    onChange = () => {},
  } = $props();
</script>
```

### SVELTE_TEST_STRUCTURE
```ts
// SOURCE: src/lib/components/UsagePanel.test.ts
import { render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
```

### WORKFLOW_ASSERTION — CI contracts are unit-tested from the renderer suite
```ts
// SOURCE: src/nativeWorkflow.test.ts:1-11, 147-157
const releaseWorkflow = readFileSync('.github/workflows/release.yml', 'utf8');
it('keeps webdriver disabled in production release builds', () => {
  expect(extractRunCommands(extractJob(releaseWorkflow, 'package'))).toContain(/* ... */);
});
```

---

## Architecture

```
                       ┌──────────────────────────── GitHub ────────────────────────────┐
  tag push v0.1.0-b5 → │ release.yml (package)                                          │
                       │   pnpm tauri build --config <generated release config>         │
                       │   → *-setup.exe(+.sig) *.msi(+.sig)                            │
                       │     *.app.tar.gz(+.sig) *.AppImage(+.sig)                      │
                       │ release.yml (publish) → DRAFT release + all artifacts + sums   │
                       └────────────────────────────────────────────────────────────────┘
                                          │  human clicks "Publish"
                                          ▼
                       ┌────────────────────────────────────────────────────────────────┐
                       │ updater-manifest.yml   on: release: [published]                 │
                       │   scripts/build_updater_manifest.py                             │
                       │   → beta.json   (always, if newer than current)                 │
                       │   → stable.json (only when the tag has no pre-release)          │
                       │   gh release upload updater *.json --clobber                    │
                       └────────────────────────────────────────────────────────────────┘
                                          │  stable, CDN-backed URL
                                          ▼
  ┌── CacheBite (native) ────────────────────────────────────────────────────────────────┐
  │ update::channel_for_version("0.1.0-beta.4") → Beta → manifest_url(Beta)              │
  │ update::UpdateService (actor, mirrors refresh::RefreshHandle)                        │
  │   ├─ startup +30s · sweep every 1h · on panel reveal (15m floor) · manual (no floor) │
  │   │     all four share one pure `update::should_check`, differing only by interval    │
  │   ├─ app.updater_builder().endpoints([url]).build()?.check()                         │
  │   │     ↳ plugin: fetch manifest → pick {os}-{arch}[-installer] → download → minisign│
  │   └─ emits `update-state` events (mirrors PROVIDER_STATE_EVENT)                      │
  │ update::ipc  get_update_state / check_for_update / install_update  (panel only)      │
  └──────────────────────────────────────────────────────────────────────────────────────┘
                                          │ typed gateway only
                                          ▼
  ┌── renderer ─────────────────────────────────────────────────────────────────────────┐
  │ gateway.ts  UpdateStateWire + 4 methods → fixtureGateway.ts mirrors them             │
  │ state/updatePresentation.ts  (pure)  wire → UpdateViewModel                          │
  │ components/UpdateNotice.svelte  banner: Install / Later / Try again                  │
  │ components/SettingsPanel.svelte  version row + status + Check for updates            │
  │ App.svelte  wiring + session-scoped `dismissedVersion`                               │
  └──────────────────────────────────────────────────────────────────────────────────────┘
```

**Why the commands live in a new `src-tauri/src/update/ipc.rs`** rather than `refresh/ipc.rs`:
`refresh/ipc.rs` is already 679 lines; adding ~150 pushes it past the 800-line ceiling in
`CLAUDE.md`. `CLAUDE.md` must be updated to say commands live in `refresh/ipc.rs` **and**
`update/ipc.rs`.

---

## Files to Change

| File | Action | Justification |
|---|---|---|
| `src-tauri/src/update/mod.rs` | CREATE | Module root; re-exports `UpdateService`, `UpdateStatus`, pure policy fns. |
| `src-tauri/src/update/channel.rs` | CREATE | Pure: `Channel`, `channel_for_version`, `manifest_url`, `should_check`. |
| `src-tauri/src/update/state.rs` | CREATE | Pure: `UpdateStatus`, `UpdateFailure`, `UpdateStateDto`, note truncation. |
| `src-tauri/src/update/service.rs` | CREATE | `UpdateService` actor; owns `watch::Sender<UpdateStatus>`; drives `ReleaseFeed`. |
| `src-tauri/src/update/feed.rs` | CREATE | `trait ReleaseFeed` + `TauriUpdaterFeed` (production) + `FixtureFeed` (E2E/tests). |
| `src-tauri/src/update/ipc.rs` | CREATE | `get_update_state`, `check_for_update`, `install_update`, `emit_update_state`. |
| `src-tauri/src/update/tests.rs` | CREATE | Unit tests for channel, throttle, state, DTO privacy. |
| `src-tauri/src/window/mod.rs` | UPDATE | Add 3 `NativeCommand` variants + `panel` allowlist entries + tests. |
| `src-tauri/src/refresh/ipc.rs` | UPDATE | One non-fatal `try_state` nudge in `begin_reveal` — revealing the panel is the only moment the notice can be seen. No other change; the file stays under the 800-line ceiling. |
| `src-tauri/src/lib.rs` | UPDATE | `pub mod update;`, plugin init, `app.manage(UpdateService)`, spawn `emit_update_state`, startup check. Both `apply_invoke_handler` arms. |
| `src-tauri/Cargo.toml` | UPDATE | `tauri-plugin-updater = "2.10"`, `semver = "1"`. |
| `src-tauri/tauri.conf.json` | UPDATE | `plugins.updater` block (pubkey, endpoints, `windows.installMode: "passive"`). `createUpdaterArtifacts` stays `false`. |
| `src/lib/api/gateway.ts` | UPDATE | `UpdateStateWire`, `UpdateFailureReason`, 4 `AppGateway` methods + `tauriGateway` impls. |
| `src/lib/api/fixtureGateway.ts` | UPDATE | Mirror the 4 methods. |
| `src/lib/state/updatePresentation.ts` | CREATE | Pure wire → `UpdateViewModel`. |
| `src/lib/state/updatePresentation.test.ts` | CREATE | Unit tests for every status and failure reason. |
| `src/lib/components/UpdateNotice.svelte` | CREATE | The banner. |
| `src/lib/components/UpdateNotice.test.ts` | CREATE | Render/interaction tests. |
| `src/lib/components/SettingsPanel.svelte` | UPDATE | Version row, status line, `Check for updates`. |
| `src/lib/components/SettingsPanel.test.ts` | UPDATE | Cover the new controls. |
| `src/App.svelte` | UPDATE | Subscribe, render banner, wire actions, session dismissal. |
| `.github/workflows/release.yml` | UPDATE | Signing env, generated release config, `--bundles app,dmg` on macOS, collect `.sig`/`.tar.gz`. |
| `.github/workflows/updater-manifest.yml` | CREATE | `on: release: [published]` → build + upload channel manifests. |
| `scripts/build_updater_manifest.py` | CREATE | Manifest builder + `--self-test`. |
| `src/nativeWorkflow.test.ts` | UPDATE | Assert the new release/manifest workflow contracts. |
| `src/securityConfig.test.ts` | UPDATE | Assert updater config invariants (pubkey non-empty, endpoints on github.com, https, no `updater:*` capability). |
| `.github/workflows/ci.yml` | UPDATE | Private-key guard; run `python3 scripts/build_updater_manifest.py --self-test`. |
| `tests/e2e/native.spec.ts` | UPDATE | Fixture-driven update-available / no-update / failed assertions. |
| `docs/beta-testing.md` | UPDATE | Replace "no auto-updater"; add the bootstrap note and the mac/Linux caveat. |
| `docs/architecture.md`, `docs/ui-contract.md`, `CLAUDE.md` | UPDATE | Document the new layer, the new commands, and the new invariants. |

**Deliberately unchanged:** `src-tauri/capabilities/overlay.json`, `src-tauri/capabilities/panel.json`
(no `updater:*` permission — the renderer never calls the plugin directly), and
`app.security.csp` in `tauri.conf.json` (updater traffic is native-side, not webview-side).

## NOT Building

- **No persisted "Later"**. Dismissal is renderer-session state. Persisting it needs a
  `SETTINGS_SCHEMA_VERSION` bump (currently 5) plus a migration struct, for a behaviour the issue
  did not ask for. The Settings view always shows the true status, so nothing is hidden permanently.
- **No explicit channel setting in Settings.** Decided: derived from the installed version.
- **No overlay/pet update indicator, no speech bubble, no system notification for updates.**
  `notificationPolicy.ts` stays provider-usage-only.
- **No silent/background auto-install.** The issue explicitly forbids it.
- **No GitHub REST API calls at runtime.** The fixed-tag manifest removes the need and the
  rate-limit exposure.
- **No delta/differential updates, no rollback UI, no update history.**
- **No code signing or notarization work.** Orthogonal to updater artifact signing, and already
  tracked separately (`production-macos-signing` environment).
- **No change to the draft-release-then-human-publishes flow.** The manifest workflow is keyed to
  `release: published` precisely to preserve it.

---

## Step-by-Step Tasks

### Task 0: Generate the signing keypair (human prerequisite — blocks Tasks 8-10)
- **ACTION**: Generate a minisign keypair and register CI secrets. This cannot be automated —
  it produces a private key that must never enter the repository.
- **IMPLEMENT**:
  ```bash
  pnpm tauri signer generate -w "$HOME/.tauri/cachebite.key"
  # → prints the PUBLIC key and writes cachebite.key (private) + cachebite.key.pub
  gh secret set TAURI_SIGNING_PRIVATE_KEY < "$HOME/.tauri/cachebite.key"
  gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD   # the passphrase chosen above
  ```
  Copy the **public** key string into `plugins.updater.pubkey` in Task 8.
- **MIRROR**: n/a (operational).
- **GOTCHA**: The passphrase may be empty, but the secret must still exist — the bundler reads the
  variable unconditionally. `.env` files are ignored by the Tauri CLI; these must be real env vars
  in the workflow. Back the private key up outside the repo: losing it means every installed client
  stops accepting updates until they hand-install a build with a new pubkey.
- **VALIDATE**: `gh secret list` shows both entries.

### Task 1: Pure channel and throttle policy (TDD — RED first)
- **ACTION**: Create `src-tauri/src/update/channel.rs` with no I/O.
- **IMPLEMENT**:
  ```rust
  use std::time::{Duration, Instant};

  /// Which manifest a build is allowed to read.
  ///
  /// Derived from the running version rather than stored: a persisted channel
  /// could record a beta opt-in that no migration would ever undo, which is the
  /// same failure mode that removed the persisted hide/show hotkey in schema v5.
  #[derive(Clone, Copy, Debug, Eq, PartialEq)]
  pub enum Channel { Stable, Beta }

  const STABLE_MANIFEST: &str =
      "https://github.com/chanhoan/CacheBite/releases/download/updater/stable.json";
  const BETA_MANIFEST: &str =
      "https://github.com/chanhoan/CacheBite/releases/download/updater/beta.json";

  /// A pre-release build follows beta; a release build never sees one.
  /// An unparseable version is treated as stable — the conservative direction.
  pub fn channel_for_version(version: &str) -> Channel {
      match semver::Version::parse(version) {
          Ok(parsed) if !parsed.pre.is_empty() => Channel::Beta,
          _ => Channel::Stable,
      }
  }

  pub fn manifest_url(channel: Channel) -> &'static str {
      match channel {
          Channel::Stable => STABLE_MANIFEST,
          Channel::Beta => BETA_MANIFEST,
      }
  }

  pub const STARTUP_CHECK_DELAY: Duration = Duration::from_secs(30);

  /// Background sweep. Deliberately slow: the notice is only ever *seen* when
  /// the panel is open, and `PANEL_OPEN_CHECK_FLOOR` already covers that moment.
  /// This exists so a session that leaves the panel open all day still notices,
  /// and so the state is warm before the panel is revealed — a check that only
  /// started on reveal would pop the banner in a moment later and resize the
  /// panel under the user's cursor.
  pub const AUTOMATIC_CHECK_INTERVAL: Duration = Duration::from_secs(60 * 60);

  /// Minimum gap between checks triggered by revealing the panel. Matches the
  /// provider poll interval (`SchedulerConfig::default().poll_interval`), so
  /// opening the panel never costs more network traffic than the usage
  /// collection the panel exists to show.
  pub const PANEL_OPEN_CHECK_FLOOR: Duration = Duration::from_secs(15 * 60);

  /// Whether an automatic check is due. A manual check from Settings does not
  /// consult this — the user asking is the signal.
  pub fn should_check(last_checked: Option<Instant>, now: Instant, interval: Duration) -> bool {
      match last_checked {
          None => true,
          Some(previous) => now.saturating_duration_since(previous) >= interval,
      }
  }
  ```
  One `should_check` serves all three cadences; only the `interval` argument differs
  (`AUTOMATIC_CHECK_INTERVAL` from the sweep, `PANEL_OPEN_CHECK_FLOOR` from the reveal,
  bypassed entirely for the manual button).
- **MIRROR**: PURE_POLICY (`window/mod.rs:130-136, 165-167`), NAMING_CONVENTION.
- **IMPORTS**: add `semver = "1"` to `src-tauri/Cargo.toml` `[dependencies]`.
- **GOTCHA**: `channel_for_version("0.1.0")` **must** be `Stable`, and `("0.1.0-beta.4")` **must** be
  `Beta`. Getting this backwards silently downgrades every stable user onto betas. Write both tests
  before the implementation.
- **VALIDATE**:
  ```bash
  cargo test --manifest-path src-tauri/Cargo.toml update::channel
  ```

### Task 2: Pure update state machine and DTO
- **ACTION**: Create `src-tauri/src/update/state.rs`.
- **IMPLEMENT**:
  ```rust
  use serde::Serialize;

  /// Why an update attempt did not complete. Typed rather than a message so the
  /// renderer never receives a URL, a response body, or a filesystem path — the
  /// same rule the provider `FailureClass` follows.
  #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
  #[serde(rename_all = "snake_case")]
  pub enum UpdateFailure {
      Offline,
      RateLimited,
      MetadataInvalid,
      ArtifactMissing,
      DownloadFailed,
      VerificationFailed,
      InstallFailed,
  }

  #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
  #[serde(tag = "status", rename_all = "snake_case")]
  pub enum UpdateStatus {
      Idle,
      Checking,
      UpToDate,
      Available { version: String, notes: Option<String> },
      Downloading { received: u64, total: Option<u64> },
      Installing { version: String },
      Failed { reason: UpdateFailure },
  }

  /// Release notes are public, but an unbounded body would resize the panel to
  /// the height of a changelog. Truncated on a char boundary.
  pub const MAX_NOTES_CHARS: usize = 400;

  pub fn truncate_notes(notes: &str) -> Option<String> { /* trim, cap, ellipsis */ }

  #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
  #[serde(rename_all = "camelCase")]
  pub struct UpdateStateDto {
      pub current_version: String,
      pub status: UpdateStatus,
  }
  ```
- **MIRROR**: ERROR_HANDLING (`refresh/ipc.rs:142-152`), TS_DIAGNOSTIC_UNION,
  GATEWAY_WIRE_MAPPING (`rename_all = "camelCase"` so `gateway.ts` needs no mapper).
- **GOTCHA**: `#[serde(tag = "status")]` on a variant carrying `version` produces
  `{"status":"available","version":"…"}` — this is the exact shape `updatePresentation.ts` will
  narrow on. Keep `rename_all = "snake_case"` on the tag values to match `CapabilityDiagnostic`.
- **VALIDATE**: a test asserting `serde_json::to_value(UpdateStatus::Available{..})` has exactly the
  keys `status`, `version`, `notes` — and a test asserting no variant serialises a `url` key.

### Task 3: Release feed abstraction (production + fixture)
- **ACTION**: Create `src-tauri/src/update/feed.rs`.
- **IMPLEMENT**:
  ```rust
  /// What the service needs from a release source, so tests and E2E never touch
  /// GitHub. Mirrors `collectors::Collector`, which exists for the same reason.
  pub trait ReleaseFeed: Send + Sync {
      fn check(&self)
          -> Pin<Box<dyn Future<Output = Result<Option<PendingUpdate>, UpdateFailure>> + Send + '_>>;
      fn install(&self, pending: PendingUpdate, on_progress: ProgressSink)
          -> Pin<Box<dyn Future<Output = Result<(), UpdateFailure>> + Send + '_>>;
  }
  ```
  `TauriUpdaterFeed { app: AppHandle }`:
  ```rust
  let updater = self.app
      .updater_builder()
      .endpoints(vec![manifest_url(channel_for_version(current)).parse()?])
      .map_err(|_| UpdateFailure::MetadataInvalid)?
      .timeout(Duration::from_secs(20))
      .on_before_exit(|| eprintln!("[CacheBite:update] installer starting; app will exit"))
      .build()
      .map_err(|_| UpdateFailure::MetadataInvalid)?;
  match updater.check().await { /* map errors below */ }
  ```
  Error mapping from `tauri_plugin_updater::Error`:
  | Plugin error | `UpdateFailure` |
  |---|---|
  | `Reqwest` with `is_connect()` / `is_timeout()` | `Offline` |
  | HTTP 403 / 429 status | `RateLimited` |
  | `Serialization` / `SemverError` / `EmptyEndpoints` | `MetadataInvalid` |
  | `TargetNotFound` / `TargetsNotFound` | `ArtifactMissing` |
  | `Reqwest` during download, `Io` | `DownloadFailed` |
  | `SignatureUtf8` / `Minisign` / `InvalidSignature` | `VerificationFailed` |
  | everything else | `InstallFailed` |

  > **`ArtifactMissing` does not imply that a newer version exists.** In `Updater::check()` the
  > platform lookup runs before the comparison result is consumed, and propagates with `?`:
  > ```rust
  > let should_update = /* semver compare */;
  > let (download_url, signature) = self.get_urls(&release, &installer)?;  // <- early return
  > let update = if should_update { Some(Update { .. }) } else { None };
  > ```
  > A manifest that omits this platform's key therefore yields `Err(TargetsNotFound)` **even when
  > the running version already matches the manifest version** — the user sees "No update is
  > published for this platform yet" instead of "Up to date". That message is honest, so do not
  > special-case it; but never treat `ArtifactMissing` as evidence that an update is waiting.

  `FixtureFeed` reads `CACHEBITE_E2E_UPDATE` ∈ `{none, available, failed}` (default `none`) and
  returns deterministic values with no network and no process exit.
- **MIRROR**: `collectors/mod.rs` `Collector` trait + `collectors::UnavailableFixtureCollector`
  (`lib.rs:127-135`) for the fixture-swap shape.
- **GOTCHA**: `Update::download_and_install` **never returns on Windows** — it calls
  `std::process::exit(0)`. Emit the `Installing` state *before* awaiting it, or the renderer's last
  observed state is `Downloading` when the app vanishes. On macOS/Linux it returns `Ok(())` and the
  caller must invoke `app.restart()`.
- **VALIDATE**: `cargo test --manifest-path src-tauri/Cargo.toml update::feed` — fixture branches
  only; the Tauri feed is exercised by the native E2E.

### Task 4: `UpdateService` actor and event emission
- **ACTION**: Create `src-tauri/src/update/service.rs` and `src-tauri/src/update/ipc.rs`.
- **IMPLEMENT**:
  - `UpdateService { status: watch::Sender<UpdateStatus>, feed: Arc<dyn ReleaseFeed>, current_version: String, last_checked: Mutex<Option<Instant>>, in_flight: AtomicBool }`.
  - `spawn_scheduler(&self)`: sleep `STARTUP_CHECK_DELAY`, then loop
    `{ if should_check(last, now, AUTOMATIC_CHECK_INTERVAL) { check().await } sleep(...) }`.
  - `check(&self, force: bool)`: `in_flight` compare-and-swap guards re-entrancy; sets `Checking`,
    then `UpToDate` / `Available` / `Failed`.
  - `check_on_panel_reveal(&self)`: fire-and-forget. Returns immediately, spawning a check only
    when `should_check(last, now, PANEL_OPEN_CHECK_FLOOR)`. **Must not block, must not fail the
    caller** — it is invoked from `toggle_panel`, and an update check has no business delaying or
    breaking the gesture that opens the panel.
  - `install(&self, app)`: only valid from `Available`; sets `Downloading` on progress, `Installing`
    before the install await, then `app.restart()` on non-Windows.
  - `pub const UPDATE_STATE_EVENT: &str = "update-state";`
  - `emit_update_state(app, service)` mirroring `emit_provider_states`.
  - Commands:
    ```rust
    #[tauri::command]
    pub fn get_update_state(
        window: tauri::WebviewWindow,
        service: State<'_, UpdateService>,
    ) -> Result<UpdateStateDto, IpcError> {
        authorize(&window, NativeCommand::GetUpdateState)?;
        Ok(service.snapshot())
    }
    ```
    plus `check_for_update` (async, `force = true`) and `install_update` (async).
- **MIRROR**: EVENT_EMISSION and AUTHORIZATION from `refresh/ipc.rs`;
  `RefreshHandle::spawn_persistent` in `refresh/actor.rs` for the actor shape.
- **IMPORTS**: `use crate::refresh::ipc::IpcError;` — reuse the existing enum rather than adding a
  second error type at the boundary. Add no new `IpcError` variants: every update failure travels in
  the `UpdateStatus::Failed` payload and the command returns `Ok`, so a failed check is a *state*,
  not an IPC error. `IpcError::Forbidden` / `ServiceUnavailable` remain for authorization and a
  missing service. `authorize` is private to `refresh/ipc.rs` — make it `pub(crate)` and import it.
- **GOTCHA**: `in_flight` must be reset on every exit path including early error returns, or a single
  failure permanently wedges the checker. Use a guard struct with `Drop`.
- **VALIDATE**: `cargo test --manifest-path src-tauri/Cargo.toml update::` plus
  `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`.

### Task 5: Command authority and handler registration
- **ACTION**: Update `src-tauri/src/window/mod.rs` and `src-tauri/src/lib.rs`.
- **IMPLEMENT**:
  - `NativeCommand::{GetUpdateState, CheckForUpdate, InstallUpdate}`.
  - `command_allowed`: add all three to the **`"panel"`** arm only. The overlay must not gain them —
    it may only read state, save its position, and toggle the panel.
  - Both `apply_invoke_handler` arms in `lib.rs` get
    `update::ipc::get_update_state, update::ipc::check_for_update, update::ipc::install_update,`.
  - `lib.rs` `run()`: add `.plugin(tauri_plugin_updater::Builder::new().build())` to the builder
    chain (desktop-only; the crate is already gated by target in `Cargo.toml`).
  - `lib.rs` `setup`: after `app.manage(collector_mode)`, construct the feed
    (`if fixture_mode { FixtureFeed::from_env() } else { TauriUpdaterFeed::new(app.handle().clone()) }`),
    `app.manage(service)`, `update::ipc::emit_update_state(app.handle(), &service)`,
    `service.spawn_scheduler()`.
  - `pub mod update;` at the top of `lib.rs`.
  - **`refresh/ipc.rs` `begin_reveal`** — the single path by which the panel is revealed (the `✕`,
    the hide/show hotkey and the fullscreen monitor only ever *hide* it). Add one non-fatal nudge,
    after `position_panel` succeeds and before the grace timer is spawned:
    ```rust
    // Revealing the panel is the only moment the update notice can be seen, so
    // it is also the only moment worth spending a check on. Best-effort via
    // try_state, mirroring how toggle_overlay_visibility reaches PanelLayoutGate:
    // a panel that opens without an update service is still a working panel.
    if let Some(update) = app.try_state::<crate::update::UpdateService>() {
        update.check_on_panel_reveal();
    }
    ```
- **MIRROR**: the existing three-place registration for `list_pet_packages`; and
  `lib.rs:247-249` for the `try_state` best-effort lookup idiom.
- **GOTCHA**: **Four** places must change per command — `NativeCommand`, `command_allowed`, both
  `apply_invoke_handler` arms — plus `AppGateway`. Missing the allowlist yields a runtime
  `Forbidden` with no compile error. Missing the `webdriver` arm breaks native E2E only.
  Also: manage the service **before** `emit_update_state` and the scheduler spawn, mirroring the
  `OverlayHideGate` ordering comment at `lib.rs:97-101`.
  The `begin_reveal` nudge must use `try_state`, not `state`, and must not use `?`. `state::<T>()`
  panics when the type is unmanaged, and `begin_reveal` runs on the `toggle_panel` command path —
  a panic or an early return there turns a missing update service into a pet whose double-click no
  longer opens the panel. Keep the reveal working regardless.
- **VALIDATE**: add to `window/tests.rs`:
  ```rust
  #[test]
  fn the_overlay_cannot_reach_update_commands() {
      for command in [
          NativeCommand::GetUpdateState,
          NativeCommand::CheckForUpdate,
          NativeCommand::InstallUpdate,
      ] {
          assert!(!command_allowed("overlay", command));
          assert!(command_allowed("panel", command));
          assert!(!command_allowed("anything-else", command));
      }
  }
  ```

### Task 6: Renderer gateway contract
- **ACTION**: Update `src/lib/api/gateway.ts` and `src/lib/api/fixtureGateway.ts`.
- **IMPLEMENT**:
  ```ts
  export type UpdateFailureReason =
    | 'offline' | 'rate_limited' | 'metadata_invalid' | 'artifact_missing'
    | 'download_failed' | 'verification_failed' | 'install_failed';

  export type UpdateStatusWire =
    | { readonly status: 'idle' }
    | { readonly status: 'checking' }
    | { readonly status: 'up_to_date' }
    | { readonly status: 'available'; readonly version: string; readonly notes: string | null }
    | { readonly status: 'downloading'; readonly received: number; readonly total: number | null }
    | { readonly status: 'installing'; readonly version: string }
    | { readonly status: 'failed'; readonly reason: UpdateFailureReason };

  export interface UpdateStateWire {
    readonly currentVersion: string;
    readonly status: UpdateStatusWire;
  }
  ```
  On `AppGateway` (all four documented as panel-only):
  ```ts
  /** Authorized for the `panel` window only (`window::command_allowed`). */
  getUpdateState(): Promise<UpdateStateWire>;
  listenUpdateState(next: (state: UpdateStateWire) => void): Promise<() => void>;
  /** Authorized for the `panel` window only. Bypasses the automatic throttle. */
  checkForUpdate(): Promise<void>;
  /** Authorized for the `panel` window only. Exits and relaunches the app on success. */
  installUpdate(): Promise<void>;
  ```
  `tauriGateway` impls — no field mapping needed (the DTO is `camelCase` on the wire):
  ```ts
  getUpdateState: () => invokeNative('get_update_state'),
  async listenUpdateState(next) {
    return listen<UpdateStateWire>('update-state', (event) => next(event.payload));
  },
  checkForUpdate: () => invokeNative('check_for_update'),
  installUpdate: () => invokeNative('install_update'),
  ```
  `fixtureGateway` returns `{ currentVersion: '0.1.0-fixture', status: { status: 'up_to_date' } }`
  and no-op listeners, matching how it already stubs `listenSettings`.
- **MIRROR**: GATEWAY_WIRE_MAPPING; the `listPetPackages` comment style for the authority note.
- **GOTCHA**: `fixtureGateway.ts` is typed as `AppGateway`; forgetting a method is a `svelte-check`
  failure, not a test failure. Update both files in the same commit.
- **VALIDATE**: `pnpm check && pnpm vitest run src/lib/api/gateway.test.ts`.

### Task 7: Presentation model, banner, settings row, wiring
- **ACTION**: Create `src/lib/state/updatePresentation.ts` + `src/lib/components/UpdateNotice.svelte`;
  update `SettingsPanel.svelte` and `App.svelte`.
- **IMPLEMENT** `updatePresentation.ts` (pure, no Svelte import):
  ```ts
  export interface UpdateViewModel {
    readonly visible: boolean;              // false → render nothing in the panel
    readonly headline: string;
    readonly detail: string | null;
    readonly primaryLabel: string | null;   // 'Install and restart' | 'Try again'
    readonly primaryEnabled: boolean;
    readonly dismissible: boolean;
    readonly settingsLine: string;          // always present, for the Settings view
  }

  export function updateViewModel(
    state: UpdateStateWire,
    dismissedVersion: string | null,
  ): UpdateViewModel;
  ```
  Copy table (kept here so the implementation needs no invention):
  | status | headline | detail | primary | dismissible | settingsLine |
  |---|---|---|---|---|---|
  | `idle` | — | — | — | — | `Not checked yet` |
  | `checking` | — | — | — | — | `Checking…` |
  | `up_to_date` | — | — | — | — | `Up to date` |
  | `available` | `Update available — {version}` | truncated notes | `Install and restart` | yes | `Update available — {version}` |
  | `downloading` | `Downloading {version}` | `{pct}%`, or `Downloading…` when `total` is null | `Install and restart` (disabled) | no | `Downloading…` |
  | `installing` | `Installing {version}…` | `CacheBite will restart.` | disabled | no | `Installing…` |
  | `failed` | `Update failed` | per-reason sentence (below) | `Try again` | yes | `Update failed` |

  Failure copy (no URLs, no paths — privacy contract):
  `offline` → "CacheBite could not reach GitHub. Check your connection." ·
  `rate_limited` → "GitHub is rate limiting downloads. Try again later." ·
  `metadata_invalid` → "The update information could not be read." ·
  `artifact_missing` → "No update is published for this platform yet." ·
  `download_failed` → "The download did not finish." ·
  `verification_failed` → "The update signature did not verify. It was not installed." ·
  `install_failed` → "The update could not be installed. Your current version is unchanged."

  `visible` is `false` for `idle` / `checking` / `up_to_date`, and for `available` when
  `dismissedVersion === state.status.version`. `downloading` / `installing` / `failed` are always
  visible so a dismissed banner cannot hide an in-flight install or its failure.

  `UpdateNotice.svelte` — `role="status"`, `aria-live="polite"`, `<button>` for Install and Later,
  styled with the existing tokens (`--color-surface`, `--color-border`, `--color-accent`,
  `--space-*`) exactly as `SettingsPanel.svelte` does.

  `SettingsPanel.svelte` — add above the hotkey block, mirroring the read-only `.field` pattern:
  ```svelte
  <div class="field">
    <span id="app-version-label">Version</span>
    <kbd class="shortcut" aria-labelledby="app-version-label">{currentVersion}</kbd>
  </div>
  <div class="field">
    <span>Updates</span>
    <button class="ghost-action" disabled={updateBusy} onclick={() => onCheckUpdate()}
      >Check for updates</button>
  </div>
  <p class="field-help">{updateLine}</p>
  ```
  New props: `currentVersion = '—'`, `updateLine = ''`, `updateBusy = false`, `onCheckUpdate = () => {}`.

  `App.svelte` — beside the existing gateway wiring (mirroring `listenProviderStates` at line 293
  and `getPlatformCapabilities` at line 380):
  ```js
  let updateState = $state(/** @type {import('./lib/api/gateway').UpdateStateWire | null} */ (null));
  let dismissedUpdateVersion = $state(/** @type {string | null} */ (null));
  const updateView = $derived(
    updateState ? updateViewModel(updateState, dismissedUpdateVersion) : null,
  );
  ```
  Subscribe in the same `Promise.all` block that registers the other listeners; seed with
  `gateway.getUpdateState().catch(() => null)`. Render `<UpdateNotice>` immediately inside the
  `{:else}` branch, **before** `<UsagePanel>`.
- **MIRROR**: SVELTE_COMPONENT, TS_DIAGNOSTIC_UNION, and the `systemGuidance.ts` +
  `panelModels.ts` split (copy lives in a plain `.ts` module, never inside the component).
- **GOTCHA**: The banner changes the panel's height. `App.svelte:418-432` already observes the panel
  and calls `resizePanel`; do **not** add a second observer. Also do not call `resizePanel` manually —
  `resize_panel` doubles as the panel-reveal gate (`ipc.rs:535-537`), and an extra call can reveal a
  panel the user just dismissed.
- **VALIDATE**:
  ```bash
  pnpm vitest run src/lib/state/updatePresentation.test.ts \
                  src/lib/components/UpdateNotice.test.ts \
                  src/lib/components/SettingsPanel.test.ts
  pnpm check
  ```

### Task 8: Tauri configuration
- **ACTION**: Update `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml`.
- **IMPLEMENT**:
  ```jsonc
  "plugins": {
    "updater": {
      "pubkey": "<public key from Task 0>",
      "endpoints": [
        "https://github.com/chanhoan/CacheBite/releases/download/updater/stable.json"
      ],
      "windows": { "installMode": "passive" }
    }
  }
  ```
  `Cargo.toml`:
  ```toml
  semver = "1"
  tauri-plugin-updater = "2.10"
  ```
- **MIRROR**: existing config shape; add `plugins` as a sibling of `bundle`.
- **GOTCHA**:
  - **Leave `bundle.createUpdaterArtifacts` at `false`.** Setting it `true` makes every local
    `pnpm tauri build` fail for anyone without `TAURI_SIGNING_PRIVATE_KEY`. CI turns it on through
    the generated release config in Task 9.
  - `endpoints` must be non-empty even though the service overrides it at runtime
    (`Error::EmptyEndpoints`). The stable URL is the correct default.
  - **No CSP change is needed** — all updater traffic is native-side, not from the webview. Do not
    widen `connect-src`.
  - **No `capabilities/*.json` change is needed** — the renderer never calls
    `@tauri-apps/plugin-updater`. Adding `updater:default` would hand the webview a bypass around
    `command_allowed`. Leave both capability files untouched.
- **VALIDATE**: add to `src/securityConfig.test.ts`:
  ```ts
  it('pins the updater to signed manifests on the CacheBite release host', () => {
    const config = JSON.parse(readFileSync(resolve('src-tauri/tauri.conf.json'), 'utf8'));
    expect(config.plugins.updater.pubkey).toMatch(/\S/);
    for (const endpoint of config.plugins.updater.endpoints as string[]) {
      expect(new URL(endpoint).protocol).toBe('https:');
      expect(new URL(endpoint).host).toBe('github.com');
    }
    expect(config.plugins.updater.dangerousInsecureTransportProtocol).toBeUndefined();
    expect(config.bundle.createUpdaterArtifacts).toBe(false);
  });

  it('does not grant renderer windows direct updater plugin permission', () => {
    for (const file of ['overlay.json', 'panel.json']) {
      const capability = JSON.parse(
        readFileSync(resolve(`src-tauri/capabilities/${file}`), 'utf8'),
      ) as { permissions: string[] };
      expect(capability.permissions.some((p) => p.startsWith('updater:'))).toBe(false);
    }
  });
  ```
  Then `pnpm vitest run src/securityConfig.test.ts && cargo check --manifest-path src-tauri/Cargo.toml`.

### Task 9: Release workflow — sign and emit updater artifacts
- **ACTION**: Update `.github/workflows/release.yml`.
- **IMPLEMENT**:
  1. In the `package` job, before the build, derive the version from the tag and write the merge
     config (a **named** step so `nativeWorkflow.test.ts` can assert on it):
     ```yaml
     - name: Derive the release version from the tag
       shell: bash
       run: |
         VERSION="${GITHUB_REF_NAME#v}"
         [ -n "$VERSION" ] || { echo "not a tag build"; exit 1; }
         # WiX rejects a non-numeric pre-release, so the MSI gets a numeric
         # ProductVersion derived from the trailing number of the pre-release.
         CORE="${VERSION%%-*}"
         case "$VERSION" in
           *-*) WIX_BUILD="$(printf '%s' "$VERSION" | sed -n 's/.*[^0-9]\([0-9][0-9]*\)$/\1/p')" ;;
           *)   WIX_BUILD=0 ;;
         esac
         WIX_BUILD="${WIX_BUILD:-0}"
         python3 - "$VERSION" "$CORE.$WIX_BUILD" <<'PY' > src-tauri/tauri.release.conf.json
         import json, sys
         print(json.dumps({
             "version": sys.argv[1],
             "bundle": {
                 "createUpdaterArtifacts": True,
                 "windows": {"wix": {"version": sys.argv[2]}},
             },
         }, indent=2))
         PY
         cat src-tauri/tauri.release.conf.json
     ```
  2. Build with the merge config and the signing secrets:
     ```yaml
     - run: pnpm tauri build --config src-tauri/tauri.release.conf.json --bundles ${{ matrix.bundles }} ${{ matrix.target_flag }}
       env:
         TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
         TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
     ```
  3. macOS matrix entry: `bundles: app,dmg` (was `dmg`) and add
     `src-tauri/target/universal-apple-darwin/release/bundle/macos/*.tar.gz*` to `paths`.
     The `.app.tar.gz` is only produced when the `app` bundle target runs.
  4. Rename the macOS updater artifact to carry the version — the bare `CacheBite.app.tar.gz` name is
     ambiguous across releases. Two explicit `mv` calls, keeping the `X` / `X.sig` pairing intact:
     ```yaml
     - name: Name the macOS updater artifact after the release
       if: runner.os == 'macOS'
       shell: bash
       run: |
         cd src-tauri/target/universal-apple-darwin/release/bundle/macos
         BASE="CacheBite_${GITHUB_REF_NAME#v}_universal.app.tar.gz"
         mv CacheBite.app.tar.gz     "$BASE"
         mv CacheBite.app.tar.gz.sig "$BASE.sig"
     ```
  5. `publish` job → `Collect installers`: extend the `find` to also copy
     `-name '*.sig' -o -name '*.tar.gz'`. `SHA256SUMS.txt` is regenerated over the whole directory,
     so it picks them up automatically.
  6. Release notes heredoc: replace the "CacheBite has no auto-updater" paragraph with the bootstrap
     note and the mac/Linux caveat (Task 12 supplies the wording).
  7. Apply the same signing env and `--config` treatment to the `signed-notarized-macos` job, or it
     will produce a DMG whose version disagrees with the rest of the release.
- **MIRROR**: the existing `Create SHA-256 checksums (Unix|Windows)` named-step style, and the
  matrix-driven `bundle_root` comment discipline.
- **GOTCHA**:
  - `--config` takes a path resolved from the **current working directory**, which is the repo root
    in these jobs. Confirm on the first run: the build log prints the merged version.
  - `python3` is present on all three GitHub runner images.
  - Windows: `shell: bash` is required for the derive step, otherwise PowerShell parses the heredoc.
- **VALIDATE**: add to `src/nativeWorkflow.test.ts`:
  ```ts
  describe('updater release automation', () => {
    it('signs updater artifacts with repository secrets', () => {
      expect(releaseWorkflow).toContain(
        'TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}',
      );
      expect(releaseWorkflow).toContain('--config src-tauri/tauri.release.conf.json');
    });
    it('bundles the macOS app archive the updater needs, not only the DMG', () => {
      expect(releaseWorkflow).toContain('bundles: app,dmg');
    });
    it('publishes signatures alongside the installers', () => {
      expect(extractNamedStep(releaseWorkflow, 'Collect installers')).toContain("-name '*.sig'");
    });
  });
  ```
  Then `pnpm vitest run src/nativeWorkflow.test.ts`.

### Task 10: Channel manifest generator + publish workflow
- **ACTION**: Create `scripts/build_updater_manifest.py` and `.github/workflows/updater-manifest.yml`.
- **IMPLEMENT** — `scripts/build_updater_manifest.py`:
  - Inputs: `--assets <json>` (array of `{name, browser_download_url}`), `--signatures <dir>` of
    downloaded `.sig` files, `--version`, `--notes-file`, `--pub-date`, `--out`, `--previous`.
  - Output — the static manifest, e.g.:
    ```jsonc
    {
      "version": "0.1.0-beta.5",
      "notes": "…",                          // truncated
      "pub_date": "2026-08-04T12:00:00Z",    // RFC 3339, GitHub `published_at` verbatim
      "platforms": {
        "windows-x86_64":      { "signature": "<minisign>", "url": "https://…-setup.exe" },
        "windows-x86_64-nsis": { "signature": "<minisign>", "url": "https://…-setup.exe" },
        "windows-x86_64-msi":  { "signature": "<minisign>", "url": "https://….msi" },
        "darwin-aarch64":      { "signature": "<minisign>", "url": "https://…universal.app.tar.gz" },
        "darwin-x86_64":       { "signature": "<minisign>", "url": "https://…universal.app.tar.gz" },
        "linux-x86_64-appimage": { "signature": "<minisign>", "url": "https://….AppImage" },
        "linux-x86_64":          { "signature": "<minisign>", "url": "https://….AppImage" }
      }
    }
    ```
  - Asset → key mapping: `-setup.exe` → `windows-x86_64-nsis` **and** `windows-x86_64`;
    `.msi` → `windows-x86_64-msi`; `universal.app.tar.gz` → `darwin-aarch64` **and**
    `darwin-x86_64` (no universal fallback exists in the plugin); `.AppImage` →
    `linux-x86_64-appimage` **and** `linux-x86_64`.
  - `signature` is the **verbatim contents** of the matching `<asset>.sig`.
  - A missing signature for a present artifact is a hard error (fail closed — never emit an entry
    the client cannot verify).
  - A missing *platform* is a warning, not an error: the manifest omits the key and the workflow
    still publishes. Be aware of what that costs the affected platform — `Updater::check()` calls
    `get_urls` before it uses the version comparison and propagates the failure with `?`, so an
    omitted key makes **every** check on that platform return `TargetsNotFound`, including checks
    where the client is already on the manifest version. Those users see "No update is published
    for this platform yet" permanently, never "Up to date". Log the omitted keys loudly in the
    workflow so a one-off upload miss is visible in the run summary rather than only in a bug
    report months later.
  - `--previous <file>`: if the existing manifest's version is `>=` the new version, exit 0 without
    writing. This is the regression guard for a human publishing an older draft after a newer one.
  - `--self-test`: assertions over synthetic asset lists (universal duplication, missing sig,
    version regression, stable-vs-beta routing); non-zero exit on failure.
  - Reuse the shebang / `argparse` shape of the existing `scripts/build-pet-packages.py`.

  `.github/workflows/updater-manifest.yml`:
  ```yaml
  name: Update channel manifests
  on:
    release:
      types: [published]
  permissions:
    contents: read
  jobs:
    manifests:
      # The `updater` release holds the manifests themselves; regenerating from
      # it would be circular.
      if: github.event.release.tag_name != 'updater'
      runs-on: ubuntu-latest
      timeout-minutes: 10
      permissions:
        contents: write
      steps:
        - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2
        - name: Build the channel manifest for this release
          env:
            GH_TOKEN: ${{ github.token }}
            TAG: ${{ github.event.release.tag_name }}
          run: |
            VERSION="${TAG#v}"
            mkdir -p work && cd work
            gh release download "$TAG" --repo "$GITHUB_REPOSITORY" --pattern '*.sig'
            gh api "repos/$GITHUB_REPOSITORY/releases/tags/$TAG" \
              --jq '[.assets[] | {name, browser_download_url}]' > assets.json
            gh api "repos/$GITHUB_REPOSITORY/releases/tags/$TAG" --jq '.body'         > notes.md
            gh api "repos/$GITHUB_REPOSITORY/releases/tags/$TAG" --jq '.published_at' > pub_date.txt
            # The `updater` release may not exist yet on the very first run.
            gh release download updater --repo "$GITHUB_REPOSITORY" --pattern '*.json' || true
            python3 ../scripts/build_updater_manifest.py \
              --version "$VERSION" --notes-file notes.md --pub-date "$(cat pub_date.txt)" \
              --signatures . --assets assets.json --out beta.json --previous beta.json
            case "$VERSION" in
              *-*) echo "pre-release: stable channel untouched" ;;
              *)   python3 ../scripts/build_updater_manifest.py \
                     --version "$VERSION" --notes-file notes.md --pub-date "$(cat pub_date.txt)" \
                     --signatures . --assets assets.json --out stable.json --previous stable.json ;;
            esac
        - name: Publish the channel manifests
          env:
            GH_TOKEN: ${{ github.token }}
          run: |
            cd work
            gh release view updater --repo "$GITHUB_REPOSITORY" >/dev/null 2>&1 || \
              gh release create updater --repo "$GITHUB_REPOSITORY" --prerelease \
                --title 'Update channel manifests' \
                --notes 'Machine-generated updater metadata. Not a CacheBite build — see the versioned releases for installers.'
            gh release upload updater --repo "$GITHUB_REPOSITORY" --clobber $(ls *.json)
  ```
- **MIRROR**: `release.yml`'s `gh release create` + `GH_TOKEN: ${{ github.token }}` pattern and the
  full-SHA action pinning discipline (`CLAUDE.md`: every action is pinned).
- **GOTCHA**:
  - The `updater` release **must** be `--prerelease` so it never takes GitHub's "Latest" badge from
    a real CacheBite release.
  - `gh release create updater` also creates a git tag named `updater`. Intended and harmless; do
    not delete it.
  - `--clobber` is required — without it the second upload fails on an existing asset name.
  - `beta.json` is updated for **every** published release including stable ones: a beta tester must
    still be offered the stable build that supersedes their pre-release.
- **VALIDATE**:
  ```bash
  python3 scripts/build_updater_manifest.py --self-test
  pnpm vitest run src/nativeWorkflow.test.ts   # with new assertions on updater-manifest.yml
  ```
  Add to `ci.yml` after the licenses step:
  ```yaml
  - run: python3 scripts/build_updater_manifest.py --self-test
  ```
  and extend the secret guard:
  ```yaml
  minisign_marker_prefix='untrusted comment: minisign '
  minisign_marker_suffix='encrypted secret key'
  private_key_marker="${minisign_marker_prefix}${minisign_marker_suffix}"
  ! git grep -nF "$private_key_marker"
  ```

### Task 11: Fixture and E2E coverage
- **ACTION**: Update `tests/e2e/native.spec.ts` and the E2E env plumbing.
- **IMPLEMENT**:
  - Native E2E already runs with `CACHEBITE_E2E_FIXTURES=1`; add `CACHEBITE_E2E_UPDATE` to select
    the scenario. Three specs:
    1. `CACHEBITE_E2E_UPDATE=available` → the panel shows `Update available` with an enabled
       `Install and restart` and a `Later`; pressing `Later` hides the banner and the panel stays
       usable.
    2. `CACHEBITE_E2E_UPDATE=none` (default) → no banner; the Settings view reports `Up to date`.
    3. `CACHEBITE_E2E_UPDATE=failed` → the banner shows the failure sentence and `Try again`, and
       `Refresh now` in `UsagePanel` still works (proving the failure is non-blocking).
    4. `CACHEBITE_E2E_UPDATE=available` with the panel toggled **closed and open again** → the
       banner is still correct and the fixture feed recorded exactly **one** check (the second
       reveal falls inside `PANEL_OPEN_CHECK_FLOOR`). Exposed for the assertion by having
       `FixtureFeed` count its `check` calls and surface the count through the existing
       `#[cfg(feature = "webdriver")] get_window_states` sibling — add a
       `get_update_probe_count` command under the same feature gate rather than widening the
       production command surface.
  - The fixture feed's `install` must **not** exit the process — it transitions to `Installing` and
    stops, so the E2E can assert the state without killing the runner.
  - `get_update_probe_count` is a **webdriver-only** command and therefore does **not** get a
    `NativeCommand` variant or a `command_allowed` entry — `get_window_states` (`ipc.rs:207-219`)
    established that pattern by calling no `authorize()` at all. The "four places per command" rule
    in Task 5 applies to production commands only; adding this one to the allowlist would leak a
    test affordance into the shipped surface.
- **MIRROR**: the labelled-window discovery + retry pattern already in `tests/e2e/native.spec.ts`
  (asserted by `nativeWorkflow.test.ts:170-197`).
- **GOTCHA**: The production-composition smoke (`CACHEBITE_EXPECTED_COLLECTOR_MODE=production`, no
  fixtures) will run the **real** `TauriUpdaterFeed` against the live `stable.json`. That is
  acceptable and is exactly the "recoverable failure" path — but the spec must assert only that the
  app starts and the panel works, never that a specific update status appears.
- **VALIDATE**:
  ```bash
  pnpm tauri build --debug --no-bundle --features webdriver
  CACHEBITE_E2E_FIXTURES=1 CACHEBITE_EXPECTED_COLLECTOR_MODE=fixture CACHEBITE_E2E_UPDATE=available pnpm test:e2e
  ```

### Task 12: Documentation
- **ACTION**: Update `docs/beta-testing.md`, `docs/architecture.md`, `docs/ui-contract.md`,
  `CLAUDE.md`, and the release-notes heredoc in `release.yml`.
- **IMPLEMENT**:
  - `docs/beta-testing.md:26-27` — replace **"There is no auto-updater. Moving to a newer beta means
    installing it over the old build."** with:
    > CacheBite checks for a newer signed release and offers it in the panel with **Install** and
    > **Later**. Installing downloads the build for your platform, verifies its signature, installs
    > it over the current one, and restarts CacheBite. Your settings, history and pets are kept.
    >
    > Builds up to and including `v0.1.0-beta.3` predate the updater and cannot update themselves —
    > install the first updater-enabled release by hand once, and later ones arrive in-app.
    >
    > In-app installation is verified on Windows. On macOS and Linux the artifacts are published and
    > signed and the app will attempt the install, but replacing an unsigned macOS bundle and
    > rewriting a running AppImage are not yet confirmed on real hardware. If it fails there, the
    > banner says so and your existing build keeps running — install by hand and report it.
  - `docs/beta-testing.md:155` — replace `- No auto-update.` with
    `- In-app update is verified on Windows only; macOS and Linux are published but unverified.`
  - `docs/architecture.md` — add the `update/` layer beside `collectors/`, `refresh/`, `store/`,
    `window/`, and reproduce the manifest flow diagram from this plan.
  - `docs/ui-contract.md` — document `UpdateViewModel`, the copy table, and the visibility rule.
  - `CLAUDE.md`:
    - Native commands list: add `get_update_state`, `check_for_update`, `install_update` and note
      that update command implementations live in `src-tauri/src/update/ipc.rs`, not `refresh/ipc.rs`.
    - Native layers: add the `update/` bullet.
    - Invariants: add
      > **Update authority and channel.** The renderer never talks to `tauri-plugin-updater`; the
      > capability files must contain no `updater:*` permission. The channel is derived from the
      > running version (`update::channel_for_version`) and never persisted — a stored channel could
      > record a beta opt-in no migration would undo. `stable.json` never lists a pre-release, so a
      > release build cannot be pulled onto beta. `bundle.createUpdaterArtifacts` stays `false` in
      > the committed config; CI enables it through the generated `src-tauri/tauri.release.conf.json`
      > so a contributor without the signing key can still build. The MSI needs a numeric
      > `bundle.windows.wix.version` because WiX rejects a non-numeric pre-release.
    - Commands section: add `python3 scripts/build_updater_manifest.py --self-test`.
- **MIRROR**: the existing `docs/beta-testing.md` voice — direct, second person, no marketing.
- **GOTCHA**: `release.yml`'s notes heredoc is `<<'NOTES'` (quoted), so `$` and backticks are
  literal. Keep it that way.
- **VALIDATE**: `pnpm lint` (prettier checks Markdown) and a manual read of the rendered release.

---

## Testing Strategy

### Unit Tests — Rust

| Test | Input | Expected Output | Edge Case? |
|---|---|---|---|
| `a_pre_release_build_follows_the_beta_channel` | `"0.1.0-beta.4"` | `Channel::Beta` | |
| `a_release_build_never_follows_the_beta_channel` | `"0.1.0"` | `Channel::Stable` | |
| `an_unparseable_version_falls_back_to_stable` | `"not-a-version"` | `Channel::Stable` | ✔ |
| `each_channel_resolves_to_its_own_manifest` | both channels | distinct https github.com URLs | |
| `the_first_check_is_always_due` | `None`, any interval | `true` | ✔ |
| `a_recent_check_is_not_repeated` | `now - 1s`, `AUTOMATIC_CHECK_INTERVAL` | `false` | |
| `an_expired_check_is_due_again` | `now - 2h`, `AUTOMATIC_CHECK_INTERVAL` | `true` | |
| `a_second_panel_reveal_inside_the_floor_does_not_recheck` | `now - 5min`, `PANEL_OPEN_CHECK_FLOOR` | `false` | ✔ |
| `a_panel_reveal_after_the_floor_rechecks` | `now - 20min`, `PANEL_OPEN_CHECK_FLOOR` | `true` | |
| `the_panel_floor_is_shorter_than_the_background_sweep` | the two constants | `PANEL_OPEN_CHECK_FLOOR < AUTOMATIC_CHECK_INTERVAL` | ✔ guards an inverted edit |
| `revealing_the_panel_never_fails_when_the_update_service_is_absent` | `begin_reveal` with no managed service | panel still revealed, no panic | ✔ security/robustness |
| `notes_are_truncated_on_a_char_boundary` | 10 000-char multibyte body | ≤ `MAX_NOTES_CHARS`, valid UTF-8 | ✔ |
| `the_update_dto_never_serialises_a_download_url` | every `UpdateStatus` variant | no `url` / `path` / `signature` key | ✔ security |
| `the_overlay_cannot_reach_update_commands` | 3 commands × 3 labels | overlay `false`, panel `true`, other `false` | ✔ security |
| `a_failed_check_releases_the_in_flight_guard` | feed returns `Err` twice | second check runs | ✔ |
| `installing_is_emitted_before_the_install_await` | fixture feed | observed states include `Installing` | ✔ |

### Unit Tests — Renderer

| Test | Input | Expected Output | Edge Case? |
|---|---|---|---|
| `hides the banner when the app is up to date` | `up_to_date` | `visible: false` | |
| `hides an available update the user dismissed` | `available 0.1.0-b5`, dismissed `0.1.0-b5` | `visible: false` | |
| `shows a newer update after an older one was dismissed` | `available 0.1.0-b6`, dismissed `0.1.0-b5` | `visible: true` | ✔ |
| `never hides an in-flight install` | `downloading`, dismissed same version | `visible: true` | ✔ |
| `never hides a failure` | `failed`, dismissed | `visible: true` | ✔ |
| `reports indeterminate progress when the size is unknown` | `downloading total: null` | `detail: 'Downloading…'` | ✔ |
| `renders a distinct sentence for every failure reason` | all 7 reasons | 7 distinct strings, none containing `http` | ✔ security |
| `disables install while downloading` | `downloading` | `primaryEnabled: false` | |
| `always reports a settings line` | all statuses | non-empty `settingsLine` | |
| `SettingsPanel invokes onCheckUpdate` | click | callback fired once | |
| `UpdateNotice install/later fire their callbacks` | clicks | each once, banner hidden after Later | |

### Edge Cases Checklist
- [ ] Empty input — manifest with an empty `platforms` map → `artifact_missing`, panel usable
- [ ] Maximum size input — 10 000-char release body truncated to `MAX_NOTES_CHARS`
- [ ] Invalid types — `notes: null`, `total: null`, unknown `status` string from a future build
- [ ] Concurrent access — manual `Check for updates` pressed while the scheduler is mid-check
- [ ] Concurrent access — `Install` pressed twice
- [ ] Concurrent access — panel double-clicked open/closed/open inside the 15-minute floor: exactly
      one check, and the reveal itself is never delayed or blocked
- [ ] Panel revealed while a check is already in flight → the `in_flight` guard drops the nudge,
      the reveal still completes
- [ ] Panel revealed with no managed `UpdateService` (a build where setup failed earlier) → panel
      opens normally, no panic from `state::<T>()`
- [ ] Network failure — offline, DNS failure, timeout, HTTP 403/429
- [ ] Permission denied — Linux AppImage on a read-only mount; macOS `/Applications` without admin
- [ ] Version regression — a newer release published, then an older draft published afterwards
- [ ] Same version — manifest advertises the installed version → no offer
- [ ] Same version **and** this platform's key is absent → `artifact_missing`, never `up_to_date`
      (`get_urls` runs before the comparison is consumed) — assert this in a fixture test so the
      status is not mistaken for a bug later
- [ ] Stable user, beta published → `stable.json` unchanged → no offer
- [ ] `updater` release absent on the very first manifest run
- [ ] Signature file missing for a present artifact → generator fails the workflow, publishes nothing

---

## Validation Commands

### Static Analysis
```bash
pnpm check
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
```
EXPECT: zero type errors, zero clippy warnings.

### Unit Tests
```bash
cargo test --manifest-path src-tauri/Cargo.toml update::
cargo test --manifest-path src-tauri/Cargo.toml window::tests
pnpm vitest run src/lib/state/updatePresentation.test.ts \
                src/lib/components/UpdateNotice.test.ts \
                src/lib/components/SettingsPanel.test.ts \
                src/securityConfig.test.ts \
                src/nativeWorkflow.test.ts
python3 scripts/build_updater_manifest.py --self-test
```
EXPECT: all pass.

### Full Test Suite
```bash
pnpm test:ci
cargo test --manifest-path src-tauri/Cargo.toml --all-features
```
EXPECT: no regressions; coverage stays ≥ 80% on branches/functions/lines/statements.

### E2E
```bash
pnpm test:e2e:renderer
pnpm tauri build --debug --no-bundle --features webdriver
CACHEBITE_E2E_FIXTURES=1 CACHEBITE_EXPECTED_COLLECTOR_MODE=fixture CACHEBITE_E2E_UPDATE=available pnpm test:e2e
CACHEBITE_E2E_FIXTURES=1 CACHEBITE_EXPECTED_COLLECTOR_MODE=fixture CACHEBITE_E2E_UPDATE=failed    pnpm test:e2e
```
EXPECT: banner present / absent / failed as selected; no network access in fixture mode.

### Release Pipeline Validation (real, required before closing the issue)
```bash
export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/cachebite.key)"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=...
printf '%s' '{"version":"0.1.0-beta.4","bundle":{"createUpdaterArtifacts":true,"windows":{"wix":{"version":"0.1.0.4"}}}}' \
  > src-tauri/tauri.release.conf.json
pnpm tauri build --config src-tauri/tauri.release.conf.json --bundles nsis,msi
ls src-tauri/target/release/bundle/nsis/   # expect *-setup.exe and *-setup.exe.sig
ls src-tauri/target/release/bundle/msi/    # expect *.msi and *.msi.sig
```
EXPECT: MSI builds without the "pre-release identifier must be numeric-only" error, and every
installer has a sibling `.sig`.

### Manual Validation (Windows, end to end — the acceptance gate)
- [ ] Tag `v0.1.0-beta.4`, let `release.yml` produce the draft, publish it by hand
- [ ] `updater-manifest.yml` runs and the `updater` release now holds `beta.json` (and no `stable.json`)
- [ ] `curl -L https://github.com/chanhoan/CacheBite/releases/download/updater/beta.json` returns a
      manifest whose `platforms` includes `windows-x86_64`, `windows-x86_64-nsis`,
      `windows-x86_64-msi`, `darwin-aarch64`, `darwin-x86_64`, `linux-x86_64`
- [ ] Install `v0.1.0-beta.4` by hand; Settings shows `Version 0.1.0-beta.4` and `Up to date`
- [ ] Tag and publish `v0.1.0-beta.5`
- [ ] Within 6 h (or via `Check for updates`) the panel shows `Update available — 0.1.0-beta.5`
- [ ] `Later` hides the banner; Settings still reports the available update
- [ ] Reopen the panel after a restart — the banner is back
- [ ] `Install and restart` → progress → app exits → NSIS runs → **CacheBite relaunches on its own**
- [ ] After the relaunch: version is `0.1.0-beta.5`; pet position, selected pet, primary provider,
      theme and usage history are all unchanged
- [ ] Disconnect the network and press `Check for updates` → "could not reach GitHub", panel and
      `Refresh now` still work
- [ ] Point a temporary local build at a manifest with a corrupted signature → status is
      `verification_failed` and **nothing is installed**

---

## Acceptance Criteria

- [ ] All tasks completed
- [ ] All validation commands pass
- [ ] Tests written and passing; coverage gate (80%) still green
- [ ] No type errors, no clippy warnings, no lint errors
- [ ] Matches the UX design above
- [ ] Windows end-to-end manual validation completed and recorded on Issue #49
- [ ] Issue #49 scope items each map to a task:
  - [ ] throttled automatic check + manual check in Settings → Tasks 1, 4, 7
  - [ ] intentional stable/beta comparison, never the installed or an older version → Tasks 1, 10
  - [ ] target version shown with explicit Install and Later, never silent → Task 7
  - [ ] correct platform/arch artifact from the official release source → Tasks 3, 8, 10
  - [ ] fail-closed authentication of metadata and artifacts → Tasks 0, 3, 8, 9
  - [ ] settings preserved and app relaunched → Tasks 4, 12 + manual validation
  - [ ] every error class recoverable and non-blocking → Tasks 2, 3, 7, 11
  - [ ] update-available / no-update / failed tested with local fixtures → Task 11
  - [ ] release automation publishes metadata, signatures and artifacts → Tasks 9, 10
  - [ ] `docs/beta-testing.md` no longer presents manual overwrite as current behaviour → Task 12

## Completion Checklist

- [ ] Code follows the discovered patterns (per-window `authorize`, pure policy fns, typed errors)
- [ ] Error handling matches the codebase style — no raw `reqwest` / `io` errors cross the boundary
- [ ] Logging follows `eprintln!` conventions and leaks no URL, path, or signature
- [ ] Tests follow the existing `#[cfg(test)] mod tests` and vitest patterns
- [ ] No hardcoded values outside named constants
- [ ] Documentation updated (`beta-testing.md`, `architecture.md`, `ui-contract.md`, `CLAUDE.md`)
- [ ] No unnecessary scope additions (see NOT Building)
- [ ] Self-contained — no questions needed during implementation

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Signing key lost or leaked | Low | **Critical** — every client stops updating, or an attacker can sign builds | Back up outside the repo; CI secret guard greps for the minisign header (Task 10); rotating means one hand-installed release |
| MSI build fails on the pre-release version | Medium | High — blocks the release | `bundle.windows.wix.version` numeric override (Task 9); verified against `msi/mod.rs:474-485`; local dry-run is in Validation |
| `--config` path resolves differently on a runner | Medium | Medium — wrong version shipped | The derive step `cat`s the generated file; the build log prints the merged version; the first release is manually inspected |
| macOS unsigned in-place replacement fails | Medium | Medium — mac testers still hand-install | Documented as unverified (Task 12); failure is a recoverable status and the old build keeps running |
| Linux AppImage on a different mount / read-only | Medium | Medium | `install_failed` status with copy that says the current version is unchanged; documented |
| Human publishes an older draft after a newer one | Low | High — clients offered a downgrade | `--previous` regression guard in the generator (Task 10) + a `--self-test` case |
| A platform's artifact fails to upload, so its manifest key is omitted | Medium | Medium — that platform reports `artifact_missing` on **every** check, never `up_to_date`, until the next release | Generator logs omitted keys into the workflow run summary (Task 10); the manual validation `curl` checks all six keys are present before the release is considered done |
| Panel height churn from the banner destabilises `resize_panel` | Low | Medium — flicker or a wrongly revealed panel | Reuse the existing `ResizeObserver`; never call `resizePanel` directly (it doubles as the reveal gate) |
| Renderer gains a bypass around `command_allowed` via `updater:*` capability | Low | High | `securityConfig.test.ts` asserts no `updater:` permission in either capability file (Task 8) |
| Users on beta.1–beta.3 never update | **Certain** | Low | Unavoidable; documented in the release notes and `beta-testing.md` |
| `updater` release confuses users browsing Releases | Medium | Low | Marked `--prerelease` with an explicit "not a CacheBite build" description |

## Notes

- **Why not the GitHub REST API at runtime**: rejected during planning. It removes the fixed-tag
  release object but adds a 60 req/h/IP unauthenticated rate limit shared across every user behind a
  NAT, more Rust code, and a channel policy that can only be corrected by shipping a new app.
- **Why not `releases/latest/download/latest.json`**: GitHub excludes pre-releases from
  `/releases/latest`, and every CacheBite release to date is a pre-release, so the URL 404s today.
- **Why the channel is derived, not stored**: identical reasoning to schema v5 dropping the persisted
  hide/show accelerator (`store/settings.rs:85-90`) — a stored value can record a state no migration
  will undo.
- **Why the cadence is what it is.** The first draft used a single 6-hour timer, which was wrong in
  both directions: too slow to feel responsive, and aimed at a moment nobody is looking. The notice
  is only visible while the panel is open, and the panel is hidden by default, so the reveal is the
  event worth spending a check on. The traffic baseline is `SchedulerConfig::default().poll_interval`
  — CacheBite already polls **each** provider every 15 minutes, so the update check is calibrated
  against it: a 15-minute floor on reveal, plus a 1-hour background sweep whose only jobs are to
  keep the state warm (so the banner does not pop in and resize the panel a moment after it opens)
  and to cover a session that leaves the panel open. Worst case is 96 checks/day against a 1-2 KB
  static file on the GitHub release CDN — under half the 192 authenticated provider calls the app
  already makes, and not subject to the `api.github.com` 60/hour REST limit, which does not apply
  to release asset downloads.
- **Why the check is not wired to `Refresh now`**: that button is provider-scoped
  (`onRefresh(selected)`) and refreshes exactly one provider's usage. Attaching an unrelated network
  operation to it would merge two independent failure modes into one control — offline would be
  ambiguous between "usage did not refresh" and "update check failed". The panel-reveal trigger
  already covers the same intent, since the button is only reachable from an open panel.
- **Settings, history and pets survive an update** because they live in the app-data directory, which
  the NSIS `/UPDATE` path does not touch, and `install_bundled_pet_packages` explicitly preserves
  user-customized packages via `should_preserve_installed` (`lib.rs:387-389`). Worth asserting during
  manual validation rather than assuming.
- **`tauri-plugin-updater` uses its own reqwest client that follows redirects**, which is required —
  GitHub release asset URLs 302 to `objects.githubusercontent.com`. Do not attempt to reuse the
  collectors' `redirect::Policy::none()` client.
- **Sources**:
  [Tauri v2 updater plugin](https://v2.tauri.app/plugin/updater/) ·
  [`tauri-plugin-updater` v2 source](https://github.com/tauri-apps/tauri-plugin-updater/blob/v2/src/updater.rs) ·
  [MSI pre-release version bug](https://github.com/tauri-apps/tauri/issues/12470) ·
  [NSIS version metadata bug](https://github.com/tauri-apps/tauri/issues/8038) ·
  [WiX version override request](https://github.com/tauri-apps/tauri/issues/8447) ·
  [Windows Installer distribution guide](https://v2.tauri.app/distribute/windows-installer/)
