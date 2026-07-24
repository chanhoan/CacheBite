# Plan: 패널 앵커를 work area 기준 세로 중심 정렬로 교체

## Summary

펫이 화면 하단에 있을 때 사용량 패널이 작업 표시줄 아래로 잘려 일부가 보이지 않는다. 원인은 (1) 패널 위치 clamp가 `monitor.size()`(작업 표시줄 포함 전체 해상도)를 화면 경계로 쓰고, (2) 패널이 항상 `pet.y`에 상단 정렬되어 하단 펫에서 아래로 길게 뻗기 때문이다. `anchor_panel`을 세로 중심 정렬로 바꾸고 clamp 기준을 `Monitor::work_area()`로 교체한다. 부수적으로 첫 오픈 시 패널이 잘못된 크기(config 520px)로 배치됐다가 실제 높이로 튀는 문제를 layout gate로 제거한다.

## User Story

As a CacheBite 사용자,
I want 펫을 화면 어느 위치에 두든 더블클릭했을 때 패널 전체가 보이기를,
So that 위치를 옮기지 않고도 사용량을 읽을 수 있다.

## Problem → Solution

**Current state**
- `position_panel`이 `monitor.position()` / `monitor.size()`로 `Rect`를 만들어 `anchor_panel`에 넘긴다 → Windows 작업 표시줄(보통 40–48px), macOS Dock/메뉴바가 "화면 안"으로 취급된다.
- `anchor_panel`의 세로 좌표는 `pet.y` 고정(상단 정렬) → 하단 펫에서 패널이 아래로 520px 뻗고, clamp가 `모니터 하단 - 패널 높이`로 끌어올려 **패널 하단이 모니터 하단에 정확히 붙는다 = 작업 표시줄 뒤로 들어간다.**
- `show_panel`이 패널의 *현재* 창 크기(첫 오픈 시 config의 520px)로 위치를 계산한 뒤 `show()` → 이후 렌더러가 실제 높이를 `resize_panel`로 통보하며 재배치 → 첫 오픈 시 위치가 튄다.

**Desired state**
- clamp 기준이 `Monitor::work_area()` → 작업 표시줄/Dock/메뉴바를 침범하지 않는다.
- 세로는 펫 중심에 패널 중심을 맞추고(`pet.center.y - panel.height/2`) work area 안으로 clamp → 상단/중앙/하단 어디에 두어도 균형 있게 붙는다.
- 첫 오픈 시 렌더러가 실제 높이를 보고할 때까지(최대 150ms) `show()`를 보류 → 잘못된 위치가 노출되지 않는다.

## Metadata

- **Complexity**: Medium
- **Source PRD**: N/A (free-form 버그 리포트)
- **PRD Phase**: N/A
- **Estimated Files**: 4 (`window/mod.rs`, `window/tests.rs`, `refresh/ipc.rs`, `lib.rs`)

---

## UX Design

### Before

```
1920x1080 모니터, 작업 표시줄 48px (work area = 1032px)
펫: x=100, y=800, 240x240   패널: 312x520

┌────────────────────────────────┐ y=0
│                                │
│                                │
│      ┌──────────┐              │ y=560  ← clamp(monitor 1080 - 520)
│      │  panel   │              │
│      │          │              │
│      │      🐱  │              │ y=800  pet.y (상단 정렬 기준점)
│      │          │              │
├──────┤          ├──────────────┤ y=1032 work area 하단
│▓▓▓▓▓▓│  잘림!   │▓▓ 작업표시줄 ▓│
└──────┴──────────┴──────────────┘ y=1080
         ↑ 48px가 작업 표시줄 뒤로 숨음
```

### After

```
┌────────────────────────────────┐ y=0
│                                │
│      ┌──────────┐              │ y=512  ← clamp(work area 1032 - 520)
│      │  panel   │              │
│      │          │              │
│      │      🐱  │              │ y=800  pet (세로 중심 정렬 후 clamp)
│      │          │              │
│      └──────────┘              │ y=1032
├────────────────────────────────┤
│▓▓▓▓▓▓▓ 작업 표시줄 ▓▓▓▓▓▓▓▓▓▓▓▓│
└────────────────────────────────┘ y=1080
         전체 노출
```

### Interaction Changes

| Touchpoint | Before | After | Notes |
|---|---|---|---|
| 화면 중앙 펫 더블클릭 | 패널 상단이 펫 상단에 정렬 | 패널 중심이 펫 중심에 정렬 | 가로 flip 로직은 그대로 |
| 화면 하단 펫 더블클릭 | 패널 하단 일부가 작업 표시줄 뒤로 | 패널 하단이 work area 하단에 붙음 | 핵심 수정 |
| 화면 상단 펫 더블클릭 | 패널 상단 = pet.y | 중심 정렬 후 work area 상단으로 clamp | macOS 메뉴바 침범도 해소 |
| 첫 오픈 | 520px 기준 위치 → 실제 높이로 재배치(튐) | 실제 높이 확정 후 표시 | 최대 150ms 지연, 이후 fallback 표시 |
| 패널 열린 채 내용 높이 변화 | 상단 고정, 아래로 성장 | 중심 유지하며 위아래로 성장 | `resize_panel` 재배치 경로 동일 |

