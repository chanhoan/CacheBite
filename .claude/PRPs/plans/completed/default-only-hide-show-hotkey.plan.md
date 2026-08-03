# Plan: Default-Only Hide/Show Shortcut

## Summary

hide/show 단축키의 사용자 지정 경로(Settings의 텍스트 입력, 영구 저장 필드, IPC
side-effect 기계)를 전부 제거하고 고정 상수 `CommandOrControl+Shift+H` 하나만
남긴다. 단축키 상태를 `settings.json`에 저장하지 않으므로 "비활성" 상태가 파일에
기록될 수 없고, 매 실행마다 무조건 등록을 시도한다. 등록 실패는 설정을 덮어쓰는
대신 기존 `PlatformCapabilities` 진단 채널로 보고한다. Settings UI는 편집 가능한
입력 대신 플랫폼별 실제 키와 현재 등록 상태를 읽기 전용으로 보여준다.

## User Story

As a CacheBite user, I want the hide/show shortcut to be a fixed, always-registered
key combination, so that it never silently turns itself off and I never have to
learn Tauri accelerator syntax to get it back.

## Problem → Solution

**현재 상태 (버그 재현 확인됨).**

`%APPDATA%\dev.cachebite.app\settings.json` 실제 내용:

```json
{
  "schema_version": 4,
  "start_at_login": true,
  "hide_show_hotkey": null
}
```

이 때문에 Settings 화면이 정확히 사용자가 보고한 상태를 렌더링한다.

| 화면에 보이는 것 | 실제 출처 |
|---|---|
| `CommandOrControl+Shift+H` (회색) | `SettingsPanel.svelte:109` 의 `placeholder` — 값이 아니라 빈 입력의 안내문구 |
| `Default: Windows/Linux Ctrl+Shift+H, macOS Cmd+Shift+H.` | `SettingsPanel.svelte:119` 고정 안내문 |
| `No shortcut active` | `SettingsPanel.svelte:120-121`, `hideShowHotkey === null` 분기 |

**근본 원인 (코드 경로 확인됨).**

1. `Settings::default()`(`store/settings.rs:53`)는 preset을 주지만, **새 파일과 v1–v3
   마이그레이션에만** 적용된다. 이미 v4인 파일은 `load_locked`(`settings.rs:167-190`)
   의 현재 스키마 분기에서 그대로 통과하므로 `null`이 영구히 남는다.
2. `null`을 쓰는 유일한 코드는 `register_startup_hotkey`(`lib.rs:236-239`)의 실패
   경로다. 등록이 한 번이라도 실패하면 `clear_hotkey()`가 파일에 `null`을 커밋한다.
3. 즉 **일시적 등록 실패가 영구 비활성으로 굳는다.** 스키마가 이미 최신이라 되돌릴
   마이그레이션도 없고, 사용자 지정 입력을 없애면 복구 경로 자체가 사라진다.

**어떤 실패였는지(추론).** 해당 파일은 `start_at_login: true`이고 이 앱에는
single-instance 플러그인이 없다(`Cargo.toml`/`lib.rs`에 `single-instance` 없음).
로그인 자동 실행 인스턴스가 조합을 이미 점유한 상태에서 두 번째 인스턴스(수동 실행
또는 `pnpm tauri dev`)가 뜨면 등록이 실패하고 `clear_hotkey()`가 두 인스턴스 공용
파일을 망가뜨린다. 다른 앱이 `Ctrl+Shift+H`를 점유한 경우도 동일 결과다. 어느
쪽이든 고쳐야 할 결함의 종류는 같으므로 원인 특정에 의존하지 않는 설계로 간다.

**해결.**

영구 저장 필드 `hide_show_hotkey`를 스키마에서 제거(v4 → v5 마이그레이션)하고,
상수 등록을 설정과 완전히 분리한다. 실패는 `PlatformCapabilities.hide_show_hotkey`
진단으로만 노출한다. 저장할 상태가 없으면 손상될 상태도 없다.

## Metadata

- **Complexity**: Large (18 files, 스키마 버전 상승 포함, 새 서브시스템 없음)
- **Source PRD**: N/A
- **PRD Phase**: N/A — GitHub issue #30 후속 정정 + 버그 수정
- **Estimated Files**: 18
- **Branch**: `feat/pet-hide-show-hotkey` (기존 구현 위에 축소 적용)

---

## UX Design

### Before

```text
┌─ Settings ──────────────────────────────────────────┐
│ Start at login                            [x]       │
│ Hide/show shortcut   [ CommandOrControl+Shift+H ]   │ ← 편집 가능, 지금은 빈 값
│ Default: Windows/Linux Ctrl+Shift+H, macOS          │
│ Cmd+Shift+H.                                        │
│ No shortcut active                                  │ ← 영구 고착된 오상태
└─────────────────────────────────────────────────────┘

실패 시: 패널 하단에 "Global shortcut could not be registered …"
         + settings.json 에 null 이 커밋되어 되돌릴 수 없음
```

### After

```text
┌─ Settings ──────────────────────────────────────────┐
│ Start at login                            [x]       │
│ Hide/show shortcut              Ctrl+Shift+H        │ ← 읽기 전용, OS 기준 실제 키
│ Hides and shows the pet. Usage keeps updating       │
│ while hidden.                                       │
└─────────────────────────────────────────────────────┘

충돌 시 (등록 실패):
│ Hide/show shortcut              Ctrl+Shift+H        │
│ Hides and shows the pet. …                          │
│ Another app is using this shortcut. Close it and    │ ← 복구 안내, 저장은 건드리지 않음
│ restart CacheBite.                                  │
```

macOS에서는 같은 자리에 `Cmd+Shift+H`가 표시된다. OS는 `navigator`가 아니라
네이티브가 준 `PlatformCapabilities.os`에서 온다.

### Interaction Changes

| Touchpoint | Before | After | Notes |
|---|---|---|---|
| Settings 단축키 행 | 텍스트 입력 + 저장 | 읽기 전용 `<kbd>` 표시 | `onChange` 미발생 |
| 표시 키 | 두 플랫폼 문자열 나열 | 현재 OS 기준 한 개 | 네이티브 `os` 사용 |
| 빈 값 입력 | 단축키 비활성화 | 불가 — 비활성 개념 자체가 없음 | |
| 등록 실패 | 패널 하단 status + `null` 영구 저장 | Settings 행에 복구 안내, 저장 무변화 | 재시작이 복구 경로 |
| 재시작 | `null`이면 등록 시도 없음 | 항상 등록 시도 | 일시 충돌이 자동 회복됨 |
| `update_settings` | hotkey 등록/해제 side effect | autostart만 | IPC 실패 모드 1개 감소 |

---

## Mandatory Reading

