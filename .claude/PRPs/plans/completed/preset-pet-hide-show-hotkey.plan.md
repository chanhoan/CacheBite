# Plan: Zero-Configuration Default Pet Hide/Show Shortcut

## Summary

Issue #30의 현재 구현은 사용자가 Settings에서 accelerator 문자열을 직접 입력하고
저장해야만 전역 hide/show shortcut이 활성화된다. 새 설치와 v3 설정 마이그레이션 시
`CommandOrControl+Shift+H`가 자동 적용되도록 바꾸되, OS/다른 앱과 충돌할 때 사용자가
다른 조합으로 바꿀 수 있는 기존 입력 경로는 유지한다.

`CommandOrControl+Shift+H`는 Tauri의 공식 cross-platform accelerator 표기이며,
macOS에서는 `Cmd+Shift+H`, Windows/Linux에서는 `Ctrl+Shift+H`로 해석된다.

## User Story

As a CacheBite user, I want the pet hide/show shortcut to work immediately after
installation, so that I can move the overlay without first learning or typing Tauri
shortcut syntax.

## Problem -> Solution

`hide_show_hotkey: None`이 기본값이라 Settings에서 문자열을 저장하기 전까지 기능이
비활성이다. -> 기본값을 `CommandOrControl+Shift+H`로 지정해 앱 시작 시 자동 등록하고,
Settings에는 현재 shortcut과 플랫폼별 실제 키를 안내한다. 사용자 지정/비활성화는
충돌 복구와 접근성을 위해 유지한다.

## Metadata

- **Complexity**: Small
- **Source PRD**: N/A
- **PRD Phase**: N/A - GitHub issue #30 구현 보정
- **Estimated Files**: 6

## UX Design

### Before

```text
첫 실행 -> shortcut 없음 -> Settings에서 accelerator 직접 입력/저장 -> 사용 가능
```

### After

```text
첫 실행 / v3 업그레이드
  -> CommandOrControl+Shift+H 자동 등록
     Windows/Linux: Ctrl+Shift+H
     macOS:         Cmd+Shift+H
  -> 즉시 pet + panel hide/show
  -> 충돌 시 pet은 보이고 Settings에서 다른 조합 지정 가능
```

### Interaction Changes

| Touchpoint | Before | After | Notes |
|---|---|---|---|
| Fresh install | Shortcut 비활성 | 기본 shortcut 자동 등록 | 저장 동작 불필요 |
| Existing v3 settings | v4 migration 후 `None` | v4 migration 후 기본 shortcut | 기존 사용자도 즉시 사용 |
| Settings | 빈 text input | 기본값과 플랫폼별 안내 | 변경/빈 값 비활성화 유지 |
| Shortcut conflict | 시작 실패 시 `None` 정리 | 동일 안전 동작 + 대체 조합 안내 | 잘못된 활성 상태 방지 |

## Mandatory Reading

| Priority | File | Lines | Why |
|---|---|---|---|
| P0 | `src-tauri/src/store/settings.rs` | 12-54, 228-243, 263-291 | schema v4, defaults, v3 migration, validation |
| P0 | `src-tauri/src/lib.rs` | 17-31, 49-65, 191-240 | plugin handler, startup registration, overlay gate, cleanup |
| P0 | `src-tauri/src/store/tests.rs` | 150-236 | migrations, valid/invalid shortcut, clear test |
| P1 | `src/lib/components/SettingsPanel.svelte` | 95-115, 117 onward | current editable field and styles |
| P1 | `src/lib/components/SettingsPanel.test.ts` | 1-70 | immutable form interaction pattern |
| P1 | `src/lib/api/fixtureGateway.ts` | 45-55 | renderer fixture defaults |
| P2 | `docs/beta-testing.md` | 82-99, 113-117 | first-run and conflict tests |

## External Documentation

| Topic | Source | Key Takeaway |
|---|---|---|
| Shortcut syntax | Context7 `/tauri-apps/tauri-plugin-global-shortcut`, official v2 README/quick reference | `CommandOrControl` maps to Cmd on macOS and Ctrl on Windows/Linux |
| Startup handler | Same official source | `Builder::new().with_handler(...)` receives `ShortcutState::Pressed`; registration can fail |