---

## Mandatory Reading

| Priority | File | Lines | Why |
|---|---|---|---|
| P0 | `src-tauri/src/window/mod.rs` | 354-371 | `anchor_panel` / `clamp_to_rect` — 수정 대상 핵심 함수 |
| P0 | `src-tauri/src/refresh/ipc.rs` | 239-316 | `show_panel` / `position_panel` / `resize_panel` / `hide_panel` |
| P0 | `src-tauri/src/window/mod.rs` | 1-28 | `Point` / `Size` / `Rect` / `Display` 타입 정의 |
| P1 | `src-tauri/src/window/tests.rs` | 90-116 | 기존 `panel_anchor_flips_then_clamps_inside_display` 테스트 |
| P1 | `src-tauri/src/window/mod.rs` | 245-262 | `panel_position` — 테스트 전용 헬퍼, 시그니처 의미 변경 대상 |
| P1 | `src-tauri/src/lib.rs` | 95-130 | `app.manage(...)` 등록 지점 + `invoke_handler!` |
| P2 | `src-tauri/src/refresh/ipc.rs` | 88-100 | `IpcError` 변형 + `authorize` |
| P2 | `src/App.svelte` | 403-424 | 렌더러 `ResizeObserver` → `resizePanel` 경로 |
| P2 | `src-tauri/tauri.conf.json` | `app.windows` | overlay 240x240, panel 312x520 (초기값) |

## External Documentation

| Topic | Source | Key Takeaway |
|---|---|---|
| `Monitor::work_area()` | `~/.cargo/registry/.../tauri-2.11.5/src/window/mod.rs:95-98` | `pub fn work_area(&self) -> &PhysicalRect<i32, u32>` — **이 프로젝트의 tauri 2.11.5에 존재함** |
| `PhysicalRect` 필드 | `~/.cargo/registry/.../tauri-runtime-2.11.3/src/dpi.rs:28-33` | `.position: PhysicalPosition<i32>`, `.size: PhysicalSize<u32>` — `.x`/`.y`, `.width`/`.height` 접근 |
| `PhysicalRect::default()` | 같은 파일 `:35-40` | position/size 모두 `(0,0)` — **work area를 얻지 못하는 백엔드에서 0으로 올 수 있음, 반드시 가드 필요** |

```
KEY_INSIGHT: tauri 2.11.5 Monitor에 work_area()가 이미 있어 새 의존성이나 플랫폼별 코드가 필요 없다.
APPLIES_TO: Task 2 (position_panel)
GOTCHA: PhysicalRect::default()가 전부 0이므로, work area가 0x0으로 오면 monitor.size()로 폴백하지 않으면
        패널이 (0,0)으로 clamp되어 좌상단에 처박힌다. Wayland/headless 백엔드에서 실제로 발생 가능.
```

```
KEY_INSIGHT: tokio가 "time" feature와 함께 이미 의존성에 있다 (src-tauri/Cargo.toml:32).
APPLIES_TO: Task 4 (layout gate fallback timer)
GOTCHA: 새 crate 추가 불필요. 기존 tauri::async_runtime::spawn 사용 패턴(ipc.rs:329)을 그대로 따를 것.
```

---

## Patterns to Mirror

### NAMING_CONVENTION
```rust
// SOURCE: src-tauri/src/window/mod.rs:354-371
pub fn anchor_panel(pet: Rect, panel: Size, display: Rect, gap: f64) -> Point { ... }
fn clamp_to_rect(position: Point, size: Size, bounds: Rect) -> Point { ... }
```
순수 기하 함수는 `window/mod.rs`에 `snake_case`로 두고, 부작용 없는 값 반환. `pub`은 IPC나 테스트에서 쓰는 것만.

### ERROR_HANDLING
```rust
// SOURCE: src-tauri/src/refresh/ipc.rs:239-247
#[tauri::command]
pub fn show_panel(window: tauri::WebviewWindow, app: AppHandle) -> Result<(), IpcError> {
    authorize(&window, NativeCommand::ShowPanel)?;
    let panel = app
        .get_webview_window("panel")
        .ok_or(IpcError::PanelUnavailable)?;
    position_panel(&window, &panel)?;
    panel.show().map_err(|_| IpcError::PanelUnavailable)
}
```
모든 커맨드는 `authorize` 먼저 → `Result<_, IpcError>` 반환 → 플랫폼 에러는 `map_err`로 타입화된 `IpcError`로 축약(원본 에러 문자열을 렌더러로 흘리지 않음 = 프라이버시 계약).