| Priority | File | Lines | Why |
|---|---|---|---|
| P0 | `src-tauri/src/store/settings.rs` | 12-56, 91-102, 147-159, 229-262, 264-293 | 스키마 상수, 기본값, V3 마이그레이션 패턴(그대로 복제할 원본), `clear_hotkey`, `validate` |
| P0 | `src-tauri/src/lib.rs` | 23-31, 54-65, 191-240 | 플러그인 핸들러, setup 순서, `OverlayHideGate`, `register_startup_hotkey` |
| P0 | `src-tauri/src/refresh/ipc.rs` | 110-247, 329-349, 367-405, 621-800 | IpcError, side-effect 기계, capabilities 커맨드, 삭제/재작성할 테스트 |
| P0 | `src-tauri/src/window/mod.rs` | 30-43, 131-173 | `CapabilityDiagnostic`, `PlatformCapabilities`, `capability()` 헬퍼 |
| P1 | `src/lib/api/gateway.ts` | 24-34, 57-65, 103-146 | DTO + wire 변환 계약 |
| P1 | `src/lib/components/SettingsPanel.svelte` | 1-33, 97-128, 176-195 | props 패턴, 단축키 마크업, 스타일 |
| P1 | `src/App.svelte` | 77-90, 130-132, 556-582, 650-694 | fallback settings, 실패 플래그, `changeSettings`, capability status 렌더링 |
| P1 | `src/lib/state/presentation.ts` | 1-28 | `SettingsStoreState` Pick 목록 |
| P2 | `src-tauri/src/store/tests.rs` | 77-88, 190-265 | 마이그레이션/hotkey 테스트 원본 |
| P2 | `src/lib/components/SettingsPanel.test.ts` | 전체 | 컴포넌트 테스트 패턴 |
| P2 | `src/App.test.ts` | 95-130, 654-675, 1005-1020 | fixture settings, hotkey 실패 테스트, capabilities 오버라이드 |
| P2 | `docs/beta-testing.md` | 86-101, 113-120 | 베타 안내 문구 |

## External Documentation

| Topic | Source | Key Takeaway |
|---|---|---|
| Accelerator 문법 | `global-hotkey-0.8.0/src/hotkey.rs:212-216` (로컬 crate 소스에서 확인) | `COMMANDORCONTROL` / `CMDORCTRL` 토큰이 파서에 존재 — `CommandOrControl+Shift+H`는 유효하며, 파싱 실패는 이 버그의 원인이 아니다 |
| 플러그인 버전 | `tauri-plugin-global-shortcut-2.3.2` | `register()`는 이미 점유된 조합에 대해 `Err`를 반환한다. 앱 종료 시 자동 해제되므로 명시적 해제 코드는 불필요 |

- **KEY_INSIGHT**: accelerator 문자열은 정상이다. 버그는 문법이 아니라 **실패를
  영구 저장하는 정책**에 있다.
- **APPLIES_TO**: `lib.rs` 등록 경로, `store/settings.rs` 스키마.
- **GOTCHA**: 등록은 프로세스 단위다. 두 인스턴스가 동시에 뜨면 뒤에 뜬 쪽은 반드시
  실패한다 — 이 경우에도 **앞선 인스턴스의 단축키는 계속 동작해야 한다.** 저장을
  건드리지 않는 것이 그 조건을 만족시키는 방법이다.

---

## Patterns to Mirror

### NAMING_CONVENTION — 상수 위치

```rust
// SOURCE: src-tauri/src/window/mod.rs:131-136
/// Whether the overlay should be shown again when fullscreen ends. `false` when
/// the user explicitly hid it via the hide/show hotkey — fullscreen exiting must
/// not silently reverse that.
pub fn should_restore_overlay_after_fullscreen(user_hidden: bool) -> bool {
    !user_hidden
}
```

`DEFAULT_HIDE_SHOW_HOTKEY`는 더 이상 저장 관심사가 아니므로 `store/settings.rs`에서
`window/mod.rs`의 이 이웃 자리로 옮긴다. `store/`는 영속성만 다룬다.

### SCHEMA_MIGRATION — v4 → v5 (그대로 복제할 원본)

```rust
// SOURCE: src-tauri/src/store/settings.rs:91-102, 229-245
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsV3 {
    schema_version: u32,
    primary_provider: Provider,
    // ... 나머지 필드
    logical_position: LogicalPosition,
}

if let Ok(previous) = serde_json::from_slice::<SettingsV3>(&bytes) {
    if previous.schema_version == 3 {
        let migrated = Settings {
            primary_provider: previous.primary_provider,
            // ... 필드 이전
            ..Settings::default()
        };
        validate(&migrated)?;
        write_json_atomically(&self.path, &migrated)?;
        return self.load_locked();
    }
}
```

**GOTCHA (치명적):** `Settings`에 `#[serde(deny_unknown_fields)]`가 걸려 있다
(`settings.rs:29`). 필드만 지우고 마이그레이션을 추가하지 않으면 기존 v4 파일이
현재 스키마로도, V1/V2/V3로도 파싱되지 않아 `quarantine()`(`settings.rs:259`)로
떨어지고 **사용자의 위치·알림·pet 설정이 전부 초기화된다.** `SettingsV4` 구조체
추가는 선택이 아니라 필수다.

### CAPABILITY_DIAGNOSTIC — 실패 보고 채널

```rust
// SOURCE: src-tauri/src/refresh/ipc.rs:334-348
let fullscreen_detection = if cfg!(windows) {
    CapabilityDiagnostic::Available
} else {
    CapabilityDiagnostic::Unavailable {
        reason: "fullscreen detection is unavailable on this build",
    }
};
Ok(PlatformCapabilities {
    os: crate::window::platform_os(std::env::consts::OS),
    always_on_top: CapabilityDiagnostic::Unavailable { .. },
    fullscreen_detection,
    autostart: CapabilityDiagnostic::Available,
})
```

단축키 등록 결과도 같은 열거형으로 보고한다. CLAUDE.md의 "Unverified platform
capabilities report `unavailable`, never a provider failure" 불변식과 일치한다.

### ERROR_HANDLING / LOGGING — 네이티브

```rust
// SOURCE: src-tauri/src/lib.rs:50-53
if let Err(error) = install_bundled_pet_packages(&app.path().resource_dir()?, &app_data) {
    eprintln!("failed to install bundled pet packages: {error}");
}
```

`eprintln!` 사용, 실패해도 앱은 계속 뜬다. 렌더러 로깅이나 사적 데이터 금지.

### RENDERER_CAPABILITY_PROP — 컴포넌트 경계

```svelte
<!-- SOURCE: src/App.svelte:650-658 -->
<SettingsPanel
  settings={$settingsStore}
  theme={themePreference}
  autostartAvailable={platformCapabilities?.autostart.status !== 'unavailable'}
  pets={petOptions}
  onChange={(settings) => void changeSettings(settings)}
  onThemeChange={changeTheme}
/>
```