- **KEY_INSIGHT**: Use canonical `CommandOrControl+Shift+H`, not two cfg-specific strings.
- **APPLIES_TO**: Native default, v3 migration, UI copy, docs, tests.
- **GOTCHA**: Another process or the OS can own the combination, so this is a default rather
  than an unchangeable binding.

## Patterns to Mirror

### NAMING_CONVENTION

// SOURCE: `src-tauri/src/store/settings.rs:12-14`

```rust
const SETTINGS_SCHEMA_VERSION: u32 = 4;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
```

Add beside it:

```rust
pub const DEFAULT_HIDE_SHOW_HOTKEY: &str = "CommandOrControl+Shift+H";
```

Keep persisted `hide_show_hotkey` and renderer `hideShowHotkey`; the wire contract does not
change.

### ERROR_HANDLING

// SOURCE: `src-tauri/src/lib.rs:229-239`

```rust
if app.global_shortcut().register(hotkey).is_err() {
    eprintln!("failed to register saved hotkey {hotkey}; clearing it");
    let _ = repository.clear_hotkey();
}
```

Preserve this safe failure behavior: failure must not hide/exit the app, and settings must
not claim an unavailable shortcut is active.

### LOGGING_PATTERN

// SOURCE: `src-tauri/src/lib.rs:50-53`

```rust
if let Err(error) = install_bundled_pet_packages(&app.path().resource_dir()?, &app_data)
{
    eprintln!("failed to install bundled pet packages: {error}");
}
```

Use native `eprintln!`; add no renderer logging or private data.

### DATA_ACCESS_PATTERN

// SOURCE: `src-tauri/src/store/settings.rs:41-53,228-242`

```rust
impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA_VERSION,
            // ...
            hide_show_hotkey: None,
        }
    }
}

let migrated = Settings {
    // v3 fields ...
    ..Settings::default()
};
```

Set `Some(DEFAULT_HIDE_SHOW_HOTKEY.into())`. The v3 migration inherits the new default
without duplicating its string.

### SERVICE_PATTERN

// SOURCE: `src-tauri/src/lib.rs:54-65`

```rust
let settings_repository = store::SettingsRepository::new(&app_data);
app.manage(OverlayHideGate::default());
if let Ok(settings) = settings_repository.load() {
    restore_window_positions(app, &settings);
    if let Some(hotkey) = &settings.hide_show_hotkey {
        register_startup_hotkey(app.handle(), &settings_repository, hotkey);
    }
}
```

No service/IPC addition is needed. The non-`None` default naturally enters this path after
`OverlayHideGate` exists.

### TEST_STRUCTURE

// SOURCE: `src-tauri/src/store/tests.rs:177-218`

```rust
#[test]
fn version_three_settings_migrate_with_no_hotkey() {
    let loaded = SettingsRepository::new(dir.path())
        .load()
        .expect("migrate v3");
    assert_eq!(loaded.schema_version, 4);
    assert_eq!(loaded.hide_show_hotkey, None);
}
```

Rename it to describe default adoption and assert
`Some(DEFAULT_HIDE_SHOW_HOTKEY.into())`. Retain custom round-trip and clear tests to prove
override and disable behavior.

## Files to Change

| File | Action | Justification |
|---|---|---|
| `src-tauri/src/store/settings.rs` | UPDATE | Add and apply the canonical default |
| `src-tauri/src/store/tests.rs` | UPDATE | Cover fresh defaults, migration, override, disable |
| `src/lib/api/fixtureGateway.ts` | UPDATE | Match native first-run default |
| `src/lib/components/SettingsPanel.svelte` | UPDATE | Explain platform mapping and recovery |
| `src/lib/components/SettingsPanel.test.ts` | UPDATE | Verify guidance and immutable updates |
| `docs/beta-testing.md` | UPDATE | Document zero-config behavior and conflicts |

## NOT Building