### GRACEFUL_DEGRADATION
```rust
// SOURCE: src-tauri/src/refresh/ipc.rs:253-258
if let (Ok(Some(monitor)), Ok(position), Ok(pet_size), Ok(panel_size)) = (
    anchor.current_monitor(),
    anchor.outer_position(),
    anchor.outer_size(),
    panel.outer_size(),
) {
    ...
}
Ok(())
```
지오메트리를 못 얻으면 조용히 통과. **headless/Wayland CI(`native-smoke.yml`)에서 `current_monitor()`가 `Ok(None)`을 반환하므로 이 관용을 반드시 유지해야 한다.** 단, 의도를 주석으로 명시하도록 개선.

### ASYNC_SPAWN
```rust
// SOURCE: src-tauri/src/refresh/ipc.rs:327-331
let app = app.clone();
tauri::async_runtime::spawn(async move {
    while states.changed().await.is_ok() { ... }
});
```
`AppHandle`을 clone해서 `tauri::async_runtime::spawn`으로 넘긴다.

### MANAGED_STATE
```rust
// SOURCE: src-tauri/src/lib.rs:100-106
app.manage(settings_repository);
app.manage(history);
app.manage(store::PetPackageRepository::new(app.path().app_data_dir()?));
app.manage(service);
app.manage(collector_mode);
```
`setup` 클로저 안에서 `app.manage(...)`, 커맨드는 `State<'_, T>` 파라미터로 수령.

### TEST_STRUCTURE
```rust
// SOURCE: src-tauri/src/window/tests.rs:90-116
#[test]
fn panel_anchor_flips_then_clamps_inside_display() {
    let bounds = Rect { x: 0.0, y: 0.0, width: 800.0, height: 600.0 };
    let pet = Rect { x: 700.0, y: 550.0, width: 80.0, height: 60.0 };
    assert_eq!(
        anchor_panel(pet, Size { width: 300.0, height: 240.0 }, bounds, 8.0),
        Point { x: 392.0, y: 360.0 }
    );
}
```
`use super::*;`로 상위 모듈 전체를 가져오고, 서술형 스네이크케이스 테스트명, 구조체 리터럴 직접 작성, `assert_eq!`로 전체 값 비교.

---

## Files to Change

| File | Action | Justification |
|---|---|---|
| `src-tauri/src/window/mod.rs` | UPDATE | `anchor_panel` 세로 중심 정렬 + 파라미터명 `display` → `work_area` + 독 주석 |
| `src-tauri/src/window/tests.rs` | UPDATE | 회귀 테스트 4건 추가 |
| `src-tauri/src/refresh/ipc.rs` | UPDATE | `work_area()` 사용, 매직넘버 상수화, layout gate 도입 |
| `src-tauri/src/lib.rs` | UPDATE | `PanelLayoutGate` managed state 등록 |

## NOT Building

- 렌더러(`src/`) 변경 없음. `gateway.ts`의 `showPanel()`/`resizePanel(height)` 시그니처와 wire DTO는 그대로 유지 — Tauri가 `State`를 주입하므로 JS 쪽 invoke 인자는 불변.
- `Display` 구조체에 `work_area` 필드 추가 안 함. `panel_position`/`PlatformWindowAdapter` 경로는 `FakePlatform`(tests.rs:291)만 구현하는 테스트 전용 스캐폴딩이라 프로덕션 경로(`position_panel`)와 무관.
- 패널 드래그 추종(패널이 열린 채 펫을 옮길 때 따라가기) 구현 안 함 — 별개 기능.
- 다중 모니터 선택 알고리즘 변경 안 함 (`current_monitor()` 유지, `nearest_display()`로 교체하지 않음).
- 패널 폭(312) 변경, 애니메이션/트랜지션 추가 안 함.
- `tauri.conf.json`의 panel 초기 크기 변경 안 함.

---

## Step-by-Step Tasks

### Task 1: `anchor_panel`을 세로 중심 정렬 + work area 기준으로 변경

- **ACTION**: `src-tauri/src/window/mod.rs:354-362`의 `anchor_panel` 본문과 시그니처 파라미터명 수정.
- **IMPLEMENT**:
  ```rust
  /// 패널을 펫 옆에 붙인다.
  ///
  /// 가로: 기본은 펫 오른쪽(`gap` 만큼 띄움). work area 오른쪽 밖으로 나가면 왼쪽으로 flip.
  /// 세로: 패널 중심을 펫 중심에 맞춘다. 상단 정렬은 하단 펫에서 패널을 화면 밖으로
  ///       밀어내므로 쓰지 않는다.
  ///
  /// `work_area`는 모니터 전체 해상도가 아니라 작업 표시줄/Dock/메뉴바를 제외한
  /// 사용 가능 영역이어야 한다. 마지막에 그 안으로 clamp한다.
  pub fn anchor_panel(pet: Rect, panel: Size, work_area: Rect, gap: f64) -> Point {
      let right = pet.x + pet.width + gap;
      let x = if right + panel.width <= work_area.x + work_area.width {
          right
      } else {
          pet.x - gap - panel.width
      };
      let y = pet.y + (pet.height - panel.height) / 2.0;
      clamp_to_rect(Point { x, y }, panel, work_area)
  }
  ```