```svelte
<!-- SOURCE: src/lib/components/SettingsPanel.svelte:3-10 -->
let {
  settings,
  theme = 'system',
  autostartAvailable = true,
  pets = [],
  onChange = () => {},
  onThemeChange = () => {},
} = $props();
```

새 props도 같은 형태의 flat prop + 기본값으로 추가한다 (객체 prop 아님).

### PURE_PRESENTATION_HELPER

```ts
// SOURCE: src/lib/state/presentation.ts:16-28
export function toSettingsStoreState(settings: AppSettings): SettingsStoreState {
  return {
    primaryProvider: settings.primaryProvider,
    // ...
  };
}
```

OS → 키 라벨 변환은 컴포넌트가 아니라 여기에 순수 함수로 둔다.

### TEST_STRUCTURE — Rust

```rust
// SOURCE: src-tauri/src/store/tests.rs:191-207
#[test]
fn version_three_settings_migrate_with_preset_hotkey() {
    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join("settings.json"), r#"{"schema_version":3,...}"#)
        .expect("write v3 settings");
    let loaded = SettingsRepository::new(dir.path()).load().expect("migrate v3");
    assert_eq!(loaded.schema_version, 4);
}
```

### TEST_STRUCTURE — 렌더러

```ts
// SOURCE: src/lib/components/SettingsPanel.test.ts:99-121
it('describes the preset mapping and inactive state', () => {
  render(SettingsPanel, { props: { settings: { /* ... */ }, pets: [] } });
  expect(
    screen.queryByText('Default: Windows/Linux Ctrl+Shift+H, macOS Cmd+Shift+H.'),
  ).not.toBeNull();
});
```

---

## Files to Change

| File | Action | Justification |
|---|---|---|
| `src-tauri/src/window/mod.rs` | UPDATE | `DEFAULT_HIDE_SHOW_HOTKEY` 이전, `PlatformCapabilities.hide_show_hotkey` 추가, `HideShowHotkeyCapability` 관리 타입 |
| `src-tauri/src/window/tests.rs` | UPDATE | `linux_wayland` 시그니처 변경 반영, 새 capability 필드 커버 |
| `src-tauri/src/store/settings.rs` | UPDATE | 스키마 v5, `hide_show_hotkey` 제거, `SettingsV4` 마이그레이션, `clear_hotkey` 삭제, `validate` 정리 |
| `src-tauri/src/store/tests.rs` | UPDATE | v4→v5 마이그레이션 테스트 추가, hotkey 전용 테스트 삭제 |
| `src-tauri/src/lib.rs` | UPDATE | 무조건 등록 + 진단 관리, `register_startup_hotkey` 재작성 |
| `src-tauri/src/refresh/ipc.rs` | UPDATE | hotkey side effect·`IpcError::HotkeyUnavailable` 제거, capabilities에 진단 추가 |
| `src/lib/api/gateway.ts` | UPDATE | `AppSettings`/`SettingsWire`에서 필드 제거, `PlatformCapabilities`에 진단 추가 |
| `src/lib/api/gateway.test.ts` | UPDATE | wire 계약 테스트 갱신 |
| `src/lib/api/fixtureGateway.ts` | UPDATE | 필드 제거, `schemaVersion` 5, 진단 추가 |
| `src/lib/state/presentation.ts` | UPDATE | Pick에서 제거, `hideShowHotkeyLabel` 추가 |
| `src/lib/state/presentation.test.ts` | UPDATE | 위 변경 반영 + 라벨 함수 테스트 |
| `src/lib/stores/settings.ts` | UPDATE | `SettingsState`에서 필드 제거 |
| `src/lib/stores/settings.test.ts` | UPDATE | 기본값 테스트 갱신 |
| `src/lib/components/SettingsPanel.svelte` | UPDATE | 입력 → 읽기 전용 표시로 교체 |
| `src/lib/components/SettingsPanel.test.ts` | UPDATE | 편집/비우기 테스트 삭제, 표시/충돌 테스트 추가 |
| `src/App.svelte` | UPDATE | fallback 정리, `hotkeySaveFailed` 제거, 새 props 전달 |
| `src/App.test.ts` | UPDATE | hotkey 실패 테스트 삭제, fixture 갱신 |
| `docs/beta-testing.md` | UPDATE | 단축키 안내 재작성 |

## NOT Building

- 사용자 지정 단축키(입력, 키 캡처 레코더, 프리셋 목록) — 이번 변경의 목적이 제거다.
- 단축키 비활성화 스위치. "비활성"이라는 저장 상태를 없애는 것이 버그 수정의 핵심이다.
- 런타임 재등록/재시도 루프. 실패 복구 경로는 재시작이다.
- single-instance 플러그인 도입 (Risks에 기록, 별도 이슈로).
- 마이그레이션 시 기존 사용자 지정 값 보존. v4의 사용자 지정 문자열은 v5에서 버려지고
  기본 조합으로 통일된다 — 요구사항 그대로.
- fullscreen 처리, `OverlayHideGate`, 알림, 폴링, 위치 복원, 패널 표시 정책 변경.
- 새 IPC 커맨드·의존성 추가.

---

## Step-by-Step Tasks

### Task 1: 네이티브 스키마 축소 테스트를 먼저 작성 (RED)

- **ACTION**: `src-tauri/src/store/tests.rs`를 프로덕션 코드보다 먼저 고친다.
- **IMPLEMENT**:
  - 삭제: `settings_reject_a_malformed_hotkey`(222), `settings_round_trip_a_valid_hotkey`(236),
    `clear_hotkey_removes_only_the_hotkey`(250), `version_four_settings_preserve_an_explicitly_disabled_hotkey`(210),
    `fresh_settings_load_from_an_empty_directory_with_the_preset_hotkey`(78).
  - `version_three_settings_migrate_with_preset_hotkey`(192) → 이름을
    `version_three_settings_migrate_to_the_current_schema`로 바꾸고
    `assert_eq!(loaded.schema_version, 5)`만 남긴다.
  - 신규 `version_four_settings_drop_the_persisted_hotkey`: 사용자가 실제로 가진 형태
    (`"hide_show_hotkey":null`, `start_at_login:true`, 0이 아닌 `logical_position`)의
    v4 JSON을 쓰고 `load()` 후 `schema_version == 5`, `logical_position`·
    `secondary_notification_enabled`·`selected_pet_id`가 보존되는지 단언한다.
  - 신규 `version_four_settings_with_a_custom_hotkey_migrate_without_quarantine`:
    `"hide_show_hotkey":"CmdOrCtrl+J"`인 v4 파일도 quarantine 없이 v5로 넘어오고
    디렉터리에 파일이 1개만 남는지(`fs::read_dir(...).count() == 1`) 확인한다.
  - import에서 `DEFAULT_HIDE_SHOW_HOTKEY` 제거.
