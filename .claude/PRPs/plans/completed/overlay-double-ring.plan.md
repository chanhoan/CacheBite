# Plan: 오버레이 Double Ring (UI 전용)

## Summary

오버레이에 **작은 원(위성 링)** 하나를 추가해 두 provider를 한눈에 보여준다. 큰 원은 지금과 똑같이 `primaryProvider`를 그리고 그 안에 펫이 들어간다. 작은 원은 설정에서 `Ring: Double`을 고른 사용자에게만 나타나며, second provider(= primary의 반대편)가 자동으로 배정되고 중앙에 해당 provider 로고가 들어간다.

**동작은 하나도 바뀌지 않는다.** 패널 열림/숨김, 드래그, 컨텍스트 메뉴, 알림, 업데이트, 새로고침, stale 판정, 펫 무드 산출 — 전부 현행 그대로다. 추가되는 것은 렌더링 표면 하나와 그것을 켜는 설정 하나뿐이다.

## User Story

CacheBite 사용자로서, Claude와 Codex 사용량을 **패널을 열지 않고** 오버레이만 보고 동시에 파악하고 싶다. 그래야 주 사용 provider가 한계에 가까워질 때 다른 쪽으로 갈아탈 판단을 즉시 내릴 수 있다.

## Problem → Solution

**현재**: 오버레이는 `primaryProvider` 한 개만 표시한다. 두 번째 provider를 보려면 더블클릭 → 패널 → 탭 전환의 3단계가 필요하다.

**목표**: `ringMode: 'double'`이면 오버레이가 큰 링(primary + 펫) 옆에 작은 링(second provider + 로고)을 함께 그린다. 기본값 `single`에서는 화면이 지금과 픽셀 단위로 동일하다.

## Metadata

- **Complexity**: Medium
- **Source PRD**: `docs/UI-plan-2.0/CacheBite v2.dc.html` (§4 ring mode, §4.1 C1–C5)
- **PRD Phase**: standalone (패널 재설계는 후속 계획)
- **Estimated Files**: 신규 3 · 수정 15 = 18

### 확정된 설계 결정

| # | 결정 | 근거 |
|---|---|---|
| 1 | 큰 원 = **항상** `primaryProvider` | 사용자 요구사항 1. 연결 상태에 따른 링 스왑·강등 **없음** — 그것은 동작 변경이다 |
| 2 | 작은 원 = `ringMode === 'double'`일 때만 | 사용자 요구사항 2 |
| 3 | 작은 원 provider = primary의 반대편, 자동 | 사용자 요구사항 3. 사용자가 손으로 배정하는 UI 없음 |
| 4 | 펫 무드·알림·패널·업데이트 전부 현행 유지 | 사용자 요구사항 4. `primaryUi` 파생 경로 무변경 |
| 5 | 큰 원 중앙 = 펫, 작은 원 중앙 = provider 로고 | 사용자 요구사항 5 |
| 6 | `ring_mode`는 `AppSettings`(Rust)에 저장 | 패널에서 고르고 오버레이에 적용되어야 한다. `theme`처럼 localStorage에 두면 창을 넘지 못해 재시작 전까지 반영되지 않는다 (`state/theme.ts:1-5`). `settings-updated` 이벤트가 이미 오버레이까지 배달한다 |
| 7 | 작은 원 provider가 미연결이면 **칩 유지 + 시스템 배지** | 강등 로직 없음 = UI 전용 원칙 유지. 설정이 화면을 그대로 설명한다 |
| 8 | 설정 UI는 Appearance와 별개인 `Ring` 항목 | 색상 테마와 링 갯수는 직교한다(다크 + 더블 조합이 가능해야 함) |
| 9 | provider 로고는 인라인 SVG로 재현 | 48px에서 선명, 다크모드 대응, `ProviderLogo.svelte` 한 곳에 격리 |
| 10 | 작은 원 위에서도 드래그·더블클릭이 먹혀야 함 | 새 UI 요소가 기존 제스처를 막으면 그것이 곧 동작 변경이다 |

---

## UX Design

### Before (= `ringMode: 'single'`, 기본값)

```
┌──────── overlay window 240×240 ────────┐
│                                        │
│         ╭──── ring 128 ────╮           │
│         │      5H arc      │           │
│         │   ╭────────╮     │           │
│         │   │  pet   │     │           │  큰 원 = primary
│         │   ╰────────╯     │           │
│         │      WK arc      │           │
│         ╰──────────────────╯           │
│                                        │
└────────────────────────────────────────┘
```

### After (`ringMode: 'double'`)

```
┌──────── overlay window 240×240 ────────┐
│                                        │
│         ╭──── ring 128 ────╮           │
│         │      5H arc      │           │  큰 원 = primary (변경 없음)
│         │   ╭────────╮     │           │         중앙 = 펫 (변경 없음)
│         │   │  pet   │     │           │
│         │   ╰────────╯     │           │  작은 원 = second provider
│         │      WK arc   ╭──┴───╮       │         중앙 = provider 로고
│         ╰───────────────┤ ⬤logo│       │         자체 5H/WK 아크
│                         ╰──────╯       │         51px, ⅓ 겹침
│    drag surface = 큰 원 ∪ 작은 원       │
└────────────────────────────────────────┘
```

### Interaction Changes

| Touchpoint | Before | After | Notes |
|---|---|---|---|
| 오버레이 표시 | primary 링 1개 | `single`: 동일 / `double`: +작은 원 | 기본 `single` → 기존 사용자 화면 불변 |
| 드래그·더블클릭·우클릭 | 큰 원 원형 영역 | 큰 원 ∪ 작은 원 | 새 요소가 제스처를 막지 않게 표면 추가 |
| 설정 | Appearance, Primary provider, Pet, … | **Ring** 항목 추가 | `Single` / `Double` |
| 접근성 | `role="img"` 1개 | `role="img"` 2개 (작은 원은 provider명 포함) | 작은 원 포인터 표면은 `aria-hidden` |
| **패널 / 알림 / 업데이트 / 펫 무드** | — | **전부 변경 없음** | 코드 경로를 건드리지 않는다 |

---

## Mandatory Reading

| Priority | File | Lines | Why |
|---|---|---|---|
| P0 | `docs/UI-plan-2.0/CacheBite v2.dc.html` | 42–173 | 기획 원본. §4 ring mode, §4.1 C1–C5 |
| P0 | `src/lib/components/SplitUsageRing.svelte` | 전체 | 작은 원이 재사용할 아크 기하 · 심각도 토큰 · aria 라벨 |
| P0 | `src/lib/components/PetOverlay.svelte` | 전체 | 작은 원이 들어갈 레이아웃과 포인터 표면 |
| P0 | `src/App.svelte` | 517–569 | 오버레이 뷰모델 조립 지점 |
| P0 | `src-tauri/src/store/settings.rs` | 12–53, 108–119, 232–281 | 스키마 상수 · 마이그레이션 아암 |
| P1 | `src/lib/api/gateway.ts` | 24–33, 167–207 | `AppSettings` ↔ `SettingsWire` 매핑 |
| P1 | `src/lib/state/presentation.ts` | 5–36 | `SettingsStoreState` Pick 목록 |
| P1 | `src/lib/stores/settings.ts` | 1–20 | `SettingsState` + `defaultSettings` |
| P1 | `src-tauri/src/store/tests.rs` | 147–212 | 마이그레이션 테스트 서식 |
| P2 | `src/lib/components/PetOverlay.test.ts` | 전체 | 확장 대상 테스트 7건 |
| P2 | `src/lib/components/SystemBadge.svelte` | 전체 | 인라인 SVG 아이콘 컴포넌트 선례 |
| P2 | `docs/ui-contract.md` | 124–166 | §4 오버레이 명세 — 갱신 대상 |
| P2 | `docs/UI-plan-2.0/Claudecode_Logo.png`, `Codex_Logo.png` | — | 로고 원본(SVG 재현 기준) |