- **MIRROR**: NAMING_CONVENTION — 순수 함수, 부작용 없음, `clamp_to_rect` 재사용.
- **IMPORTS**: 없음 (동일 모듈 내 타입).
- **GOTCHA**: `pet.y + (pet.height - panel.height) / 2.0`는 `pet.y + pet.height/2.0 - panel.height/2.0`과 수학적으로 같지만 부동소수 반올림이 한 번 적으므로 전자를 쓴다. `clamp_to_rect`는 이미 `maximum_y.max(bounds.y)`로 "패널이 work area보다 큰 경우"를 처리하므로 별도 가드 불필요.
- **VALIDATE**: `cargo test --manifest-path src-tauri/Cargo.toml window::tests` — 기존 `panel_anchor_flips_then_clamps_inside_display`와 `panel_position` 테스트는 **값이 바뀌지 않아 그대로 통과해야 한다**(아래 Notes의 계산 참조). 통과하지 않으면 구현이 잘못된 것.

### Task 2: `position_panel`이 monitor bounds 대신 work area를 쓰도록 변경

- **ACTION**: `src-tauri/src/refresh/ipc.rs:249-288`의 `position_panel` 교체 + 파일 상단(`use` 블록 아래)에 상수 추가.
- **IMPLEMENT**:
  ```rust
  /// 패널과 펫 사이 간격 (물리 픽셀).
  const PANEL_ANCHOR_GAP: f64 = 12.0;
  /// 패널 고정 폭 (논리 픽셀). 높이만 콘텐츠에 맞춰 변한다.
  const PANEL_WIDTH_LOGICAL: f64 = 312.0;
  ```
  ```rust
  fn position_panel(
      anchor: &tauri::WebviewWindow,
      panel: &tauri::WebviewWindow,
  ) -> Result<(), IpcError> {
      // 지오메트리를 못 얻는 환경(headless / 일부 Wayland 컴포지터)에서는
      // current_monitor()가 Ok(None)을 준다. 이때 배치를 건너뛰는 것은 의도된
      // 동작이다 — 실패로 승격시키면 native-smoke의 headless 잡이 깨진다.
      let (Ok(Some(monitor)), Ok(position), Ok(pet_size), Ok(panel_size)) = (
          anchor.current_monitor(),
          anchor.outer_position(),
          anchor.outer_size(),
          panel.outer_size(),
      ) else {
          return Ok(());
      };

      let usable = usable_area(&monitor);
      let anchored = crate::window::anchor_panel(
          crate::window::Rect {
              x: f64::from(position.x),
              y: f64::from(position.y),
              width: f64::from(pet_size.width),
              height: f64::from(pet_size.height),
          },
          crate::window::Size {
              width: f64::from(panel_size.width),
              height: f64::from(panel_size.height),
          },
          usable,
          PANEL_ANCHOR_GAP,
      );
      panel
          .set_position(tauri::PhysicalPosition::new(
              anchored.x.round() as i32,
              anchored.y.round() as i32,
          ))
          .map_err(|_| IpcError::PanelUnavailable)
  }

  /// 작업 표시줄/Dock/메뉴바를 제외한 사용 가능 영역.
  ///
  /// work area를 보고하지 않는 백엔드는 PhysicalRect::default()(전부 0)를 주므로,
  /// 그 경우 모니터 전체 영역으로 폴백한다. 0x0을 그대로 쓰면 패널이 (0,0)으로
  /// clamp되어 좌상단에 처박힌다.
  fn usable_area(monitor: &tauri::Monitor) -> crate::window::Rect {
      let work_area = monitor.work_area();
      if work_area.size.width > 0 && work_area.size.height > 0 {
          return crate::window::Rect {
              x: f64::from(work_area.position.x),
              y: f64::from(work_area.position.y),
              width: f64::from(work_area.size.width),
              height: f64::from(work_area.size.height),
          };
      }
      let position = monitor.position();
      let size = monitor.size();
      crate::window::Rect {
          x: f64::from(position.x),
          y: f64::from(position.y),
          width: f64::from(size.width),
          height: f64::from(size.height),
      }
  }
  ```
- **MIRROR**: GRACEFUL_DEGRADATION(지오메트리 미확보 시 통과), ERROR_HANDLING(`map_err`로 `IpcError` 축약).
- **IMPORTS**: 추가 없음. `tauri::Monitor`는 경로 전체를 써서 참조.
- **GOTCHA**:
  - `if let ... {}` → `let ... else { return Ok(()) }`로 바꿔 중첩을 줄이되 **의미(조용한 통과)는 유지**해야 한다. `Err(IpcError::PanelUnavailable)`로 바꾸면 headless CI가 깨진다.
  - `work_area.position` / `work_area.size`는 필드(메서드 아님). `monitor.position()` / `monitor.size()`는 메서드. 혼동 주의.
  - `f64::from(u32)`와 `f64::from(i32)` 모두 유효 — `as f64` 쓰지 말 것(기존 코드 스타일이 `f64::from`).