- **MIRROR**: `TEST_STRUCTURE — Rust`, `legacy_settings_are_migrated_and_rewritten`(143).
- **IMPORTS**: 기존 `TempDir`, `fs`, `SettingsRepository`, `Settings`.
- **GOTCHA**: quarantine 검증은 파일 개수로 한다 — quarantine은 별도 사본을 남기므로
  개수가 1이면 정상 마이그레이션이 증명된다.
- **VALIDATE**: `cargo test --manifest-path src-tauri/Cargo.toml store::tests` 가 컴파일
  실패 또는 assert 실패로 떨어진다.

### Task 2: 스키마 v5로 올리고 `hide_show_hotkey` 제거 (GREEN)

- **ACTION**: `src-tauri/src/store/settings.rs` 수정.
- **IMPLEMENT**:
  - `SETTINGS_SCHEMA_VERSION = 5`.
  - `pub const DEFAULT_HIDE_SHOW_HOTKEY` 줄(13) 삭제 — Task 4에서 `window/mod.rs`로 이전.
  - `Settings`에서 `hide_show_hotkey` 필드(39)와 `Default` 초기화(53) 삭제.
  - `SettingsV4` 구조체 추가 (V3와 동일 필드 + `hide_show_hotkey: Option<String>`),
    `#[serde(deny_unknown_fields)]` 유지.
  - `load_locked`의 V3 분기 **앞에** V4 분기 삽입(버전 내림차순 유지):
    `schema_version == 4`일 때 hotkey를 제외한 전 필드를 옮기고 `..Settings::default()`,
    `validate` → `write_json_atomically` → `return self.load_locked();`.
  - `clear_hotkey()`(147-159) 삭제.
  - `validate()`에서 hotkey 분기(283-291)와 `use std::str::FromStr` 삭제.
- **MIRROR**: `SCHEMA_MIGRATION`.
- **IMPORTS**: 없음(삭제만). `tauri_plugin_global_shortcut`는 `settings.rs`에서 완전히
  빠지며, `Cargo.toml` 의존성은 `lib.rs`가 계속 쓰므로 유지한다.
- **GOTCHA**: V4 분기를 V3보다 뒤에 두면 안 된다. 또한 `SettingsV4`는
  `hide_show_hotkey` 필드를 반드시 선언해야 한다 — `deny_unknown_fields` 때문에
  누락 시 파싱이 실패하고 quarantine으로 떨어진다.
- **VALIDATE**: Task 1 테스트 전부 통과, `cargo fmt --check` 통과.

### Task 3: `apply_hotkey_change` 계열 side effect 제거

- **ACTION**: `src-tauri/src/refresh/ipc.rs` 정리.
- **IMPLEMENT**:
  - `IpcError::HotkeyUnavailable`(119) 삭제. `SettingsRollbackFailed`는 유지.
  - `SettingsEffectError` 열거형(123-128), `apply_settings_side_effects`(173-213),
    `apply_hotkey_change`(215-247) 삭제.
  - `persist_and_apply_settings`를 autostart만 다루도록 축소:

    ```rust
    fn persist_and_apply_settings<SaveSettings, SetAutostart>(
        previous: &Settings,
        settings: &Settings,
        mut save_settings: SaveSettings,
        mut set_autostart: SetAutostart,
    ) -> Result<(), IpcError>
    where
        SaveSettings: FnMut(&Settings) -> io::Result<()>,
        SetAutostart: FnMut(bool) -> Result<(), ()>,
    {
        save_settings(settings).map_err(|error| {
            if error.kind() == io::ErrorKind::InvalidData {
                IpcError::InvalidSettings
            } else {
                IpcError::PersistenceUnavailable
            }
        })?;
        if previous.start_at_login != settings.start_at_login
            && set_autostart(settings.start_at_login).is_err()
        {
            if save_settings(previous).is_err() {
                eprintln!("failed to restore persisted settings after settings update failure");
                return Err(IpcError::SettingsRollbackFailed);
            }
            return Err(IpcError::ServiceUnavailable);
        }
        Ok(())
    }
    ```
  - `update_settings`(367-405)에서 `use tauri_plugin_global_shortcut::GlobalShortcutExt;`,
    `let shortcuts = ...`, 두 hotkey 클로저(395-396) 제거.
  - `settings_effect_tests` 모듈: `settings()` 헬퍼에서 hotkey 인자 제거하고
    hotkey 전용 테스트 5개(640, 661, 686, 707, 728) 삭제.
    `autostart_compensation_failure_is_reported_as_rollback_failure`(739)는 시나리오가
    사라지므로 삭제하고, 대신 `failed_side_effect_restores_previous_persisted_settings`(765)를
    autostart 실패 기준으로 재작성해 `Err(IpcError::ServiceUnavailable)` +
    `saved == [next, previous]`를 단언한다.
    `persistence_compensation_failure_is_reported_as_rollback_failure`(787)도 autostart
    실패 기준으로 유지한다.
- **MIRROR**: `ERROR_HANDLING`, 기존 `RefCell` 기반 클로저 테스트 스타일.
- **IMPORTS**: 테스트 모듈 `use super::{...}`에서 `apply_settings_side_effects`,
  `SettingsEffectError` 제거.
- **GOTCHA**: 저장 성공 후 side effect 실패 시 **저장을 되돌린다**는 기존 계약을
  유지해야 한다. autostart 하나만 남았다고 rollback을 지우면 회귀다.
- **VALIDATE**: `cargo test --manifest-path src-tauri/Cargo.toml refresh::ipc` 통과,
  `cargo clippy --all-features -- -D warnings` 무경고.

### Task 4: 상수 이전 + capability 진단 채널 추가

- **ACTION**: `src-tauri/src/window/mod.rs`에 단축키 정책을 모은다.
- **IMPLEMENT**:
  - `should_restore_overlay_after_fullscreen` 바로 위에 상수와 doc comment 추가:

    ```rust
    /// The one hide/show combination CacheBite claims. `CommandOrControl` is
    /// Tauri's cross-platform token: Cmd on macOS, Ctrl on Windows and Linux.
    /// It is not user-configurable — a fixed binding cannot record itself as
    /// disabled, which is how a single failed registration used to become
    /// permanent.
    pub const DEFAULT_HIDE_SHOW_HOTKEY: &str = "CommandOrControl+Shift+H";
    ```
  - `PlatformCapabilities`(37-43)에 `pub hide_show_hotkey: CapabilityDiagnostic` 추가.
  - `linux_wayland`(145-155)에 세 번째 파라미터 `hide_show_hotkey: bool`를 추가하고
    기존 `capability()` 헬퍼로 변환. reason 문자열:
    `"compositor does not permit a global shortcut"`.
  - 관리 상태 타입 추가:

    ```rust
    /// The startup registration result for [`DEFAULT_HIDE_SHOW_HOTKEY`], managed
    /// so `get_platform_capabilities` can report a conflict without the settings
    /// file ever recording one.
    pub struct HideShowHotkeyCapability(pub CapabilityDiagnostic);
    ```
