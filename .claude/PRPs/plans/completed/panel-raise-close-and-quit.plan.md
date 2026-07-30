# Plan: 클릭 패널 최상단 복귀 · 명시적 닫기(✕) · 실제 종료(Quit)

## Summary

클릭 패널이 다른 창 뒤로 밀려난 뒤 펫을 더블클릭해도 돌아오지 않는 버그를 고치고,
패널 닫기를 우측 상단 `✕`(절대 위치 오버레이)로 분리하며, 푸터의 `Quit` 버튼을 실제
프로세스 종료(`app.exit(0)`)에 연결한다. `✕`는 레이아웃 흐름에서 빠져 있어 기존 패널
치수와 측정 높이를 전혀 바꾸지 않는다.

## User Story

CacheBite 사용자로서, 패널을 열어둔 채 다른 앱을 쓰다가 펫을 더블클릭하면 패널이 다시
맨 앞으로 나오고, 패널은 우측 상단 `✕`로 닫고, CacheBite 자체는 `Quit`으로 끝내고
싶다 — 작업관리자를 열지 않고.

## Problem → Solution

| # | 현재 (Problem) | 목표 (Solution) |
| --- | --- | --- |
| 1 | 패널이 `alwaysOnTop: false`라 다른 앱을 클릭하면 뒤로 밀려남. 사용자 체감은 "사라짐". 작업표시줄 항목만 남는다. | 패널을 always-on-top으로 띄워 외부 클릭에도 계속 떠 있게 한다(의도된 설계). |
| 2 | `show_panel`이 `is_visible()==true`면 조기 반환 → 뒤에 살아있는 패널에 더블클릭해도 **무반응**. 작업표시줄로만 앞으로 올릴 수 있다. | `show_panel`이 항상 패널을 최상단으로 올리고 포커스한다. 더블클릭·작업표시줄 둘 다 동작. |
| 3 | 푸터 버튼이 `Quit` 라벨을 달고 `hidePanel()`을 호출. 실제 종료 경로(`gateway.quit()`)는 렌더러 호출 지점이 **0건**. | `✕`(우측 상단, 절대 위치) = 패널 숨김, 푸터 `Quit` = `app.exit(0)`. |

## Metadata