- **VALIDATE**: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-features -- -D warnings` — 경고 0.

### Task 3: `resize_panel`의 매직넘버를 상수로 교체

- **ACTION**: `src-tauri/src/refresh/ipc.rs:304`의 `312.0`을 `PANEL_WIDTH_LOGICAL`로 교체.
- **IMPLEMENT**:
  ```rust
  panel
      .set_size(tauri::LogicalSize::new(PANEL_WIDTH_LOGICAL, height.ceil()))
      .map_err(|_| IpcError::PanelUnavailable)?;
  ```
- **MIRROR**: 코딩 스타일 규칙(매직넘버 금지). Task 2에서 이미 추가한 상수 재사용.
- **IMPORTS**: 없음.
- **GOTCHA**: `set_size`는 **논리** 픽셀, `position_panel`이 읽는 `outer_size()`는 **물리** 픽셀이다. HiDPI에서 두 값은 다르며, 이는 버그가 아니라 의도된 것 — `anchor_panel`은 전부 물리 픽셀로 계산한다. 상수 이름에 `_LOGICAL`을 남겨 이 구분을 드러낼 것.
- **VALIDATE**: `cargo test --manifest-path src-tauri/Cargo.toml --all-features`.

### Task 4: 첫 오픈 위치 튐 제거 — layout gate 도입

- **ACTION**: `src-tauri/src/refresh/ipc.rs`에 `PanelLayoutGate` 추가, `show_panel` / `resize_panel` / `hide_panel` 수정.
- **IMPLEMENT**:
  ```rust
  use std::sync::atomic::{AtomicBool, Ordering};
  use std::time::Duration;

  /// 렌더러가 실제 콘텐츠 높이를 보고할 때까지 기다리는 최대 시간.
  /// 이 안에 안 오면 config 기본 크기 그대로라도 보여준다 — 영원히
  /// 숨겨진 패널보다는 한 번 튀는 편이 낫다.
  const PANEL_LAYOUT_GRACE: Duration = Duration::from_millis(150);

  /// show_panel이 요청됐지만 아직 실제 높이를 못 받은 상태인지.
  #[derive(Default)]
  pub struct PanelLayoutGate {
      awaiting_layout: AtomicBool,
  }
  ```
  ```rust
  #[tauri::command]
  pub fn show_panel(
      window: tauri::WebviewWindow,
      app: AppHandle,
      gate: State<'_, PanelLayoutGate>,
  ) -> Result<(), IpcError> {
      authorize(&window, NativeCommand::ShowPanel)?;
      let panel = app
          .get_webview_window("panel")
          .ok_or(IpcError::PanelUnavailable)?;
      position_panel(&window, &panel)?;

      // 이미 떠 있으면 재배치만 하고 끝낸다. gate를 다시 세우면
      // 열려 있는 패널이 폴백 타이머에 의해 재표시되는 잡음이 생긴다.
      if panel.is_visible().unwrap_or(false) {
          return Ok(());
      }
      gate.awaiting_layout.store(true, Ordering::SeqCst);

      let deadline_app = app.clone();
      tauri::async_runtime::spawn(async move {
          tokio::time::sleep(PANEL_LAYOUT_GRACE).await;
          let gate = deadline_app.state::<PanelLayoutGate>();
          if gate.awaiting_layout.swap(false, Ordering::SeqCst) {
              if let Some(panel) = deadline_app.get_webview_window("panel") {
                  let _ = panel.show();
              }
          }
      });
      Ok(())
  }
  ```
  `resize_panel`의 마지막 `position_panel(&overlay, &panel)` 호출을 다음으로 교체:
  ```rust
      position_panel(&overlay, &panel)?;
      if gate.awaiting_layout.swap(false, Ordering::SeqCst) {
          panel.show().map_err(|_| IpcError::PanelUnavailable)?;
      }
      Ok(())
  ```
  (시그니처에 `gate: State<'_, PanelLayoutGate>` 추가)

  `hide_panel`도 gate를 내려야 한다:
  ```rust
  #[tauri::command]
  pub fn hide_panel(
      window: tauri::WebviewWindow,
      gate: State<'_, PanelLayoutGate>,
  ) -> Result<(), IpcError> {
      authorize(&window, NativeCommand::HidePanel)?;
      // 대기 중인 폴백 타이머가 방금 닫은 패널을 되살리지 못하게 한다.
      gate.awaiting_layout.store(false, Ordering::SeqCst);
      window.hide().map_err(|_| IpcError::PanelUnavailable)
  }
  ```