- **MIRROR**: `CAPABILITY_DIAGNOSTIC`, `capability()`(167-173).
- **IMPORTS**: 없음.
- **GOTCHA**: `CapabilityDiagnostic::Unavailable`의 `reason`은 `&'static str`이다 —
  런타임 문자열 포맷 금지.
- **VALIDATE**: `cargo check` 통과(호출부는 Task 5·6에서 수정).

### Task 5: 시작 시 무조건 등록하고 결과를 진단으로 보고

- **ACTION**: `src-tauri/src/lib.rs`의 setup 경로 수정.
- **IMPLEMENT**:
  - `register_startup_hotkey`(229-240)를 다음으로 교체:

    ```rust
    /// Claims the fixed hide/show shortcut, reporting the outcome instead of
    /// persisting it.
    ///
    /// A conflict — another app, or a second CacheBite instance started while
    /// the login-launched one already holds the combination — used to clear the
    /// saved hotkey, turning one transient failure into a permanently disabled
    /// shortcut with no way back. Nothing is written now: the next launch tries
    /// again.
    fn register_default_hotkey(app: &tauri::AppHandle) -> window::CapabilityDiagnostic {
        use tauri_plugin_global_shortcut::GlobalShortcutExt;

        if app
            .global_shortcut()
            .register(window::DEFAULT_HIDE_SHOW_HOTKEY)
            .is_err()
        {
            eprintln!("failed to register the hide/show shortcut; another application may own it");
            return window::CapabilityDiagnostic::Unavailable {
                reason: "another application already owns this shortcut",
            };
        }
        window::CapabilityDiagnostic::Available
    }
    ```
  - setup(59-65)을 다음 순서로 바꾼다:

    ```rust
    app.manage(OverlayHideGate::default());
    app.manage(window::HideShowHotkeyCapability(register_default_hotkey(
        app.handle(),
    )));
    if let Ok(settings) = settings_repository.load() {
        restore_window_positions(app, &settings);
    }
    ```
- **MIRROR**: `LOGGING_PATTERN`, 기존 `app.manage` 순서 주석(55-58).
- **IMPORTS**: 없음.
- **GOTCHA**: 등록은 반드시 `OverlayHideGate` 관리 **이후**여야 한다 — 플러그인은
  빌더 체인에서 이미 살아 있어서 등록 직후 눌린 키가 `app.state::<OverlayHideGate>()`
  에서 패닉할 수 있다(기존 주석이 지적한 그대로). 그리고 등록은 이제 설정 로드
  성공 여부와 무관해야 하므로 `if let Ok(settings)` 블록 **밖**에 둔다.
- **VALIDATE**: `cargo check --all-features`, `cargo clippy -- -D warnings` 통과.

### Task 6: `get_platform_capabilities`에 진단 노출

- **ACTION**: `src-tauri/src/refresh/ipc.rs:329-349` 수정.
- **IMPLEMENT**: 시그니처에 `hotkey: State<'_, HideShowHotkeyCapability>` 추가,
  응답에 `hide_show_hotkey: hotkey.0.clone()` 추가.
- **MIRROR**: `CAPABILITY_DIAGNOSTIC`, 기존 `State<'_, CollectorModeDto>` 주입(102-108).
- **IMPORTS**: 파일 상단 `crate::window::{...}` import 블록에 `HideShowHotkeyCapability` 추가.
- **GOTCHA**: `State` 추출은 미관리 시 패닉한다. Task 5의 `app.manage`가 무조건
  실행되는지 확인할 것 — 조건 블록 안에 들어가면 안 된다.
- **VALIDATE**: `cargo test --manifest-path src-tauri/Cargo.toml --all-features` 통과.

### Task 7: `window/tests.rs` 갱신

- **ACTION**: 새 필드·시그니처 반영.
- **IMPLEMENT**: `PlatformCapabilities::linux_wayland(false, false)`(304) →
  `(false, false, false)`. `hide_show_hotkey`가 `Unavailable`인지 단언 추가.
  `Available` 케이스도 한 줄 추가(`linux_wayland(true, true, true)`).
- **MIRROR**: 기존 `assert!(matches!(..., CapabilityDiagnostic::Unavailable { .. }))` 스타일(300-312).
- **IMPORTS**: 없음.
- **GOTCHA**: 없음.
- **VALIDATE**: `cargo test --manifest-path src-tauri/Cargo.toml window::tests` 통과.

### Task 8: 렌더러 계약에서 필드 제거, 진단 추가

- **ACTION**: `src/lib/api/gateway.ts` 및 그 테스트/픽스처 수정.
- **IMPLEMENT**:
  - `AppSettings`(24-34)에서 `hideShowHotkey` 제거.
  - `SettingsWire`(103-113)에서 `hide_show_hotkey` 제거.
  - `fromSettings`(134), `toSettings`(145)의 해당 줄 제거.
  - `PlatformCapabilities`(60-65)에 `readonly hide_show_hotkey: CapabilityDiagnostic;` 추가.
  - `gateway.test.ts`: wire 픽스처(39)에서 `hide_show_hotkey` 제거,
    `updateSettings` 왕복 테스트(82-88)에서 hotkey 필드 제거,
    capabilities 테스트(116-117)에 `hide_show_hotkey: { status: 'available' }` 추가.
  - `fixtureGateway.ts`: `hideShowHotkey`(51) 제거, `schemaVersion: 4` → `5`,
    `getPlatformCapabilities`(93-98)에 `hide_show_hotkey: { status: 'available' }` 추가.
- **MIRROR**: 기존 snake_case ↔ camelCase 변환 규약. capability 필드는 네이티브
  serde 이름 그대로 snake_case를 쓴다(`always_on_top` 선례).
- **IMPORTS**: 없음.
- **GOTCHA**: `AppSettings`는 wire와 1:1이다. 한쪽만 지우면 `svelte-check`가
  잡아준다 — 반드시 양쪽 모두.
- **VALIDATE**: `pnpm check`, `pnpm vitest run src/lib/api/gateway.test.ts` 통과.

### Task 9: presentation/store 레이어 정리 + 라벨 헬퍼