- **Complexity**: Medium
- **Source PRD**: N/A — GitHub issue [#29](https://github.com/chanhoan/CacheBite/issues/29) + 사용자 직접 제공 요구사항 3건
- **PRD Phase**: N/A (standalone)
- **Estimated Files**: 13
- **Related**: #30 (hide/show 전역 shortcut). 이슈 소유자 코멘트: "hide shortcut이 생기면
  동작하는 quit이 더 중요해진다 — hide가 사실상의 종료 수단이 되면 안 된다. #29가 먼저
  랜딩해야 한다."

---

## UX Design

### Before

```text
┌─ 패널 (312px, alwaysOnTop: false) ─────┐
│  [Claude ★] [Codex]                    │
│  ─────────────────────────────────     │
│  5-hour   ▓▓▓▓▓▓░░░░  74%              │
│  Weekly   ▓▓░░░░░░░░  20%              │
│  ● Fresh · captured 2 min ago          │
│  ─────────────────────────────────     │
│  [ Refresh now ][ Set as primary ]     │
│  [ Settings ]     [ Quit ]  ← hide만 함│
└────────────────────────────────────────┘

다른 앱 클릭 → 패널이 뒤로 밀려 안 보임
펫 더블클릭 → 무반응 (is_visible()==true 조기 반환)
작업표시줄 클릭 → 앞으로 나옴 (유일한 복귀 수단)
CacheBite 종료 → 작업관리자뿐
```

### After

```text
┌─ 패널 (312px, alwaysOnTop: true) ──────┐
│  [Claude ★] [Codex]              (✕)   │  ← 절대 위치, 레이어 위
│  ─────────────────────────────────     │     흐름에서 제외 → 높이 불변
│  5-hour   ▓▓▓▓▓▓░░░░  74%              │
│  Weekly   ▓▓░░░░░░░░  20%              │
│  ● Fresh · captured 2 min ago          │
│  ─────────────────────────────────     │
│  [ Refresh now ][ Set as primary ]     │
│  [ Settings ]     [ Quit ]  ← app.exit │
└────────────────────────────────────────┘

다른 앱 클릭 → 패널은 계속 떠 있음 (always-on-top)
펫 더블클릭 → 항상 최상단으로 올라옴 + 포커스
작업표시줄 클릭 → 여전히 앞으로 나옴 (기존 경로 유지)
패널 닫기 → ✕
CacheBite 종료 → Quit
```

### Interaction Changes

| Touchpoint | Before | After | Notes |
| --- | --- | --- | --- |
| 펫 더블클릭 (패널 숨김) | `show_panel` → 게이트 + grace 타이머로 표시 | 동일 + 표시 직후 포커스 | grace 경로 유지 |
| 펫 더블클릭 (패널 이미 보임) | **무반응** (조기 반환) | 최상단으로 올림 + 포커스 | 버그 #2의 핵심 |
| 다른 앱 클릭 | 패널이 뒤로 밀림 | 패널 계속 떠 있음 | Wayland에서 거부될 수 있음 → 더블클릭 복귀가 폴백 |
| 작업표시줄 항목 클릭 | 앞으로 나옴 | 변화 없음 | `skipTaskbar`를 추가하지 **않는다** |
| 패널 우측 상단 | (없음) | `✕` → `hidePanel()` | 절대 위치, `aria-label="Close usage panel"` |
| 푸터 `Quit` | `hidePanel()` | `quit()` → `app.exit(0)` | 확인 절차 없음 (사용자 결정) |
| 패널 최소화 상태 | 더블클릭 무반응 | `unminimize()` 후 포커스 | `is_visible()`은 최소화 시에도 true |

### Edge Cases for UX

- Settings 화면(`showSettings === true`)에서는 `UsagePanel`이 언마운트되므로 `✕`가 사라진다.
  기존 `← Back`으로 돌아온 뒤 `✕`를 눌러야 한다. **이 계획 범위 밖** — 아래 NOT Building 참조.
- `✕`가 Codex 탭의 우측 상단 모서리 약 **14×18px**을 덮는다 (계산은 Task 3 GOTCHA).
  Codex 탭은 140px 중 126px이 남으므로 클릭 가능. 사용자가 "우선 레이어 위에 올라간
  느낌으로"를 명시했으므로 리플로 없는 순수 오버레이를 유지한다.

---

## Mandatory Reading

| Priority | File | Lines | Why |
| --- | --- | --- | --- |
| P0 | `src-tauri/src/refresh/ipc.rs` | 269-302 | `show_panel` — 버그 #2의 조기 반환 지점 |
| P0 | `src-tauri/src/refresh/ipc.rs` | 374-418 | `resize_panel` / `hide_panel` / `quit` — 게이트와 표시 경로 |
| P0 | `src/lib/components/UsagePanel.svelte` | 전체 (201줄) | `✕` 추가 · 푸터 `Quit` 재배선 대상 |
| P0 | `src/App.svelte` | 656-672 | `UsagePanel` 배선 지점 (`onClose`는 663줄) |
| P1 | `src/lib/api/gateway.ts` | 93-99, 248-258 | `showPanel`/`hidePanel`/`quit` 계약과 인가 주석 |
| P1 | `src-tauri/src/window/mod.rs` | 56-104 | `NativeCommand` + `command_allowed` — `Quit`은 panel 전용 |
| P1 | `src/lib/styles/global.css` | 24-35 | `main.panel` — `overflow: hidden`, `border-radius: 14px` |
| P1 | `src/App.svelte` | 413-434 | `ResizeObserver` 높이 측정 — `✕`가 이걸 흔들면 안 됨 |
| P2 | `src/lib/components/ProviderTabs.svelte` | 33-48 | 탭이 `flex: 1`로 헤더 전폭 → `✕` 겹침 계산의 근거 |
| P2 | `src/lib/components/UsagePanel.test.ts` | 130-165 | 갱신해야 하는 기존 테스트 2개 |
| P2 | `src/App.test.ts` | 76-153, 960-968 | `fixture()` 헬퍼와 갱신 대상 테스트 |
| P2 | `src-tauri/src/window/tests.rs` | 1-2, 315-343 | `use super::*` + 인가 테스트 패턴 |
| P2 | `docs/ui-contract.md` | 166-198 | §5 클릭 패널 정보 구조 — ASCII와 규칙 갱신 |
| P2 | `tests/e2e/renderer.spec.ts` | 150-190 | 패널 셸 레이아웃 e2e 어서션 패턴 |

## External Documentation

| Topic | Source | Key Takeaway |
| --- | --- | --- |
| `WebviewWindow` 창 제어 | Tauri 2 `tauri::WebviewWindow` | `show()`, `set_focus()`, `is_minimized()`, `unminimize()` 모두 `tauri::Result<T>` 반환. `set_focus()`가 래이즈까지 담당한다. |
| `alwaysOnTop` 창 설정 | Tauri 2 window config schema | `app.windows[].alwaysOnTop: boolean`. 이 저장소의 `overlay` 창이 이미 사용 중 (`tauri.conf.json:29`) — 새 문법 도입 아님. |

> 이 변경에 새로운 크레이트·패키지·외부 API는 없다. 필요한 IPC 명령(`quit`), 인가
> 정책, 게이트 상태 관리 모두 이미 존재한다. 추가 리서치 불필요.

---

## Patterns to Mirror

### NAMING_CONVENTION — Rust: snake_case 함수 + 의도를 적는 doc comment

```rust
// SOURCE: src-tauri/src/refresh/ipc.rs:304-318
pub(crate) fn position_panel(
    anchor: &tauri::WebviewWindow,
    panel: &tauri::WebviewWindow,
) -> Result<(), IpcError> {
    // Headless runners and some Wayland compositors report no monitor. Skipping
    // placement there is deliberate — promoting it to an error would break the
    // fixture jobs in native-smoke.yml.
    let (Ok(Some(monitor)), Ok(position), Ok(pet_size), Ok(panel_size)) = (
```

**핵심**: 헤드리스/Wayland에서 실패하는 창 조작은 **에러로 승격하지 않는다**. `reveal_panel`의
`set_focus()`도 이 선례를 따라 best-effort로 처리한다 (Task 2).

### ERROR_HANDLING — 타입 에러 + `map_err`로 IpcError 매핑

```rust
// SOURCE: src-tauri/src/refresh/ipc.rs:385-398
    let panel = app
        .get_webview_window("panel")
        .ok_or(IpcError::PanelUnavailable)?;
    panel
        .set_size(tauri::LogicalSize::new(PANEL_WIDTH_LOGICAL, height.ceil()))
        .map_err(|_| IpcError::PanelUnavailable)?;
```

`IpcError` 변주는 `Forbidden | ServiceUnavailable | InvalidSettings | InvalidPanelSize |
PersistenceUnavailable | PanelUnavailable` (`ipc.rs:108-115`). **새 변주를 추가하지 않는다** —
창 조작 실패는 전부 `PanelUnavailable`.

### AUTHORIZATION — 모든 커맨드 첫 줄에서 창 라벨 인가

```rust
// SOURCE: src-tauri/src/refresh/ipc.rs:117-121, 413-418
fn authorize(window: &tauri::WebviewWindow, command: NativeCommand) -> Result<(), IpcError> {
    command_allowed(window.label(), command)
        .then_some(())
        .ok_or(IpcError::Forbidden)
}

#[tauri::command]
pub fn quit(window: tauri::WebviewWindow, app: AppHandle) -> Result<(), IpcError> {
    authorize(&window, NativeCommand::Quit)?;
    app.exit(0);
    Ok(())
}
```

**핵심**: `Quit`은 `panel` 창에서만 인가된다 (`window/mod.rs:100`, 테스트 `window/tests.rs:335,338`).
`Quit` 버튼은 반드시 `UsagePanel`(panel 창)에만 둔다 — overlay에서 호출하면 `Forbidden`.

### PURE_POLICY_FUNCTION — 창 정책은 순수 함수로 뽑아 `window/`에서 테스트

```rust
// SOURCE: src-tauri/src/window/mod.rs:151-162
pub fn apply_fullscreen(state: &RuntimeState, fullscreen: bool) -> RuntimeState {
    RuntimeState {
        fullscreen,
        overlay_visible: !fullscreen,
        panel_visible: if fullscreen {
            false
        } else {
            state.panel_visible
        },
        ..state.clone()
    }
}
```

### RUST_TEST_STRUCTURE — `use super::*`, 서술형 이름, 관련 단정을 한 테스트에 묶음

```rust
// SOURCE: src-tauri/src/window/tests.rs:1, 315-326
use super::*;

#[test]
fn native_commands_are_authorized_by_window_label() {
    assert!(command_allowed("overlay", NativeCommand::GetCollectorMode));
    assert!(command_allowed("overlay", NativeCommand::ShowPanel));
    assert!(!command_allowed("overlay", NativeCommand::ResizePanel));
    // The pet picker lives in the panel; the overlay has no use for the list.
    assert!(!command_allowed("overlay", NativeCommand::ListPetPackages));
    assert!(command_allowed("panel", NativeCommand::ListPetPackages));
```

### SVELTE_COMPONENT_PROPS — JSDoc 타입 + 기본값 no-op 콜백

```svelte
<!-- SOURCE: src/lib/components/UsagePanel.svelte:6-19 -->
  /** @typedef {import('./panelModels').PanelProviderModel} PanelProvider */
  /** @type {{ providers: ...; onSettings?: () => void; onClose?: () => void }} */
  let {
    providers,
    selected,
    primary = selected,
    refreshing,
    nowMs = Date.now(),
    onRefresh = () => {},
    onSelect = () => {},
    onPrimary = () => {},
    onSettings = () => {},
    onClose = () => {},
  } = $props();
```

**핵심**: 이 컴포넌트는 `.svelte` + JSDoc(`lang="ts"` 아님). `onQuit`도 동일 형식으로 추가.

### SVELTE_BUTTON_STYLING — 시맨틱 토큰 기반, ghost/위험 변주

```svelte
<!-- SOURCE: src/lib/components/UsagePanel.svelte:179-193 -->
  .ghost-action {
    min-height: 1.875rem;
    border: 1px solid transparent;
    background: transparent;
    color: var(--color-text-muted);
    font-weight: 500;
  }
  .ghost-action:hover,
  .ghost-action:focus-visible {
    color: var(--color-text);
  }
  .ghost-action.quit:hover,
  .ghost-action.quit:focus-visible {
    color: var(--sev-exhausted);
  }
```

보조 선례 — transparent에서 hover 시 `border-color`/`color`를 승격시키는 패턴:

```svelte
<!-- SOURCE: src/App.svelte:736-751 -->
  .settings-back {
    justify-self: start;
    min-height: 2rem;
    padding: 0 var(--space-3);
    border: 1px solid transparent;
    border-radius: 0.5rem;
    background: transparent;
    color: var(--color-text-muted);
    font: inherit;
    font-weight: 500;
    cursor: pointer;
  }
  .settings-back:hover,
  .settings-back:focus-visible {
    border-color: var(--color-border);
    color: var(--color-text);
  }
```

### GATEWAY_METHOD — 얇은 `invokeNative` 위임 + 인가 범위 JSDoc

```typescript
// SOURCE: src/lib/api/gateway.ts:96-99, 257-258
  /** Authorized for the `panel` window only (`window::command_allowed`). */
  hidePanel(): Promise<void>;
  /** Authorized for the `panel` window only (`window::command_allowed`). */
  quit(): Promise<void>;
// ...
  hidePanel: () => invokeNative('hide_panel'),
  quit: () => invokeNative('quit'),
```

**핵심**: `quit`은 게이트웨이·픽스처·인가·Rust 테스트까지 **이미 완비**되어 있다. 이 계획에서
게이트웨이 인터페이스 변경은 **없다**.

### RENDERER_TEST_STRUCTURE — Testing Library, 접근성 이름으로 조회

```typescript
// SOURCE: src/lib/components/UsagePanel.test.ts:130-145
  it('closes the panel when Quit is pressed rather than acting on the window itself', async () => {
    const onClose = vi.fn();
    render(UsagePanel, {
      props: {
        providers: bothProviders('active'),
        selected: 'claude',
        primary: 'claude',
        refreshing: false,
        nowMs: NOW,
        onClose,
      },
    });

    await fireEvent.click(screen.getByRole('button', { name: 'Quit' }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });
```

### APP_TEST_STRUCTURE — 창 라벨은 query string으로, 게이트웨이는 vi.fn 스파이

```typescript
// SOURCE: src/App.test.ts:960-968
  it('closes the panel through the Quit button without quitting CacheBite', async () => {
    window.history.replaceState({}, '', '/?window=panel');
    const { gateway } = fixture();
    render(App, { props: { gateway, notificationAdapter: notifications } });

    await fireEvent.click(await screen.findByRole('button', { name: 'Quit' }));
    expect(gateway.hidePanel).toHaveBeenCalledOnce();
    expect(gateway.quit).not.toHaveBeenCalled();
  });
```

### GATEWAY_TEST_STRUCTURE — `mockIPC`로 커맨드 이름·인자 검증

```typescript
// SOURCE: src/lib/api/gateway.test.ts:138-145
    await tauriGateway.refreshProvider('codex');
    await tauriGateway.showPanel();
    await tauriGateway.hidePanel();
    expect(invoked).toHaveBeenCalledWith('refresh_provider', {
      provider: 'codex',
    });
    expect(invoked).toHaveBeenCalledWith('show_panel', {});
    expect(invoked).toHaveBeenCalledWith('hide_panel', {});
```

### E2E_LAYOUT_ASSERTION — `browser.execute`로 계산된 스타일·박스 수집

```typescript
// SOURCE: tests/e2e/renderer.spec.ts:154-172
    const layout = await browser.execute(() => {
      const panel = document.querySelector<HTMLElement>('main.panel');
      const header = panel?.querySelector<HTMLElement>('.usage-panel > header');
      const body = panel?.querySelector<HTMLElement>('.usage-panel > .body');
      const footer = panel?.querySelector<HTMLElement>('.usage-panel > footer');
      if (!panel || !header || !body || !footer) {
        throw new Error('panel layout missing');
      }
      const shellStyle = getComputedStyle(panel);
      return {
        outerWidth: panel.getBoundingClientRect().width,
        outerHeight: panel.getBoundingClientRect().height,
```

---

## Files to Change

| File | Action | Justification |
| --- | --- | --- |
| `src-tauri/src/window/mod.rs` | UPDATE | `PanelReveal` enum + `panel_reveal()` 순수 정책 함수 추가 |
| `src-tauri/src/window/tests.rs` | UPDATE | `panel_reveal` 결정 테스트 추가 |
| `src-tauri/src/refresh/ipc.rs` | UPDATE | `reveal_panel()` 헬퍼 추가, `show_panel`/`resize_panel`이 이를 사용 |
| `src-tauri/tauri.conf.json` | UPDATE | `panel` 창에 `"alwaysOnTop": true` |
| `src/lib/components/UsagePanel.svelte` | UPDATE | `✕` 절대 위치 버튼 추가, 푸터 `Quit`을 `onQuit`으로 재배선, `onQuit` prop 추가 |
| `src/App.svelte` | UPDATE | `onQuit={() => void gateway.quit()}` 전달 (663줄 인근) |
| `src/lib/components/UsagePanel.test.ts` | UPDATE | 기존 2개 테스트 갱신 (`✕`/`Quit` 분리) |
| `src/App.test.ts` | UPDATE | 960줄 테스트 갱신 + `Quit` → `quit()` 테스트 추가 |
| `src/lib/api/gateway.test.ts` | UPDATE | `quit` invoke 커맨드 이름 어서션 추가 (현재 미검증) |
| `tests/e2e/renderer.spec.ts` | UPDATE | `✕`가 흐름에서 제외되어 셸 레이아웃을 바꾸지 않음을 검증 |
| `docs/ui-contract.md` | UPDATE | §5 ASCII·규칙에 `✕`, 항상 위, 더블클릭 래이즈 반영 |
| `docs/beta-testing.md` | UPDATE | First run에 닫기/종료 방법 추가, 77-78줄의 거짓 트레이 서술 정정 |
| `CLAUDE.md` | UPDATE | Invariants에 패널 표시 정책 1줄 추가 |

## NOT Building

- **시스템 트레이 아이콘.** 이 저장소에는 트레이 코드가 전혀 없고(`grep`으로 확인),
  `tauri` 크레이트의 `tray-icon` feature도 비활성이다. 사용자가 언급한 "시스템 트레이"는
  패널 창의 **작업표시줄 항목**(`panel` 창에 `skipTaskbar`가 없어 생김)이다. 트레이 추가는
  별도 이슈로 분리한다.
- **패널 `skipTaskbar` 추가.** 작업표시줄 항목이 현재 유일하게 동작하는 복귀 수단이고,
  사용자가 "더블클릭을 하던 시스템 트레이를 누르던" 둘 다 되기를 요구했다. 유지한다.
- **포커스 상실(blur)·외부 클릭·Escape 기반 자동 닫기.** 사용자가 "외부 클릭이 와도 떠있게
  하는건 의도된 설계"라고 확인했다. 패널은 `✕`로만 닫힌다.
- **종료 확인 대화상자.** 사용자가 "확인 없이 즉시 종료"를 선택했다.
- **Settings 화면의 `✕`.** `showSettings === true`면 `UsagePanel`이 언마운트되어 `✕`가
  사라진다. 기존 `← Back` 경로가 있으므로 이번 범위에서 제외한다.
- **`✕` 추가에 따른 헤더 `padding-right` 보정.** 사용자가 리플로 없는 순수 오버레이를
  명시적으로 요구했다("절대좌표로 위치하는 것처럼, 다른 컴포넌트의 레이어 위에").
  탭 히트 영역 겹침은 감수하고 기록한다.
- **전역 hide/show shortcut** (#30). 별도 이슈.
- **`gateway.ts` / `fixtureGateway.ts` 변경.** `quit()`은 이미 두 곳 모두에 존재한다.

---

## Step-by-Step Tasks

### Task 1: `panel_reveal` 순수 정책 함수 + 테스트

- **ACTION**: `src-tauri/src/window/mod.rs`에 `PanelReveal` enum과 `panel_reveal()`을 추가하고,
  `src-tauri/src/window/tests.rs`에 결정 테스트를 추가한다.
- **IMPLEMENT**: `mod.rs`의 `command_allowed` 함수 **바로 아래**(104줄 이후, `PlatformWindowAdapter`
  trait 앞)에 삽입한다.

  ```rust
  /// What `show_panel` must do for a panel in the given visibility state.
  #[derive(Clone, Copy, Debug, Eq, PartialEq)]
  pub enum PanelReveal {
      /// Already on screen: raise and focus it, leaving the layout gate alone —
      /// the height the renderer last reported still applies.
      RaiseExisting,
      /// Off screen: arm the layout gate so the reveal waits for the renderer to
      /// measure its content height.
      AwaitLayout,
  }

  /// A visible panel is raised, never skipped.
  ///
  /// The panel is not modal and does not follow focus, so it can sit behind
  /// another window while still reporting `is_visible() == true`. Returning
  /// early on that state is what made a double-click on the pet a no-op, with no
  /// way back other than the taskbar entry.
  pub fn panel_reveal(visible: bool) -> PanelReveal {
      if visible {
          PanelReveal::RaiseExisting
      } else {
          PanelReveal::AwaitLayout
      }
  }
  ```

  `tests.rs`의 `native_commands_are_authorized_by_window_label`(315-343줄) **뒤**에 추가:

  ```rust
  #[test]
  fn a_visible_panel_is_raised_rather_than_left_behind_another_window() {
      assert_eq!(panel_reveal(true), PanelReveal::RaiseExisting);
      assert_eq!(panel_reveal(false), PanelReveal::AwaitLayout);
  }
  ```

- **MIRROR**: PURE_POLICY_FUNCTION (`window/mod.rs:151-162` `apply_fullscreen`),
  RUST_TEST_STRUCTURE (`window/tests.rs:1,315`).
- **IMPORTS**: 없음. `mod.rs`는 `serde`만 쓰고 이 코드는 std만 필요하다. `tests.rs`는
  `use super::*`(1줄)로 자동 노출된다.
- **GOTCHA**: 이 함수는 **결정만** 고정한다. 실제 래이즈가 동작한다는 증거가 아니다.
  이 저장소에 정반대 실패 사례가 기록되어 있다 —
  `.claude/PRPs/plans/completed/stabilization-and-quality-pass.plan.md:528`: *"순수 함수는
  만들고 배선은 잊었다 … 단위 테스트가 순수 함수를 통과시키므로 테스트 그린이 기능 동작을
  보증하지 못했다."* 따라서 Task 2의 배선과 Manual Validation 체크리스트가 필수다.
- **VALIDATE**:
  ```bash
  cargo test --manifest-path src-tauri/Cargo.toml window::tests
  ```
  EXPECT: `a_visible_panel_is_raised_rather_than_left_behind_another_window` 통과.

---

### Task 2: `reveal_panel` 헬퍼 + `show_panel` 항상 최상단 (버그 #2)

- **ACTION**: `src-tauri/src/refresh/ipc.rs`에 `reveal_panel()`을 추가하고, `show_panel`의
  조기 반환을 래이즈로 바꾸고, 패널을 표시하는 나머지 두 지점(grace 타이머, `resize_panel`)도
  이 헬퍼로 통일한다.
- **IMPLEMENT**:

  1. `use crate::{…}` 블록(8-15줄)의 `window::{…}`를 확장:

  ```rust
  use crate::{
      domain::{FailureClass, Provider, ProviderUsageSnapshot, UnavailableReason},
      store::{
          HistoryRepository, HistoryStore, LogicalPosition, PetPackage, PetPackageRepository,
          PetSummary, Settings, SettingsRepository,
      },
      window::{
          command_allowed, panel_reveal, CapabilityDiagnostic, NativeCommand, PanelReveal,
          PlatformCapabilities,
      },
  };
  ```

  2. `show_panel` **바로 위**(269줄 `#[tauri::command]` 앞)에 헬퍼를 추가:

  ```rust
  /// Brings the panel to the front of the window stack.
  ///
  /// Every path that reveals the panel goes through here so a double-click on
  /// the pet always has a visible effect.
  fn reveal_panel(panel: &tauri::WebviewWindow) -> Result<(), IpcError> {
      // A minimised panel still reports `is_visible() == true`, so restoring it
      // has to happen before the raise or the focus lands on nothing.
      if panel.is_minimized().unwrap_or(false) {
          let _ = panel.unminimize();
      }
      panel.show().map_err(|_| IpcError::PanelUnavailable)?;
      // Best-effort, mirroring `position_panel`: headless runners and restrictive
      // Wayland compositors refuse focus changes, and promoting that to an error
      // would fail the fixture jobs in native-smoke.yml for a panel that is in
      // fact on screen.
      let _ = panel.set_focus();
      Ok(())
  }
  ```

  3. `show_panel`의 조기 반환(283-286줄)을 교체:

  ```rust
      // Already on screen: the panel is not modal and does not follow focus, so
      // it may be buried behind another window. Raise it instead of returning
      // without a visible effect.
      match panel_reveal(panel.is_visible().unwrap_or(false)) {
          PanelReveal::RaiseExisting => return reveal_panel(&panel),
          PanelReveal::AwaitLayout => {}
      }
      gate.awaiting_layout.store(true, Ordering::SeqCst);
  ```

  4. grace 타이머(296-298줄)의 `panel.show()`를 교체:

  ```rust
              if let Some(panel) = deadline_app.get_webview_window("panel") {
                  let _ = reveal_panel(&panel);
              }
  ```

  5. `resize_panel`의 게이트 플립(395-398줄)을 교체:

  ```rust
      // The measurement this resize carries is what show_panel was waiting for.
      if gate.awaiting_layout.swap(false, Ordering::SeqCst) {
          reveal_panel(&panel)?;
      }
  ```

- **MIRROR**: NAMING_CONVENTION (`ipc.rs:304-318` `position_panel` — 헤드리스에서 에러 승격 금지),
  ERROR_HANDLING (`ipc.rs:385-398` `map_err(|_| IpcError::PanelUnavailable)`).
- **IMPORTS**: `window::{panel_reveal, PanelReveal}` 추가. `tauri::Manager`는 이미 5줄에 있어
  `get_webview_window`가 동작한다. `is_minimized`/`unminimize`/`set_focus`/`show`는
  `tauri::WebviewWindow`의 고유 메서드로 추가 import가 필요 없다.
- **GOTCHA**:
  - `set_focus()`를 `?`로 전파하면 **안 된다**. `native-smoke.yml`의 headless Wayland/X11 fixture
    잡에서 포커스 변경이 거부될 수 있고, 그러면 화면에 떠 있는 패널에 대해 `show_panel`이
    실패하며 스모크가 깨진다. `position_panel`의 선례를 그대로 따른다.
  - `position_panel(&window, &panel)`은 가시성 검사 **앞**(281줄)에서 이미 호출되므로 손대지
    않는다. 이것이 멀티 스크린 요구를 충족한다 — 펫이 다른 디스플레이로 옮겨간 뒤
    더블클릭하면 래이즈 전에 그 디스플레이 기준으로 재앵커된다.
  - `resize_panel`에서는 `reveal_panel(&panel)?`로 **전파해도 된다** — 이 경로는 렌더러가
    호출하고, `IpcError`는 렌더러의 `.catch()`(`App.svelte:426-429`)가 받아 `trace`로 남긴다.
  - `unminimize()` 실패도 무시한다. 최소화 버튼이 없는 창(`decorations: false`)이라 정상
    경로에서는 발생하지 않고, 실패해도 뒤따르는 `show()`가 여전히 의미 있다.
  - `match`의 `AwaitLayout => {}` 팔을 `_ => {}`로 쓰지 않는다 — `PanelReveal`은 창 정책
    enum이므로 exhaustive match를 유지한다 (rules: "business-critical enum에 와일드카드 금지").
- **VALIDATE**:
  ```bash
  cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
  cargo clippy --manifest-path src-tauri/Cargo.toml --all-features -- -D warnings
  cargo test --manifest-path src-tauri/Cargo.toml --all-features
  ```
  EXPECT: fmt/clippy 무경고, 전체 테스트 통과.

---

### Task 3: 패널 우측 상단 `✕` — 절대 위치 오버레이 (요구 #3)

- **ACTION**: `src/lib/components/UsagePanel.svelte`에 `✕` 버튼을 추가한다. 레이아웃 흐름에서
  제외되어 헤더/본문/푸터 치수와 셸 측정 높이를 **전혀** 바꾸지 않아야 한다.
- **IMPLEMENT**:

  1. `<section class="usage-panel">`(27줄)의 **첫 자식**으로 버튼을 넣는다 (`<h2>` 앞):

  ```svelte
  <section class="usage-panel" aria-label="Usage panel">
    <button
      class="close-panel"
      type="button"
      aria-label="Close usage panel"
      onclick={() => onClose()}>×</button
    >
    <h2 class="visually-hidden">Usage panel</h2>
  ```

  2. `<style>` 블록을 수정한다. `.usage-panel` 규칙(96-99줄)에 `position: relative`를 더하는
     것이 핵심 — 이것이 절대 위치의 컨테이닝 블록을 만든다.

  ```css
    .usage-panel {
      position: relative;
      width: 100%;
      color: var(--color-text);
    }
    /* Out of flow on purpose: the close control layers over the header instead of
       reserving a column, so adding it leaves every existing box — and the height
       the ResizeObserver reports to `resize_panel` — untouched. */
    .close-panel {
      position: absolute;
      z-index: 2;
      top: 0.375rem;
      right: 0.375rem;
      display: grid;
      width: 1.5rem;
      height: 1.5rem;
      min-height: 0;
      place-items: center;
      padding: 0;
      border: 1px solid transparent;
      border-radius: 0.375rem;
      background: transparent;
      color: var(--color-text-faint);
      font-size: 1rem;
      font-weight: 500;
      line-height: 1;
    }
    .close-panel:hover,
    .close-panel:focus-visible {
      border-color: var(--color-border);
      background: var(--color-surface-sunken);
      color: var(--color-text);
    }
  ```

- **MIRROR**: SVELTE_BUTTON_STYLING — `UsagePanel.svelte:179-193`의 ghost 버튼 토큰 사용과
  `App.svelte:736-751` `.settings-back`의 transparent → hover 시 `border-color`/`color` 승격 패턴.
- **IMPORTS**: 없음. `onClose`는 이미 선언된 prop(18줄)이며 시그니처 변경이 없다.
- **GOTCHA**:
  - `button { min-height: 2.25rem }`(147-153줄)이 이 요소에도 적용된다 — 반드시
    `min-height: 0`으로 덮어써야 24×24px이 나온다. 같은 규칙의 `cursor: pointer`,
    `font: inherit`, `border-radius`는 상속받아도 무해하므로 재선언하지 않는다.
  - **탭 겹침(정량)**: 패널 폭 312px, 헤더 padding `var(--space-3) var(--space-4) 0`
    = `0.75rem 1rem 0` → 탭 영역 x=16~296. `ProviderTabs`의 버튼은 `flex: 1`이라
    Claude 16~156 / Codex 156~296. `✕`는 `right: 0.375rem`(6px) + 24px 폭 → x=282~306.
    세로로 `top: 6px`~30px, 탭 행은 y≈12~52px. 따라서 **Codex 탭 우측 상단 약 14×18px**이
    덮인다. Codex 탭은 140px 중 126px이 클릭 가능하게 남는다. 사용자가 리플로 없는 오버레이를
    명시적으로 요구했으므로 이대로 진행하고, 실사용에서 거슬리면 헤더 `padding-right` 보정을
    후속 조정으로 남긴다.
  - `main.panel`은 `overflow: hidden` + `border-radius: 14px`(`global.css:28-30`)이다.
    6px 인셋이면 라운드 코너에 잘리지 않는다. 인셋을 0으로 줄이면 잘린다.
  - `z-index: 2`인 이유: `ProviderTabs`는 z-index를 쓰지 않지만 `PetOverlay`의
    `.interaction-surface`가 `z-index: 1`(`PetOverlay.svelte:84`)을 쓴다. 같은 스택 컨텍스트가
    아니지만 관례를 맞춰 1보다 크게 둔다.
  - `aria-label="Close usage panel"`은 **의도적으로** 기존 테스트가 부재를 단정하던 바로 그
    이름이다 (`UsagePanel.test.ts:161`). Task 5에서 그 단정을 뒤집는다.
  - `×`는 U+00D7 MULTIPLICATION SIGN(문자 `x`나 U+2715가 아님) — 시각적 균형이 맞고
    `aria-label`이 접근성 이름을 담당한다.
  - `type="button"`을 명시한다. 이 컴포넌트에 `<form>`은 없지만 기존 버튼들과 달리 새로 추가하는
    컨트롤이므로 기본 `submit` 동작 가능성을 원천 차단한다.
- **VALIDATE**:
  ```bash
  pnpm check
  pnpm lint
  ```
  EXPECT: svelte-check 0 에러, eslint/prettier 통과. (동작 검증은 Task 5.)

---

### Task 4: 푸터 `Quit`을 실제 종료로 배선 (요구 #3)

- **ACTION**: `UsagePanel`에 `onQuit` prop을 추가해 푸터 `Quit`에 연결하고, `App.svelte`에서
  `gateway.quit()`을 전달한다. `onClose`는 `✕` 전용으로 남는다.
- **IMPLEMENT**:

  1. `UsagePanel.svelte` JSDoc 타입(7줄) 끝에 `onQuit`을 추가:

  ```javascript
    /** @type {{ providers: { claude: PanelProvider; codex: PanelProvider }; selected: import('../contracts/domain').Provider; primary?: import('../contracts/domain').Provider; refreshing: boolean; nowMs?: number; onRefresh?: (provider: import('../contracts/domain').Provider) => void; onSelect?: (provider: import('../contracts/domain').Provider) => void; onPrimary?: (provider: import('../contracts/domain').Provider) => void; onSettings?: () => void; onClose?: () => void; onQuit?: () => void }} */
  ```

  2. 구조 분해(18줄 뒤)에 기본값을 추가:

  ```javascript
      onClose = () => {},
      onQuit = () => {},
    } = $props();
  ```

  3. 푸터 버튼(90줄)의 핸들러를 교체:

  ```svelte
        <button class="ghost-action quit" onclick={() => onQuit()}>Quit</button>
  ```

  4. `src/App.svelte`의 `UsagePanel` 배선(663줄 인근)에 한 줄 추가:

  ```svelte
          onClose={() => void gateway.hidePanel()}
          onQuit={() => void gateway.quit()}
          onSettings={() => (showSettings = true)}
  ```

- **MIRROR**: SVELTE_COMPONENT_PROPS (`UsagePanel.svelte:6-19`),
  AUTHORIZATION (`ipc.rs:413-418` — `Quit`은 panel 전용).
- **IMPORTS**: 없음. `gateway.quit()`은 `AppGateway`(`gateway.ts:99`)에 이미 있고
  `tauriGateway`(`:258`)·`rendererFixtureGateway`(`fixtureGateway.ts:115`)·`App.test.ts`의
  `fixture()`(`:145`)에 모두 구현돼 있다.
- **GOTCHA**:
  - `Quit`은 `panel` 창에서만 인가된다(`window/mod.rs:100`). `UsagePanel`은 panel 창에서만
    마운트되므로(`App.svelte:640`) 안전하다. 이 버튼을 overlay 쪽으로 옮기면 `IpcError::Forbidden`.
  - `void`로 프로미스를 버리는 것은 이 파일의 확립된 관례다(`App.svelte:663`, `:701`).
    `gateway.quit()`은 성공 시 프로세스가 사라지므로 `.catch()`를 붙일 의미가 없다.
  - `.ghost-action.quit` 스타일(190-193줄)은 hover 시 `--sev-exhausted`(위험색)로 바뀐다 —
    라벨이 이제 진짜 종료를 의미하므로 **그대로 유지**한다. 이제서야 정확한 시각 신호가 된다.
  - `onClose`를 지우지 않는다. `✕`가 계속 사용한다.
- **VALIDATE**:
  ```bash
  pnpm check
  ```
  EXPECT: 타입 에러 0건 (`onQuit`이 JSDoc 타입과 사용처 양쪽에 존재).

---

### Task 5: 렌더러 테스트 — 기존 3개 갱신 + 신규 2개

- **ACTION**: 현재 동작을 고정하고 있는 테스트를 갱신하고, `✕`/`Quit` 분리와 `quit` 커맨드
  이름을 검증하는 테스트를 추가한다.
- **IMPLEMENT**:

  1. `src/lib/components/UsagePanel.test.ts:130-145` 교체:

  ```typescript
    it('hides the panel through the close control and quits through the footer button', async () => {
      const onClose = vi.fn();
      const onQuit = vi.fn();
      render(UsagePanel, {
        props: {
          providers: bothProviders('active'),
          selected: 'claude',
          primary: 'claude',
          refreshing: false,
          nowMs: NOW,
          onClose,
          onQuit,
        },
      });

      await fireEvent.click(
        screen.getByRole('button', { name: 'Close usage panel' }),
      );
      expect(onClose).toHaveBeenCalledTimes(1);
      expect(onQuit).not.toHaveBeenCalled();

      await fireEvent.click(screen.getByRole('button', { name: 'Quit' }));
      expect(onQuit).toHaveBeenCalledTimes(1);
      expect(onClose).toHaveBeenCalledTimes(1);
    });
  ```

  2. `UsagePanel.test.ts:147-165`의 이름과 부재 단정을 뒤집는다:

  ```typescript
    it('opens settings through its callback and exposes the panel close control', async () => {
      const onSettings = vi.fn();
      render(UsagePanel, {
        props: {
          providers: bothProviders('active'),
          selected: 'claude',
          primary: 'claude',
          refreshing: false,
          nowMs: NOW,
          onSettings,
        },
      });

      expect(
        screen.getByRole('button', { name: 'Close usage panel' }),
      ).toBeTruthy();
      await fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
      expect(onSettings).toHaveBeenCalledTimes(1);
    });
  ```

  3. `src/App.test.ts:960-968` 교체 (한 개 → 두 개):

  ```typescript
    it('hides the panel through the close control without quitting CacheBite', async () => {
      window.history.replaceState({}, '', '/?window=panel');
      const { gateway } = fixture();
      render(App, { props: { gateway, notificationAdapter: notifications } });

      await fireEvent.click(
        await screen.findByRole('button', { name: 'Close usage panel' }),
      );
      expect(gateway.hidePanel).toHaveBeenCalledOnce();
      expect(gateway.quit).not.toHaveBeenCalled();
    });

    it('exits CacheBite through the footer Quit button', async () => {
      window.history.replaceState({}, '', '/?window=panel');
      const { gateway } = fixture();
      render(App, { props: { gateway, notificationAdapter: notifications } });

      await fireEvent.click(await screen.findByRole('button', { name: 'Quit' }));
      expect(gateway.quit).toHaveBeenCalledOnce();
      expect(gateway.hidePanel).not.toHaveBeenCalled();
    });
  ```

  4. `src/lib/api/gateway.test.ts` — 138-145줄의 커맨드 이름 검증 블록에 `quit`을 추가:

  ```typescript
      await tauriGateway.showPanel();
      await tauriGateway.hidePanel();
      await tauriGateway.quit();
      expect(invoked).toHaveBeenCalledWith('show_panel', {});
      expect(invoked).toHaveBeenCalledWith('hide_panel', {});
      expect(invoked).toHaveBeenCalledWith('quit', {});
  ```

- **MIRROR**: RENDERER_TEST_STRUCTURE (`UsagePanel.test.ts:130-145`),
  APP_TEST_STRUCTURE (`App.test.ts:960-968`),
  GATEWAY_TEST_STRUCTURE (`gateway.test.ts:138-145`).
- **IMPORTS**: 없음. `vi`, `fireEvent`, `screen`, `bothProviders`, `NOW`, `fixture`,
  `notifications`가 각 파일에 이미 있다.
- **GOTCHA**:
  - `App.test.ts`의 `afterEach`(163-167줄)가 `window.history.replaceState({}, '', '/')`로
    리셋하므로 각 테스트가 `?window=panel`을 **직접** 설정해야 한다.
  - `App.test.ts`에서는 `findByRole`(비동기)을 쓴다 — 패널은 게이트웨이 하이드레이션 후에
    마운트된다. `UsagePanel.test.ts`는 컴포넌트를 직접 렌더하므로 동기 `getByRole`을 쓴다.
  - `gateway.quit`은 `fixture()`(`App.test.ts:145`)에 이미 `vi.fn`으로 있어 추가 작업이 없다.
  - 기존 테스트 **이름**까지 갱신한다. 이름을 그대로 두면 거짓 문서가 된다 — 이슈 #29가
    지적한 것이 바로 이 두 테스트가 버그 동작을 고정하고 있다는 점이다.
- **VALIDATE**:
  ```bash
  pnpm vitest run src/lib/components/UsagePanel.test.ts
  pnpm vitest run src/App.test.ts
  pnpm vitest run src/lib/api/gateway.test.ts
  pnpm vitest run -t "exits CacheBite through the footer Quit button"
  ```
  EXPECT: 전부 통과.

---

### Task 6: 패널 always-on-top (버그 #1)

- **ACTION**: `src-tauri/tauri.conf.json`의 `panel` 창에 `"alwaysOnTop": true`를 추가한다.
- **IMPLEMENT**: 34-44줄의 `panel` 창 정의를 다음으로 바꾼다 (`skipTaskbar`는 **추가하지 않는다**):

  ```json
        {
          "label": "panel",
          "url": "index.html?window=panel",
          "title": "CacheBite Usage",
          "width": 312,
          "height": 520,
          "visible": false,
          "transparent": true,
          "decorations": false,
          "alwaysOnTop": true,
          "resizable": false
        }
  ```

- **MIRROR**: 같은 파일 `overlay` 창(20-33줄)이 이미 `"alwaysOnTop": true`를 쓴다 — 새 문법이 아니다.
- **IMPORTS**: 없음 (설정 파일).
- **GOTCHA**:
  - Linux Wayland 컴포지터는 always-on-top을 거부할 수 있다. `PlatformCapabilities`가 이미
    이를 모델링한다(`window/mod.rs:113-123` `linux_wayland(always_on_top, …)`). 거부되면
    capability 진단이 `unavailable`로 남고, **provider 실패로 보고해서는 안 된다**
    (CLAUDE.md invariant: "Unverified platform capabilities report `unavailable`, never a
    provider failure"). 그 환경에서는 Task 2의 더블클릭 래이즈가 폴백이다.
  - 전체화면 정책과 충돌하지 않는다. `synchronize_fullscreen`(`window/mod.rs:231-244`)이
    전체화면 진입 시 `WindowCommand::HidePanel`을 실행하므로 always-on-top이어도
    전체화면 앱 위에 남지 않는다.
  - `native-smoke.yml`의 headless 잡에서 always-on-top이 적용되지 않아도 창 생성 자체는
    실패하지 않는다 — Tauri는 이 플래그를 best-effort로 적용한다.
- **VALIDATE**:
  ```bash
  node -e "JSON.parse(require('fs').readFileSync('src-tauri/tauri.conf.json','utf8'));console.log('ok')"
  cargo test --manifest-path src-tauri/Cargo.toml --all-features
  ```
  EXPECT: JSON 파싱 성공, Rust 테스트 통과. 실제 always-on-top 동작은 Manual Validation에서.

---

### Task 7: e2e — `✕`가 셸 레이아웃을 바꾸지 않음을 검증

- **ACTION**: `tests/e2e/renderer.spec.ts`에 `✕`가 흐름에서 제외됨을 검증하는 테스트를 추가한다.
- **IMPLEMENT**: `'uses the unified 312px vibrancy panel shell'` 테스트(150줄) **뒤**에 추가:

  ```typescript
    it('layers the close control over the header without reserving space', async () => {
      await browser.url('/?window=panel&fixture=e2e');
      await expect($('section[aria-label="Usage panel"]')).toBeDisplayed();

      const geometry = await browser.execute(() => {
        const panel = document.querySelector<HTMLElement>('main.panel');
        const close = panel?.querySelector<HTMLElement>('.close-panel');
        const header = panel?.querySelector<HTMLElement>('.usage-panel > header');
        if (!panel || !close || !header) {
          throw new Error('panel close control missing');
        }
        const panelBox = panel.getBoundingClientRect();
        const closeBox = close.getBoundingClientRect();
        const headerBox = header.getBoundingClientRect();
        return {
          position: getComputedStyle(close).position,
          // Out of flow: the header still starts at the shell's top edge and
          // spans its full inner width, exactly as it did without the control.
          headerTopOffset: Math.round(headerBox.top - panelBox.top),
          headerWidth: Math.round(headerBox.width),
          panelWidth: Math.round(panelBox.width),
          closeInsideShell:
            closeBox.right <= panelBox.right && closeBox.top >= panelBox.top,
        };
      });

      expect(geometry.position).toBe('absolute');
      expect(geometry.headerTopOffset).toBe(0);
      expect(geometry.headerWidth).toBe(geometry.panelWidth);
      expect(geometry.closeInsideShell).toBe(true);
    });
  ```

- **MIRROR**: E2E_LAYOUT_ASSERTION (`tests/e2e/renderer.spec.ts:154-172`).
- **IMPORTS**: 없음. `browser`, `$`, `expect`는 WebdriverIO 전역이며 이 파일에서 이미 사용 중이다.
- **GOTCHA**:
  - `headerWidth === panelWidth`가 성립하는 이유: `header`는 `.usage-panel`(`width: 100%`)의
    블록 자식이고 `main.panel`은 `padding: 0`이다(`global.css:27`). `getBoundingClientRect`가
    양쪽 모두 테두리를 포함하므로 같은 기준이다. 이 어서션이 실패하면 `✕`가 실제로 공간을
    차지했다는 뜻 — 정확히 잡고 싶은 회귀다.
  - jsdom이 아니라 실제 브라우저여야 계산된 스타일이 신뢰 가능하므로 단위 테스트가 아니라
    e2e에 둔다.
  - `?window=panel&fixture=e2e`가 필수다. `fixture=e2e`는 `App.svelte:55-60`에서 dev +
    localhost 조건과 함께 검사된다.
- **VALIDATE**:
  ```bash
  pnpm test:e2e:renderer
  ```
  EXPECT: 신규 테스트를 포함해 전체 통과.

---

### Task 8: 문서 갱신

- **ACTION**: `docs/ui-contract.md`, `docs/beta-testing.md`, `CLAUDE.md`를 실제 동작과 맞춘다.
- **IMPLEMENT**:

  1. `docs/ui-contract.md` §5(170-188줄) ASCII에 `✕`를 반영하고 푸터 라벨 의미를 명확히 한다:

  ```text
  ┌──────────────────────────────────┐
  │ 헤더                        (✕)   │  ✕ = 패널 닫기 (절대 위치, 흐름 밖)
  │  [Claude ★] [Codex]  탭           │  ★ = 주 provider 표시
  │  plan_type (있을 때만)             │
  ├──────────────────────────────────┤
  │ 본문 (선택된 탭의 provider)         │
  │  5시간 게이지  ▓▓▓▓░░ 68%          │
  │    리셋까지 1시간 12분              │
  │  주간 게이지   ▓▓░░░░ 31%          │
  │    리셋: 월요일 09:00              │
  │  캡처 시각 · source 라벨            │
  │  상태 줄: stale/오류/인증 안내       │
  ├──────────────────────────────────┤
  │ 푸터                              │
  │  [지금 새로고침] [주 provider로 설정] │
  │  [설정] [종료]                     │  종료 = CacheBite 프로세스 종료
  └──────────────────────────────────┘
  ```

  그리고 190줄 "규칙:" 목록에 네 항목을 추가한다:

  ```markdown
  - 패널은 **항상 위**로 떠 있고 외부 클릭으로 닫히지 않는다. 유일한 닫기 수단은 헤더 우측
    상단의 `✕`이며, 이는 패널만 숨긴다(프로세스는 계속 실행된다).
  - `✕`는 레이아웃 흐름 밖의 절대 위치 요소다. 추가·제거가 헤더·본문·푸터의 치수나
    `resize_panel`에 보고되는 측정 높이를 바꾸지 않아야 한다.
  - 펫 더블클릭은 **토글이 아니라 표시·전면화**다. 이미 열려 있지만 다른 창 뒤에 있는
    패널은 최상단으로 올라오고 포커스를 받는다. 무반응이어서는 안 된다.
  - 푸터의 "종료"는 CacheBite 프로세스를 종료한다(`app.exit(0)`). 패널 숨김과 혼용하지 않는다.
  ```

  2. `docs/beta-testing.md` "First run"(82-90줄)에 두 항목을 추가한다:

  ```markdown
  - The panel stays on top until you close it. Click the **×** at its top-right
    to hide it — CacheBite keeps running. Double-click the pet to bring the panel
    back to the front.
  - **Quit** in the panel footer exits CacheBite. That is the supported way to
    stop it; you should never need Task Manager.
  ```

  3. `docs/beta-testing.md` 77-78줄의 거짓 트레이 서술을 정정한다. CacheBite에는 트레이가
     없으므로 현재 문장은 존재하지 않는 기능을 암시한다:

  ```markdown
  On Debian/Ubuntu the runtime dependency is `libwebkit2gtk-4.1-0`. CacheBite
  ships no tray icon — the pet and its panel are the whole surface.
  ```

  4. `CLAUDE.md` "Invariants — do not break" 목록에 한 항목을 추가한다:

  ```markdown
  - **Panel visibility policy:** the panel is always-on-top and is dismissed only by its
    explicit `✕` control — never by focus loss or an outside click. `show_panel` must raise
    and focus an already-visible panel rather than returning early, or a double-click on the
    pet becomes a no-op. The footer `Quit` is `app.exit(0)`, not a panel hide.
  ```

- **MIRROR**: `docs/ui-contract.md` §5의 기존 ASCII + "규칙:" 불릿 형식,
  `CLAUDE.md` Invariants의 `**제목:** 설명` 형식.
- **IMPORTS**: 없음.
- **GOTCHA**:
  - `docs/ui-contract.md`는 **한국어**, `docs/beta-testing.md`와 `CLAUDE.md`는 **영어**다.
    각 문서의 언어를 유지한다.
  - `docs/ui-contract.md`는 CLAUDE.md가 "presentation contract의 source of truth"로 지정한
    문서다. 갱신하지 않으면 계약이 코드와 어긋난다.
  - 77-78줄 정정은 이슈 #29가 직접 지적한 항목이다 — "reads as though a tray exists — it
    does not."
- **VALIDATE**:
  ```bash
  pnpm lint
  ```
  EXPECT: prettier가 마크다운 포맷을 문제 삼지 않음. 정확성은 Manual Validation에서.

---

## Testing Strategy

### Unit Tests

| Test | Input | Expected Output | Edge Case? |
| --- | --- | --- | --- |
| `panel_reveal` — 보이는 패널 | `visible = true` | `PanelReveal::RaiseExisting` | 아니오 (핵심 회귀) |
| `panel_reveal` — 숨은 패널 | `visible = false` | `PanelReveal::AwaitLayout` | 아니오 |
| `✕` 클릭 | `onClose`, `onQuit` 스파이 | `onClose` 1회, `onQuit` 0회 | 아니오 |
| 푸터 `Quit` 클릭 | `onClose`, `onQuit` 스파이 | `onQuit` 1회, `onClose` 0회 | 아니오 |
| `✕` 존재 | 기본 props | `Close usage panel` 버튼 조회 성공 | 부재 단정 뒤집기 |
| App: `✕` → 게이트웨이 | panel 창 마운트 | `hidePanel` 1회, `quit` 0회 | 아니오 |
| App: `Quit` → 게이트웨이 | panel 창 마운트 | `quit` 1회, `hidePanel` 0회 | 아니오 |
| `tauriGateway.quit()` | `mockIPC` | `invoke('quit', {})` | 커맨드 이름 오타 방지 |
| e2e: `✕` 흐름 제외 | 실제 브라우저 | `position: absolute`, `headerTopOffset === 0`, `headerWidth === panelWidth` | 레이아웃 회귀 |

### Edge Cases Checklist

- [ ] 패널이 이미 보이는 상태에서 더블클릭 → 최상단으로 올라옴 (버그 #2 회귀)
- [ ] 패널이 최소화된 상태에서 더블클릭 → `unminimize` 후 전면 (`is_visible()`이 true인 함정)
- [ ] `set_focus()`가 거부되는 headless/Wayland → `show_panel`이 **에러를 내지 않음**
- [ ] always-on-top이 거부되는 Wayland → capability가 `unavailable`, provider 실패 아님
- [ ] 전체화면 앱 진입 → 패널이 always-on-top이어도 숨겨짐
- [ ] `✕` 추가 후 `resize_panel`이 다른 높이를 받지 않음 (`ResizeObserver` 무동요)
- [ ] Settings 화면에서 `✕` 부재 → `← Back`으로 복귀 가능 (알려진 범위 밖)
- [ ] `Quit`을 overlay 창에서 호출할 경로가 없음 (`Forbidden` 회피)
- [ ] 멀티 스크린: 펫을 다른 디스플레이로 옮긴 뒤 더블클릭 → 그 디스플레이에 재앵커 후 전면
- [ ] 라이트/다크 양 테마에서 `✕` hover/focus 상태가 보임

---

## Validation Commands

### Static Analysis

```bash
pnpm check
pnpm lint
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-features -- -D warnings
```

EXPECT: 타입 에러 0건, lint 0건, clippy 경고 0건.

### Unit Tests

```bash
pnpm vitest run src/lib/components/UsagePanel.test.ts
pnpm vitest run src/App.test.ts
pnpm vitest run src/lib/api/gateway.test.ts
cargo test --manifest-path src-tauri/Cargo.toml window::tests
```

EXPECT: 전부 통과.

### Full Test Suite

```bash
pnpm test:ci
cargo test --manifest-path src-tauri/Cargo.toml --all-features
```

EXPECT: 회귀 없음. `test:ci`는 svelte-check + eslint + prettier + vitest 커버리지
(branches/functions/lines/statements **80%** 게이트, `vite.config.ts`) + vite build를 모두 실행한다.

### Browser / E2E Validation

```bash
pnpm test:e2e:renderer
```

EXPECT: 신규 `layers the close control over the header without reserving space` 포함 전체 통과.

### Native Build

```bash
pnpm tauri dev
```

EXPECT: 두 창이 뜨고 네이티브 콘솔에 `[CacheBite:native] setup:ready`가 찍힌다.

### Manual Validation

> **필수.** 이 변경의 핵심(창 래이즈·포커스·always-on-top)은 Tauri 런타임 없이 단위
> 테스트로 증명할 수 없다. `panel_reveal` 테스트가 그린이라는 것은 배선이 맞다는 뜻이
> 아니다 (Task 1 GOTCHA 참조).

버그 #2 (더블클릭 복귀):
- [ ] 펫 더블클릭 → 패널이 뜬다
- [ ] 다른 앱(에디터, 브라우저)을 클릭한다
- [ ] 펫을 다시 더블클릭 → **패널이 최상단으로 올라오고 포커스를 받는다** (이전: 무반응)
- [ ] 작업표시줄의 "CacheBite Usage" 항목 클릭 → 여전히 전면으로 나온다 (회귀 없음)

버그 #1 (항상 위):
- [ ] 패널을 열어둔 채 다른 앱을 최대화 → **패널이 계속 보인다**
- [ ] Linux Wayland에서 always-on-top이 거부되더라도 앱이 정상 동작하고 provider 실패가
      보고되지 않는다. 더블클릭 복귀로 커버된다.

요구 #3 (`✕` / `Quit`):
- [ ] 헤더 우측 상단에 `✕`가 보이고, 패널이 잘리거나 치수가 달라지지 않았다
- [ ] `✕` 클릭 → 패널만 숨는다. 펫은 남고 프로세스도 살아 있다
- [ ] Codex 탭 클릭 → `✕`에 가려진 우측 상단 모서리를 빼고 정상 동작한다
- [ ] 푸터 `Quit` 클릭 → **CacheBite가 완전히 종료된다.** 작업관리자에 프로세스가 없고 펫도 사라진다
- [ ] 라이트/다크 양 테마에서 `✕` hover/focus 상태가 시각적으로 구분된다
- [ ] 멀티 스크린: 펫을 보조 디스플레이로 옮기고 더블클릭 → 그 디스플레이에 앵커되어 전면에 뜬다

---

## Acceptance Criteria

- [ ] 이미 보이는 패널에 펫 더블클릭 시 최상단으로 올라온다 (버그 #2)
- [ ] 작업표시줄 항목 클릭 복귀 경로가 그대로 동작한다
- [ ] 다른 앱을 클릭해도 패널이 떠 있다 (버그 #1, 플랫폼이 허용하는 범위)
- [ ] 패널 우측 상단 `✕`가 패널만 숨긴다
- [ ] `✕` 추가로 패널 치수·측정 높이가 변하지 않는다 (e2e로 검증)
- [ ] 푸터 `Quit`이 `app.exit(0)`으로 CacheBite를 종료한다
- [ ] 모든 Validation Commands 통과
- [ ] 커버리지 게이트 80% 유지
- [ ] 타입 에러·lint 에러·clippy 경고 0건
- [ ] Manual Validation 체크리스트 전부 확인

## Completion Checklist

- [ ] 코드가 발견된 패턴을 따른다 (헤드리스 창 조작 실패는 에러로 승격하지 않음)
- [ ] 에러 처리가 코드베이스 스타일과 일치한다 (`map_err(|_| IpcError::PanelUnavailable)`, 새 변주 없음)
- [ ] 로깅 관례를 따른다 — **새 로그를 추가하지 않는다**. 창 조작 실패는 조용히 저하되고,
      렌더러 측 실패는 기존 `trace()`(`App.svelte:428`)가 받는다
- [ ] 프라이버시 계약 유지: 이 변경은 자격 증명·계정 식별자·provider 응답을 다루지 않는다.
      새 DTO·새 로그 문자열 없음
- [ ] 테스트가 테스트 패턴을 따른다 (접근성 이름 조회, AAA, 서술형 이름)
- [ ] 하드코딩 값 없음 — 색·간격은 `tokens.css` 변수 사용
- [ ] 문서 갱신 완료 (`ui-contract.md`, `beta-testing.md`, `CLAUDE.md`)
- [ ] 불필요한 범위 추가 없음 (트레이·shortcut·확인 대화상자 미포함)
- [ ] 자기완결적 — 구현 중 추가 질문 불필요

## Risks

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| `set_focus()`가 headless/Wayland에서 실패해 `show_panel`이 에러를 반환하고 `native-smoke.yml` fixture 잡이 깨진다 | 높음 | 높음 | `reveal_panel`에서 `set_focus()`를 `let _ =`로 best-effort 처리 (`position_panel` 선례). Task 2 GOTCHA |
| always-on-top 패널이 다른 앱 작업을 방해한다고 느껴진다 | 중간 | 중간 | 사용자가 의도된 설계로 확인. `✕`로 즉시 닫을 수 있고, 전체화면 시에는 `synchronize_fullscreen`이 숨긴다 |
| `✕`가 Codex 탭 우측 상단 ~14×18px을 덮어 클릭이 어긋난다 | 확실 (설계상) | 낮음 | Codex 탭 140px 중 126px 잔존. 사용자가 리플로 없는 오버레이를 명시 요구. 실사용 후 헤더 `padding-right` 보정을 후속으로 남김 |
| `Quit`에 확인이 없어 오클릭으로 종료된다 | 낮음 | 중간 | 사용자가 확인 없음을 선택. `.ghost-action.quit` hover가 위험색(`--sev-exhausted`)으로 바뀌어 시각 경고. 재실행 비용 낮음 |
| `panel_reveal` 순수 테스트가 그린이지만 실제 래이즈가 배선되지 않는다 | 중간 | 높음 | 이 저장소의 기록된 실패 모드(`stabilization-and-quality-pass.plan.md:528`). Manual Validation 체크리스트를 필수로 요구 |
| Wayland에서 always-on-top 거부를 provider 실패로 보고한다 | 낮음 | 높음 | CLAUDE.md invariant. `PlatformCapabilities`가 이미 `unavailable`로 모델링. Task 6 GOTCHA |
| `✕` 추가가 `ResizeObserver` 높이를 흔들어 `resize_panel` 루프가 생긴다 | 낮음 | 중간 | 절대 위치라 구조적으로 불가. e2e 어서션으로 고정 |

## Notes

### 진단 근거 (사용자 요구 1·2번의 실제 원인)

사용자가 "시스템 트레이"로 지칭한 것은 **패널 창의 작업표시줄 항목**이다. 근거:

- 저장소 전체 `grep`에 트레이 코드가 없다. `tray`/`TrayIcon` 일치는 CI의 시스템 패키지
  (`libappindicator3-dev`)와 `Cargo.lock`의 전이 의존성뿐이다. `tauri` 크레이트에
  `tray-icon` feature도 켜져 있지 않다 (`Cargo.toml:26`).
- `panel` 창에는 `skipTaskbar`가 없다(`tauri.conf.json:34-44`). `overlay`만 `skipTaskbar: true`다.
  그래서 패널만 작업표시줄에 나타난다.

"외부 클릭 시 hide된다"는 관찰의 실제 메커니즘은 **숨김이 아니라 z-order 후퇴**다:

- Rust에 `on_window_event`가 없고 `WindowEvent::Focused` 처리도 없다.
- 렌더러의 유일한 `blur` 리스너(`App.svelte:471`)는 overlay 전용이고 제스처가 열려 있을 때만
  등록되며, 드래그 래치 해제만 한다.
- 패널은 `alwaysOnTop: false` + `transparent` + `decorations: false`라 다른 창이 앞에 오면
  시각적으로 사라진 것처럼 보인다.

그리고 그 상태에서 `is_visible()`은 **여전히 true**이므로 `show_panel`(`ipc.rs:283-285`)이
조기 반환한다 — 이것이 더블클릭 무반응의 정확한 원인이다.

### `quit` 경로는 이미 완비되어 있다

`AppGateway.quit()`(`gateway.ts:99`, `:258`) → 네이티브 `quit`(`ipc.rs:413-418`) → `app.exit(0)`.
`panel` 창 전용 인가(`window/mod.rs:100`)와 Rust 인가 테스트(`window/tests.rs:335`, `:338`),
픽스처 미러(`fixtureGateway.ts:115`)까지 존재한다. 렌더러 호출 지점만 0건이었다.
따라서 Task 4는 배선 한 줄이 본질이고, 게이트웨이 계약 변경은 없다.

### 이 변경이 뒤집는 설계 결정 — 사실은 뒤집지 않는다

`docs/superpowers/specs/2026-07-23-circular-overlay-interaction-design.md:20-21`은
*"Add a close control to the information panel. It hides only the panel and does not quit
CacheBite."*라고 적었다. 그 결정 자체는 유효하다 — 닫기 컨트롤은 남는다. 잘못된 것은
그것이 `Quit` 라벨을 달았고, 별도의 종료 수단이 끝까지 추가되지 않았다는 점이다.
이 계획은 그 결정을 뒤집지 않고 **분리**한다: `✕` = 패널 숨김(원래 의도),
`Quit` = 프로세스 종료(누락분).

### 후속 이슈 후보 (이 계획 범위 밖)

1. **시스템 트레이 아이콘** — Show / Settings / Quit. `tauri` crate에 `tray-icon` feature
   활성화가 필요하다. Linux CI는 이미 `libappindicator3-dev`를 설치하고(`ci.yml:35`,
   `native-smoke.yml:56,100`, `release.yml:50`) `tray-icon`이 `Cargo.lock`에 해소돼 있어
   진입 장벽이 낮다. 패널이 닫힌 동안 앱이 "살아 있는 곳"을 제공한다.
2. **Settings 화면의 `✕`** — 현재 `← Back` 후에만 닫을 수 있다.
3. **`✕`와 Codex 탭 히트 영역 겹침 보정** — 헤더 `padding-right`로 해결 가능하지만
   사용자가 우선 순수 오버레이를 요구했다.
4. **#30 전역 hide/show shortcut** — 이 이슈가 먼저 랜딩해야 한다 (소유자 코멘트).