- **MIRROR**: ASYNC_SPAWN(`app.clone()` → `tauri::async_runtime::spawn`), ERROR_HANDLING(`authorize` 먼저), MANAGED_STATE(`State<'_, T>` 수령).
- **IMPORTS**: `std::sync::atomic::{AtomicBool, Ordering}`, `std::time::Duration`. `tauri::State`는 이미 `use`되어 있는지 확인 — 다른 커맨드가 `State<'_, RefreshService>`를 쓰므로 있음.
- **GOTCHA**:
  - `show_panel`은 여전히 `position_panel`을 **먼저** 호출한다. 폴백 타이머가 발동할 때 위치가 아예 안 잡혀 있으면 안 되기 때문. 렌더러가 제때 응답하면 `resize_panel`이 정확한 크기로 한 번 더 재배치한다.
  - `AppHandle::state::<T>()`는 등록되지 않은 타입에 대해 **panic**한다. Task 5의 `app.manage(...)` 등록이 반드시 선행되어야 한다.
  - Tauri는 `State` 파라미터를 자동 주입하므로 **JS 쪽 `invoke('show_panel', {})` 인자는 변경 불필요**. `src/lib/api/gateway.ts:238`과 `gateway.test.ts:144`는 손대지 않는다.
  - `Ordering::SeqCst`를 쓴다. 성능이 문제되는 경로가 아니고 추론이 가장 쉽다.
- **VALIDATE**: `cargo test --manifest-path src-tauri/Cargo.toml --all-features` + `pnpm test` (렌더러 계약이 안 깨졌는지).

### Task 5: `PanelLayoutGate`를 managed state로 등록

- **ACTION**: `src-tauri/src/lib.rs`의 `setup` 클로저 안, 기존 `app.manage(...)` 블록(100-106행 근처)에 한 줄 추가.
- **IMPLEMENT**:
  ```rust
  app.manage(refresh::ipc::PanelLayoutGate::default());
  ```
- **MIRROR**: MANAGED_STATE — 같은 블록의 `app.manage(collector_mode);` 바로 아래에 배치.
- **IMPORTS**: 없음 (`refresh::ipc` 경로가 이미 `invoke_handler!`에서 쓰임).
- **GOTCHA**: `PanelLayoutGate`가 `pub`이어야 `lib.rs`에서 보인다. Task 4에서 `pub struct`로 선언할 것. 이걸 빠뜨리면 컴파일은 통과하지만 `show_panel` 첫 호출에서 `State` 추출 실패 → panic.
- **VALIDATE**: `cargo build --manifest-path src-tauri/Cargo.toml --all-features` — 컴파일 성공.

### Task 6: 회귀 테스트 추가

- **ACTION**: `src-tauri/src/window/tests.rs`의 `panel_anchor_flips_then_clamps_inside_display`(90-116행) 바로 뒤에 테스트 4건 추가.
- **IMPLEMENT**:
  ```rust
  /// 1920x1080 모니터 + 48px 작업 표시줄 = work area 1032px.
  /// 하단에 놓인 240x240 펫에서 312x520 패널이 작업 표시줄을 침범하면 안 된다.
  #[test]
  fn panel_anchor_keeps_panel_above_taskbar_for_bottom_pet() {
      let work_area = Rect { x: 0.0, y: 0.0, width: 1920.0, height: 1032.0 };
      let pet = Rect { x: 100.0, y: 800.0, width: 240.0, height: 240.0 };
      let panel = Size { width: 312.0, height: 520.0 };

      let anchored = anchor_panel(pet, panel, work_area, 12.0);

      assert_eq!(anchored, Point { x: 352.0, y: 512.0 });
      assert!(anchored.y + panel.height <= work_area.y + work_area.height);
  }

  /// 여유가 있으면 clamp 없이 펫 중심에 패널 중심을 맞춘다.
  /// (상단 정렬이었다면 y=400이 나온다 — 두 정책을 구분하는 테스트.)
  #[test]
  fn panel_anchor_centers_vertically_on_pet_when_space_allows() {
      let work_area = Rect { x: 0.0, y: 0.0, width: 1920.0, height: 1032.0 };
      let pet = Rect { x: 800.0, y: 400.0, width: 240.0, height: 240.0 };

      assert_eq!(
          anchor_panel(pet, Size { width: 312.0, height: 520.0 }, work_area, 12.0),
          Point { x: 1052.0, y: 260.0 }
      );
  }

  /// 상단 펫: 중심 정렬 결과가 work area 상단보다 위이므로 clamp.
  /// work area가 y=25에서 시작하는 macOS 메뉴바 상황도 함께 검증한다.
  #[test]
  fn panel_anchor_clamps_to_work_area_top_for_top_pet() {
      let work_area = Rect { x: 0.0, y: 25.0, width: 1440.0, height: 875.0 };
      let pet = Rect { x: 40.0, y: 30.0, width: 240.0, height: 240.0 };

      assert_eq!(
          anchor_panel(pet, Size { width: 312.0, height: 520.0 }, work_area, 12.0),
          Point { x: 292.0, y: 25.0 }
      );
  }

  /// 패널이 work area보다 높으면 상단에 붙인다 (하단으로 흘러넘치지 않게).
  #[test]
  fn panel_anchor_pins_oversized_panel_to_work_area_top() {
      let work_area = Rect { x: 0.0, y: 0.0, width: 1920.0, height: 1032.0 };
      let pet = Rect { x: 100.0, y: 800.0, width: 240.0, height: 240.0 };

      assert_eq!(
          anchor_panel(pet, Size { width: 312.0, height: 1200.0 }, work_area, 12.0),
          Point { x: 352.0, y: 0.0 }
      );
  }
  ```
