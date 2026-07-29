# Plan: User-Selectable Pet (provider 종속 해제 + `cat` → `tabby` 개명)

## Summary

펫 선택이 primary provider에 묶여 있어 Claude=cat / Codex=corgi로 강제된다. 이 결합을 끊고 Settings에서 펫을 독립적으로 고르게 한다. 목록은 설치된 패키지를 네이티브에서 열거해 채우므로, 이후 버전에서 번들 펫이 늘어도 렌더러 코드를 고칠 필요가 없다. 함께 `cat` 패키지를 `tabby`로 개명하고(치즈태비 레퍼런스, corgi와 같은 품종/유형 층위) 기존 사용자 마이그레이션을 포함한다.

## User Story

As a CacheBite 사용자,
I want primary provider와 무관하게 펫을 직접 고르고,
So that Claude를 주력으로 쓰면서도 corgi를 띄울 수 있다.

## Problem → Solution

`App.svelte:503-510`이 primary provider 변경 시 `selectedPetId`를 `PROVIDER_PET[provider]`로 덮어써서 사용자가 고른 펫이 즉시 되돌려진다 → 결합 제거 + Settings에 펫 드롭다운 추가 + 설치 패키지 동적 열거.

## Metadata

- **Complexity**: Medium
- **Source PRD**: N/A (free-form 요청)
- **PRD Phase**: N/A
- **Estimated Files**: 20 (Rust 6, Python 1, 리소스 2, 렌더러 6, 테스트 5)

---

## UX Design

### Before

```
Settings
────────────────────────────────
Appearance          [ System ▾ ]
Native notifications        [ ]
Secondary provider notif.   [ ]
Primary provider    [ Claude ▾ ]  ← 이걸 바꾸면
Speech bubbles              [x]      펫이 강제로 cat/corgi로 바뀜
Start at login              [ ]

펫 선택 UI 없음. Claude=cat, Codex=corgi 고정.
```

### After

```
Settings
────────────────────────────────
Appearance          [ System ▾ ]
Native notifications        [ ]
Secondary provider notif.   [ ]
Primary provider    [ Claude ▾ ]  ← 게이지 데이터 소스만 바뀜
Pet                 [ Tabby  ▾ ]  ← 신규. 독립 선택
                       Corgi
                       Tabby
Speech bubbles              [x]
Start at login              [ ]
```

### Interaction Changes

| Touchpoint | Before | After | Notes |
|---|---|---|---|
| Settings > Primary provider 변경 | 펫이 provider 기본 펫으로 강제 교체 | 펫 그대로 유지, 게이지 데이터 소스만 변경 | `PROVIDER_PET` 오버라이드 제거 |
| UsagePanel > "Set as primary" 버튼 | 동일하게 펫 강제 교체 | 동일하게 펫 유지 | 같은 `changeSettings` 경로를 탐 |
| Settings > Pet | 없음 | 설치된 패키지 드롭다운 | 신규 |
| 펫 변경 시 오버레이 | N/A | 즉시 새 펫으로 교체 | 기존 `settings-updated` → `listenSettings` → `loadPetPackage()` 경로 재사용 (`App.svelte:293-297`). **추가 배선 불필요** |
| 기존 사용자 첫 실행 (업데이트 후) | `cat` 펫 | `Tabby` 펫 (같은 아트) | settings 값 마이그레이션 + 구 패키지 제거 |

**Edge case (UX):** 펫 목록 로드가 실패하면 드롭다운을 현재 선택값 하나만 가진 상태로 렌더한다. 빈 `<select>`로 두면 사용자가 현재 펫이 뭔지도 알 수 없다.

---

## Mandatory Reading

| Priority | File | Lines | Why |
|---|---|---|---|
| P0 | `src/App.svelte` | 69-72, 80-92, 260-277, 293-297, 497-531, 595-612 | 결합 지점 전부. `PROVIDER_PET`, 기본 settings, `loadPetPackage`, settings 리스너, `changeSettings`, SettingsPanel 마운트 |
| P0 | `src-tauri/src/store/pets.rs` | 전체 (177) | `PetPackageRepository` — `list()`를 여기 추가. `should_preserve_installed`의 하드코딩 제거 대상 |
| P0 | `src-tauri/src/store/settings.rs` | 40-53, 131-152, 200-220 | 기본값, `"idle"` 값 마이그레이션 패턴(이걸 그대로 미러링), `validate` |
| P0 | `src-tauri/src/refresh/ipc.rs` | 106-121, 171-183 | `IpcError`, `authorize` 헬퍼, `get_pet_package` — 새 커맨드가 따를 형태 |
| P0 | `src-tauri/src/window/mod.rs` | 57-102 | `NativeCommand` enum + `command_allowed` 화이트리스트 |
| P0 | `src-tauri/src/lib.rs` | 203-229 | `install_bundled_pet_packages` — 하드코딩 `["cat","corgi"]` 제거 + 은퇴 패키지 정리 |
| P1 | `src/lib/api/gateway.ts` | 24-33, 47-50, 65-93, 95-135, 149-172 | `AppSettings`, `PetPackageModel`, `AppGateway` 인터페이스, wire 변환 함수 |
| P1 | `src/lib/state/presentation.ts` | 5-24 | `SettingsStoreState` (Pick 기반) |
| P1 | `src/lib/stores/settings.ts` | 4-17 | `SettingsState` — presentation.ts와 **별도 선언**. 둘 다 고쳐야 함 |
| P1 | `src/lib/components/SettingsPanel.svelte` | 1-19, 53-64, 113-129 | props 형태, `asProvider` 내로잉 패턴, `.field` select 스타일 |
| P1 | `scripts/build-pet-packages.py` | 22-37, 197-247 | `PET_SOURCES` 매핑, `build_package`가 프레임 파일명과 displayName을 생성하는 방식 |
| P2 | `src/App.test.ts` | 40-107, 461-486 | gateway 목 fixture (새 메서드 추가 필요), 펫 리로드 테스트 |
| P2 | `src-tauri/src/store/tests.rs` | 1-20, 540-620 | `TempDir` 헬퍼, settings/pets 테스트 패턴 |
| P2 | `src-tauri/src/window/tests.rs` | 315-340 | `command_allowed` 테스트 — 새 커맨드 추가 필요 |