- **ACTION**: `src/lib/state/presentation.ts`, `src/lib/stores/settings.ts` 수정.
- **IMPLEMENT**:
  - `SettingsStoreState`의 Pick 목록(5-14)에서 `'hideShowHotkey'` 제거,
    `toSettingsStoreState`(26)에서 해당 줄 제거.
  - 라벨 헬퍼 추가:

    ```ts
    /** The hide/show shortcut as the running platform actually spells it. The
     *  binding is fixed; only its rendering differs. */
    export function hideShowHotkeyLabel(
      os: PlatformCapabilities['os'],
    ): string {
      return os === 'macos' ? 'Cmd+Shift+H' : 'Ctrl+Shift+H';
    }
    ```
    (`import type { AppSettings, PlatformCapabilities } from '../api/gateway';`)
  - `stores/settings.ts`: `SettingsState`(11)와 `defaultSettings`(21)에서 필드 제거.
  - `presentation.test.ts`(17, 42), `stores/settings.test.ts`(24) 갱신 +
    `hideShowHotkeyLabel`의 macos/windows/linux 세 케이스 테스트 추가.
- **MIRROR**: `PURE_PRESENTATION_HELPER`.
- **IMPORTS**: 위 참조.
- **GOTCHA**: `navigator.platform`을 쓰지 말 것. OS는 네이티브 `PlatformCapabilities`
  에서만 온다(privacy/contract 일관성).
- **VALIDATE**: `pnpm vitest run src/lib/state/presentation.test.ts src/lib/stores/settings.test.ts` 통과.

### Task 10: SettingsPanel을 읽기 전용 표시로 교체

- **ACTION**: `src/lib/components/SettingsPanel.svelte` 수정.
- **IMPLEMENT**:
  - props에 추가: `hideShowHotkeyLabel = 'Ctrl+Shift+H'`,
    `hideShowHotkeyAvailable = true`. JSDoc 타입 주석(2행)도 함께 확장.
  - `hideShowHotkeyHelpId` 옆에 `hideShowHotkeyLabelId` 상수 추가.
  - 106-127행 블록을 다음으로 교체:

    ```svelte
    <div class="field">
      <span id={hideShowHotkeyLabelId}>Hide/show shortcut</span>
      <kbd class="shortcut" aria-labelledby={hideShowHotkeyLabelId}
        >{hideShowHotkeyLabel}</kbd
      >
    </div>
    <p id={hideShowHotkeyHelpId} class="field-help">
      Hides and shows the pet. Usage keeps updating while hidden.
      {#if !hideShowHotkeyAvailable}
        <span class="field-state"
          >Another app is using this shortcut. Close it and restart CacheBite.</span
        >
      {/if}
    </p>
    ```
  - 스타일: `.field input[type='text']`(176-184) 블록을 `.shortcut` 규칙으로 교체 —
    `font: inherit; padding: 0.15rem 0.4rem; border: 1px solid var(--color-border);
    border-radius: 0.3rem; background: var(--color-surface); color: var(--color-text);`.
    `.field-help`, `.field-state`는 그대로 둔다.
- **MIRROR**: `RENDERER_CAPABILITY_PROP`, 기존 `.field` 레이아웃(160-167).
- **IMPORTS**: 없음.
- **GOTCHA**: `<label>`은 form control과 짝지어야 의미가 있다. 입력이 사라졌으므로
  `<label>` 대신 `<div class="field">` + `<span id>` + `aria-labelledby`를 쓴다.
  접근 가능 이름이 유지되는지 테스트로 고정할 것.
- **VALIDATE**: `pnpm vitest run src/lib/components/SettingsPanel.test.ts` 통과.

### Task 11: SettingsPanel 테스트 재작성

- **ACTION**: `src/lib/components/SettingsPanel.test.ts` 수정.
- **IMPLEMENT**:
  - 모든 `settings` 픽스처에서 `hideShowHotkey` 제거.
  - `emits immutable setting changes`(7)에서 hotkey `fireEvent.change`(44-46)와
    단언(66-68) 삭제. 나머지 필드 단언은 유지.
  - `clears the hotkey when the input is emptied`(72) 삭제.
  - `describes the preset mapping and inactive state`(99) →
    `shows the fixed shortcut for the running platform`으로 재작성:
    `hideShowHotkeyLabel: 'Cmd+Shift+H'` prop을 주고 화면에 그 텍스트가 있는지,
    그리고 텍스트 입력이 **없는지**(`screen.queryByRole('textbox')` 가 null) 단언.
  - 신규 `explains how to recover when the shortcut is taken`:
    `hideShowHotkeyAvailable: false`일 때 안내 문구가 보이고, `true`일 때는
    보이지 않는지 단언.
- **MIRROR**: `TEST_STRUCTURE — 렌더러`.
- **IMPORTS**: 기존과 동일.
- **GOTCHA**: `queryByRole('textbox')` 단언은 단축키 입력이 되살아나는 회귀를 막는
  가드다 — 반드시 포함할 것.
- **VALIDATE**: 위 파일 focused 실행 통과.

### Task 12: App.svelte 배선 정리

- **ACTION**: `src/App.svelte` 수정.
- **IMPLEMENT**:
  - fallback `appSettings`(77-90): `hideShowHotkey: null` 삭제,
    `schemaVersion: 3` → `5`로 교정(주석이 "Must match the Rust default"라고 요구하는데
    이미 어긋나 있다).
  - `hotkeySaveFailed` 상태(131) 및 대입(562, 576-577) 삭제.
    `catch` 블록은 `settingsSaveFailed = true;`로 단순화.
  - status 블록(678-682)을 `{#if settingsSaveFailed}<p role="status">Settings could
    not be saved</p>{/if}`로 축소.
  - `<SettingsPanel>`(650-658)에 두 props 추가:

    ```svelte
    hideShowHotkeyLabel={hideShowHotkeyLabel(platformCapabilities?.os ?? 'linux')}
    hideShowHotkeyAvailable={platformCapabilities?.hide_show_hotkey.status !== 'unavailable'}
    ```
  - `hideShowHotkeyLabel`을 `./lib/state/presentation`에서 import.
- **MIRROR**: 바로 위 `autostartAvailable` 라인,
  `data-platform={platformCapabilities?.os ?? 'linux'}`(635)의 fallback 규약.
- **IMPORTS**: 기존 presentation import에 `hideShowHotkeyLabel` 추가.
- **GOTCHA**: `platformCapabilities`는 로드 전 `null`이다. `!== 'unavailable'`
  비교는 그 경우 `true`가 되어 낙관적으로 표시된다 — autostart와 동일한 기존 규약이니
  일부러 맞춘다.
- **VALIDATE**: `pnpm check`, `pnpm vitest run src/App.test.ts` 통과.

### Task 13: App.test.ts 갱신