- **MIRROR**: TEST_STRUCTURE — `use super::*;`(파일 상단에 이미 있음), 서술형 스네이크케이스명, 구조체 리터럴, `assert_eq!` 전체 값 비교.
- **IMPORTS**: 없음.
- **GOTCHA**: `Rect`/`Size`/`Point`는 `f64` 필드이므로 리터럴에 `.0`을 반드시 붙인다. 기존 테스트에 `display(...)` 헬퍼(tests.rs:3)가 있지만 그건 `Display`용이라 `anchor_panel`에는 쓰지 않는다.
- **VALIDATE**: `cargo test --manifest-path src-tauri/Cargo.toml window::tests` — 신규 4건 통과 + 기존 전부 통과.

---

## Testing Strategy

### Unit Tests

| Test | Input | Expected Output | Edge Case? |
|---|---|---|---|
| `panel_anchor_keeps_panel_above_taskbar_for_bottom_pet` | work `{0,0,1920,1032}`, pet `{100,800,240,240}`, panel `312x520`, gap 12 | `{352, 512}`, 패널 하단 ≤ 1032 | ✅ 핵심 회귀 |
| `panel_anchor_centers_vertically_on_pet_when_space_allows` | work `{0,0,1920,1032}`, pet `{800,400,240,240}` | `{1052, 260}` | 정책 구분 |
| `panel_anchor_clamps_to_work_area_top_for_top_pet` | work `{0,25,1440,875}`(macOS 메뉴바), pet `{40,30,240,240}` | `{292, 25}` | ✅ 상단 경계 |
| `panel_anchor_pins_oversized_panel_to_work_area_top` | panel 높이 1200 > work 1032 | `{352, 0}` | ✅ 오버사이즈 |
| `panel_anchor_flips_then_clamps_inside_display` (기존) | bounds `{0,0,800,600}`, pet `{700,550,80,60}` | `{392, 360}` **변경 없음** | 가로 flip 회귀 |
| `recovers_panel_position_...` (기존, tests.rs:250) | `panel_position(...)` | `{392, 360}` **변경 없음** | 어댑터 경로 회귀 |

### Edge Cases Checklist

- [x] 패널이 work area보다 큼 → 상단 고정 (Task 6 테스트 4)
- [x] work area 원점이 0이 아님 (macOS 메뉴바 / 좌측 보조 모니터 음수 좌표) → `clamp_to_rect`가 `bounds.x`/`bounds.y` 기준으로 처리
- [x] work area가 `0x0`으로 보고됨 (Wayland/headless) → `usable_area`가 monitor 전체로 폴백
- [x] `current_monitor()`가 `Ok(None)` → 배치 스킵, `show()`는 진행
- [x] 렌더러가 `resize_panel`을 영영 호출 안 함 → 150ms 폴백 타이머가 표시
- [x] 패널이 이미 열린 상태에서 `show_panel` 재호출 → 재배치만, gate 미설정
- [x] gate 대기 중 사용자가 패널을 닫음 → `hide_panel`이 gate 해제, 타이머 no-op
- [x] 동시 접근: `AtomicBool` + `SeqCst`. `swap`이 원자적이라 이중 `show()` 불가

---

## Validation Commands

### Static Analysis
```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-features -- -D warnings
```
EXPECT: 출력 없음 / 경고 0

### Unit Tests (영향 범위)
```bash
cargo test --manifest-path src-tauri/Cargo.toml window::tests
```
EXPECT: 신규 4건 포함 전부 통과

### Full Native Suite
```bash
cargo test --manifest-path src-tauri/Cargo.toml --all-features
```
EXPECT: 회귀 없음 (특히 `lib.rs`의 `collector_mode_distinguishes_...`)

### Renderer Contract
```bash
pnpm test:ci
```
EXPECT: svelte-check + eslint + prettier + vitest 80% 커버리지 + vite build 전부 통과.
`gateway.test.ts`의 `expect(invoked).toHaveBeenCalledWith('show_panel', {})`가 그대로 통과해야 한다 — 통과하지 않으면 Task 4에서 IPC 인자를 잘못 건드린 것.

### Manual Validation
```bash
pnpm tauri dev
```
- [ ] 펫을 화면 **하단**으로 드래그 → 더블클릭 → 패널 전체가 보이고 작업 표시줄에 가려지지 않음
- [ ] 펫을 화면 **상단**으로 드래그 → 더블클릭 → 패널 상단이 화면 밖으로 안 나감
- [ ] 펫을 화면 **중앙**에 → 더블클릭 → 패널 중심이 펫 중심과 대략 일치
- [ ] 펫을 화면 **우측 끝**으로 → 더블클릭 → 패널이 펫 왼쪽으로 flip (기존 동작 유지)
- [ ] **콜드 스타트 직후** 첫 더블클릭 → 패널이 잘못된 위치에서 튀지 않음
- [ ] 패널을 연 채 히스토리 탭 전환 등으로 높이 변화 → 중심 유지하며 위아래로 성장, 화면 밖 안 나감
- [ ] 패널 닫기 직후 재오픈 → 정상 표시 (gate 잔류 없음)
- [ ] (가능하면) 보조 모니터를 **주 모니터 왼쪽**에 배치(음수 좌표) 후 그 모니터에서 위 항목 재확인