- A permanently unchangeable shortcut.
- Separate persisted Windows/macOS strings.
- A key-capture recorder.
- A new IPC command, schema version, DTO field, or dependency.
- Hidden-state persistence.
- Changes to fullscreen, notifications, polling, position restoration, or quit behavior.

## Step-by-Step Tasks

### Task 1: Write failing native default and migration tests

- **ACTION**: Update Rust tests before production code.
- **IMPLEMENT**: Import `DEFAULT_HIDE_SHOW_HOTKEY`; assert `Settings::default()` contains
  it; change v3 migration to expect it. Retain custom round-trip and `clear_hotkey()` tests.
- **MIRROR**: `TEST_STRUCTURE` and existing `TempDir`/raw JSON arrangements.
- **IMPORTS**: Add `DEFAULT_HIDE_SHOW_HOTKEY` to the existing store test imports.
- **GOTCHA**: Persisted v4 `null` is an explicit disable and must remain `None`.
- **VALIDATE**: Focused Rust tests fail before Task 2 and pass after it.

### Task 2: Make the cross-platform shortcut the native default

- **ACTION**: Add one source-of-truth constant and use it in settings defaults.
- **IMPLEMENT**: Define `DEFAULT_HIDE_SHOW_HOTKEY` and change the default from `None` to
  `Some(DEFAULT_HIDE_SHOW_HOTKEY.into())`. Keep schema v4 and migration shape unchanged.
- **MIRROR**: `NAMING_CONVENTION`, `DATA_ACCESS_PATTERN`, `SERVICE_PATTERN`.
- **IMPORTS**: None.
- **GOTCHA**: Use documented `CommandOrControl+Shift+H`; registration stays in startup code.
- **VALIDATE**: Store tests pass and `cargo fmt --check` is clean.

### Task 3: Align renderer fixtures and Settings guidance

- **ACTION**: Match native defaults and explain the behavior.
- **IMPLEMENT**: Set fixture `hideShowHotkey` to `CommandOrControl+Shift+H`. Add concise copy:
  “Default: Ctrl+Shift+H on Windows/Linux, Cmd+Shift+H on macOS.” For an empty value, say no
  shortcut is active and another can be entered. Preserve immutable spread updates.
- **MIRROR**: Current Settings field markup and component `onChange` test.
- **IMPORTS**: None.
- **GOTCHA**: Do not infer OS from `navigator`; list both mappings. Keep the input editable
  as the global-conflict recovery path.
- **VALIDATE**: Focused component tests pass for copy, edit, and clear.

### Task 4: Update beta guidance and conflict coverage

- **ACTION**: Rewrite first-run and conflict instructions.
- **IMPLEMENT**: State the preset works immediately and Settings is for override/disable.
  Verify a conflict leaves the app visible, does not claim activation, and permits replacement.
- **MIRROR**: `docs/beta-testing.md:82-99,101-117`.
- **IMPORTS**: N/A.
- **GOTCHA**: Keep internal `CommandOrControl` syntax out of user-facing instructions.
- **VALIDATE**: Review rendered Markdown and complete Windows manual checks; validate macOS
  before claiming macOS release readiness.

### Task 5: Run regression validation and inspect the scoped diff

- **ACTION**: Run native/renderer gates and audit the adjustment.
- **IMPLEMENT**: Run formatting, lint, types, coverage, native tests, audit, and diff checks.
- **MIRROR**: Repository scripts and manifest commands.
- **IMPORTS**: N/A.
- **GOTCHA**: The worktree already contains issue #30 implementation; do not revert it.
- **VALIDATE**: All commands below pass and no schema/DTO/IPC churn appears.

## Testing Strategy

### Unit Tests

| Test | Input | Expected Output | Edge Case? |
|---|---|---|---|
| Fresh default | No settings file | Default shortcut configured | No |
| v3 migration | Valid v3 JSON | v4 plus default shortcut | Yes |
| v4 disabled | v4 with `null` | Remains `None` | Yes |
| Custom round trip | Valid custom accelerator | Exact value preserved | Yes |
| Explicit clear | Configured shortcut | Only shortcut cleared | Yes |
| Settings guidance | Default fixture | Both platform keys visible | No |
| Override/disable | Custom/empty input | Trimmed value/`null` emitted | Yes |