## External Documentation

외부 조사 불필요 — 전부 기존 내부 패턴이다. SVG 아크·`pathLength`·`stroke-dasharray`는 이미 `SplitUsageRing.svelte`에 있고, 설정 마이그레이션은 `settings.rs`에 5회 선례가 있다.

---

## Patterns to Mirror

### RING_GEOMETRY
```svelte
<!-- SOURCE: src/lib/components/SplitUsageRing.svelte:21-47 -->
<svg data-testid="usage-ring" data-stale={stale} class:stale class="ring"
     viewBox="0 0 100 100" role="img" aria-label={ringLabel}>
  <path class="track" d="M 8 50 A 42 42 0 0 1 92 50" pathLength="100" />
  <path class="usage" data-severity={session.severity}
        d="M 8 50 A 42 42 0 0 1 92 50" pathLength="100"
        stroke-dasharray={`${percent(session)} 100`} aria-hidden="true" />
  <path class="track" d="M 92 50 A 42 42 0 0 1 8 50" pathLength="100" />
  <path class="usage" data-severity={weekly.severity}
        d="M 92 50 A 42 42 0 0 1 8 50" pathLength="100"
        stroke-dasharray={`${percent(weekly)} 100`} aria-hidden="true" />
</svg>
```
상반원 = session(5H), 하반원 = weekly(WK). 작은 원도 **정확히 이 두 경로**를 쓴다 — 새 기하를 발명하지 않는다.

### SEVERITY_TOKENS
```css
/* SOURCE: src/lib/components/SplitUsageRing.svelte:75-96 */
path { stroke-width: 6.5; }
.track { stroke: var(--sev-unknown); opacity: 0.28; }
.usage { stroke: var(--sev-unknown); }
.usage[data-severity='ok'] { stroke: var(--sev-ok); }
.usage[data-severity='warn'] { stroke: var(--sev-warn); }
.usage[data-severity='critical'] { stroke: var(--sev-critical); }
.usage[data-severity='exhausted'] { stroke: var(--sev-exhausted); }
```
색은 언제나 `--sev-*` 토큰. 하드코딩 금지 (`src/lib/styles/tokens.css`가 라이트/다크 단일 원본).

### OVERLAY_LAYOUT
```svelte
<!-- SOURCE: src/lib/components/PetOverlay.svelte:36-66 -->
{#if model.system === 'active'}
  <SplitUsageRing session={model.session} weekly={model.weekly} stale={model.stale} />
{/if}
<div class="pet"><PetAnimation animation={model.animation} label={model.petName} /></div>
{#if model.system !== 'active'}
  <div class="badge-position"><SystemBadge system={model.system} /></div>
{/if}
<div class="interaction-surface" data-testid="overlay-pointer-surface"
     role="button" tabindex="0"
     aria-label="Move pet; double-click or press Enter to show or hide usage; right-click for the menu"
     style="clip-path: circle(50% at 50% 50%)"
     onpointerdown={onPointerDown} ondblclick={() => onToggle()}
     oncontextmenu={contextMenu} onkeydown={keydown}></div>
```
`active`면 링, 아니면 배지 — 이 분기 규칙을 작은 원도 그대로 따른다.

### GLOBAL_STYLE_PENETRATION
```css
/* SOURCE: src/lib/components/PetOverlay.svelte:76-90 */
.overlay :global(.ring) { position: absolute; inset: 0; width: 100%; height: 100%; }
.pet { position: absolute; inset: 16%; }
.badge-position { position: absolute; right: 2%; bottom: 2%; }
```
자식 컴포넌트의 배치는 부모가 `:global()`로 잡는다. 작은 원도 동일 방식.

### WIRE_MAPPING
```ts
// SOURCE: src/lib/api/gateway.ts:188-207
const fromSettings = (wire: SettingsWire): AppSettings => ({
  schemaVersion: wire.schema_version,
  primaryProvider: wire.primary_provider,
  ...
});
const toSettings = (settings: AppSettings): SettingsWire => ({
  schema_version: settings.schemaVersion,
  primary_provider: settings.primaryProvider,
  ...
});
```
Rust는 snake_case, 렌더러는 camelCase. **양방향 둘 다** 고쳐야 한다.

### SETTINGS_STORE
```ts
// SOURCE: src/lib/stores/settings.ts:4-20
export interface SettingsState {
  readonly primaryProvider: Provider;
  readonly selectedPetId: string;
  ...
}
export const defaultSettings: SettingsState = Object.freeze({
  primaryProvider: 'claude',
  // Must match the Rust default (`store/settings.rs`).
  selectedPetId: 'tabby',
  ...
});
```
기본값은 Rust 기본값과 반드시 일치해야 하며 `Object.freeze`로 잠근다.

### SETTINGS_MIGRATION
```rust
// SOURCE: src-tauri/src/store/settings.rs:249-265
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsV3 { schema_version: u32, primary_provider: Provider, /* … */ }

if let Ok(previous) = serde_json::from_slice::<SettingsV3>(&bytes) {
    if previous.schema_version == 3 {
        let migrated = Settings { primary_provider: previous.primary_provider,
                                  /* … */ ..Settings::default() };
        validate(&migrated)?;
        write_json_atomically(&self.path, &migrated)?;
        return self.load_locked();   // 재읽기로 pet id 복구까지 태운다
    }
}
```
버전별 구조체 + `schema_version` 가드 + `..Settings::default()` + `validate` 후 기록 + `load_locked()` 재귀.

### MIGRATION_TEST
```rust
// SOURCE: src-tauri/src/store/tests.rs:177-189
#[test]
fn version_three_settings_migrate_to_the_current_schema() {
    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join("settings.json"),
        r#"{"schema_version":3,"primary_provider":"claude",...}"#)
        .expect("write v3 settings");
    let loaded = SettingsRepository::new(dir.path()).load().expect("migrate v3");
    assert_eq!(loaded.schema_version, 5);
}
```

### COMPONENT_TEST
```ts
// SOURCE: src/lib/components/PetOverlay.test.ts:16-49
it('shows two accessible usage arcs for active usage', () => {
  render(PetOverlay, { props: { model: {
    system: 'active', stale: false,
    session: { usedPercent: 74, severity: 'warn' },
    weekly: { usedPercent: 93, severity: 'critical' },
    animation, petName: 'Geometric pet', size: 160,
  } } });
  expect(screen.getByRole('img', { name: 'Provider usage: 5-hour 74%, Weekly 93%' })).toBeTruthy();
});
```
`@testing-library/svelte` + 역할/접근명 기준 질의. `afterEach(cleanup)` 필수.