---

## Acceptance Criteria

- [ ] Task 1–6 완료
- [ ] 하단 펫 더블클릭 시 패널이 작업 표시줄에 가려지지 않음
- [ ] 세로 정렬이 중심 기준으로 동작
- [ ] 첫 오픈 시 위치 튐 없음
- [ ] 가로 flip 동작 회귀 없음
- [ ] `cargo clippy -- -D warnings` 경고 0
- [ ] `pnpm test:ci` 통과 (커버리지 80% 유지)
- [ ] 렌더러 `src/` 변경 0줄

## Completion Checklist

- [ ] 순수 기하 로직은 `window/mod.rs`, 플랫폼 접근은 `refresh/ipc.rs`에 유지 (레이어 경계 준수)
- [ ] `IpcError`로 타입화된 에러만 렌더러로 나감 (프라이버시 계약)
- [ ] 매직넘버 `12.0` / `312.0` 상수화 완료
- [ ] headless 관용(`current_monitor() == Ok(None)` 시 스킵) 유지 — 주석으로 이유 명시
- [ ] 새 의존성 0개
- [ ] `docs/architecture.md` / `docs/ui-contract.md`는 패널 앵커 규칙을 문서화하고 있지 않으므로 수정 불필요 (grep 확인 완료)

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `work_area()`가 `0x0`을 반환해 패널이 좌상단에 처박힘 | Medium (Wayland/headless) | High | `usable_area`의 폴백 가드 (Task 2). 수동 검증에 Linux 항목 포함 |
| 150ms 폴백 타이머가 짧아 느린 머신에서 여전히 튐 | Medium | Low | 튐의 크기가 이전보다 작아질 뿐 회귀는 아님. 필요 시 상수만 조정 |
| `AppHandle::state::<PanelLayoutGate>()`가 등록 누락으로 panic | Low | High | Task 5가 선행 조건. `cargo build` 후 콜드 스타트 수동 확인 |
| 세로 중심 정렬이 기존 사용자에게 위치 변화로 느껴짐 | High | Low | 의도된 UX 변경 (사용자 승인 완료) |
| Task 4가 `native-smoke.yml`의 헤드리스 잡에서 타이머로 인해 불안정 | Low | Medium | 스모크는 패널을 열지 않으므로 `show_panel` 미호출 → gate는 항상 false |

## Notes

**기존 테스트가 왜 그대로 통과하는가.** `panel_anchor_flips_then_clamps_inside_display`는 bounds `{0,0,800,600}`, pet `{700,550,80,60}`, panel `300x240`이다.
- 상단 정렬: `y = 550` → clamp `max_y = 600-240 = 360` → **360**
- 중심 정렬: `y = 550 + (60-240)/2 = 460` → clamp → **360**

두 정책이 같은 값을 내므로 이 테스트는 정책을 **구분하지 못한다**. Task 6의 `panel_anchor_centers_vertically_on_pet_when_space_allows`가 구분 역할을 맡는다. 기존 테스트는 가로 flip 회귀 방지용으로 남긴다.

**`panel_position` / `PlatformWindowAdapter`는 프로덕션 미사용.** `window/mod.rs:252`의 `panel_position`은 `tests.rs:251`에서만 호출되고, `PlatformWindowAdapter` 구현체는 `FakePlatform`(tests.rs:291) 하나뿐이다. 프로덕션 패널 배치는 전적으로 `refresh/ipc.rs:position_panel` → `current_monitor()` 경로다. 이번 변경에서 두 경로의 "화면 영역" 정의가 갈린다(`Display.bounds`는 전체 모니터, `usable_area`는 work area). **기존 데드 코드이므로 이번 스코프에서 건드리지 않되, 별도로 정리하거나 `Display`에 `work_area`를 추가할지 판단이 필요하다.**

**논리/물리 픽셀.** `resize_panel`은 `LogicalSize`로 크기를 설정하고, `position_panel`은 `outer_size()`(물리)를 읽어 `PhysicalPosition`으로 배치한다. `anchor_panel` 내부는 전부 물리 픽셀로 일관되므로 문제없지만, HiDPI에서 `PANEL_WIDTH_LOGICAL`(312)과 `panel_size.width`(예: 468 @1.5x)가 다르다는 점을 헷갈리지 말 것.

**`resize_panel` 직후 `outer_size()`의 신선도.** `set_size` 후 곧바로 `outer_size()`를 읽는다. 플랫폼에 따라 리사이즈가 이벤트 루프를 거쳐 반영되면 이전 크기를 읽을 수 있다. 현재 코드도 동일한 구조이며 실사용에서 문제가 보고되지 않았으므로 이번 스코프에서 변경하지 않는다. 수동 검증에서 "높이 변화 시 화면 밖으로 안 나감" 항목이 실패하면 이 지점을 의심할 것.