- **ACTION**: `src/App.test.ts` 수정.
- **IMPLEMENT**:
  - settings 픽스처(103)에서 `hideShowHotkey` 제거, `schemaVersion`이 있으면 5로.
  - capabilities 픽스처(125-127, 1011-1015)에
    `hide_show_hotkey: { status: 'available' as const }` 추가.
  - `shows a distinct message when the hotkey fails to register`(654-675) 삭제 —
    `hotkey_unavailable` IPC 오류가 더 이상 존재하지 않는다.
  - 신규 테스트: `getPlatformCapabilities`가 `hide_show_hotkey: { status:
    'unavailable', reason: ... }`을 돌려줄 때 Settings 화면에 복구 안내가 뜨는지 확인.
- **MIRROR**: 기존 capabilities 오버라이드 테스트(1005-1020) 구조.
- **IMPORTS**: 없음.
- **GOTCHA**: `settingsSaveFailed` 경로 테스트가 따로 있다면 유지한다. 삭제 대상은
  hotkey 전용 분기뿐이다.
- **VALIDATE**: `pnpm vitest run src/App.test.ts` 통과.

### Task 14: 베타 문서 갱신

- **ACTION**: `docs/beta-testing.md` 수정.
- **IMPLEMENT**:
  - 86-96행: Settings 항목 나열에서 "hide/show shortcut"을 빼고, 단축키는 고정이며
    설정 대상이 아니라고 명시. "Use **Settings** only if you want to replace the
    preset or disable the shortcut entirely."(95-96) 삭제.
  - 새 문구: `Ctrl+Shift+H`(Windows/Linux) / `Cmd+Shift+H`(macOS)가 항상 활성이며,
    숨긴 동안에도 폴링은 계속되고, Settings에는 현재 키가 읽기 전용으로 표시된다.
  - 117-119행 충돌 항목 재작성: 다른 앱이 조합을 점유한 경우 CacheBite는 계속 보이고,
    Settings가 복구 방법(충돌 앱 종료 후 재시작)을 안내하며, 설정 파일은 변경되지
    않는다는 점을 확인하도록.
- **MIRROR**: 기존 불릿 톤과 굵은 강조 스타일.
- **IMPORTS**: N/A.
- **GOTCHA**: 사용자 문서에 내부 토큰 `CommandOrControl`을 노출하지 말 것.
- **VALIDATE**: 렌더링된 Markdown 확인 + Manual Validation 체크리스트 수행.

### Task 15: 전체 게이트 실행 및 diff 감사

- **ACTION**: 아래 Validation Commands 전부 실행하고 변경 범위를 점검한다.
- **IMPLEMENT**: 정적 분석 → 단위 → 전체 → 빌드 → audit → diff 순서.
- **MIRROR**: 저장소 스크립트(`package.json`, `Cargo.toml`).
- **IMPORTS**: N/A.
- **GOTCHA**: 실제 기기 검증 시 기존 `settings.json`을 **백업 후** 마이그레이션이
  값을 보존하는지 확인할 것. `%APPDATA%\dev.cachebite.app\settings.json`.
- **VALIDATE**: 모든 명령 통과, 커버리지 80% 유지, 계획에 없는 파일 변경 없음.

---

## Testing Strategy

### Unit Tests

| Test | Input | Expected Output | Edge Case? |
|---|---|---|---|
| v4 → v5 마이그레이션 | `hide_show_hotkey: null`인 실제 v4 JSON | `schema_version == 5`, 위치·알림·pet 보존 | Yes |
| v4 사용자 지정 값 | `"hide_show_hotkey":"CmdOrCtrl+J"` | quarantine 없이 v5, 파일 1개 | Yes |
| v3 → v5 마이그레이션 | 기존 v3 JSON | `schema_version == 5` | Yes |
| 신규 설치 | 빈 디렉터리 | `Settings::default()`에 hotkey 필드 없음 | No |
| 잘못된 설정 | 깨진 JSON | 기존 quarantine 동작 유지 | Yes |
| autostart 실패 | `set_autostart` Err | `ServiceUnavailable` + 이전 설정 복원 | Yes |
| autostart 실패 + 복원 실패 | 두 save 모두 Err | `SettingsRollbackFailed` | Yes |
| capabilities 보고 | 등록 성공/실패 | `Available` / `Unavailable{reason}` | Yes |
| `linux_wayland` | `(false,false,false)` | 세 진단 모두 `Unavailable` | No |
| 라벨 헬퍼 | `'macos'` / `'windows'` / `'linux'` | `Cmd+Shift+H` / `Ctrl+Shift+H` / `Ctrl+Shift+H` | No |
| 패널 표시 | `hideShowHotkeyLabel='Cmd+Shift+H'` | 해당 텍스트 표시, textbox 없음 | No |
| 패널 충돌 안내 | `hideShowHotkeyAvailable=false` | 복구 문구 표시 | Yes |
| wire 왕복 | settings DTO | `hide_show_hotkey` 키가 양방향 모두 없음 | No |

### Edge Cases Checklist

- [ ] 기존 v4 파일이 quarantine 없이 마이그레이션되고 위치/알림/pet 설정이 보존된다
- [ ] v4에 사용자 지정 문자열이 있어도 손실은 hotkey 하나뿐이다
- [ ] 설정 로드가 실패해도 단축키 등록은 시도된다 (등록이 `if let Ok(settings)` 밖)
- [ ] 등록 실패 후에도 앱은 정상적으로 뜨고 오버레이가 보인다
- [ ] 등록 실패가 `settings.json`을 수정하지 않는다
- [ ] 두 번째 인스턴스가 실패해도 첫 인스턴스의 단축키가 계속 동작한다
- [ ] 재시작 시 등록이 다시 시도되어 일시 충돌이 자동 회복된다
- [ ] fullscreen 종료가 사용자 hide 상태를 되살리지 않는다 (기존 불변식)
- [ ] 단축키로 숨긴 동안에도 폴링/알림이 계속된다
- [ ] 저장된 per-display 위치 복원이 그대로다
- [ ] Settings 화면에 텍스트 입력이 없다 (회귀 가드)
- [ ] `platformCapabilities`가 아직 `null`일 때 UI가 깨지지 않는다

---

## Validation Commands

### Static Analysis

```bash
pnpm check
pnpm lint
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-features -- -D warnings
```

EXPECT: 타입/린트/포맷 오류 0, Rust 경고 0.

### Unit Tests

```bash
cargo test --manifest-path src-tauri/Cargo.toml store::tests --all-features
cargo test --manifest-path src-tauri/Cargo.toml window::tests --all-features
cargo test --manifest-path src-tauri/Cargo.toml refresh::ipc --all-features
pnpm vitest run src/lib/components/SettingsPanel.test.ts src/lib/api/gateway.test.ts src/lib/state/presentation.test.ts src/lib/stores/settings.test.ts
```

EXPECT: focused 테스트 전부 통과.

### Full Test Suite

```bash
pnpm test:ci
cargo test --manifest-path src-tauri/Cargo.toml --all-features
```