### Edge Cases Checklist

- [x] Empty input means explicit disable
- [ ] Maximum input remains governed by existing parser/input behavior
- [x] Invalid types rejected by Serde
- [x] Concurrent registration uses existing manager/lock
- [x] Network failure is N/A
- [x] Registration denied leaves the app visible
- [ ] Default already owned by another process
- [ ] Fullscreen exit respects user-hidden state
- [ ] Saved per-display position remains unchanged

## Validation Commands

### Static Analysis

```bash
corepack pnpm check
corepack pnpm lint
cargo clippy --manifest-path src-tauri/Cargo.toml --all-features -- -D warnings
```

EXPECT: Zero type/lint errors or Rust warnings.

### Unit Tests

```bash
cargo test --manifest-path src-tauri/Cargo.toml store::tests --all-features
corepack pnpm exec vitest run src/lib/components/SettingsPanel.test.ts
```

EXPECT: Focused tests pass.

### Full Test Suite

```bash
corepack pnpm test:coverage
cargo test --manifest-path src-tauri/Cargo.toml --all-features
```

EXPECT: All tests pass and coverage remains at least 80%.

### Build

```bash
corepack pnpm build
cargo check --manifest-path src-tauri/Cargo.toml --all-features
```

EXPECT: Renderer and native builds succeed.

### Dependency and Diff Checks

```bash
cargo audit --file src-tauri/Cargo.lock
git diff --check
git diff -- src-tauri/src/store/settings.rs src-tauri/src/store/tests.rs src/lib/api/fixtureGateway.ts src/lib/components/SettingsPanel.svelte src/lib/components/SettingsPanel.test.ts docs/beta-testing.md
```

EXPECT: No high/critical advisory, whitespace error, unrelated change, schema bump, or new
dependency.

### Manual Validation

- [ ] Clean launch: shortcut works without opening Settings.
- [ ] Windows/Linux: `Ctrl+Shift+H` hides/shows pet and panel.
- [ ] macOS: `Cmd+Shift+H` hides/shows pet and panel.
- [ ] Polling continues and saved position is restored.
- [ ] Fullscreen exit does not re-show a user-hidden pet.
- [ ] Custom shortcut replaces the preset.
- [ ] Empty value disables it and Settings explains the state.
- [ ] Default conflict leaves CacheBite visible and permits replacement.
- [ ] Restart preserves custom or disabled v4 state.

## Acceptance Criteria

- [ ] Fresh installs immediately use the platform-appropriate preset.
- [ ] Schema v3 migration receives the preset.
- [ ] Schema v4 custom/disabled values are preserved.
- [ ] Users can override/disable to recover from conflicts.
- [ ] Registration failure never hides/exits or claims activation.
- [ ] All validation commands and coverage gates pass.
- [ ] No type, lint, formatting, or audit errors.
- [ ] UX copy matches the platform mapping.

## Completion Checklist

- [ ] Code follows discovered patterns
- [ ] Error handling and logging match codebase style
- [ ] Tests follow local patterns
- [ ] No duplicated native default string
- [ ] Documentation updated
- [ ] No unnecessary schema, IPC, DTO, or dependency changes
- [ ] Self-contained - no questions needed during implementation

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Default already owned | Medium | Medium | Editable override and fail-visible cleanup |
| Platform strings drift | Low | Medium | One official cross-platform constant |
| v3 migration loses data | Low | High | Retain fields and exact JSON test |
| Explicit v4 `null` re-enabled | Low | Medium | Preserve current-schema `null` and test it |
| Fixture/native drift | Medium | Low | Update fixture and assert UI |

## Notes

- Product decision: make the requested keys a zero-configuration preset, not an unchangeable
  binding. This preserves issue #30's user-configurable criterion while removing setup.
- The branch already contains the dependency, plugin handler, fullscreen coordination,
  settings field, rollback ordering fix, and UI error path. Reuse them.
- No PRD phase was supplied, so no PRD update is required.