### SETTINGS_CONTROL
```svelte
<!-- SOURCE: src/lib/components/SettingsPanel.svelte:86-97 -->
<label class="field"
  >Primary provider <select
    value={settings.primaryProvider}
    onchange={(event) =>
      onChange({ ...settings, primaryProvider: asProvider(event.currentTarget.value) })}
    ><option value="claude">Claude</option><option value="codex">Codex</option
    ></select
  ></label
>
```
`<select>`의 DOM 타입은 `string`이므로 **경계에서 좁히는 `as*` 헬퍼**를 거친다(`asProvider`, `asTheme`). `ringMode`도 `asRingMode`가 필요하다.

---

## Files to Change

### 신규

| File | Action | Justification |
|---|---|---|
| `src/lib/components/ProviderLogo.svelte` | CREATE | Claude/Codex 인라인 SVG 마크 |
| `src/lib/components/SatelliteRing.svelte` | CREATE | 작은 원(퍽 + 축소 링 + 로고 + 배지) |
| `src/lib/components/SatelliteRing.test.ts` | CREATE | 작은 원 렌더/라벨/상태 테스트 |

### 수정

| File | Action | Justification |
|---|---|---|
| `src-tauri/src/store/settings.rs` | UPDATE | `RingMode` 열거형, 스키마 v6, v5→v6 마이그레이션 |
| `src-tauri/src/store/tests.rs` | UPDATE | v5→v6 테스트 추가 + 기존 어서션 5→6 갱신 |
| `src/lib/contracts/domain.ts` | UPDATE | `secondaryProvider()` 헬퍼 |
| `src/lib/api/gateway.ts` | UPDATE | `RingMode` 타입, `ringMode` 필드, 양방향 매핑 |
| `src/lib/api/fixtureGateway.ts` | UPDATE | `schemaVersion: 6`, `ringMode: 'single'` |
| `src/lib/state/presentation.ts` | UPDATE | `SettingsStoreState`에 `ringMode` 편입 |
| `src/lib/state/presentation.test.ts` | UPDATE | 위 필드 반영 |
| `src/lib/stores/settings.ts` | UPDATE | `SettingsState` + `defaultSettings`에 `ringMode` |
| `src/lib/stores/settings.test.ts` | UPDATE | 기본값 어서션 반영 |
| `src/lib/components/models.ts` | UPDATE | `SatelliteRingModel`, `PetOverlayViewModel.satellite` |
| `src/lib/components/SplitUsageRing.svelte` | UPDATE | `name` prop(기본 `'Provider'`) — 작은 원 라벨 구분용 |
| `src/lib/components/PetOverlay.svelte` | UPDATE | 작은 원 렌더 + 작은 원 포인터 표면 |
| `src/lib/components/PetOverlay.test.ts` | UPDATE | 기존 7건에 `satellite: null` + 신규 케이스 |
| `src/lib/components/SettingsPanel.svelte` | UPDATE | Ring 셀렉트 |
| `src/lib/components/SettingsPanel.test.ts` | UPDATE | 셀렉트 동작 테스트 |
| `src/App.svelte` | UPDATE | 작은 원 모델 조립 + 크기 클램프 |
| `src/App.test.ts` | UPDATE | 설정 픽스처에 `ringMode` |
| `docs/ui-contract.md` | UPDATE | §4에 링 모드 절 추가 |
| `CLAUDE.md` | UPDATE | 불변식에 링 모드 항목 추가 |

## NOT Building