EXPECT: 회귀 없음, 커버리지 branches/functions/lines/statements 80% 이상 유지.

### E2E

```bash
pnpm test:e2e:renderer
```

EXPECT: 통과. (`tests/e2e/renderer.spec.ts`는 단축키를 참조하지 않지만
`fixtureGateway` 변경이 영향을 줄 수 있다.)

### Build

```bash
pnpm build
cargo check --manifest-path src-tauri/Cargo.toml --all-features
```

EXPECT: 렌더러/네이티브 빌드 성공.

### Dependency and Diff Checks

```bash
cargo audit --file src-tauri/Cargo.lock
git diff --check
git diff --stat
```

EXPECT: high/critical 권고 없음, 공백 오류 없음, 계획된 18개 파일 외 변경 없음,
새 의존성 없음.

### Manual Validation

- [ ] 기존 `settings.json`(현재 `hide_show_hotkey: null` 상태)로 실행 → `Ctrl+Shift+H`가
      **즉시 동작**하고 위치/알림/pet 설정이 보존된다
- [ ] 파일이 v5로 재작성되고 `hide_show_hotkey` 키가 사라졌다
- [ ] Settings 화면에 `Ctrl+Shift+H`가 읽기 전용으로 표시되고 입력창이 없다
- [ ] "No shortcut active" 문구가 어디에도 나타나지 않는다
- [ ] 단축키로 숨기면 pet과 패널이 함께 사라지고, 다시 누르면 돌아온다
- [ ] 숨긴 동안 폴링이 계속되어 다시 표시했을 때 사용량이 갱신되어 있다
- [ ] 다른 앱이 `Ctrl+Shift+H`를 점유한 상태로 실행 → 앱은 정상 표시되고 Settings에
      복구 안내가 뜨며 `settings.json`은 변하지 않는다
- [ ] 충돌 앱 종료 후 CacheBite 재시작 → 단축키가 다시 활성화된다
- [ ] 두 인스턴스 동시 실행 시 첫 인스턴스의 단축키가 계속 동작한다
- [ ] `start_at_login` 토글이 이전과 동일하게 동작한다 (side-effect 축소 회귀 확인)
- [ ] macOS에서 `Cmd+Shift+H` 표시 및 동작 (macOS 릴리스 선언 전 필수)

---

## Acceptance Criteria

- [ ] `settings.json`에 hide/show 단축키 상태가 저장되지 않는다
- [ ] 기존 v4 파일이 데이터 손실·quarantine 없이 v5로 마이그레이션된다
- [ ] 앱 실행마다 고정 조합 등록을 시도한다 (설정 로드 성공 여부와 무관)
- [ ] 등록 실패가 설정 파일을 수정하지 않으며 앱을 숨기거나 종료시키지 않는다
- [ ] 등록 실패가 `PlatformCapabilities.hide_show_hotkey` 진단으로 보고된다
- [ ] Settings에 단축키 텍스트 입력이 존재하지 않는다
- [ ] Settings가 현재 OS 기준 실제 키를 표시한다
- [ ] `IpcError::HotkeyUnavailable` 및 관련 side-effect 코드가 제거되었다
- [ ] 모든 검증 명령과 80% 커버리지 게이트를 통과한다

## Completion Checklist

- [ ] 발견된 패턴을 따랐다 (마이그레이션, capability 진단, flat props, 순수 헬퍼)
- [ ] 에러 처리·로깅이 기존 스타일과 일치한다 (`eprintln!`, 사적 데이터 없음)
- [ ] 테스트가 기존 구조(AAA, `TempDir`, `RefCell` 클로저, testing-library)를 따른다
- [ ] 하드코딩된 키 문자열이 `DEFAULT_HIDE_SHOW_HOTKEY` 한 곳에만 존재한다
- [ ] 문서(`docs/beta-testing.md`)가 갱신되었다
- [ ] 불필요한 스키마/IPC/DTO/의존성 변경이 없다
- [ ] 계획 외 파일이 변경되지 않았다
- [ ] 자기완결적 — 구현 중 추가 질문이 필요 없다

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `deny_unknown_fields` 때문에 기존 v4 파일이 quarantine되어 사용자 설정 초기화 | High(마이그레이션 누락 시) | High | `SettingsV4` 구조체 + v4 분기를 Task 2에서 필수로 구현, 실제 파일 형태로 테스트, 수동 검증 항목 포함 |
| 고정 조합을 다른 앱이 이미 점유 | Medium | Medium | 저장 미변경 + capability 진단 + 재시작 복구 안내. 재시작마다 재시도 |
| 두 인스턴스 동시 실행(로그인 자동 실행 + 수동) | Medium | Low | 저장을 건드리지 않으므로 첫 인스턴스는 무사. single-instance 도입은 별도 이슈 |
| 사용자 지정 단축키를 쓰던 베타 사용자가 값을 잃음 | Low | Low | 요구된 제품 결정. 베타 문서에 명시 |
| `IpcError` variant 제거로 렌더러 처리 누락 | Low | Medium | `svelte-check` + App.test.ts에서 hotkey 분기 제거를 함께 수행 |
| capability 상태 미관리로 `State` 추출 패닉 | Low | High | `app.manage`를 조건 블록 밖 무조건 경로에 배치(Task 5 GOTCHA), 네이티브 스모크로 확인 |
| 커버리지 80% 하회 (코드 삭제로 분모 변화) | Low | Medium | 새 마이그레이션·진단·라벨 테스트로 상쇄, `pnpm test:ci`로 확인 |

## Notes

- 이 계획은 `.claude/PRPs/plans/preset-pet-hide-show-hotkey.plan.md`(커밋 `1aa030a`)를
  **대체한다.** 그 계획은 "preset + 사용자 지정 유지"였고, 이번 결정은 "고정 전용"이다.
- 보고된 UI 증상 3줄은 모두 단일 원인(`hide_show_hotkey: null` 영구 고착)에서 나온다.
  별도 UI 버그가 아니므로 별도 수정 항목을 만들지 않았다.
- accelerator 파싱은 정상임을 로컬 crate 소스(`global-hotkey-0.8.0/src/hotkey.rs:212`)
  에서 확인했다. 문법 변경은 불필요하다.
- `schemaVersion: 3` fallback(`App.svelte:78`)은 이미 어긋난 값이며 Task 12에서 함께
  교정한다. 이 값은 `getSettings()` 실패 시에만 쓰이고 어떤 분기에도 영향을 주지
  않지만, 주석이 요구하는 계약을 지키기 위해 5로 맞춘다.
- single-instance 플러그인 부재는 이 버그의 유력한 촉발 요인이지만, 도입은 시작 흐름
  전반(창 포커스 전달, 인자 처리)에 영향을 주므로 범위 밖으로 둔다. 별도 이슈 권장.
