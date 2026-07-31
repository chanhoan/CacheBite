# Plan: Global Shortcut to Hide/Show the Pet Overlay

## Summary
Add a persisted, user-configurable global hotkey that toggles the pet overlay (and its
anchored panel) between hidden and shown, without stopping the app or its usage polling.
Uses `tauri-plugin-global-shortcut` on the native side; the renderer only gains a text
field in Settings and two new status messages. No new IPC command is needed — the toggle
is driven entirely by the OS-level hotkey callback in Rust.

## User Story
As a beta tester running CacheBite alongside full-screen or focus-heavy work,
I want to press a shortcut I choose to make the pet disappear and reappear,
so that I can reclaim my screen without killing the app (and losing usage polling).

## Problem → Solution
Today the pet is either on screen and always-on-top, or the process is dead. The only way
to get an unobstructed screen is to kill CacheBite. → A persisted `hide_show_hotkey`
setting registers a global OS shortcut at startup and on every settings save; pressing it
toggles the overlay (and panel) hidden/shown in place, reusing whatever position was last
saved — no window is destroyed or recreated.

## Metadata
- **Complexity**: Large (new native dependency, settings schema migration, cross-cutting
  window-visibility interaction with existing fullscreen handling, native+renderer surface)
- **Source PRD**: N/A
- **PRD Phase**: N/A — planned directly from GitHub issue [#30](https://github.com/chanhoan/CacheBite/issues/30)
- **Estimated Files**: 15 (7 Rust, 7 TS/Svelte, 1 doc)

---

## UX Design

### Before
```
┌─────────────────────────────┐
│ Pet is always on screen,     │
│ always-on-top.                │
│ Only way to clear it: kill    │
│ the process (loses polling).  │
└─────────────────────────────┘
```

### After
```
┌─────────────────────────────┐
│ Settings → "Hide/show         │
│ shortcut" text field, e.g.    │
│ "CmdOrCtrl+Shift+H".          │
│ Press it → pet + panel        │
│ vanish. Press again → pet     │
│ reappears at its last saved   │
│ position on its last display. │
│ Polling keeps running either  │
│ way.                          │
└─────────────────────────────┘
```

### Interaction Changes
| Touchpoint | Before | After | Notes |
|---|---|---|---|
| Settings panel | No hotkey field | New text field "Hide/show shortcut" | Empty/whitespace → no hotkey configured (`null`) |
| Global OS hotkey (new) | N/A | Toggles overlay+panel hidden/shown | Only active window-level effect; no renderer button added |
| Settings save failure | Generic "Settings could not be saved" | Same generic message for malformed hotkey syntax; **new** distinct message "Global shortcut could not be registered — it may already be in use" when the OS rejects a syntactically valid combo | See Task 9 |
| Fullscreen auto-hide (existing, Windows-only) | Always re-shows the pet when fullscreen ends | Only re-shows it if the user hasn't hidden it via the hotkey | See "Critical Interaction" below |

---

## Critical Interaction — Do Not Skip

`src-tauri/src/lib.rs:174-202` (`start_fullscreen_monitor`, Windows-only, already shipped) polls
every 500ms and calls `overlay.hide()` / `overlay.show()` directly based on whether the
foreground window is fullscreen. This is a **second, independent caller** of
`hide()`/`show()` on the exact same `overlay` window. Without coordination, this bug
appears: user hides the pet via hotkey → user enters a fullscreen game (monitor hides it,
harmless no-op) → user exits the game → the monitor's existing code
unconditionally calls `overlay.show()`, **silently reversing the user's explicit hide.**

Fix: introduce one shared `AtomicBool` ("has the user explicitly hidden it via hotkey?")
that both call sites consult before showing. See Task 5. This mirrors the existing
`PanelLayoutGate` pattern at `src-tauri/src/refresh/ipc.rs:37-40` — a tiny `Default`-derived
struct wrapping an `AtomicBool`, managed via `app.manage(...)`.

**Do not** attempt to resurrect `RuntimeState` / `apply_fullscreen` / `synchronize_fullscreen`
/ `PlatformWindowAdapter` / `WindowCommand` in `src-tauri/src/window/mod.rs:56-187,256-269`.
This is a fully-designed, unit-tested, but **orphaned** abstraction — `start_fullscreen_monitor`
in `lib.rs` never calls it; it exists only in `window/tests.rs`. It looks like the "right"
place to add this feature, but wiring it in for real means implementing `PlatformWindowAdapter`
for actual Tauri window handles, which is a much larger refactor than this issue needs and
risks regressing the working fullscreen-hide path. Leave it alone; use the small
`AtomicBool` gate instead (Task 5).

---

## Mandatory Reading

| Priority | File | Lines | Why |
|---|---|---|---|
| P0 | `src-tauri/src/lib.rs` | 17-131, 174-202, 340-399 | `setup()`/builder chain, the fullscreen monitor this must coordinate with, `restore_window_positions` (position-recovery precedent) |
| P0 | `src-tauri/src/store/settings.rs` | 1-238 (whole file) | Settings struct, schema-version migration chain, `validate()`, `save_position` (read-modify-write-under-lock precedent for the new `clear_hotkey`) |
| P0 | `src-tauri/src/refresh/ipc.rs` | 100-124, 219-270 | `IpcError` enum, `authorize()`, and the `start_at_login` toggle-on-change + rollback-on-failure block in `update_settings` — the exact pattern the hotkey re-registration mirrors |
| P0 | `src-tauri/src/store/tests.rs` | 128-175 | Exact shape of a schema-migration test (`version_two_settings_migrate_with_secondary_notifications_off`) to copy for the new V3→V4 case |
| P1 | `src-tauri/src/window/mod.rs` | 56-135, 168-187 | `NativeCommand`/`command_allowed` (confirm no new command needed), `panel_reveal` (precedent for a tiny named pure policy function), and the orphaned `RuntimeState` block — read it to understand why NOT to touch it |
| P1 | `src-tauri/Cargo.toml` | 21-34 | Existing plugin dependency declarations to mirror |
| P1 | `src/lib/api/gateway.ts` | 24-33, 102-142 | `AppSettings`, `SettingsWire`, `fromSettings`/`toSettings` — the field must be added in 4 places here |
| P1 | `src/App.svelte` | 77-89, 129-132, 534-579, 674-688 | Default `appSettings` object, `settingsSaveFailed` state + the `changeSettings` try/catch this plan extends, and the status-message rendering block to mirror |
| P2 | `src/lib/components/SettingsPanel.svelte` | 33-104 | Field/toggle markup and CSS classes (`.field`, `.toggle`) to mirror for the new text input |
| P2 | `src/lib/state/presentation.ts` | 1-26 | `SettingsStoreState` (a `Pick<AppSettings, ...>`) and `toSettingsStoreState` — both need the new field |
| P2 | `src/App.test.ts` | 631-651 | Exact test pattern for a rejected `updateSettings` call and the resulting status message — mirror for the new hotkey-failure test |

## External Documentation

| Topic | Source | Key Takeaway |
|---|---|---|
| `tauri-plugin-global-shortcut` install | github.com/tauri-apps/plugins-workspace (v2, `plugins/global-shortcut/README.md`) | Add `tauri-plugin-global-shortcut = "2.3.2"` to `[dependencies]` (confirmed current version via docs.rs "All Items" page — **re-verify against crates.io at implementation time**, it moves independently of `tauri`). No JS package needed — this plan never calls the plugin from the renderer. |
| Runtime (not just setup-time) registration | docs.rs `tauri_plugin_global_shortcut` public API | `GlobalShortcutExt` trait (implemented for `AppHandle`, same shape as `tauri_plugin_autostart::ManagerExt` already used at `ipc.rs:254-256`) exposes `.global_shortcut()`, which has `.register(shortcut)`, `.unregister(shortcut)`, `.is_registered(shortcut)`, all generic over `S: TryInto<ShortcutWrapper>` — `&str` works directly, e.g. `manager.register("CmdOrCtrl+Shift+H")`. |
| Shortcut string parsing/validation | Same | `Shortcut` implements `FromStr` (via the `global_hotkey` crate) and is re-exported at the crate root as `tauri_plugin_global_shortcut::Shortcut`, independent of any running app — usable for pure validation in `store/settings.rs` without touching Tauri runtime state. |
| Registration failure | Same | Returns `Err(tauri_plugin_global_shortcut::Error)` — this is exactly the "registration failure is a real state" case the issue calls out; it must propagate to a rollback + a distinct `IpcError`, not be swallowed. |
| Handler wiring | plugins-workspace README example | `Builder::new().with_handler(\|app, shortcut, event\| { if event.state == ShortcutState::Pressed { ... } }).build()`, added via `.plugin(...)` on the top-level `tauri::Builder` chain (same place `tauri_plugin_notification::init()`/`tauri_plugin_autostart::init(...)` are added today at `lib.rs:17-22`) — **not** inside `setup()`. |
| Tauri IPC error shape on the JS side | Tauri v2 `@tauri-apps/api/core` `invoke()` + `ipc-protocol.js` internals | A `#[tauri::command]` returning `Err(e)` where `e: Serialize` rejects the JS `Promise` with the **raw deserialized value**, not a wrapped `Error`. Since `IpcError` is a fieldless enum with `#[serde(rename_all = "snake_case")]` and no `tag`/`content`, `Err(IpcError::HotkeyUnavailable)` rejects with the bare string `"hotkey_unavailable"`. **GOTCHA**: `App.test.ts`'s existing tests always mock rejections as `new Error('...')` (a testing convention that was never load-bearing because nothing inspected error content before). The new test must reject with the raw string `'hotkey_unavailable'`, not `new Error(...)`, or the new branch will never trigger and the test will give false confidence. |

---

## Architecture

### Approach
Two independent halves that meet at one persisted setting:
1. **Native**: a new `hide_show_hotkey: Option<String>` settings field, validated (parsed)
   at the storage boundary; registered with `tauri-plugin-global-shortcut` at startup and
   re-registered on every `update_settings` call where it changed (mirrors the existing
   `start_at_login` ⇄ `tauri_plugin_autostart` toggle exactly); a plugin-level handler that
   toggles overlay+panel visibility on every press, coordinated with the existing
   Windows-only fullscreen monitor via one shared `AtomicBool`.
2. **Renderer**: one new field threaded through the existing settings pipe
   (`AppSettings` → `SettingsStoreState` → `SettingsPanel`), plus one new failure message
   distinguishing "shortcut syntax invalid" (already covered by the existing generic
   `settingsSaveFailed` path) from "shortcut syntactically valid but the OS/another app
   already owns it" (new, distinct message).

### Alternatives Considered
- **A renderer-triggered `toggle_overlay`/`hide_pet` IPC command** (with a `NativeCommand`
  authorization-table entry, as the issue's notes speculate might be needed): rejected.
  The issue's actual ask is purely hotkey-driven ("Press it again to hide it"); there is no
  request for an in-panel "Hide" button, and adding one is scope creep the issue doesn't ask
  for. If a future issue wants an in-panel toggle too, it can call the same native toggle
  function this plan introduces.
- **A new `PlatformCapabilities.hotkey` static capability field** (mirroring
  `always_on_top`/`fullscreen_detection`/`autostart`): rejected. Those three are fixed
  properties of the OS/build, checked once. Whether *a specific hotkey string* registers
  successfully is inherently dynamic (depends on what else is running) and can only be known
  at the moment of registration — which is exactly what the new `IpcError::HotkeyUnavailable`
  from `update_settings` already reports, more precisely than a static capability could.
- **Persisting "is currently hidden" across restarts**: rejected. Only the hotkey *binding*
  is persisted. Every launch starts with the pet visible (subject to the pre-existing,
  independent fullscreen check). This avoids a second persistence dimension the issue never
  asked for and matches the mental model "restarting resets transient UI state."
- **A capture-keystrokes-live hotkey recorder widget** in Settings: rejected as scope creep.
  A plain `<input type="text">` (matching every other Settings field's plain-HTML-control
  style) with the string parsed/validated in Rust is sufficient and consistent with the
  codebase's existing minimalism.

### Scope
- New `hide_show_hotkey` setting, persisted, editable from the Settings panel.
- Global registration at startup (best-effort, self-healing if it fails) and on every
  settings save (hard failure surfaced to the UI).
- Toggle handler hides/shows the overlay and, if visible, the panel.
- Correct composition with the existing Windows-only fullscreen auto-hide.
- Schema migration (v3 → v4) so existing beta testers' settings files are not quarantined.

### NOT Building
- Any renderer-visible button or menu item for hiding/showing (hotkey-only, per the issue).
- Any change to bubble/notification suppression logic. Bubbles render *inside* the overlay
  window (`src/App.svelte`'s `.overlay-stack` in the non-panel branch), so hiding that window
  via `overlay.hide()` already makes any open bubble invisible for free — no renderer code
  change needed. Native OS notifications (`notificationPolicy.ts`) are independent of window
  visibility today (they already fire from a hidden/backgrounded webview) and the issue does
  not ask to mute them while hidden — usage alerts arguably matter *more* while the pet is
  tucked away and can't be glanced at. This is a deliberate decision, not an oversight.
- A static `PlatformCapabilities.hotkey` field (see Alternatives Considered).
- Any change to the orphaned `RuntimeState`/`PlatformWindowAdapter` abstraction.
- Fixing the pre-existing, unrelated gap where `interactionStore.setFullscreen` in
  `src/lib/stores/interaction.ts` is never actually called from `App.svelte` (discovered
  during exploration; out of scope for this issue).

---

## Patterns to Mirror

### SCHEMA_MIGRATION_STRUCT
// SOURCE: src-tauri/src/store/settings.rs:76-86, 183-198
```rust
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsV2 {
    schema_version: u32,
    primary_provider: Provider,
    selected_pet_id: String,
    bubble_enabled: bool,
    start_at_login: bool,
    notification_enabled: bool,
    logical_position: LogicalPosition,
}
// ...
if let Ok(previous) = serde_json::from_slice::<SettingsV2>(&bytes) {
    if previous.schema_version == 2 {
        let migrated = Settings {
            primary_provider: previous.primary_provider,
            selected_pet_id: previous.selected_pet_id,
            bubble_enabled: previous.bubble_enabled,
            start_at_login: previous.start_at_login,
            notification_enabled: previous.notification_enabled,
            logical_position: previous.logical_position,
            ..Settings::default()
        };
        validate(&migrated)?;
        write_json_atomically(&self.path, &migrated)?;
        return self.load_locked();
    }
}
```
**GOTCHA**: it is tempting to skip this because the new field is `Option<String>`, and
serde's derive macro auto-defaults a genuinely *absent* `Option<T>` JSON key to `None`
without needing `#[serde(default)]`. That part is true — but it does **not** save you from
needing a migration struct here, because `load_locked()`'s fast path
(`serde_json::from_slice::<Settings>(&bytes)`) is gated by `validate(&settings)`, which
rejects on `schema_version != SETTINGS_SCHEMA_VERSION` regardless of whether the rest of the
struct happened to parse. Every existing beta tester's file is stamped `"schema_version":3`.
Without a `SettingsV3` struct + migration branch, `load_locked()` falls through the whole
chain to `quarantine(&self.path)`, **silently wiping every beta tester's saved position,
provider, pet choice, and notification settings** on first launch after the update. Copy the
`SettingsV2` shape above verbatim as `SettingsV3` (rename fields to match the *current*
`Settings` struct, which already includes `secondary_notification_enabled`), gate on
`previous.schema_version == 3`, and set `hide_show_hotkey: None` in the migrated struct.

### READ_MODIFY_WRITE_UNDER_LOCK
// SOURCE: src-tauri/src/store/settings.rs:117-129
```rust
pub fn save_position(&self, logical_position: LogicalPosition) -> io::Result<()> {
    let _guard = self
        .lock
        .lock()
        .map_err(|_| io::Error::other("settings lock poisoned"))?;
    let current = self.load_locked()?;
    let updated = Settings {
        logical_position,
        ..current
    };
    validate(&updated)?;
    write_json_atomically(&self.path, &updated)
}
```
Mirror exactly for the new `SettingsRepository::clear_hotkey(&self) -> io::Result<()>`
(Task 3), swapping `logical_position` for `hide_show_hotkey: None`.

### TOGGLE_SETTING_WITH_ROLLBACK
// SOURCE: src-tauri/src/refresh/ipc.rs:235-270 (`update_settings`)
```rust
if previous.start_at_login != settings.start_at_login {
    use tauri_plugin_autostart::ManagerExt;

    let manager = app.autolaunch();
    let result = if settings.start_at_login {
        manager.enable()
    } else {
        manager.disable()
    };
    if result.is_err() {
        let _ = repository.save(&previous);
        return Err(IpcError::ServiceUnavailable);
    }
}
```
Mirror exactly for the hotkey re-registration block (Task 8), using
`tauri_plugin_global_shortcut::GlobalShortcutExt` in place of
`tauri_plugin_autostart::ManagerExt`, and a new `IpcError::HotkeyUnavailable` in place of
`IpcError::ServiceUnavailable`.

### SMALL_ATOMIC_GATE
// SOURCE: src-tauri/src/refresh/ipc.rs:34-40
```rust
/// Tracks a `show_panel` request that is still waiting for the renderer to
/// report its content height, so the panel is revealed only once it has been
/// placed at its final size.
#[derive(Default)]
pub struct PanelLayoutGate {
    awaiting_layout: AtomicBool,
}
```
Mirror for the new `OverlayHideGate` in `lib.rs` (Task 5).

### PURE_NAMED_POLICY_FUNCTION
// SOURCE: src-tauri/src/window/mod.rs:106-129
```rust
/// What `show_panel` must do for a panel in the given visibility state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PanelReveal {
    RaiseExisting,
    AwaitLayout,
}

pub fn panel_reveal(visible: bool) -> PanelReveal {
    if visible {
        PanelReveal::RaiseExisting
    } else {
        PanelReveal::AwaitLayout
    }
}
```
Mirror for the new one-line `should_restore_overlay_after_fullscreen` (Task 6) — same
granularity, same "small pure function gets its own unit test" convention as
`src-tauri/src/window/tests.rs:345-349`.

### SETTINGS_PANEL_FIELD
// SOURCE: src/lib/components/SettingsPanel.svelte:65-76
```svelte
<label class="field"
  >Primary provider <select
    value={settings.primaryProvider}
    onchange={(event) =>
      onChange({
        ...settings,
        primaryProvider: asProvider(event.currentTarget.value),
      })}
    ><option value="claude">Claude</option><option value="codex">Codex</option
    ></select
  ></label
>
```
Mirror the `.field` wrapper markup/CSS for a text `<input>` instead of a `<select>` (Task 12).

### GATEWAY_WIRE_MAPPING
// SOURCE: src/lib/api/gateway.ts:123-142
```typescript
const fromSettings = (wire: SettingsWire): AppSettings => ({
  schemaVersion: wire.schema_version,
  primaryProvider: wire.primary_provider,
  // ...
  logicalPosition: wire.logical_position,
});
const toSettings = (settings: AppSettings): SettingsWire => ({
  schema_version: settings.schemaVersion,
  primary_provider: settings.primaryProvider,
  // ...
  logical_position: settings.logicalPosition,
});
```
Add `hideShowHotkey`/`hide_show_hotkey` to both directions plus the `AppSettings` and
`SettingsWire` type declarations (Task 10).

### SETTINGS_SAVE_FAILURE_MESSAGE
// SOURCE: src/App.svelte:674-682
```svelte
{#if settingsSaveFailed}<p role="status">
    Settings could not be saved
  </p>{/if}
{#if platformCapabilities?.autostart.status === 'unavailable'}
  <p role="status">{platformCapabilities.autostart.reason}</p>
{/if}
```
Mirror for the new `hotkeySaveFailed` branch (Task 14), placed as a sibling `{:else if}` so
the two are mutually exclusive.

---

## Files to Change

| File | Action | Justification |
|---|---|---|
| `src-tauri/Cargo.toml` | UPDATE | Add `tauri-plugin-global-shortcut` dependency |
| `src-tauri/src/store/settings.rs` | UPDATE | New field, schema bump 3→4, `SettingsV3` migration struct, `validate()` parses the hotkey, new `clear_hotkey()` method |
| `src-tauri/src/store/tests.rs` | UPDATE | Migration test, validate-rejects-malformed-hotkey test, `clear_hotkey` test |
| `src-tauri/src/window/mod.rs` | UPDATE | One new pure function `should_restore_overlay_after_fullscreen` |
| `src-tauri/src/window/tests.rs` | UPDATE | Unit test for the new pure function |
| `src-tauri/src/refresh/ipc.rs` | UPDATE | New `IpcError::HotkeyUnavailable` variant; `update_settings` re-registers the hotkey on change with rollback |
| `src-tauri/src/lib.rs` | UPDATE | Register plugin + handler; `OverlayHideGate`; `toggle_overlay_visibility`; startup registration with self-healing; fullscreen monitor consults the gate before re-showing |
| `src/lib/api/gateway.ts` | UPDATE | `AppSettings.hideShowHotkey`, `SettingsWire.hide_show_hotkey`, both mapping directions |
| `src/lib/api/gateway.test.ts` | UPDATE | Fixture + mapping assertion for the new field |
| `src/lib/api/fixtureGateway.ts` | UPDATE | `getSettings()` fixture includes the new field |
| `src/lib/state/presentation.ts` | UPDATE | `SettingsStoreState` pick + `toSettingsStoreState` |
| `src/lib/components/SettingsPanel.svelte` | UPDATE | New text field for the hotkey |
| `src/lib/components/SettingsPanel.test.ts` | UPDATE | Assert the field emits an immutable change |
| `src/App.svelte` | UPDATE | Default `appSettings.hideShowHotkey`, `hotkeySaveFailed` state, catch-block branching, status message |
| `src/App.test.ts` | UPDATE | Test the distinct hotkey-failure message (raw-string rejection, see External Documentation GOTCHA) |
| `docs/beta-testing.md` | UPDATE | Mention the new Settings field in the "First run" list |

## NOT Building
- In-panel hide/show button or menu entry.
- Bubble/notification suppression while hidden (see NOT Building above for rationale).
- A `PlatformCapabilities.hotkey` static capability.
- Persistence of hidden/shown state across app restarts.
- A live keystroke-capture widget for entering the hotkey.
- Any fix to the pre-existing dead `interactionStore.setFullscreen` wiring gap.
- Any change to `src-tauri/capabilities/overlay.json` or `panel.json` — no renderer code
  ever calls the plugin directly (it's Rust-only), so no new permission entries are needed.

---

## Step-by-Step Tasks

### Task 1: Add the native dependency
- **ACTION**: Add `tauri-plugin-global-shortcut` to `src-tauri/Cargo.toml`.
- **IMPLEMENT**: Insert `tauri-plugin-global-shortcut = "2.3.2"` in the `[dependencies]`
  block (`src-tauri/Cargo.toml:27-28`), directly after `tauri-plugin-autostart = "2.5.1"`.
  Add it unconditionally (not under a `target.'cfg(not(any(target_os = "android", target_os
  = "ios")))'` gate like the plugin's own README suggests) — this project has no mobile
  build target today, and every other Tauri plugin here (`notification`, `autostart`) is
  declared unconditionally too.
- **MIRROR**: `src-tauri/Cargo.toml:27-28`.
- **IMPORTS**: N/A (Cargo.toml).
- **GOTCHA**: Verify `2.3.2` is still current on crates.io before pinning (the automated
  check during planning hit crates.io's anti-scraping rate limit; docs.rs confirmed `2.3.2`
  as of this plan's research, sourced independently from the plugins-workspace README's
  generic `"2.0.0"` floor). Any `2.x` compatible with `tauri = "2.8.5"` and Rust 1.77.2
  (this crate's own stated MSRV) is fine.
- **VALIDATE**: `cargo check --manifest-path src-tauri/Cargo.toml` resolves and updates
  `Cargo.lock`.

### Task 2: Extend the Settings schema with the new field
- **ACTION**: Add `hide_show_hotkey: Option<String>` to the `Settings` struct and its
  `Default` impl in `src-tauri/src/store/settings.rs`.
- **IMPLEMENT**: In the `Settings` struct (`settings.rs:27-38`), append
  `pub hide_show_hotkey: Option<String>,` after `logical_position`. In `impl Default for
  Settings` (`settings.rs:40-53`), append `hide_show_hotkey: None,`. Bump
  `SETTINGS_SCHEMA_VERSION` from `3` to `4` (`settings.rs:12`).
- **MIRROR**: The existing `notification_enabled`/`secondary_notification_enabled` fields
  already in the struct — same style, just `Option<String>` instead of `bool`.
- **IMPORTS**: None new.
- **GOTCHA**: None yet at this step — the schema-version bump is what makes the migration
  in Task 4 mandatory. Do not skip Task 4.
- **VALIDATE**: `cargo check --manifest-path src-tauri/Cargo.toml` — it will fail to compile
  until every `Settings { .. }` literal elsewhere (tests, `Settings::default()` call sites)
  is updated; that's expected and resolved by later tasks / `..Settings::default()` spreads.

### Task 3: Validate the hotkey string and add `clear_hotkey`
- **ACTION**: Reject malformed hotkey strings at the storage boundary; add a targeted-update
  method to clear a hotkey that failed to register.
- **IMPLEMENT**: In `validate()` (`settings.rs:217-237`), add:
  ```rust
  if let Some(hotkey) = &settings.hide_show_hotkey {
      use std::str::FromStr;
      if tauri_plugin_global_shortcut::Shortcut::from_str(hotkey).is_err() {
          return Err(io::Error::new(io::ErrorKind::InvalidData, "hotkey is invalid"));
      }
  }
  ```
  Add a new `SettingsRepository` method mirroring `save_position` exactly (see
  READ_MODIFY_WRITE_UNDER_LOCK pattern above):
  ```rust
  pub fn clear_hotkey(&self) -> io::Result<()> {
      let _guard = self
          .lock
          .lock()
          .map_err(|_| io::Error::other("settings lock poisoned"))?;
      let current = self.load_locked()?;
      let updated = Settings {
          hide_show_hotkey: None,
          ..current
      };
      validate(&updated)?;
      write_json_atomically(&self.path, &updated)
  }
  ```
- **MIRROR**: `save_position` at `settings.rs:117-129`.
- **IMPORTS**: `std::str::FromStr` (function-local `use`, matching the file's existing style
  of no blanket top-level imports for narrowly-used traits).
- **GOTCHA**: `Shortcut::from_str` is independent of any running Tauri app/event loop — it's
  pure string parsing (backed by the `global_hotkey` crate), so this works fine inside the
  storage layer without pulling in `AppHandle`.
- **VALIDATE**: New unit test (Task 4) — `SettingsRepository::save` rejects a settings value
  with `hide_show_hotkey: Some("not a real shortcut".into())`.

### Task 4: Migrate existing v3 settings files to v4
- **ACTION**: Add a `SettingsV3` legacy struct and migration branch so every existing beta
  tester's on-disk settings file survives the upgrade instead of being quarantined.
- **IMPLEMENT**: Add, mirroring `SettingsV2` exactly (`settings.rs:76-86`):
  ```rust
  #[derive(Deserialize)]
  #[serde(deny_unknown_fields)]
  struct SettingsV3 {
      schema_version: u32,
      primary_provider: Provider,
      selected_pet_id: String,
      bubble_enabled: bool,
      start_at_login: bool,
      notification_enabled: bool,
      secondary_notification_enabled: bool,
      logical_position: LogicalPosition,
  }
  ```
  Add a migration branch in `load_locked()` immediately after the existing `SettingsV2`
  branch (`settings.rs:183-198`), before the `LegacySettings` fallback:
  ```rust
  if let Ok(previous) = serde_json::from_slice::<SettingsV3>(&bytes) {
      if previous.schema_version == 3 {
          let migrated = Settings {
              primary_provider: previous.primary_provider,
              selected_pet_id: previous.selected_pet_id,
              bubble_enabled: previous.bubble_enabled,
              start_at_login: previous.start_at_login,
              notification_enabled: previous.notification_enabled,
              secondary_notification_enabled: previous.secondary_notification_enabled,
              logical_position: previous.logical_position,
              ..Settings::default()
          };
          validate(&migrated)?;
          write_json_atomically(&self.path, &migrated)?;
          return self.load_locked();
      }
  }
  ```
- **MIRROR**: SCHEMA_MIGRATION_STRUCT pattern above (`settings.rs:76-86,183-198`).
- **IMPORTS**: None new (same `Deserialize`/`serde_json` already imported).
- **GOTCHA**: See the SCHEMA_MIGRATION_STRUCT pattern's GOTCHA note above — this is not
  optional even though the new field is `Option<String>`. Read it before skipping this task.
- **VALIDATE**: New test in `store/tests.rs` mirroring
  `version_two_settings_migrate_with_secondary_notifications_off`
  (`store/tests.rs:161-175`):
  ```rust
  #[test]
  fn version_three_settings_migrate_with_no_hotkey() {
      let dir = TempDir::new().expect("temp dir");
      fs::write(
          dir.path().join("settings.json"),
          r#"{"schema_version":3,"primary_provider":"claude","selected_pet_id":"idle","bubble_enabled":true,"start_at_login":false,"notification_enabled":true,"secondary_notification_enabled":false,"logical_position":{"x":0.0,"y":0.0}}"#,
      )
      .expect("write v3 settings");
      let loaded = SettingsRepository::new(dir.path())
          .load()
          .expect("migrate v3");
      assert_eq!(loaded.schema_version, 4);
      assert_eq!(loaded.hide_show_hotkey, None);
  }
  ```
  Also add a rejection test: `SettingsRepository::save` with an invalid hotkey string returns
  `Err` with `ErrorKind::InvalidData`, and a round-trip test with a valid hotkey string
  (e.g. `"CmdOrCtrl+Shift+H"`) succeeds — mirror `settings_round_trip_is_versioned_and_atomic`
  (`store/tests.rs:57-74`).

### Task 5: Add the shared hide/fullscreen coordination gate
- **ACTION**: Add `OverlayHideGate` and wire it into both the (new) hotkey toggle and the
  (existing) fullscreen monitor in `src-tauri/src/lib.rs`.
- **IMPLEMENT**:
  ```rust
  /// Whether the overlay is hidden because the user explicitly toggled it via the
  /// hide/show hotkey — as opposed to the (independent, Windows-only) fullscreen
  /// monitor hiding it automatically. Both hide/show call sites consult this so
  /// exiting fullscreen never resurrects a pet the user explicitly hid.
  #[derive(Default)]
  struct OverlayHideGate {
      user_hidden: std::sync::atomic::AtomicBool,
  }
  ```
  Manage it in `setup()` (near `app.manage(refresh::ipc::PanelLayoutGate::default());` at
  `lib.rs:107`): `app.manage(OverlayHideGate::default());`.
  Add the toggle function:
  ```rust
  fn toggle_overlay_visibility(app: &tauri::AppHandle) {
      use std::sync::atomic::Ordering;
      use tauri::Manager;
      let gate = app.state::<OverlayHideGate>();
      let hidden = {
          let next = !gate.user_hidden.load(Ordering::SeqCst);
          gate.user_hidden.store(next, Ordering::SeqCst);
          next
      };
      let Some(overlay) = app.get_webview_window("overlay") else {
          return;
      };
      if hidden {
          let _ = overlay.hide();
          if let Some(panel) = app.get_webview_window("panel") {
              let _ = panel.hide();
          }
          return;
      }
      #[cfg(windows)]
      if window::foreground_window_is_fullscreen() {
          // Leave it hidden; start_fullscreen_monitor's exit edge will show it
          // once fullscreen ends, now that user_hidden is false.
          return;
      }
      let _ = overlay.show();
  }
  ```
  Update `start_fullscreen_monitor`'s show branch (`lib.rs:188-193`):
  ```rust
  if let Some(overlay) = app.get_webview_window("overlay") {
      let _ = if fullscreen {
          overlay.hide()
      } else if window::should_restore_overlay_after_fullscreen(
          app.state::<OverlayHideGate>().user_hidden.load(Ordering::SeqCst),
      ) {
          overlay.show()
      } else {
          Ok(())
      };
  }
  ```
- **MIRROR**: `PanelLayoutGate` (`refresh/ipc.rs:34-40`) for the gate shape;
  `start_fullscreen_monitor` (`lib.rs:174-202`) for the surrounding style (best-effort
  `let _ = ...`, no propagated errors).
- **IMPORTS**: `std::sync::atomic::{AtomicBool, Ordering}` (already imported in this scope
  for `PanelLayoutGate` usage patterns elsewhere — `refresh/ipc.rs:1` — add locally to
  `lib.rs` if not already present).
- **GOTCHA**: `toggle_overlay_visibility` must **flip the flag first, then act** — the
  ordering above stores `next` before touching any window, so a rapid double-press can't
  race two divergent reads of the old value into two `hide()` or two `show()` calls in a row.
- **VALIDATE**: Covered indirectly by Task 6's unit test for the pure predicate; the
  imperative wiring itself is exercised by manual testing (Task 16) since it requires a real
  window/OS.

### Task 6: Add the pure fullscreen-restore predicate
- **ACTION**: Add a tiny, named, unit-tested pure function for "should exiting fullscreen
  restore the overlay?" instead of an inline boolean in `lib.rs`.
- **IMPLEMENT**: In `src-tauri/src/window/mod.rs`, near `panel_reveal`
  (`window/mod.rs:117-129`):
  ```rust
  /// Whether the overlay should be shown again when fullscreen ends. `false` when
  /// the user explicitly hid it via the hide/show hotkey — fullscreen exiting must
  /// not silently reverse that.
  pub fn should_restore_overlay_after_fullscreen(user_hidden: bool) -> bool {
      !user_hidden
  }
  ```
- **MIRROR**: `panel_reveal` (`window/mod.rs:123-129`) — same one-line-body, doc-commented,
  publicly-exported policy function style.
- **IMPORTS**: None new.
- **GOTCHA**: Resist the urge to make this take/return a richer type (e.g. reusing
  `RuntimeState`) — see "Critical Interaction" above for why that's out of scope here.
- **VALIDATE**: New test in `window/tests.rs` mirroring
  `a_visible_panel_is_raised_rather_than_left_behind_another_window`
  (`window/tests.rs:346-349`):
  ```rust
  #[test]
  fn fullscreen_exit_does_not_restore_a_hotkey_hidden_overlay() {
      assert!(should_restore_overlay_after_fullscreen(false));
      assert!(!should_restore_overlay_after_fullscreen(true));
  }
  ```

### Task 7: Register the plugin and startup hotkey
- **ACTION**: Wire the plugin into the builder chain; register the saved hotkey (if any) at
  startup, self-healing if registration fails.
- **IMPLEMENT**: Add to the builder chain in `run()` (`lib.rs:17-22`), after the autostart
  plugin:
  ```rust
  .plugin(
      tauri_plugin_global_shortcut::Builder::new()
          .with_handler(|app, _shortcut, event| {
              if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                  toggle_overlay_visibility(app);
              }
          })
          .build(),
  )
  ```
  In `setup()`, extend the existing settings-load block (`lib.rs:45-48`):
  ```rust
  let settings_repository = store::SettingsRepository::new(&app_data);
  if let Ok(settings) = settings_repository.load() {
      restore_window_positions(app, &settings);
      if let Some(hotkey) = &settings.hide_show_hotkey {
          register_startup_hotkey(app.handle(), &settings_repository, hotkey);
      }
  }
  ```
  Add the helper:
  ```rust
  fn register_startup_hotkey(
      app: &tauri::AppHandle,
      repository: &store::SettingsRepository,
      hotkey: &str,
  ) {
      use tauri_plugin_global_shortcut::GlobalShortcutExt;
      if app.global_shortcut().register(hotkey).is_err() {
          eprintln!("failed to register saved hotkey {hotkey}; clearing it");
          let _ = repository.clear_hotkey();
      }
  }
  ```
- **MIRROR**: The existing `if let Err(error) = install_bundled_pet_packages(...) {
  eprintln!(...) }` tolerance style at `lib.rs:41-44`, and `restore_window_positions`'s
  call site for where a settings-derived side effect belongs.
- **IMPORTS**: `tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState}` (function
  or module scoped, matching the file's existing `use tauri_plugin_autostart::ManagerExt;`
  style at `ipc.rs:254` — narrow, local imports).
- **GOTCHA**: `settings_repository` is still a locally-owned variable at this point in
  `setup()` — it isn't moved into `app.manage(settings_repository)` until `lib.rs:100`, well
  after this block, so passing `&settings_repository` here is safe. Double-check the exact
  field/method name for the press-state check against whichever `2.3.x` version actually
  gets pinned in Task 1 — the plugin's README shows `event.state == ShortcutState::Pressed`
  as a field comparison; if the pinned version exposes it as a method (`event.state()`)
  instead, `cargo check` will fail loudly and the fix is mechanical.
- **VALIDATE**: `cargo check --manifest-path src-tauri/Cargo.toml --all-features`. Startup
  registration itself needs manual verification (Task 16) — it can't be unit tested without
  a real OS-level shortcut manager.

### Task 8: Re-register the hotkey on settings save
- **ACTION**: Extend `update_settings` to unregister the old hotkey and register the new one
  whenever it changes, rolling back and surfacing a distinct error on registration failure.
- **IMPLEMENT**: Add a new `IpcError` variant (`ipc.rs:109-118`):
  ```rust
  pub enum IpcError {
      Forbidden,
      ServiceUnavailable,
      InvalidSettings,
      InvalidPanelSize,
      PersistenceUnavailable,
      PanelUnavailable,
      HotkeyUnavailable,
  }
  ```
  In `update_settings`, immediately after the existing `start_at_login` block
  (`ipc.rs:253-266`), add (see TOGGLE_SETTING_WITH_ROLLBACK pattern above):
  ```rust
  if previous.hide_show_hotkey != settings.hide_show_hotkey {
      use tauri_plugin_global_shortcut::GlobalShortcutExt;

      let manager = app.global_shortcut();
      if let Some(old) = &previous.hide_show_hotkey {
          let _ = manager.unregister(old.as_str());
      }
      if let Some(new_hotkey) = &settings.hide_show_hotkey {
          if manager.register(new_hotkey.as_str()).is_err() {
              let _ = repository.save(&previous);
              return Err(IpcError::HotkeyUnavailable);
          }
      }
  }
  ```
- **MIRROR**: TOGGLE_SETTING_WITH_ROLLBACK pattern above (`ipc.rs:253-266`).
- **IMPORTS**: `tauri_plugin_global_shortcut::GlobalShortcutExt` (function-scoped `use`,
  matching `use tauri_plugin_autostart::ManagerExt;` two lines above).
- **GOTCHA**: Malformed hotkey syntax never reaches this block — it's already rejected by
  `repository.save(&settings)` a few lines earlier (via `validate()`, Task 3), which returns
  `IpcError::InvalidSettings` before this new block ever runs. This block only fires for
  syntactically-valid strings the OS refuses to bind (already taken) — that distinction is
  exactly what lets the renderer show two different messages (Task 14).
- **VALIDATE**: No new Rust unit test — `#[tauri::command]` functions require a live
  `tauri::WebviewWindow`/`AppHandle` and this codebase has no existing harness for
  command-level tests (confirmed: no such tests exist for any of the other six `IpcError`
  variants either). Covered by `cargo check`/`clippy` for compilation correctness and by the
  renderer-side test in Task 15 for the observable behavior, plus manual testing (Task 16)
  for the real OS registration path.

### Task 9: Thread the field through the TypeScript gateway
- **ACTION**: Add `hideShowHotkey`/`hide_show_hotkey` to `AppSettings`, `SettingsWire`, and
  both mapping functions.
- **IMPLEMENT**: In `src/lib/api/gateway.ts`:
  - `AppSettings` (`gateway.ts:24-33`): add `readonly hideShowHotkey: string | null;`
  - `SettingsWire` (`gateway.ts:102-111`): add `hide_show_hotkey: string | null;`
  - `fromSettings` (`gateway.ts:123-132`): add `hideShowHotkey: wire.hide_show_hotkey,`
  - `toSettings` (`gateway.ts:133-142`): add `hide_show_hotkey: settings.hideShowHotkey,`
- **MIRROR**: GATEWAY_WIRE_MAPPING pattern above.
- **IMPORTS**: None new.
- **GOTCHA**: None — this is a direct passthrough field, no transformation needed (unlike
  `getPetPackage`'s asset URL rewriting).
- **VALIDATE**: `pnpm check` (svelte-check/tsc) passes; `pnpm vitest run
  src/lib/api/gateway.test.ts`.

### Task 10: Update the gateway test fixture and mapping assertion
- **ACTION**: Add the field to the shared `settingsWire` fixture and assert it round-trips.
- **IMPLEMENT**: In `src/lib/api/gateway.test.ts`, add `hide_show_hotkey: null,` to the
  `settingsWire` const (`gateway.test.ts:30-39`). Extend the `'maps settings in both
  directions'` test (`gateway.test.ts:64-85`) to also send a non-null value through
  `updateSettings` and assert `calls[1]?.[1]` contains `hide_show_hotkey: 'CmdOrCtrl+Shift+H'`
  (mirroring how `notification_enabled: true` is asserted today).
- **MIRROR**: `gateway.test.ts:30-39,64-85`.
- **IMPORTS**: None new.
- **GOTCHA**: None.
- **VALIDATE**: `pnpm vitest run src/lib/api/gateway.test.ts`.

### Task 11: Update the renderer fixture gateway
- **ACTION**: Add the field to `rendererFixtureGateway.getSettings()`.
- **IMPLEMENT**: In `src/lib/api/fixtureGateway.ts:42-51`, add `hideShowHotkey: null,`.
- **MIRROR**: The existing field list in that object.
- **IMPORTS**: None new.
- **GOTCHA**: None — this fixture backs the renderer-only E2E suite
  (`pnpm test:e2e:renderer`), which cannot exercise real OS hotkey registration; leaving it
  `null` is correct and sufficient.
- **VALIDATE**: `pnpm check` (TypeScript will fail to compile `fixtureGateway.ts` against the
  now-widened `AppSettings` type until this is added).

### Task 12: Add `SettingsStoreState` field
- **ACTION**: Add the field to the panel-facing settings view-model.
- **IMPLEMENT**: In `src/lib/state/presentation.ts:5-26`, add `'hideShowHotkey'` to the
  `Pick<AppSettings, ...>` union (line 5-13) and `hideShowHotkey: settings.hideShowHotkey,`
  to `toSettingsStoreState` (line 15-26).
- **MIRROR**: The existing `secondaryNotificationsEnabled` entry in both places.
- **IMPORTS**: None new.
- **GOTCHA**: None.
- **VALIDATE**: `pnpm check`.

### Task 13: Add the Settings panel text field
- **ACTION**: Add a text input for the hotkey to `SettingsPanel.svelte`.
- **IMPLEMENT**: In `src/lib/components/SettingsPanel.svelte`, add (after the "Start at
  login" toggle, `SettingsPanel.svelte:95-103`):
  ```svelte
  <label class="field"
    >Hide/show shortcut <input
      type="text"
      placeholder="e.g. CmdOrCtrl+Shift+H"
      value={settings.hideShowHotkey ?? ''}
      onchange={(event) => {
        const value = event.currentTarget.value.trim();
        onChange({ ...settings, hideShowHotkey: value === '' ? null : value });
      }}
    /></label
  >
  ```
  Add `readonly hideShowHotkey: string | null` to the component's `SettingsStoreState`
  prop-typed JSDoc `@type` comment implicitly via the shared type import — no separate change
  needed since `settings` is already typed as `SettingsStoreState` (Task 12 covers it).
- **MIRROR**: SETTINGS_PANEL_FIELD pattern above (`.field` wrapper class, same `onChange`
  spread style). Add matching `input[type="text"]` styling next to the existing `select`
  rule in the `<style>` block (`SettingsPanel.svelte:144-151`) so it visually matches (same
  padding/border/radius/background/color/font).
- **IMPORTS**: None new.
- **GOTCHA**: Empty/whitespace-only input must normalize to `null`, not `''` — an empty
  string would fail `Shortcut::from_str("")` server-side and surface as a confusing generic
  "Settings could not be saved" the instant the user clears the field, rather than cleanly
  disabling the feature.
- **VALIDATE**: `pnpm vitest run src/lib/components/SettingsPanel.test.ts` (after Task 14).

### Task 14: Test the Settings panel field
- **ACTION**: Assert the new field emits an immutable settings change.
- **IMPLEMENT**: In `src/lib/components/SettingsPanel.test.ts`, add `hideShowHotkey: null,`
  to the rendered `settings` prop (mirroring lines 12-19), then:
  ```typescript
  await fireEvent.change(screen.getByLabelText('Hide/show shortcut'), {
    target: { value: 'CmdOrCtrl+Shift+H' },
  });
  expect(onChange).toHaveBeenCalledWith(
    expect.objectContaining({ hideShowHotkey: 'CmdOrCtrl+Shift+H' }),
  );
  ```
- **MIRROR**: The existing `fireEvent.change`/`toHaveBeenCalledWith` pairs in the same test
  (`SettingsPanel.test.ts:29-45`).
- **IMPORTS**: None new.
- **GOTCHA**: None.
- **VALIDATE**: `pnpm vitest run src/lib/components/SettingsPanel.test.ts`.

### Task 15: Surface the distinct hotkey-registration-failure message
- **ACTION**: Add `hotkeySaveFailed` state to `App.svelte`, branch on it in the
  `changeSettings` catch block, and render a distinct message.
- **IMPLEMENT**: In `src/App.svelte`:
  - Default settings object (`App.svelte:77-89`): add `hideShowHotkey: null,`.
  - New state near `settingsSaveFailed` (`App.svelte:129`): `let hotkeySaveFailed =
    $state(false);`.
  - In `changeSettings`'s try branch (`App.svelte:557-562`), reset both flags on success:
    `settingsSaveFailed = false; hotkeySaveFailed = false;`.
  - Replace the catch block (`App.svelte:563-574`)'s bare `catch { ... settingsSaveFailed =
    true; }` with:
    ```typescript
    } catch (error) {
      const reconciled = await serializeNotification((current) =>
        configureNotifications(
          current,
          appSettings.notificationsEnabled,
          notificationAdapter,
        ),
      ).catch(() => notificationState);
      notificationState = reconciled;
      notificationDiagnostic = reconciled.diagnostic;
      hotkeySaveFailed = error === 'hotkey_unavailable';
      settingsSaveFailed = !hotkeySaveFailed;
    }
    ```
  - Add the message near the existing `settingsSaveFailed` block
    (`App.svelte:674-676`):
    ```svelte
    {#if hotkeySaveFailed}<p role="status">
        Global shortcut could not be registered — it may already be in use
      </p>{:else if settingsSaveFailed}<p role="status">
        Settings could not be saved
      </p>{/if}
    ```
- **MIRROR**: SETTINGS_SAVE_FAILURE_MESSAGE pattern above.
- **IMPORTS**: None new.
- **GOTCHA**: This is the exact spot the External Documentation section's IPC-error-shape
  research matters: `error === 'hotkey_unavailable'` only works because a `#[tauri::command]`
  `Err(IpcError::HotkeyUnavailable)` rejects the JS promise with the **raw string**
  `"hotkey_unavailable"` (confirmed from Tauri's IPC callback mechanism), not a wrapped
  `Error` object. Do not write `(error as Error).message === 'hotkey_unavailable'` — that
  will always be `false` against the real `tauriGateway` and the message will never appear
  in production, even though a naive test using `mockRejectedValueOnce(new Error(...))`
  (copying the surrounding tests' convention) would make it *look* like it passed.
- **VALIDATE**: Covered by Task 16.

### Task 16: Test the distinct hotkey-registration-failure message
- **ACTION**: Add an `App.test.ts` test proving the new message renders (and the generic one
  does not) when `updateSettings` rejects with the hotkey-specific error.
- **IMPLEMENT**: Mirror `'restores the previous primary when saving a primary change fails'`
  (`App.test.ts:631-651`), but:
  ```typescript
  it('shows a distinct message when the hotkey fails to register', async () => {
    window.history.replaceState({}, '', '/?window=panel');
    const { gateway } = fixture();
    // NOT `new Error(...)` — see the gateway IPC-error-shape GOTCHA in the plan:
    // Err(IpcError::HotkeyUnavailable) rejects with the raw string on the real gateway.
    vi.mocked(gateway.updateSettings).mockRejectedValueOnce('hotkey_unavailable');
    render(App, { props: { gateway, notificationAdapter: notifications } });

    await fireEvent.click(await screen.findByRole('button', { name: 'Settings' }));
    await fireEvent.change(screen.getByLabelText('Hide/show shortcut'), {
      target: { value: 'CmdOrCtrl+Shift+H' },
    });

    await screen.findByText(
      'Global shortcut could not be registered — it may already be in use',
    );
    expect(screen.queryByText('Settings could not be saved')).toBeNull();
  });
  ```
- **MIRROR**: `App.test.ts:631-651`.
- **IMPORTS**: None new.
- **GOTCHA**: See Task 15's gotcha — must reject with the raw string, not `new Error(...)`.
- **VALIDATE**: `pnpm vitest run src/App.test.ts`.

### Task 17: Update beta docs
- **ACTION**: Mention the new setting in the "First run" section.
- **IMPLEMENT**: In `docs/beta-testing.md:87-88`, extend the Settings-view bullet:
  `primary provider (which provider drives the ring), pet selection, bubbles, notifications,
  start at login, hide/show shortcut.` Optionally add one sentence noting the shortcut
  toggles the pet (and panel) hidden/shown without stopping polling.
- **MIRROR**: Existing prose style in that section.
- **IMPORTS**: N/A.
- **GOTCHA**: None.
- **VALIDATE**: Manual read-through; no automated check on prose docs.

---

## Testing Strategy

### Unit Tests

| Test | Input | Expected Output | Edge Case? |
|---|---|---|---|
| `version_three_settings_migrate_with_no_hotkey` | on-disk v3 JSON, no `hide_show_hotkey` key | Migrates to v4, `hide_show_hotkey: None` | Yes — the schema-version gotcha |
| `SettingsRepository::save` with malformed hotkey | `hide_show_hotkey: Some("garbage")` | `Err(InvalidData)` | Yes — boundary validation |
| `SettingsRepository::save`/`load` round trip with valid hotkey | `hide_show_hotkey: Some("CmdOrCtrl+Shift+H")` | Round-trips unchanged | No |
| `clear_hotkey` | Existing settings with a hotkey set | File rewritten with `hide_show_hotkey: None`, all other fields unchanged | No |
| `should_restore_overlay_after_fullscreen` | `true` / `false` | `false` / `true` | Yes — the core coordination bug this plan prevents |
| Gateway mapping | `hide_show_hotkey: 'X'` wire value | `hideShowHotkey: 'X'` model value, round-trips through `updateSettings` | No |
| `SettingsPanel` field change | User types a hotkey string | `onChange` called with `hideShowHotkey` set, `null` when cleared | Yes — empty-string-to-null normalization |
| `App.svelte` hotkey failure message | `updateSettings` rejects with `'hotkey_unavailable'` | Distinct message shown, generic one absent | Yes — the IPC-error-shape gotcha |

### Edge Cases Checklist
- [x] Empty input — normalized to `null` client-side (Task 13), never reaches Rust as `""`.
- [x] Maximum size input — not applicable (arbitrary-length string, OS-level parser rejects
      nonsense; no artificial length cap needed since there's no rendering/storage size
      concern for a short hotkey string).
- [x] Invalid types — `Shortcut::from_str` rejects unparseable strings at the boundary.
- [x] Concurrent access — `SettingsRepository`'s existing `Mutex` lock already serializes all
      reads/writes including the new `clear_hotkey`; no new concurrency surface introduced.
- [x] Network failure — not applicable (no network I/O in this feature).
- [x] Permission denied — covered by the OS-registration-failure path
      (`IpcError::HotkeyUnavailable`); a hotkey another app or the OS itself already owns is
      exactly this case.
- [x] Two independent hide triggers racing (fullscreen monitor vs. hotkey toggle) — covered
      by `should_restore_overlay_after_fullscreen` + `OverlayHideGate` (Tasks 5-6).
- [x] Existing beta testers' on-disk settings files — covered by the v3→v4 migration
      (Task 4).

---

## Validation Commands

### Static Analysis
```bash
pnpm check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
```
EXPECT: Zero type errors, zero clippy warnings.

### Unit Tests
```bash
pnpm vitest run src/lib/api/gateway.test.ts src/lib/components/SettingsPanel.test.ts src/App.test.ts
cargo test --manifest-path src-tauri/Cargo.toml store:: window:: --all-features
```
EXPECT: All pass, including the new migration, validation, and coordination-predicate tests.

### Full Test Suite
```bash
pnpm test:ci
cargo test --manifest-path src-tauri/Cargo.toml --all-features
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
```
EXPECT: No regressions; coverage gate (80% branches/functions/lines/statements) still met —
the new renderer code paths (Tasks 13-15) are exercised by Tasks 14 and 16's tests.

### Dependency/Security Gates (informational — these run automatically in CI, not something
to hand-maintain)
```bash
cargo install cargo-audit --version 0.22.2 --locked
cargo audit --file src-tauri/Cargo.lock
```
EXPECT: No new advisories from `tauri-plugin-global-shortcut` or its transitive deps
(`global-hotkey`, platform FFI crates). If one surfaces, pin a patched point release rather
than suppressing it — there is no existing `cargo audit` ignore-list mechanism in this repo
to fall back on (verified: no `audit.toml`/`.cargo/audit.toml` present).

### Browser Validation
```bash
pnpm dev --host 127.0.0.1 &
pnpm test:e2e:renderer
```
EXPECT: Passes — the fixture gateway (Task 11) keeps `hideShowHotkey: null`, so this suite
never touches real OS registration; it only proves the field doesn't break renderer startup.

### Manual Validation
- [ ] Fresh app start with no hotkey configured: no crash, no spurious log line.
- [ ] Set a hotkey (e.g. `CmdOrCtrl+Shift+H`) in Settings → save succeeds → press it → pet
      and open panel both disappear → press it again → pet reappears at the same position on
      the same display.
- [ ] Set a hotkey already bound by another running app (e.g. a common combo your OS/DE
      reserves) → save fails → the distinct "already be in use" message appears, not the
      generic one.
- [ ] Change the hotkey to a different value while the old one is still bound → old combo no
      longer triggers the toggle, new one does.
- [ ] Clear the hotkey field (empty it, save) → the old combo no longer triggers anything.
- [ ] Windows only: hide via hotkey, then enter and exit a fullscreen application → pet stays
      hidden after exiting fullscreen (does **not** reappear).
- [ ] Windows only: do *not* hide via hotkey, enter and exit a fullscreen application → pet
      reappears after exiting fullscreen, exactly as it does today (no regression).
- [ ] Multi-monitor: drag the pet to a secondary display, hide it, unplug/disable that
      display, show it again → pet recovers onto a remaining display rather than being
      stranded off-screen (existing `clamp_window` behavior via `restore_window_positions`
      is unaffected by this feature since `hide()`/`show()` never move the window — this
      check confirms that assumption holds in practice).
- [ ] Quit and relaunch the app with a hotkey configured → it re-registers automatically
      (press it, pet still toggles) without needing to revisit Settings.
- [ ] Simulate a startup registration failure (configure a hotkey, then externally bind the
      same combo in another app before relaunching CacheBite) → app still starts normally,
      and re-opening Settings shows the hotkey field now empty (self-healed via
      `clear_hotkey`), not still showing the stale, non-functional value.

---

## Acceptance Criteria
- [ ] All 17 tasks completed.
- [ ] All validation commands pass.
- [ ] Tests written and passing (unit + the manual checklist above, since native global-hotkey
      firing cannot be exercised by the existing automated E2E harnesses).
- [ ] No type errors, no clippy warnings.
- [ ] Existing beta testers' settings files migrate cleanly (v3 → v4) with no data loss.
- [ ] Matches UX design: hotkey-only toggle, no new buttons; distinct failure message for a
      taken-vs-malformed hotkey.

## Completion Checklist
- [ ] Code follows discovered patterns (schema migration, read-modify-write-under-lock,
      toggle-with-rollback, small atomic gates, pure named policy functions).
- [ ] Error handling matches codebase style (`let _ = ...` for best-effort native calls,
      typed `IpcError` variants for the renderer boundary).
- [ ] No hardcoded values beyond the plugin version pin (Task 1's GOTCHA covers re-verifying
      it).
- [ ] Documentation updated (`docs/beta-testing.md`).
- [ ] No unnecessary scope additions (see NOT Building).
- [ ] Self-contained — no questions needed during implementation (the schema-migration
      gotcha, the fullscreen-coordination bug, and the IPC-error-shape gotcha are the three
      places a same-shaped-but-uninformed implementation would most likely go wrong; all
      three are called out explicitly above with the exact fix).

## Risks
| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Skipping the v3→v4 migration struct, quarantining every beta tester's settings file | Medium (the `Option<String>`-auto-defaults-on-missing-key serde behavior makes it *look* safe to skip) | High — silent data loss (position, provider, pet, notification prefs) for existing users | Task 4 spells out exactly why `validate()`'s `schema_version` check still requires it; the SCHEMA_MIGRATION_STRUCT pattern's GOTCHA note is written specifically to head this off |
| Fullscreen monitor re-showing a hotkey-hidden pet (see Critical Interaction) | Medium if not addressed, near-zero once Tasks 5-6 land | Medium — confusing, hard-to-reproduce-by-description UX bug reported later as its own issue | `OverlayHideGate` + `should_restore_overlay_after_fullscreen`, unit tested |
| `cargo audit` flags an advisory in `global-hotkey`'s dependency tree | Low | Low-Medium — blocks CI until resolved | No existing ignore-list mechanism in this repo (verified); mitigation is pinning a patched point release, not suppression |
| Exact pinned `tauri-plugin-global-shortcut` version's `ShortcutEvent.state` being a method rather than a field | Low | Low — `cargo check` fails loudly, one-line mechanical fix | Called out explicitly in Task 7's GOTCHA |
| Global-hotkey registration behaving differently across Windows/macOS/Linux X11/Wayland (project's Windows CI/native-smoke coverage is strongest; this plan was researched primarily against Windows, matching the reporting beta tester's platform in issue #30) | Medium on Linux Wayland specifically (`global_hotkey` has known limited/portal-gated support there) | Medium — feature silently no-ops on Wayland | Not building a static capability field for this (see Alternatives Considered) means a Wayland user who sets a hotkey and it silently never fires would get no explicit signal today; flagging as an accepted gap for a possible fast-follow rather than blocking this issue, since issue #30 itself is Windows-reported and the codebase's own `PlatformCapabilities::linux_wayland` precedent shows Wayland-specific degradation is already a known, separately-tracked axis in this project |

## Notes
- No `NativeCommand`/authorization-table change, no `capabilities/*.json` change, and no npm
  package addition are needed anywhere in this plan — the entire feature is Rust-native and
  OS-hotkey-driven, which is a meaningfully smaller renderer/IPC surface than the issue's own
  speculative notes implied.
- The three "gotcha" sections above (schema migration, fullscreen coordination, IPC error
  shape) are the highest-value things this plan captures — each is the kind of detail that
  would very plausibly be gotten wrong by a same-shaped-but-independently-derived
  implementation, and each would fail silently or look like it passed under a naive test.