## External Documentation

없음. 순수 내부 패턴만 사용한다. Tauri IPC/이벤트/asset protocol 모두 이미 이 저장소에 확립된 형태를 그대로 따른다.

---

## Data Contracts Touched

### `settings.json` — `$APPDATA/dev.cachebite.app/settings.json`

snake_case, `#[serde(deny_unknown_fields)]`. 날짜 필드 없음.

```json
{
  "schema_version": 3,
  "primary_provider": "claude",
  "selected_pet_id": "tabby",
  "bubble_enabled": true,
  "start_at_login": false,
  "notification_enabled": false,
  "secondary_notification_enabled": false,
  "logical_position": { "x": 0.0, "y": 0.0 }
}
```

이 플랜은 `selected_pet_id` **값만** 치환한다 (`"cat"` → `"tabby"`). 필드 추가/삭제 없음 → `schema_version`은 3 유지.

### `manifest.json` — `$APPDATA/dev.cachebite.app/pets/<id>/manifest.json`

camelCase, `#[serde(deny_unknown_fields)]`. 날짜 필드 없음.

```json
{
  "id": "tabby",
  "displayName": "Tabby",
  "version": 1,
  "defaultSize": { "width": 128, "height": 128 },
  "animations": {
    "idle": {
      "type": "frames",
      "frames": ["frames/tabby_idle_01.png"],
      "frameDurationMs": 240
    }
  },
  "states": { "idle": "idle" }
}
```

신규 IPC `list_pet_packages`가 읽는 필드는 `id`, `displayName` 두 개뿐이다. 나머지는 `PetPackageRepository::load()`가 검증 목적으로만 파싱한다.

---

## Patterns to Mirror

### NAMING_CONVENTION — Rust 저장소 메서드

```rust
// SOURCE: src-tauri/src/store/pets.rs:51
pub fn load(&self, id: &str) -> io::Result<PetPackage> {
    validate_id(id)?;
    let pets = fs::canonicalize(&self.pets_root)?;
    let root = fs::canonicalize(self.pets_root.join(id))?;
```

snake_case 메서드, `io::Result<T>` 반환, 인자 검증 먼저.

### ERROR_HANDLING — IPC 커맨드

```rust
// SOURCE: src-tauri/src/refresh/ipc.rs:171-183
#[tauri::command]
pub fn get_pet_package(
    window: tauri::WebviewWindow,
    settings: State<'_, SettingsRepository>,
    pets: State<'_, PetPackageRepository>,
) -> Result<PetPackage, IpcError> {
    authorize(&window, NativeCommand::GetPetPackage)?;
    let id = settings
        .load()
        .map_err(|_| IpcError::PersistenceUnavailable)?
        .selected_pet_id;
    pets.load(&id).map_err(|_| IpcError::PersistenceUnavailable)
}
```

**GOTCHA:** raw `io::Error`를 절대 렌더러로 넘기지 않는다. 경로가 새어나간다(프라이버시 계약). 반드시 `IpcError` 변종으로 뭉갠다.

### ERROR_HANDLING — 조용한 실패 허용 지점

```rust
// SOURCE: src-tauri/src/store/pets.rs:97-100
pub fn should_preserve_installed(&self, id: &str, bundled_version: u32) -> bool {
    let Ok(package) = self.load(id) else {
        return false;
    };
```

let-else로 조기 반환. 손상된 패키지는 에러가 아니라 "없는 것"으로 취급한다.

### VALUE_MIGRATION — settings 값 마이그레이션 (schema 버전 안 올림)

```rust
// SOURCE: src-tauri/src/store/settings.rs:137-151
if let Ok(settings) = serde_json::from_slice::<Settings>(&bytes) {
    if validate(&settings).is_ok() {
        if settings.selected_pet_id == "idle" {
            let migrated = Settings {
                selected_pet_id: match settings.primary_provider {
                    Provider::Claude => "cat".into(),
                    Provider::Codex => "corgi".into(),
                },
                ..settings
            };
            write_json_atomically(&self.path, &migrated)?;
            return Ok(migrated);
        }
        return Ok(settings);
    }
}
```

**이 패턴을 정확히 미러링한다.** 구조가 안 바뀌므로 `SETTINGS_SCHEMA_VERSION`은 3에서 올리지 않는다. 마이그레이션 후 즉시 디스크에 되쓴다.

### AUTHORIZATION — 창별 커맨드 화이트리스트

```rust
// SOURCE: src-tauri/src/window/mod.rs:73-99
pub fn command_allowed(window_label: &str, command: NativeCommand) -> bool {
    match window_label {
        "overlay" => matches!(
            command,
            NativeCommand::GetCollectorMode
                | NativeCommand::GetProviderStates
                ...
        ),
        "panel" => matches!(command, ... | NativeCommand::UpdateSettings | ...),
        _ => false,
    }
}
```

**GOTCHA:** 기본 거부. 새 커맨드는 명시적으로 넣지 않으면 `Forbidden`이다.

### GATEWAY_WIRE — snake_case ↔ camelCase 변환

```typescript
// SOURCE: src/lib/api/gateway.ts:109, 165-171
type PetPackageWire = { manifest: unknown; asset_base_url: string };

async getPetPackage() {
  const wire = await invokeNative<PetPackageWire>('get_pet_package');
  return {
    manifest: wire.manifest,
    assetBaseUrl: `${convertFileSrc(wire.asset_base_url).replace(/\/$/, '')}/`,
  };
},
```

wire 타입은 파일 로컬(non-export), 모델 타입은 export.

### SVELTE_PROPS — SettingsPanel 경계 내로잉

```svelte
<!-- SOURCE: src/lib/components/SettingsPanel.svelte:11-14 -->
// The <select> only ever holds the two option values below, but its DOM type
// is plain string. Narrow at the boundary rather than trusting the markup.
/** @param {string} value @returns {import('../contracts/domain').Provider} */
const asProvider = (value) => (value === 'codex' ? 'codex' : 'claude');
```

**GOTCHA:** `SettingsPanel.svelte`는 `<script lang="ts">`가 **아니다**. JSDoc 타입 주석을 쓴다. TS 문법을 넣으면 깨진다.

### SVELTE_FIELD — select 행 마크업