- **클릭 패널 변경 일체.** 목업 §5는 현행 탭 기반 구현과 다르지만 **다음 계획**으로 넘긴다. `UsagePanel.svelte` / `ProviderTabs.svelte` / `panelModels.ts` / `tauri.conf.json`의 패널 폭(312px)은 손대지 않는다.
- **링 스왑·자동 강등.** primary가 미연결이어도 큰 원은 primary에 고정된다. second provider가 미연결이어도 `double`이면 작은 원을 그린다(결정 #1, #7).
- **펫 무드 산출 변경.** `primaryUi`/`primaryPresentation` 파생 경로를 건드리지 않는다.
- **알림·말풍선·업데이트·새로고침·stale·전체화면·핫키 로직.** 전부 무변경.
- 링 위치를 사용자가 손으로 provider에 배정하는 UI.
- 작은 원 클릭 → 해당 provider로 패널 열기 같은 신규 제스처. 작은 원은 기존 제스처의 **추가 타격 영역**일 뿐이다.
- 오버레이 창 크기 변경 (240×240 유지 — 작은 원이 기존 여백 안에 들어간다).
- 심각도 임계값(warn 70 / critical 90), 펫 에셋 키.

---

## Step-by-Step Tasks

### Task 1: second provider 헬퍼

- **ACTION**: `src/lib/contracts/domain.ts`에 함수 하나 추가.
- **IMPLEMENT**:
  ```ts
  /**
   * 작은 원에 배정되는 provider. 사용자가 손으로 고르지 않고 primary의
   * 반대편이 자동으로 결정된다(UI 계약 C2).
   */
  export const secondaryProvider = (primary: Provider): Provider =>
    primary === 'claude' ? 'codex' : 'claude';
  ```
- **MIRROR**: `src/lib/contracts/domain.ts:1-4` (같은 파일의 타입 정의 옆에 둔다)
- **IMPORTS**: 없음
- **GOTCHA**: 별도 정책 모듈(`ringMode.ts` 등)을 만들지 마라. 큰 원이 항상 primary로 고정되면서 판단 로직이 이 한 줄로 축소되었다 — 파일을 새로 파는 것은 YAGNI다.
- **VALIDATE**: `pnpm check`

### Task 2: 네이티브 설정 스키마 v6

- **ACTION**: `src-tauri/src/store/settings.rs` 수정.
- **IMPLEMENT**:
  1. `const SETTINGS_SCHEMA_VERSION: u32 = 6;`
  2. 새 열거형 (파일 상단, `LogicalPosition` 근처):
     ```rust
     /// 오버레이가 링을 몇 개 그리는지. 순수 표시 선호이며 수집·새로고침
     /// 정책에는 어떤 영향도 주지 않는다.
     #[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
     #[serde(rename_all = "snake_case")]
     pub enum RingMode {
         #[default]
         Single,
         Double,
     }
     ```
  3. `Settings`에 `pub ring_mode: RingMode,` 추가, `Default`에 `ring_mode: RingMode::Single,`.
  4. v5 구조체 추가 (현행 v5 필드 그대로, `ring_mode` 없음):
     ```rust
     #[derive(Deserialize)]
     #[serde(deny_unknown_fields)]
     struct SettingsV5 {
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
  5. `load_locked`의 현행 스키마 시도 **직후**에 v5 아암 삽입 — `SETTINGS_MIGRATION` 패턴 그대로, `if previous.schema_version == 5` 가드 필수.
- **MIRROR**: `SETTINGS_MIGRATION`
- **IMPORTS**: 추가 없음 (`serde` 이미 사용 중)
- **GOTCHA**: `SettingsV4`는 `hide_show_hotkey: Option<String>`을 갖는데 serde는 **누락된 `Option` 필드를 `None`으로 채운다**. 따라서 v5 파일은 `SettingsV5`뿐 아니라 `SettingsV4`로도 파싱에 성공한다. 두 아암 모두 `schema_version` 가드가 있어야만 안전하다 — 새 아암의 가드를 절대 빼지 마라.
- **GOTCHA**: `RingMode`를 `crate::domain`이 아니라 `store/settings.rs`에 둔다. 지속되는 **사용자 선호**이지 provider 도메인 개념이 아니다.
- **GOTCHA**: `validate()`는 손대지 않는다 — 열거형이라 불가능한 값이 표현되지 않는다.
- **GOTCHA**: `refresh/ipc.rs`의 `get_settings`/`update_settings`는 `Settings`를 직접 주고받으므로(`refresh/ipc.rs:240,325`) serde로 자동 전파된다. **새 IPC 명령도, `NativeCommand` 항목도, `command_allowed` 변경도, `apply_invoke_handler` 두 아암 수정도 필요 없다.**
- **VALIDATE**: `cargo test --manifest-path src-tauri/Cargo.toml store::tests`

### Task 3: 마이그레이션 테스트

- **ACTION**: `src-tauri/src/store/tests.rs`에 테스트 추가 + 기존 어서션 갱신.
- **IMPLEMENT**:
  ```rust
  #[test]
  fn version_five_settings_migrate_with_ring_mode_single() {
      let dir = TempDir::new().expect("temp dir");
      fs::write(
          dir.path().join("settings.json"),
          r#"{"schema_version":5,"primary_provider":"codex","selected_pet_id":"corgi","bubble_enabled":true,"start_at_login":true,"notification_enabled":true,"secondary_notification_enabled":true,"logical_position":{"x":-232.5,"y":220.5}}"#,
      )
      .expect("write v5 settings");

      let loaded = SettingsRepository::new(dir.path()).load().expect("migrate v5");

      assert_eq!(loaded.schema_version, 6);
      assert_eq!(loaded.ring_mode, RingMode::Single);
      assert_eq!(loaded.primary_provider, Provider::Codex);
      assert_eq!(loaded.selected_pet_id, "corgi");
      assert!(loaded.secondary_notification_enabled);
      assert_eq!(loaded.logical_position.x, -232.5);
  }
  ```
- **MIRROR**: `MIGRATION_TEST`
- **IMPORTS**: 기존 `use` 블록에 `RingMode` 추가
- **GOTCHA**: 기존 테스트들이 `assert_eq!(loaded.schema_version, 5)`로 하드코딩되어 있다(tests.rs:157, 172, 188, 205 등). **전부 6으로 갱신**해야 한다. `grep -n "schema_version, 5" src-tauri/src/store/tests.rs`로 남김없이 찾아라.
- **VALIDATE**: `cargo test --manifest-path src-tauri/Cargo.toml --all-features`

### Task 4: 게이트웨이 계약 확장

- **ACTION**: `src/lib/api/gateway.ts`, `src/lib/api/fixtureGateway.ts` 수정.
- **IMPLEMENT**:
  - `gateway.ts`: `AppSettings` 위에 타입 정의 추가 —
    ```ts
    /** 오버레이가 그리는 링 갯수. 순수 표시 선호. */
    export type RingMode = 'single' | 'double';
    ```
    `AppSettings`에 `readonly ringMode: RingMode;`, `SettingsWire`에 `ring_mode: RingMode;`,
    `fromSettings`에 `ringMode: wire.ring_mode`, `toSettings`에 `ring_mode: settings.ringMode`.
  - `fixtureGateway.ts`: `getSettings`의 `schemaVersion: 5` → `6`, `ringMode: 'single'` 추가.
- **MIRROR**: `WIRE_MAPPING`
- **IMPORTS**: 없음 (같은 파일에 정의)
- **GOTCHA**: `RingMode`는 `gateway.ts`에 둔다 — `CollectorMode`·`PanelVisibility`·`UpdateFailureReason`과 같은 자리다. `CLAUDE.md`가 "Every wire DTO is defined here. This is the contract"라고 못박고 있다.
- **GOTCHA**: `fromSettings`와 `toSettings` **둘 다** 고쳐야 한다. 한쪽만 고치면 저장 시 필드가 유실되고 Rust가 `missing field`로 거부한다.
- **GOTCHA**: `fixtureGateway.updateSettings`는 `async (settings) => settings`라 그대로 통과한다 — 추가 작업 없음.
- **VALIDATE**: `pnpm check`

### Task 5: 설정 스토어/프레젠테이션 배선

- **ACTION**: `src/lib/state/presentation.ts`, `src/lib/stores/settings.ts` 수정.
- **IMPLEMENT**:
  - `presentation.ts`: `SettingsStoreState`의 `Pick<AppSettings, …>` 유니온에 `| 'ringMode'` 추가, `toSettingsStoreState` 반환 객체에 `ringMode: settings.ringMode,` 추가.
  - `stores/settings.ts`: `SettingsState`에 `readonly ringMode: RingMode;` 추가, `defaultSettings`에 `ringMode: 'single',` 추가.
- **MIRROR**: `SETTINGS_STORE`, `src/lib/state/presentation.ts:25-36`
- **IMPORTS**: `stores/settings.ts`에 `import type { RingMode } from '../api/gateway';` (타입 전용 import이며, `presentation.ts`가 이미 같은 방향으로 `AppSettings`를 import한다)
- **GOTCHA**: `defaultSettings`의 주석 관례를 지켜라 — `selectedPetId` 위에 "Must match the Rust default (`store/settings.rs`)"가 붙어 있다. `ringMode: 'single'`도 Rust `RingMode::Single`과 일치해야 한다.
- **GOTCHA**: `setRingMode` 같은 전용 setter를 추가하지 마라. 설정 변경은 `SettingsPanel` → `onChange` → `changeSettings` → `settingsStore.replace()` 경로로 흐르며, 기존 `setPrimary`/`setBubbles` 등은 다른 호출자를 위한 것이다.
- **GOTCHA**: `presentation.test.ts` / `stores/settings.test.ts`의 객체 전체 동등 비교 어서션이 깨진다 — 픽스처에 `ringMode` 추가.
- **VALIDATE**: `pnpm vitest run src/lib/state/presentation.test.ts src/lib/stores/settings.test.ts`

### Task 6: provider 로고 SVG

- **ACTION**: `src/lib/components/ProviderLogo.svelte` 생성.
- **IMPLEMENT**: `provider: Provider` prop을 받아 인라인 `<svg>` 반환. `aria-hidden="true"`(작은 원의 `role="img"`가 이미 이름을 갖는다).
  - **Claude Code** (`docs/UI-plan-2.0/Claudecode_Logo.png`, 640×640, 1.4KB): 평면 2색 픽셀 마스코트 — 주황 전경, 배경 투명, 눈은 배경 구멍. 좌표를 눈으로 추정하지 말고 PNG에서 직사각형을 추출하라:
    ```bash
    python3 - <<'PY'
    from PIL import Image
    from collections import Counter
    im = Image.open('docs/UI-plan-2.0/Claudecode_Logo.png').convert('RGBA')
    px, (w, h) = im.load(), im.size
    print('colors:', Counter(im.getdata()).most_common(4))   # 전경색 1개인지 검증
    fg = Counter(im.getdata()).most_common(2)[1][0]
    for y in range(h):                                        # 행별 런렝스 → <rect>
        x = 0
        while x < w:
            if px[x, y] == fg:
                s = x
                while x < w and px[x, y] == fg: x += 1
                print(f'<rect x="{s}" y="{y}" width="{x-s}" height="1"/>')
            else: x += 1
    PY
    ```
    출력이 640행이면 세로로 동일한 연속 행을 병합해 `<rect>` 수를 줄여라. 전경색은 하드코딩하지 말고 `--logo-claude` 토큰으로 `tokens.css`에 둔다. `viewBox="0 0 640 640"`.
  - **Codex** (`docs/UI-plan-2.0/Codex_Logo.png`, 2048×2048): 8개 로브가 겹친 구름형 블롭 + 세로 그라디언트 + 검은 `>` 셰브론과 `_` 언더바. `viewBox="0 0 200 200"`, 중심 (100,100) 기준 45° 간격 8개 원(반지름 ≈ 46, 중심 거리 ≈ 52)의 합집합. `<linearGradient x1="0" y1="0" x2="0" y2="1">`로 위 연보라 → 아래 파랑 — 스톱 색은 PNG 상단·하단에서 샘플링해서 쓴다. 셰브론·언더바는 `<path stroke="#000" stroke-linecap="round" stroke-linejoin="round" stroke-width="14" fill="none">`.
  - 최종 검증: 두 SVG를 48px로 렌더해 원본 PNG의 48px 축소본과 나란히 눈으로 대조.
- **MIRROR**: `src/lib/components/SystemBadge.svelte` (인라인 SVG 아이콘 컴포넌트 선례 — prop으로 분기하고 `viewBox` 고정)
- **IMPORTS**: `import type { Provider } from '../contracts/domain';`
- **GOTCHA**: 원본 PNG는 `docs/`에 **소스로만** 남긴다. 릴리스 번들에 들어가는 것은 SVG뿐이다 — `docs/UI-plan/` 아트를 직접 번들하지 않는 기존 규칙과 같다. `src-tauri/tauri.conf.json`의 `bundle.resources`도 건드리지 않는다.
- **GOTCHA**: Codex 마크는 자체 그라디언트로 어두운 배경에서도 보이지만 Claude 마스코트는 단색이다. 다크 모드 대비를 확인하고 필요하면 `tokens.css`에 다크 변형을 둔다.
- **GOTCHA**: `img-src` CSP는 `'self' asset: http://asset.localhost data:`인데, **인라인 SVG는 `img-src` 대상이 아니다**(DOM 요소이지 이미지 요청이 아님). CSP 변경 불필요 — 이것이 PNG 대신 SVG를 고른 부수 이점이다.
- **GOTCHA (법무)**: 두 마크 모두 **타사 상표**다. 다시 그린다고 상표 의무가 사라지지 않는다. 배포 전 Anthropic·OpenAI 브랜드 가이드라인 확인이 필요하다 — Risks 참조.
- **VALIDATE**: `pnpm check` + 48px 시각 대조

### Task 7: `SplitUsageRing`에 이름 prop

- **ACTION**: `src/lib/components/SplitUsageRing.svelte` 수정.
- **IMPLEMENT**: props를 `let { session, weekly, stale, name = 'Provider' } = $props();`로 바꾸고 `ringLabel`을
  `` `${name} usage: ${label('5-hour', session)}, ${label('Weekly', weekly)}` ``로.
  JSDoc 타입에도 `name?: string` 추가.
- **MIRROR**: `src/lib/components/SplitUsageRing.svelte:2-18`
- **IMPORTS**: 없음
- **GOTCHA**: 기본값 `'Provider'`를 지키면 큰 원의 접근명이 그대로라 **기존 테스트 5건이 깨지지 않는다**. 큰 원은 `double` 모드에서도 이름을 넘기지 않는다 — 이름이 필요한 쪽은 "다른 하나"인 작은 원이다. 이 비대칭은 의도이며 `ui-contract.md`에 기록한다.
- **VALIDATE**: `pnpm vitest run src/lib/components/PetOverlay.test.ts` (전건 통과여야 함)

### Task 8: 뷰모델 확장

- **ACTION**: `src/lib/components/models.ts` 수정.
- **IMPLEMENT**:
  ```ts
  import type { Provider } from '../contracts/domain';

  /** 작은 원. `ringMode === 'double'`일 때만 채워진다. */
  export interface SatelliteRingModel {
    readonly provider: Provider;
    /** 접근명에 쓰이는 표시명 — 'Claude' | 'Codex'. */
    readonly providerName: string;
    readonly system: SystemState;
    readonly stale: boolean;
    readonly session: RingWindowModel;
    readonly weekly: RingWindowModel;
  }

  export interface PetOverlayViewModel {
    // …기존 필드 그대로…
    /** `null`이면 single 모드. 큰 원은 이 값과 무관하게 언제나 primary다. */
    readonly satellite: SatelliteRingModel | null;
  }
  ```
- **MIRROR**: `src/lib/components/models.ts:4-20`
- **IMPORTS**: `import type { Provider } from '../contracts/domain';`
- **GOTCHA**: `satellite`를 **필수**로 둔다(옵셔널 아님). 호출부가 명시적으로 결정하게 만드는 것이 이 파일의 기존 스타일이다(모든 필드가 `readonly` 필수). 대가로 `PetOverlay.test.ts`의 기존 모델 리터럴 7곳에 `satellite: null`을 넣어야 한다.
- **VALIDATE**: `pnpm check`

### Task 9: 작은 원 컴포넌트

- **ACTION**: `src/lib/components/SatelliteRing.svelte` 생성.
- **IMPLEMENT**:
  ```svelte
  <script>
    import ProviderLogo from './ProviderLogo.svelte';
    import SplitUsageRing from './SplitUsageRing.svelte';
    import SystemBadge from './SystemBadge.svelte';
    /** @type {{ model: import('./models').SatelliteRingModel }} */
    let { model } = $props();
  </script>

  <div class="satellite" data-testid="satellite-ring">
    <div class="puck"></div>
    {#if model.system === 'active'}
      <SplitUsageRing session={model.session} weekly={model.weekly}
                      stale={model.stale} name={model.providerName} />
    {/if}
    <div class="logo"><ProviderLogo provider={model.provider} /></div>
    {#if model.system !== 'active'}
      <div class="satellite-badge"><SystemBadge system={model.system} /></div>
    {/if}
  </div>
  ```
  스타일 — 작은 원은 큰 원 변의 **40%**, 우하단에 ⅓ 겹침:
  ```css
  .satellite { position: absolute; right: -4.3%; bottom: -5.5%;
               width: 40%; height: 40%; }
  .puck { position: absolute; inset: 0; border-radius: 50%;
          background: var(--color-surface);
          box-shadow: 0 3px 8px rgb(20 25 30 / 22%); }
  .satellite :global(.ring) { position: absolute; inset: 8%;
                              width: 84%; height: 84%; }
  /* 작은 원에서 6.5는 안 보인다 — 기획 C3의 18/200 = 9%를 따른다. */
  .satellite :global(.ring path) { stroke-width: 9; }
  /* 5H / WK 글자는 이 크기에서 읽히지 않는다(기획 C3: 칩에 라벨 없음). */
  .satellite :global(.ring-label) { display: none; }
  .logo { position: absolute; inset: 26%; }
  .satellite-badge { position: absolute; right: 0; bottom: 0; }
  ```
- **MIRROR**: `OVERLAY_LAYOUT`(active면 링, 아니면 배지), `GLOBAL_STYLE_PENETRATION`
- **IMPORTS**: 위 스크립트 블록 참조
- **GOTCHA**: `right`/`bottom` 백분율은 **부모 크기 기준**이며 부모는 `.overlay`(= `model.size`). 목업 비율(184px 링에 74px 칩, `right:-8px; top:120px`)을 128px로 환산하면 작은 원 51px, 오버행 우측 5.5px(4.3%) · 하단 7px(5.5%)이다.
- **GOTCHA**: `stroke-width: 9`를 CSS로 덮는다. `SplitUsageRing`의 `path { stroke-width: 6.5 }`는 컴포넌트 스코프 스타일이므로 `:global()` 특이성으로 이겨야 한다. 안 먹으면 `.satellite :global(.ring) path`로 한 단계 더 구체화하라.
- **GOTCHA**: 큰 원 배지(`.badge-position`, `right:2% bottom:2%`)와 작은 원이 화면상 같은 구석에 온다. 큰 원이 `active`가 아니고 `double`인 경우 둘이 겹치는지 육안 확인하고, 겹치면 큰 원 배지를 옮기지 말고 **작은 원을 z-index로 위에** 둬라(작은 원이 새 요소이므로 기존 배치를 존중한다).
- **VALIDATE**: `pnpm vitest run src/lib/components/SatelliteRing.test.ts`

### Task 10: `PetOverlay`에 작은 원과 포인터 표면

- **ACTION**: `src/lib/components/PetOverlay.svelte` 수정.
- **IMPLEMENT**: 기존 `.interaction-surface` **뒤에** 추가 (기존 마크업은 한 줄도 바꾸지 않는다):
  ```svelte
  {#if model.satellite}
    <SatelliteRing model={model.satellite} />
    <div
      class="satellite-surface"
      data-testid="overlay-satellite-pointer-surface"
      aria-hidden="true"
      onpointerdown={onPointerDown}
      onpointermove={onPointerMove}
      onpointerup={onPointerUp}
      onpointercancel={onPointerCancel}
      ondblclick={() => onToggle()}
      oncontextmenu={contextMenu}
    ></div>
  {/if}
  ```
  스타일: `.satellite-surface`는 `.satellite`와 동일 좌표·크기(`right: -4.3%; bottom: -5.5%; width: 40%; height: 40%`), `border-radius: 50%`, `clip-path: circle(50% at 50% 50%)`, `z-index: 1`, `cursor: grab`, `touch-action: none`. `:active`에 `cursor: grabbing`.
- **MIRROR**: `src/lib/components/PetOverlay.svelte:51-65`, `91-102`
- **IMPORTS**: `import SatelliteRing from './SatelliteRing.svelte';`
- **GOTCHA**: 작은 원 표면에 `role="button"`이나 `aria-label`을 **붙이지 마라**. 기존 표면과 같은 이름을 가지면 `screen.getByRole('button', { name })`가 다중 매치로 던진다. 작은 원은 이미 접근 가능한 컨트롤의 중복 타격 영역이므로 `aria-hidden="true"` + 역할 없음이 옳다. 키보드 접근(`Enter`/`Space`)은 기존 표면이 계속 담당한다.
- **GOTCHA**: `onkeydown`·`tabindex`도 붙이지 마라 — 포커스 불가 요소이므로 죽은 코드다.
- **GOTCHA**: `.overlay`에 `overflow: hidden`을 추가하지 마라. 작은 원은 의도적으로 부모 상자 밖으로 나간다.
- **GOTCHA**: `contextMenu` 핸들러를 재정의하지 말고 기존 것을 재사용하라 — 웹뷰 기본 메뉴 차단(`event.preventDefault()`)이 작은 원 위에서도 동일하게 필요하다.
- **VALIDATE**: `pnpm vitest run src/lib/components/PetOverlay.test.ts`

### Task 11: `App.svelte` 조립

- **ACTION**: `src/App.svelte` 수정.
- **IMPLEMENT**:
  - 새 파생 하나 추가:
    ```ts
    const satelliteProvider = $derived(
      appSettings.ringMode === 'double'
        ? secondaryProvider(appSettings.primaryProvider)
        : null,
    );
    ```
  - `overlayModel`(550–569) 객체에 `satellite` 한 필드만 추가. **기존 필드와 `primaryState`/`primaryPresentation`/`primaryUi` 파생은 한 줄도 건드리지 않는다.**
    ```ts
    satellite: satelliteProvider
      ? {
          provider: satelliteProvider,
          providerName: satelliteProvider === 'claude' ? 'Claude' : 'Codex',
          system: panelProviders[satelliteProvider].system,
          stale: panelProviders[satelliteProvider].stale,
          session: {
            usedPercent: panelProviders[satelliteProvider].session.usedPercent,
            severity: panelProviders[satelliteProvider].session.severity,
          },
          weekly: {
            usedPercent: panelProviders[satelliteProvider].weekly.usedPercent,
            severity: panelProviders[satelliteProvider].weekly.severity,
          },
        }
      : null,
    ```
  - `overlaySize`(544–549) 클램프에 작은 원 여유분 반영:
    ```ts
    // 작은 원은 링 상자 밖으로 우 4.3% · 하 5.5% 삐져나온다. double 모드에서
    // 매니페스트가 창 크기에 육박하면 잘리므로 미리 줄인다.
    const SATELLITE_BOUNDS_FACTOR = 1.06;
    const overlaySize = $derived(
      Math.min(
        satelliteProvider ? OVERLAY_WINDOW_PX / SATELLITE_BOUNDS_FACTOR : OVERLAY_WINDOW_PX,
        petPackage?.manifest.defaultSize.width ?? OVERLAY_WINDOW_PX,
      ),
    );
    ```
  - 기본 설정 리터럴(`src/App.svelte:82-92` 부근)에 `ringMode: 'single',` 추가.
- **MIRROR**: `src/App.svelte:517-569`
- **IMPORTS**: `secondaryProvider`를 `./lib/contracts/domain`에서 (기존 `Provider` 타입 import 줄 확인 후 값 import 추가)
- **GOTCHA**: **`primaryProvider`를 쓰는 다른 지점을 절대 건드리지 마라.** 알림/말풍선(`240-260`, `476-515`)의 `context.primary`는 "어느 provider의 이벤트가 알림을 띄우는가"이지 링 배정이 아니다. 요구사항 4가 명시적으로 이 기능들의 불변을 요구한다.
- **GOTCHA**: 펫 무드는 `primaryUi.petMood`(526–540)를 그대로 쓴다. 큰 원이 항상 primary이므로 바뀔 이유가 없다.
- **GOTCHA**: `panelProviders`(517–520)는 이미 두 provider 모두 계산해 두므로 작은 원용 별도 파생을 만들지 마라(중복 계산).
- **GOTCHA**: `satelliteProvider`는 연결 상태를 보지 않는다 — 결정 #7대로 미연결이어도 작은 원을 그리고 배지로 알린다.
- **VALIDATE**: `pnpm vitest run src/App.test.ts && pnpm check`

### Task 12: 설정 UI

- **ACTION**: `src/lib/components/SettingsPanel.svelte` 수정.
- **IMPLEMENT**: `asRingMode` 헬퍼(기존 `asProvider`/`asTheme` 옆) + Primary provider 셀렉트 **바로 아래** 배치:
  ```js
  /** @param {string} value @returns {import('../api/gateway').RingMode} */
  const asRingMode = (value) => (value === 'double' ? 'double' : 'single');
  ```
  ```svelte
  <label class="field"
    >Ring <select
      value={settings.ringMode}
      onchange={(event) =>
        onChange({ ...settings, ringMode: asRingMode(event.currentTarget.value) })}
      ><option value="single">Single</option><option value="double">Double</option
      ></select
    ></label
  >
  <p class="field-help">Double shows the second provider in a small ring.</p>
  ```
- **MIRROR**: `SETTINGS_CONTROL`, `src/lib/components/SettingsPanel.svelte:135-142`(`.field-help` 서식)
- **IMPORTS**: 없음(JSDoc 타입 참조만)
- **GOTCHA**: Primary provider 아래에 두는 것이 중요하다 — "어느 쪽이 큰 원인가"의 답이 바로 위 컨트롤이기 때문이다.
- **GOTCHA**: Appearance(theme) 셀렉트와 합치지 마라. 색상 테마와 링 갯수는 직교하며, 합치면 "다크 + 더블" 조합이 표현 불가능해진다.
- **VALIDATE**: `pnpm vitest run src/lib/components/SettingsPanel.test.ts`

### Task 13: 문서 갱신

- **ACTION**: `docs/ui-contract.md`, `CLAUDE.md` 수정.
- **IMPLEMENT**:
  - `ui-contract.md` §4.1 뒤에 **§4.4 링 모드** 신설:
    - `ring_mode: single | double`, 기본 `single`.
    - 큰 원은 **항상** `primary_provider`이며 중앙에 펫이 들어간다. 연결 상태에 따른 스왑 없음.
    - 작은 원은 `double`일 때만, `secondaryProvider(primary)`가 자동 배정되고 중앙에 provider 로고가 들어간다.
    - 작은 원 기하: 큰 원 변의 40%, 우하단 ⅓ 겹침, stroke-width 9, 5H/WK 라벨 없음.
    - 링별 상태 독립: 한쪽의 `stale`/비-`active`가 다른 쪽에 영향을 주지 않는다(§5 provider 독립성).
    - 미연결 provider도 작은 원을 유지하고 §4.2 배지로 표시한다.
    - 큰 원 접근명이 `'Provider usage:'`로 남고 작은 원만 provider명을 갖는 비대칭과 그 근거.
  - `CLAUDE.md` "Invariants" 절에 추가: *`ring_mode`는 순수 표시 선호다. 큰 링은 언제나 `primary_provider`에 묶이고 작은 링은 그 반대편에 자동 배정되며, 어느 쪽도 설정을 되쓰지 않는다. 링 모드는 수집·새로고침·알림·펫 무드 산출에 아무 영향을 주지 않는다 — 이 경계가 무너지면 표시 선호가 동작을 바꾸게 된다.*
- **MIRROR**: `docs/ui-contract.md:132-166`의 절 구성과 표 서식
- **VALIDATE**: 수동 검토

---

## Testing Strategy

### Unit Tests

| Test | Input | Expected Output | Edge Case? |
|---|---|---|---|
| `secondaryProvider('claude')` | `'claude'` | `'codex'` | |
| `secondaryProvider('codex')` | `'codex'` | `'claude'` | |
| `toSettingsStoreState`가 `ringMode` 전달 | `ringMode: 'double'`인 `AppSettings` | 반환 객체에 `ringMode: 'double'` | |
| `defaultSettings`가 Rust 기본값과 일치 | — | `ringMode === 'single'` | ✔ 기본값 계약 |

### Component Tests

| Test | 대상 | Expected |
|---|---|---|
| `single`이면 작은 원 없음 | `PetOverlay` `satellite: null` | `queryByTestId('satellite-ring')` → `null` |
| `double`이면 작은 원 렌더 | `satellite: {…active…}` | 작은 원 존재 + `getByRole('img', { name: 'Codex usage: 5-hour 41%, Weekly 58%' })` |
| 큰 원 접근명 불변 | `double` 모델 | `'Provider usage: …'` 여전히 존재 |
| 작은 원 표면이 제스처를 받음 | 작은 원 표면에 `dblClick` | `onToggle` 1회 호출 |
| 작은 원 표면이 우클릭을 가로챔 | 작은 원 표면에 `contextmenu` | `onShowMenu` 호출 + `defaultPrevented === true` |
| 작은 원 표면이 접근성 트리에 없음 | `double` 모델 | `getAllByRole('button')` 길이 1 |
| 작은 원만 stale | 큰 원 `stale:false`, 작은 원 `stale:true` | 작은 원 링 `data-stale="true"`, 큰 원 `"false"` |
| 작은 원만 오류 | 작은 원 `system:'auth_required'` | 작은 원 배지 표시 + 작은 원 링 없음 + **큰 원 아크 정상** |
| 큰 원만 오류 | 큰 원 `system:'offline'`, 작은 원 `active` | 큰 원 배지 + 작은 원 아크 정상 (독립성) |
| 작은 원에 로고 표시 | `provider: 'codex'` | `ProviderLogo`의 Codex `<svg>` 렌더 |
| Ring 셀렉트 | `SettingsPanel` | `double` 선택 시 `onChange`가 `ringMode:'double'`로 호출 |

### Rust Tests

| Test | Expected |
|---|---|
| `version_five_settings_migrate_with_ring_mode_single` | `schema_version == 6`, `ring_mode == Single`, 나머지 필드 보존 |
| 기존 v1–v4 마이그레이션 | 어서션 `schema_version == 6`으로 갱신 후 전건 통과 |
| `settings_round_trip_is_versioned_and_atomic` | `ring_mode` 왕복 보존 |

### Edge Cases Checklist

- [ ] `double` + second provider `auth_required` → 작은 원 유지, 자물쇠 배지 (결정 #7)
- [ ] `double` + primary `auth_required` → 큰 원 배지, 작은 원 정상 아크 (독립성)
- [ ] 둘 다 비-`active` → 배지 2개, 서로 겹치지 않음
- [ ] 작은 원 provider가 `unknown` 심각도 → 회색 트랙, 아크 채움 0
- [ ] 부팅 직후 `loading` → 작은 원에 스피너 배지, 큰 원 정상
- [ ] 매니페스트 `defaultSize.width`가 240 → `double`에서 약 226으로 클램프, 작은 원 안 잘림
- [ ] 말풍선 표시 중 `double` → `.overlay` `max-width: 9.5rem`(152px)이지만 링 128px이라 축소 없음
- [ ] 다크 모드 → 퍽(`--color-surface`)·로고·아크 대비 확인
- [ ] 패널에서 `double`로 변경 → 오버레이가 `settings-updated`로 즉시 반영 (재시작 불필요)
- [ ] 기존 v5 `settings.json`으로 실행 → 격리(quarantine) 없이 v6 승격, 위치·알림 설정 보존

---

## Validation Commands

### Static Analysis
```bash
pnpm check
```
EXPECT: svelte-check 오류 0

```bash
cargo clippy --manifest-path src-tauri/Cargo.toml --all-features -- -D warnings
```
EXPECT: 경고 0

### Unit Tests
```bash
pnpm vitest run src/lib/components/PetOverlay.test.ts src/lib/components/SatelliteRing.test.ts src/lib/components/SettingsPanel.test.ts src/lib/state/presentation.test.ts src/lib/stores/settings.test.ts
```
EXPECT: 전건 통과

```bash
cargo test --manifest-path src-tauri/Cargo.toml store::tests
```
EXPECT: 전건 통과 (v5 마이그레이션 포함)

### Full Suite
```bash
pnpm test:ci
```
EXPECT: svelte-check + eslint + prettier + vitest(브랜치/함수/라인/구문 80% 게이트) + vite build 전부 통과

```bash
cargo test --manifest-path src-tauri/Cargo.toml --all-features
```
EXPECT: 회귀 없음 — 특히 `collector_mode_distinguishes_*`, `window::tests`가 그대로 통과해야 한다(동작 무변경 증거)

### E2E
```bash
pnpm test:e2e:renderer
```
EXPECT: `tests/e2e/renderer.spec.ts` 통과 — 기본값이 `single`이므로 기존 오버레이 단언이 그대로 성립해야 한다

### Browser Validation
```bash
pnpm tauri dev
```
EXPECT: 설정에서 Ring을 `Double`로 바꾸면 오버레이에 작은 원이 **즉시** 나타난다(`settings-updated` 경유, 재시작 불필요). 작은 원 위에서 드래그·더블클릭·우클릭 동작. `Single`로 되돌리면 픽셀 단위로 이전 화면.

### Manual Validation
- [ ] `Single`(기본) 화면이 변경 전과 동일한지 스크린샷 대조
- [ ] `Double`에서 작은 원이 창 밖으로 잘리지 않는지 (240px 경계)
- [ ] 작은 원 로고가 48px에서 원본 PNG와 식별 가능하게 닮았는지
- [ ] 라이트/다크 양쪽에서 퍽·로고·아크 대비 확인
- [ ] 패널 토글·펫 숨김 핫키·컨텍스트 메뉴·알림이 `Double`에서도 동일 동작 (요구사항 4)
- [ ] 기존 v5 `settings.json`을 둔 채 실행 → 설정이 초기화되지 않는지

---

## Acceptance Criteria

- [ ] 전 태스크 완료
- [ ] 전 검증 명령 통과
- [ ] 신규 코드 테스트 작성 및 통과, 커버리지 게이트(80%) 유지
- [ ] 타입 오류 0 / 린트 오류 0
- [ ] `Single` 기본값에서 기존 오버레이와 시각적으로 동일
- [ ] 패널·알림·업데이트·핫키·펫 무드 코드 경로 무변경 (diff로 확인)
- [ ] `docs/ui-contract.md` §4.4 및 `CLAUDE.md` 불변식 갱신

## Completion Checklist

- [ ] 색상은 전부 `--sev-*` / `--color-*` 토큰 경유 (하드코딩 0)
- [ ] `deny_unknown_fields` 마이그레이션 아암이 `schema_version` 가드를 가짐
- [ ] 게이트웨이 매핑 양방향 갱신
- [ ] 렌더러 DTO에 자격증명·계정 식별자 없음 (프라이버시 계약 유지)
- [ ] 신규 IPC 명령 없음 → `command_allowed` 변경 불필요 (확인 완료)
- [ ] 패널 관련 파일(`UsagePanel`, `ProviderTabs`, `panelModels`) 무변경
- [ ] `tauri.conf.json` 무변경 (창 크기·CSP·번들 리소스 전부 그대로)
- [ ] 원본 PNG는 `docs/`에만 존재, 번들에 미포함
- [ ] `defaultSettings.ringMode`가 Rust `RingMode::Single`과 일치

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| **타사 상표 사용** — Claude Code·Codex 마크를 배포 바이너리에 포함 | 높음 | 높음 | 배포 전 양사 브랜드 가이드라인 확인. 불가 시 문자 마크(`C`/`X`)로 교체 — 경계가 `ProviderLogo.svelte` 하나라 교체 비용이 작다 |
| 기존 v1–v4 마이그레이션 테스트의 하드코딩된 `schema_version, 5` 누락 | 중간 | 중간 | `grep -n "schema_version, 5"`로 전수 확인 (Task 3 GOTCHA) |
| 작은 원 `:global()` 스트로크 오버라이드가 특이성에서 짐 | 중간 | 낮음 | 실패 시 `.satellite :global(.ring) path`로 구체화 (Task 9 GOTCHA) |
| 큰 원 배지와 작은 원이 우하단에서 겹침 | 중간 | 낮음 | 육안 확인 후 z-index 조정. 기존 `.badge-position`은 옮기지 않는다 (Task 9 GOTCHA) |
| "UI만" 원칙이 리팩터링 유혹으로 무너짐 | 중간 | 높음 | `App.svelte`에서 `primaryState`/`primaryUi` 파생과 알림 경로를 **읽기만** 하고 수정하지 않는다. 완료 시 `git diff`로 해당 라인 무변경 확인 (Acceptance Criteria) |
| Codex 로고 SVG 재현이 원본과 눈에 띄게 다름 | 중간 | 낮음 | 48px 나란히 대조를 Task 6의 종료 조건으로 명시. 작은 원에서는 48px로만 보이므로 세부 오차 허용폭이 크다 |
| 커버리지 80% 게이트 하락 | 낮음 | 중간 | 신규 코드가 컴포넌트 위주라 `SatelliteRing.test.ts`로 커버. `secondaryProvider`는 2줄 순수 함수 |

## Notes

- **이번 계획의 성격**: 이것은 **표시 표면 추가**이지 동작 변경이 아니다. 검증의 핵심은 "새 UI가 잘 나오는가"만큼이나 "**나머지가 그대로인가**"이며, 후자는 `cargo test --all-features` 전건 통과와 `git diff` 검토로 확인한다.
- **초안에서 제거된 것**: 이전 계획에는 연결 상태에 따라 큰 원이 secondary로 스왑되고 `double`이 `single`로 자동 강등되는 `resolveRingBinding` 정책 모듈이 있었다. 사용자 요구사항 1·3에 따라 전부 삭제했다 — 큰 원은 항상 primary, 작은 원은 항상 그 반대편이다. 그 결과 정책 모듈이 `secondaryProvider` 한 줄로 축소되어 `contracts/domain.ts`에 흡수되었고, `App.svelte`의 펫 무드 파생을 건드릴 이유도 사라졌다.
- **설정 저장 위치를 `AppSettings`로 정한 이유**: 사용자는 이를 "테마"라고 불렀지만, 기존 `theme`는 렌더러 전용 localStorage라 창 경계를 넘지 못한다(`state/theme.ts:1-5`). 링 모드는 **패널에서 고르고 오버레이에 적용**되어야 하므로 `settings-updated` 이벤트가 있는 `AppSettings` 경로가 유일하게 성립한다. 대가는 Rust 파일 2개의 필드 추가이며 로직 변경은 없다.
- **다음 계획(클릭 패널)에 넘길 델타**: 목업 §5의 패널은 현행 탭 기반 구현과 다르다. 후속 계획의 기준선으로 쓸 차이 목록:

  | 항목 | 현행 | 목업 §5 |
  |---|---|---|
  | 레이아웃 | `ProviderTabs` 탭 (한 번에 하나) | 2컬럼 grid (동시) |
  | 헤더 | `h2.visually-hidden` + ✕ | "Usage" 텍스트 + ✕ |
  | 새로고침 | `Refresh now` (선택 provider) | `Refresh both` (양쪽) |
  | primary 전환 | `Set as primary` | `Set Codex as Primary` |
  | freshness | 하단 1개 | 컬럼마다 |
  | 폭 | 312px (`tauri.conf.json`) | 380px |
  | 시스템 안내문 | `guidance` (`role="status"`) | 없음 |

- **로고 파일명 확인**: `Claudecode_Logo.png`가 주황 픽셀 마스코트, `Codex_Logo.png`가 파랑 구름 마크로 파일명과 내용이 일치한다.