```svelte
<!-- SOURCE: src/lib/components/SettingsPanel.svelte:53-64 -->
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

`.field` 클래스가 label 좌 / control 우 배치를 담당한다. 스타일 추가 불필요.

### TEST_STRUCTURE — Rust store 테스트

```rust
// SOURCE: src-tauri/src/store/tests.rs:594-608
#[test]
fn pet_package_loader_returns_safe_asset_url_and_rejects_escaping_assets() {
    let dir = TempDir::new().expect("temp dir");
    let root = dir.path().join("pets/pet-2");
    fs::create_dir_all(root.join("frames")).unwrap();
    fs::write(root.join("frames/idle.svg"), "<svg/>").unwrap();
    let manifest = serde_json::json!({"id":"pet-2","displayName":"Pet",...});
    fs::write(root.join("manifest.json"), serde_json::to_vec(&manifest).unwrap()).unwrap();
    let package = PetPackageRepository::new(dir.path()).load("pet-2").expect("load package");
```

`TempDir` 헬퍼(파일 내 정의) + `serde_json::json!` 매크로로 manifest 작성.

### TEST_STRUCTURE — Svelte 컴포넌트 테스트

```typescript
// SOURCE: src/lib/components/SettingsPanel.test.ts:24-37
await fireEvent.change(screen.getByLabelText('Primary provider'), {
  target: { value: 'codex' },
});
expect(onChange).toHaveBeenCalledWith(
  expect.objectContaining({ primaryProvider: 'codex' }),
);
```

`getByLabelText` + `fireEvent.change` + `objectContaining`.

---

## Files to Change

| File | Action | Justification |
|---|---|---|
| `src-tauri/src/store/pets.rs` | UPDATE | `PetSummary` 타입 + `list()` 추가; `should_preserve_installed`의 하드코딩 displayName 제거 |
| `src-tauri/src/store/mod.rs` | UPDATE | `PetSummary`, `BundledPetInfo`, `bundled_manifest_info` re-export |
| `src-tauri/src/store/settings.rs` | UPDATE | 기본값 `tabby`, `cat`→`tabby` 값 마이그레이션, `"idle"` 마이그레이션 맵 갱신 |
| `src-tauri/src/refresh/ipc.rs` | UPDATE | `list_pet_packages` 커맨드 추가 |
| `src-tauri/src/window/mod.rs` | UPDATE | `NativeCommand::ListPetPackages` + panel 화이트리스트 |
| `src-tauri/src/lib.rs` | UPDATE | 번들 설치를 리소스 디렉터리 스캔으로; 은퇴 패키지 제거; invoke_handler 등록 |
| `src-tauri/resources/pets/cat/` | DELETE | `tabby`로 대체 |
| `src-tauri/resources/pets/tabby/` | CREATE | 빌드 스크립트가 생성 |
| `scripts/build-pet-packages.py` | UPDATE | `PET_SOURCES` 키 `cat` → `tabby` |
| `src/lib/api/gateway.ts` | UPDATE | `PetSummaryModel`, `listPetPackages()` |
| `src/lib/api/fixtureGateway.ts` | UPDATE | `listPetPackages` 구현 (인터페이스 충족) |
| `src/lib/state/presentation.ts` | UPDATE | `SettingsStoreState`에 `selectedPetId` |
| `src/lib/stores/settings.ts` | UPDATE | `SettingsState`에 `selectedPetId` + 기본값 |
| `src/lib/components/SettingsPanel.svelte` | UPDATE | Pet 드롭다운 + `pets` prop |
| `src/App.svelte` | UPDATE | `PROVIDER_PET` 제거, 오버라이드 제거, 펫 목록 로드, 기본값 `tabby` |
| `src/App.test.ts` | UPDATE | fixture에 `listPetPackages` 추가; 결합 해제 테스트 |
| `src/lib/components/SettingsPanel.test.ts` | UPDATE | 펫 선택 테스트 |
| `src/lib/assets/manifest.test.ts` | UPDATE | import 경로 `cat` → `tabby` |
| `src-tauri/src/store/tests.rs` | UPDATE | `list()` 테스트, 마이그레이션 테스트 |
| `src-tauri/src/window/tests.rs` | UPDATE | 새 커맨드 인가 테스트 |

## NOT Building

- **펫 가져오기 UI** (파일 선택 → 검증 → 설치). 별도 건. `tauri-plugin-dialog`, zip-slip 방어, 롤백, 보안 검토가 필요하다.
- **manifest 스펙 공개 문서**. 서드파티 펫 배포를 지원하지 않기로 결정했다 (CacheBite 고유 양식). `README.md:37`의 "Additional user-supplied packages can follow the same manifest contract" 문장도 이번엔 건드리지 않는다 — 사실 관계상 틀린 말은 아니다.
- **펫 미리보기 썸네일**. 드롭다운은 displayName 텍스트만.
- **선택된 펫이 사라졌을 때 자동 폴백**. 기존 "Pet package unavailable" 동작 유지. 마이그레이션이 정상 동작하면 발생하지 않는다.
- **`SETTINGS_SCHEMA_VERSION` 올리기**. 구조가 안 바뀐다. 값 마이그레이션만 한다.
- **corgi 개명**. 그대로 둔다.
- **설치 폴더에서 발견된 임의 패키지 삭제**. 은퇴 목록(`cat`)만 명시적으로 지운다.

---

## Step-by-Step Tasks

### Task 1: 펫 패키지 열거 (Rust)

- **ACTION**: `src-tauri/src/store/pets.rs`에 `PetSummary` 타입과 `PetPackageRepository::list()` 추가
- **IMPLEMENT**:
  ```rust
  #[derive(Clone, Debug, Serialize)]
  #[serde(rename_all = "camelCase")]
  pub struct PetSummary {
      pub id: String,
      pub display_name: String,
  }

  impl PetPackageRepository {
      /// Installed packages that load cleanly, sorted by display name.
      /// A malformed package is skipped, never an error: one bad directory
      /// must not empty the picker.
      pub fn list(&self) -> io::Result<Vec<PetSummary>> {
          let mut summaries: Vec<PetSummary> = fs::read_dir(&self.pets_root)?
              .flatten()
              .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
              .filter_map(|entry| {
                  let id = entry.file_name().into_string().ok()?;
                  let package = self.load(&id).ok()?;
                  Some(PetSummary {
                      id: package.manifest.id,
                      display_name: package.manifest.display_name,
                  })
              })
              .collect();
          summaries.sort_by(|a, b| a.display_name.cmp(&b.display_name));
          Ok(summaries)
      }
  }
  ```
- **MIRROR**: NAMING_CONVENTION, ERROR_HANDLING(조용한 실패 허용 지점)
- **IMPORTS**: 기존 `fs`, `io`, `Serialize` 그대로. 추가 없음
- **GOTCHA**: `self.load()`를 재사용해야 한다. 직접 manifest를 파싱하면 경로 이스케이프 검증(`pets.rs:76-90`)을 우회하게 되어 보안 계약이 깨진다. 또 `install_bundled_pet_packages`가 만드는 `.{id}.installing` 스테이징 디렉터리는 `is_valid_pet_id`가 선행 `.`을 거부하므로 `load()`에서 자동으로 걸러진다 — 별도 필터 불필요
- **VALIDATE**: `cargo test --manifest-path src-tauri/Cargo.toml store::tests`

### Task 2: `should_preserve_installed` 하드코딩 제거 (Rust)

- **ACTION**: `pets.rs:97-124`의 `match id { "cat" => "Cat", "corgi" => "Corgi", _ => return true }`를 번들 manifest에서 읽도록 변경
- **IMPLEMENT**: 시그니처를 `should_preserve_installed(&self, id: &str, bundled: &BundledPetInfo) -> bool`로 바꾸고, `bundled_manifest_version`을 `bundled_manifest_info`로 확장:
  ```rust
  #[derive(Clone, Debug)]
  pub struct BundledPetInfo {
      pub version: u32,
      pub display_name: String,
  }

  /// Reads `version` and `displayName` from a bundled manifest, tolerant of any
  /// other shape. `None` when missing or unparseable.
  pub fn bundled_manifest_info(manifest_path: &Path) -> Option<BundledPetInfo> {
      #[derive(Deserialize)]
      #[serde(rename_all = "camelCase")]
      struct Probe {
          #[serde(default)]
          version: u32,
          display_name: String,
      }
      let bytes = fs::read(manifest_path).ok()?;
      let probe: Probe = serde_json::from_slice(&bytes).ok()?;
      Some(BundledPetInfo { version: probe.version, display_name: probe.display_name })
  }
  ```
  그리고 `should_preserve_installed` 안의 `bundled_name`을 `&bundled.display_name`로, `package.manifest.version < bundled_version`을 `< bundled.version`으로 교체. `_ => return true` 분기는 사라진다
- **MIRROR**: 기존 `bundled_manifest_version`의 VersionProbe 패턴
- **IMPORTS**: 추가 없음
- **GOTCHA**: Probe에 `deny_unknown_fields`를 붙이면 안 된다. 번들 manifest의 다른 필드에서 파싱이 터진다. 기존 `VersionProbe`도 안 붙여놨다. `rename_all = "camelCase"`는 필요하다 — manifest는 `displayName`이다
- **VALIDATE**: `cargo test --manifest-path src-tauri/Cargo.toml` — `lib.rs`의 `upgrades_a_legacy_bundled_pet_package_to_current_frame_names` 등 기존 테스트가 계속 통과해야 한다

### Task 3: `list_pet_packages` IPC 커맨드 (Rust)

- **ACTION**: `window/mod.rs`에 enum 변종 + 인가, `refresh/ipc.rs`에 커맨드, `lib.rs`에 등록
- **IMPLEMENT**:
  - `window/mod.rs:62` `GetPetPackage` 아래에 `ListPetPackages,` 추가
  - `command_allowed`의 `"panel"` arm에 `| NativeCommand::ListPetPackages` 추가. **overlay arm에는 넣지 않는다** — Settings UI는 panel 창에만 있다
  - `refresh/ipc.rs`:
    ```rust
    #[tauri::command]
    pub fn list_pet_packages(
        window: tauri::WebviewWindow,
        pets: State<'_, PetPackageRepository>,
    ) -> Result<Vec<PetSummary>, IpcError> {
        authorize(&window, NativeCommand::ListPetPackages)?;
        pets.list().map_err(|_| IpcError::PersistenceUnavailable)
    }
    ```
  - `lib.rs:119` `refresh::ipc::get_pet_package,` 아래 `refresh::ipc::list_pet_packages,` 추가
- **MIRROR**: ERROR_HANDLING(IPC 커맨드), AUTHORIZATION
- **IMPORTS**: `refresh/ipc.rs:10-13`의 `crate::store::{...}` use에 `PetSummary` 추가
- **GOTCHA**: `store/mod.rs:7`의 `pub use pets::{...}`에 `PetSummary`, `BundledPetInfo`, `bundled_manifest_info`를 추가하지 않으면 컴파일이 안 된다. `bundled_manifest_version`은 Task 2에서 대체되므로 제거한다
- **VALIDATE**: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-features -- -D warnings`

### Task 4: 번들 설치 확장성 + 은퇴 패키지 정리 (Rust)

- **ACTION**: `lib.rs:203-229` `install_bundled_pet_packages` 재작성
- **IMPLEMENT**:
  ```rust
  /// Packages that shipped in an earlier release and were renamed or dropped.
  /// Removed from the install directory so a stale copy never shows up in the
  /// picker beside its replacement.
  const RETIRED_PET_IDS: &[&str] = &["cat"];

  fn install_bundled_pet_packages(resource_dir: &Path, app_data: &Path) -> io::Result<()> {
      let bundled_pets = resource_dir.join("resources").join("pets");
      let installed_pets = app_data.join("pets");
      fs::create_dir_all(&installed_pets)?;
      for retired in RETIRED_PET_IDS {
          let stale = installed_pets.join(retired);
          if stale.exists() {
              fs::remove_dir_all(&stale)?;
          }
      }
      let repository = store::PetPackageRepository::new(app_data);
      for entry in fs::read_dir(&bundled_pets)? {
          let entry = entry?;
          if !entry.file_type()?.is_dir() {
              continue;
          }
          let Ok(package_id) = entry.file_name().into_string() else {
              continue;
          };
          let Some(bundled) = store::bundled_manifest_info(&entry.path().join("manifest.json"))
          else {
              continue;
          };
          if repository.should_preserve_installed(&package_id, &bundled) {
              continue;
          }
          // ...기존 destination/staging/copy/rename 블록 그대로...
      }
      Ok(())
  }
  ```
- **MIRROR**: 기존 함수의 staging → rename → 실패 시 staging 정리 흐름을 **그대로 보존**한다
- **IMPORTS**: 추가 없음
- **GOTCHA**: 은퇴 정리는 반드시 설치 루프 **앞**에 온다. 뒤에 두면 미래에 은퇴 id가 번들 id와 겹칠 때 방금 설치한 걸 지운다
- **GOTCHA**: `RETIRED_PET_IDS` 정리는 사용자 데이터를 지운다. 이름을 바꾼 커스텀 `cat` 패키지가 있어도 지워진다 — 커스텀 펫 미지원 결정에 따른 의도된 동작이다
- **VALIDATE**: `cargo test --manifest-path src-tauri/Cargo.toml lib::tests` — 기존 4개 번들 설치 테스트의 `["cat","corgi"]` 픽스처를 `["tabby","corgi"]`로 갱신해야 한다

### Task 5: settings 기본값 + `cat` → `tabby` 마이그레이션 (Rust)

- **ACTION**: `store/settings.rs` 수정
- **IMPLEMENT**:
  - `:45` `selected_pet_id: "cat".into()` → `"tabby".into()`
  - `:139-149` 조건을 확장:
    ```rust
    // `idle` is an animation key that a very old build wrote as a package id.
    // `cat` was renamed to `tabby`; both need the stored value repaired in
    // place, or get_pet_package fails against a package that no longer exists.
    let repaired = match settings.selected_pet_id.as_str() {
        "idle" => Some(match settings.primary_provider {
            Provider::Claude => "tabby",
            Provider::Codex => "corgi",
        }),
        "cat" => Some("tabby"),
        _ => None,
    };
    if let Some(id) = repaired {
        let migrated = Settings { selected_pet_id: id.into(), ..settings };
        write_json_atomically(&self.path, &migrated)?;
        return Ok(migrated);
    }
    return Ok(settings);
    ```
  - `SettingsV1`/`SettingsV2` 블록의 `write_json_atomically(&self.path, &migrated)?;` 뒤를 `return Ok(migrated);` 대신 `return self.load_locked();`로 교체
- **MIRROR**: VALUE_MIGRATION
- **IMPORTS**: 추가 없음
- **GOTCHA**: `SETTINGS_SCHEMA_VERSION`을 올리지 않는다. 필드가 안 바뀌었다. 올리면 `validate()`가 기존 파일을 전부 거부하고 격리(quarantine)해서 사용자 설정이 초기화된다
- **GOTCHA**: V1/V2 경로는 `selected_pet_id`를 그대로 옮기므로 `"cat"` 값이 마이그레이션되지 않는다. `load_locked()` 재진입으로 해결한다 — 재진입 시 최신 스키마로 파싱되어 위 `repaired` 블록을 타므로 무한 루프가 아니다
- **VALIDATE**: `cargo test --manifest-path src-tauri/Cargo.toml store::tests` — `assert_eq!(Settings::default().selected_pet_id, "cat")` (`tests.rs:545`)을 `"tabby"`로 갱신

### Task 6: `tabby` 패키지 생성 (Python + 리소스)

- **ACTION**: 빌드 스크립트 매핑 변경 후 재생성, 구 디렉터리 삭제
- **IMPLEMENT**:
  - `scripts/build-pet-packages.py:24` `PET_SOURCES = {"cat": "cat", "corgi": "corgi"}` → `PET_SOURCES = {"tabby": "cat", "corgi": "corgi"}`
  - `rm -rf src-tauri/resources/pets/cat`
  - `python3 scripts/build-pet-packages.py`
- **MIRROR**: 스크립트가 이미 `package_id`를 프레임 파일명(`{package_id}_{state}_{n}.png`, `:224`)과 displayName(`package_id.title()`, `:230`)에 반영한다
- **IMPORTS**: N/A
- **GOTCHA**: `PET_SOURCES`는 `{패키지 id: 소스 아트 폴더}` 매핑이다. 값은 `"cat"`으로 **유지**해야 한다 — `docs/UI-plan/assets/pet/cat/`의 원본 아트는 이름을 바꾸지 않는다
- **GOTCHA**: `iter_source_frames()`의 `relative[:2] == ("cat", "critical")` (`:186`)는 **소스** 경로 기준이므로 그대로 둔다
- **GOTCHA**: `PACKAGE_VERSION`(`:36`)은 올리지 않는다. 새 id는 설치된 사본이 없으므로 강제 재설치 로직이 필요 없다
- **VALIDATE**:
  ```bash
  ls src-tauri/resources/pets/tabby/frames | head -3   # tabby_idle_01.png ...
  python3 -c "import json;m=json.load(open('src-tauri/resources/pets/tabby/manifest.json'));print(m['id'],m['displayName'])"
  # tabby Tabby
  ```

### Task 7: 게이트웨이 계약 확장 (TS)

- **ACTION**: `src/lib/api/gateway.ts`
- **IMPLEMENT**:
  - `PetPackageModel` 아래에 모델 타입 추가:
    ```typescript
    export interface PetSummaryModel {
      readonly id: string;
      readonly displayName: string;
    }
    ```
  - `AppGateway` 인터페이스의 `getPetPackage()` 아래에 `listPetPackages(): Promise<readonly PetSummaryModel[]>;`
  - `tauriGateway`에 `listPetPackages: () => invokeNative('list_pet_packages'),`
- **MIRROR**: GATEWAY_WIRE
- **IMPORTS**: 추가 없음
- **GOTCHA**: Rust `PetSummary`에 `#[serde(rename_all = "camelCase")]`를 붙였으므로 wire가 이미 `displayName`이다. **변환 함수가 필요 없다** — `getPlatformCapabilities`처럼 직통으로 넘긴다. Task 1의 serde 어트리뷰트를 빠뜨리면 여기서 `display_name`이 와서 조용히 `undefined`가 된다
- **VALIDATE**: `pnpm check`

### Task 8: fixture 게이트웨이 + 테스트 목 갱신 (TS)

- **ACTION**: `src/lib/api/fixtureGateway.ts`, `src/App.test.ts`
- **IMPLEMENT**:
  - `fixtureGateway.ts`의 `getPetPackage` 아래:
    ```typescript
    listPetPackages: async () => [
      { id: 'fixture-pet', displayName: 'Fixture Pet' },
    ],
    ```
  - `App.test.ts:78`의 `getPetPackage` 목 아래:
    ```typescript
    listPetPackages: vi.fn(async () => [
      { id: 'fixture-pet', displayName: 'Fixture Pet' },
      { id: 'corgi', displayName: 'Corgi' },
    ]),
    ```
- **MIRROR**: 두 파일 모두 `AppGateway`를 구조적으로 만족시키는 형태
- **IMPORTS**: 추가 없음
- **GOTCHA**: 둘 다 안 고치면 `AppGateway` 타입 불만족으로 `pnpm check`가 깨진다
- **VALIDATE**: `pnpm check`

### Task 9: 뷰 모델에 `selectedPetId` 노출 (TS)

- **ACTION**: `src/lib/state/presentation.ts`, `src/lib/stores/settings.ts`
- **IMPLEMENT**:
  - `presentation.ts:5-12` `Pick<...>` 유니온에 `| 'selectedPetId'` 추가, `toSettingsStoreState`에 `selectedPetId: settings.selectedPetId,` 추가
  - `stores/settings.ts:4-10` `SettingsState`에 `readonly selectedPetId: string;` 추가, `defaultSettings:11-17`에 `selectedPetId: 'tabby',` 추가
- **MIRROR**: 두 타입의 기존 필드 순서/스타일
- **IMPORTS**: 추가 없음
- **GOTCHA**: **두 타입은 별도 선언이다.** `SettingsStoreState`(presentation.ts)와 `SettingsState`(stores/settings.ts)는 구조가 같지만 서로를 참조하지 않는다. 하나만 고치면 `App.svelte`에서 구조적 불일치가 난다
- **VALIDATE**: `pnpm check`

### Task 10: Settings에 펫 드롭다운 추가 (Svelte)

- **ACTION**: `src/lib/components/SettingsPanel.svelte`
- **IMPLEMENT**:
  - props에 `pets = []` 추가 (JSDoc 타입: `import('../api/gateway').PetSummaryModel[]`)
  - 폴백 파생값:
    ```javascript
    // A failed enumeration must still show what is currently selected —
    // an empty <select> hides the active pet entirely.
    const petOptions = $derived(
      pets.some((pet) => pet.id === settings.selectedPetId)
        ? pets
        : [
            { id: settings.selectedPetId, displayName: settings.selectedPetId },
            ...pets,
          ],
    );
    ```
  - Primary provider 필드 **아래**에 삽입:
    ```svelte
    <label class="field"
      >Pet <select
        value={settings.selectedPetId}
        onchange={(event) =>
          onChange({ ...settings, selectedPetId: event.currentTarget.value })}
        >{#each petOptions as pet (pet.id)}<option value={pet.id}
            >{pet.displayName}</option
          >{/each}</select
      ></label
    >
    ```
- **MIRROR**: SVELTE_FIELD, SVELTE_PROPS
- **IMPORTS**: 추가 없음
- **GOTCHA**: 이 파일은 `<script>`이지 `<script lang="ts">`가 **아니다**. 타입은 JSDoc으로만 쓴다
- **GOTCHA**: `asProvider` 같은 내로잉이 여기엔 필요 없다 — `selectedPetId`는 자유 문자열이고 검증은 네이티브 `validate()`가 한다
- **GOTCHA**: 스타일 추가 불필요. `.field` + `select` 규칙이 이미 있다
- **VALIDATE**: `pnpm vitest run src/lib/components/SettingsPanel.test.ts`

### Task 11: 결합 제거 + 목록 배선 (Svelte)

- **ACTION**: `src/App.svelte`
- **IMPLEMENT**:
  1. `:69-72` `PROVIDER_PET` 상수 **삭제**
  2. `:86` `selectedPetId: 'cat'` → `'tabby'` (주석의 "Must match the Rust default" 유지)
  3. `:503-510` primary 변경 시 펫 오버라이드 블록 **삭제**:
     ```javascript
     // 삭제 대상
     const primaryChanged = next.primaryProvider !== appSettings.primaryProvider;
     if (primaryChanged) {
       merged = { ...merged, selectedPetId: PROVIDER_PET[next.primaryProvider] };
     }
     ```
  4. `:529-531`의 `if (primaryChanged && windowLabel === 'overlay')`를 펫 변경 기준으로 교체. `primaryChanged` 자리에 다음을 둔다:
     ```javascript
     const petChanged = next.selectedPetId !== appSettings.selectedPetId;
     // ...
     if (petChanged && windowLabel === 'overlay') {
       await loadPetPackage();
     }
     ```
  5. 펫 목록 상태 + 로더 추가:
     ```javascript
     let petOptions = $state<readonly PetSummaryModel[]>([]);

     const loadPetOptions = async () => {
       // Enumeration is presentation-only: a failure leaves the picker showing
       // just the active pet rather than blocking the panel.
       petOptions = await gateway.listPetPackages().catch(() => []);
     };
     ```
     panel 창 기동 경로에서 호출한다 (`getHistory`/`getPlatformCapabilities`를 부르는 지점과 같은 곳, `windowLabel !== 'overlay'` 가드 안)
  6. `:600` SettingsPanel에 `pets={petOptions}` prop 전달
- **MIRROR**: 기존 `loadPetPackage`의 `.catch(() => null)` 방어 패턴
- **IMPORTS**: `./lib/api/gateway`의 type import 목록에 `type PetSummaryModel` 추가
- **GOTCHA**: `changeSettings`의 `merged`는 notification 블록이 여전히 재할당하므로 `let` 유지가 맞다. `const`로 바꾸면 깨진다
- **GOTCHA**: 오버레이 창은 `list_pet_packages` 인가가 없다(Task 3). `loadPetOptions`를 오버레이에서 호출하면 `Forbidden`이 난다 — 반드시 panel 가드 안에서만 호출한다
- **GOTCHA**: `:293-297`의 `listenSettings` 안 `petChanged` 로직은 **그대로 둔다**. 다른 창이 펫을 바꿨을 때 오버레이를 갱신하는 경로이고, 이번 기능이 바로 그 경로를 탄다
- **VALIDATE**: `pnpm vitest run src/App.test.ts`

### Task 12: 테스트 갱신 및 추가

- **ACTION**: 아래 5개 파일
- **IMPLEMENT**:
  - `src/lib/assets/manifest.test.ts:3,25` — `cat/manifest.json` import를 `tabby/manifest.json`으로, 케이스 배열 `['cat', catManifest]` → `['tabby', tabbyManifest]`
  - `src/App.test.ts:60` fixture `getSettings` 목의 `selectedPetId: 'cat'` → `'tabby'`
  - `src/App.test.ts:461-470` — "펫을 강제 교체한다"는 기존 단언 반전. primary만 바뀌고 `selectedPetId`는 `'tabby'`를 **유지**하는지 확인:
    ```typescript
    await waitFor(() =>
      expect(gateway.updateSettings).toHaveBeenCalledWith(
        expect.objectContaining({ primaryProvider: 'codex', selectedPetId: 'tabby' }),
      ),
    );
    ```
  - `src/App.test.ts` 신규: panel 창에서 Pet 드롭다운을 바꾸면 `updateSettings`가 `{ selectedPetId: 'corgi', primaryProvider: 'claude' }`로 호출되는지
  - `SettingsPanel.test.ts` — props에 `selectedPetId: 'tabby'`, `pets: [{id:'tabby',displayName:'Tabby'},{id:'corgi',displayName:'Corgi'}]` 추가 + `getByLabelText('Pet')` 변경 → `objectContaining({ selectedPetId: 'corgi' })` 단언
  - `src-tauri/src/store/tests.rs` 신규: `list()`가 유효 패키지만 displayName 순으로 반환하고 손상된 디렉터리를 건너뛰는지; `"cat"`이 로드 시 `"tabby"`로 마이그레이션되고 디스크에 되쓰이는지; V2 스키마 + `"cat"` 조합이 재진입 경로로 마이그레이션되는지
  - `src-tauri/src/window/tests.rs:323` 부근: `assert!(command_allowed("panel", NativeCommand::ListPetPackages)); assert!(!command_allowed("overlay", NativeCommand::ListPetPackages));`
  - `src-tauri/src/lib.rs` 테스트 4개(`:382,416,450,481,521`)의 `["cat","corgi"]` 픽스처 → `["tabby","corgi"]`, manifest JSON 문자열의 `"id":"cat"` → `"id":"tabby"`, `should_preserve_installed` 호출부를 `&BundledPetInfo{...}` 시그니처에 맞게 갱신
- **MIRROR**: TEST_STRUCTURE (Rust/Svelte 양쪽)
- **VALIDATE**: 아래 Validation Commands 전부

---

## Testing Strategy

### Unit Tests

| Test | Input | Expected Output | Edge Case? |
|---|---|---|---|
| `list()` 정상 열거 | `pets/` 아래 tabby, corgi 유효 패키지 | `[{corgi,Corgi},{tabby,Tabby}]` (displayName 정렬) | |
| `list()` 손상 패키지 무시 | 유효 1 + manifest 깨진 디렉터리 1 | 유효한 1개만 | ✅ |
| `list()` 빈 디렉터리 | `pets/` 존재하나 비어 있음 | `[]`, Err 아님 | ✅ |
| `list()` 루트 부재 | `pets/` 자체가 없음 | `Err` (IPC가 `PersistenceUnavailable`로 뭉갬) | ✅ |
| settings `cat` 마이그레이션 | `selected_pet_id: "cat"` | `"tabby"` 반환 + 디스크에 되쓰임 | |
| settings `idle` 마이그레이션 | `selected_pet_id: "idle"`, Claude | `"tabby"` | ✅ |
| settings V2 → V3 + `cat` | schema 2 + `"cat"` | `"tabby"`, schema 3 | ✅ 재진입 경로 |
| 커맨드 인가 | `("overlay", ListPetPackages)` | `false` | ✅ |
| 커맨드 인가 | `("panel", ListPetPackages)` | `true` | |
| primary 변경 시 펫 유지 | primary claude→codex, pet=tabby | `updateSettings`에 `selectedPetId: 'tabby'` | |
| 펫 변경 시 primary 유지 | pet tabby→corgi, primary=claude | `updateSettings`에 `primaryProvider: 'claude'` | |
| 드롭다운 폴백 | `pets: []`, `selectedPetId: 'tabby'` | option 1개(`tabby`) 렌더 | ✅ |
| 목록 로드 실패 | `listPetPackages` reject | 패널 정상 렌더, 현재 펫만 표시 | ✅ |
| 펫 변경 → 오버레이 리로드 | `emitSettings({selectedPetId:'corgi'})` | `getPetPackage` 2회 호출 | |

### Edge Cases Checklist

- [x] 빈 입력 — 펫 목록 `[]`
- [x] 잘못된 타입 — 손상된 manifest 디렉터리
- [x] 권한 거부 — overlay 창의 `list_pet_packages`
- [x] 마이그레이션 경로 — V1/V2/V3 × `cat`/`idle`
- [x] 은퇴 패키지 잔존 — `$APPDATA/pets/cat/` 제거 확인
- [ ] 동시 접근 — 해당 없음 (`list()`는 읽기 전용, settings는 기존 `path_lock` 사용)
- [ ] 네트워크 실패 — 해당 없음 (로컬 파일시스템만)

---

## Validation Commands

### Static Analysis

```bash
pnpm check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-features -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml --check
```
EXPECT: 에러 0

> **GOTCHA:** `cargo`는 반드시 `--manifest-path src-tauri/Cargo.toml`을 붙인다. 저장소 루트에는 `Cargo.toml`이 없어 `could not find Cargo.toml`로 실패한다.

### Unit Tests

```bash
pnpm vitest run src/App.test.ts src/lib/components/SettingsPanel.test.ts src/lib/assets/manifest.test.ts
cargo test --manifest-path src-tauri/Cargo.toml --all-features store::tests
cargo test --manifest-path src-tauri/Cargo.toml --all-features window::tests
```
EXPECT: 전부 통과

### Full Test Suite

```bash
pnpm test:ci
cargo test --manifest-path src-tauri/Cargo.toml --all-features
```
EXPECT: 회귀 없음, 커버리지 게이트 80% 유지 (branches/functions/lines/statements)

### Asset Regeneration

```bash
python3 scripts/build-pet-packages.py
git status --short src-tauri/resources/pets
```
EXPECT: `pets/tabby/` 추가, `pets/cat/` 삭제, `pets/corgi/` 변경 없음

### Manual Validation

- [ ] `pnpm tauri dev` 기동 → 오버레이에 Tabby 표시
- [ ] 펫 더블클릭 → 패널 → Settings → **Pet** 드롭다운에 Corgi/Tabby 2개
- [ ] Pet을 Corgi로 변경 → **오버레이 펫이 즉시 corgi로 교체**
- [ ] Primary provider를 Codex로 변경 → **펫은 corgi 유지**(강제 교체 없음), 게이지 데이터만 Codex
- [ ] Primary=Codex, Pet=Tabby 조합 설정 → 앱 재시작 → 조합 유지
- [ ] 업그레이드 시나리오: `%APPDATA%/dev.cachebite.app/settings.json`에 `"selected_pet_id": "cat"` 수동 기입 + `pets/cat/` 존재 상태로 기동 → Tabby로 정상 표시, `pets/cat/` 삭제됨, 드롭다운에 "Cat" 안 뜸

---

## Acceptance Criteria

- [ ] Primary provider 변경이 `selectedPetId`를 건드리지 않는다
- [ ] Settings에서 펫을 독립적으로 고를 수 있다
- [ ] 펫 목록이 설치된 패키지에서 동적으로 온다 (번들에 폴더 추가만으로 확장 가능)
- [ ] `cat` → `tabby` 개명 완료, 기존 사용자가 깨지지 않는다
- [ ] 구 `cat` 패키지가 설치 디렉터리에서 제거된다
- [ ] 모든 validation 명령 통과
- [ ] 커버리지 80% 유지

## Completion Checklist

- [ ] 발견된 패턴을 따랐다 (IPC 인가, wire 변환, 값 마이그레이션)
- [ ] 에러 처리가 `IpcError` 변종으로 뭉개진다 (경로 누출 없음)
- [ ] 렌더러 DTO/로그에 자격 증명·경로 없음 (프라이버시 계약)
- [ ] `PetSummary`에 `rename_all = "camelCase"` 있음
- [ ] `SettingsState`와 `SettingsStoreState` **둘 다** 갱신
- [ ] `fixtureGateway.ts`와 `App.test.ts` 목 **둘 다** 갱신
- [ ] 하드코딩 펫 id가 남아 있지 않다 (`grep -rn '"cat"' src src-tauri/src` — 은퇴 목록 제외)
- [ ] 불필요한 범위 추가 없음 (가져오기 UI, 스펙 문서 등)

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| 기존 사용자 settings의 `"cat"`이 마이그레이션 안 됨 → "Pet package unavailable" | Medium | High | Task 5의 값 마이그레이션 + V1/V2 경로 재진입. 수동 검증 시나리오로 확인 |
| `$APPDATA/pets/cat/`이 남아 드롭다운에 "Cat"과 "Tabby"가 동시 노출 | Medium | Medium | `RETIRED_PET_IDS` 정리를 설치 루프 앞에서 수행 |
| `PetSummary` serde 어트리뷰트 누락 → 렌더러에서 `displayName` undefined (빈 드롭다운) | Low | Medium | Task 7 GOTCHA에 명시. gateway 테스트로 커버 |
| overlay 창에서 `listPetPackages` 호출 → `Forbidden` | Low | Low | panel 가드 안에서만 호출. 인가 테스트로 커버 |
| `SETTINGS_SCHEMA_VERSION`을 실수로 올림 → 전 사용자 설정 격리 | Low | High | Task 5 GOTCHA에 명시. `store::tests`가 잡는다 |
| 커버리지 80% 미달 (신규 분기 추가) | Low | Medium | Testing Strategy의 edge case 테이블을 전부 구현 |
| 아트 재생성이 다른 픽셀 산출 (Pillow/numpy 버전 차이) | Low | Low | `git diff --stat src-tauri/resources/pets/corgi`로 corgi 무변경 확인. 변하면 환경 문제 |

## Notes

**결정 기록:**
- **동적 열거 채택 이유** — 사용자가 "버전 업데이트에 따른 펫 추가 확장성은 챙긴다"고 명시. 렌더러 하드코딩 리스트는 펫을 추가할 때마다 함께 고쳐야 하는 부채이고, 열거는 번들 폴더 추가만으로 끝난다. 코드량 차이는 Rust 함수 하나 + IPC 하나 수준.
- **커스텀 펫 미지원 확정** — CacheBite 고유 manifest 양식이라 서드파티 배포 생태계를 만들 수 없다는 판단. 열거는 구현 방식일 뿐, "커스텀 펫 지원"을 표방하지 않는다. manifest 스펙 문서화와 가져오기 UI는 별도 건.
- **`tabby` 선택 이유** — `corgi`가 품종명이므로 같은 층위(줄무늬 유형). 5자로 가장 짧고 영어권에서 즉시 이해된다. `cheddar`(치즈+주황, CacheBite 먹는 것 테마와 통일)와 `ginger`가 대안이었다.

**기존 배선 재사용:** 펫 변경 → 오버레이 반영은 `update_settings` → `settings-updated` 이벤트 → `App.svelte:293-297` `listenSettings`의 `petChanged` → `loadPetPackage()` 경로가 **이미 존재한다**. 새 이벤트나 배선이 필요 없다.

**부수 정리:** Task 2가 `pets.rs:101-104`의 `"cat" => "Cat"` 하드코딩을 없애고, Task 4가 `lib.rs:208`의 `["cat","corgi"]`를 없앤다. 둘 다 개명 때문에 어차피 손대야 하는 자리이고, 동시에 사용자가 요구한 확장성을 만든다.

**소스 아트는 건드리지 않는다.** `docs/UI-plan/assets/pet/cat/`은 그대로다. `PET_SOURCES`가 `{패키지 id: 소스 폴더}` 매핑이라 `{"tabby": "cat"}`으로 분리된다.
