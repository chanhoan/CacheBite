# Plan: 펫 더블클릭으로 사용량 패널 토글 (issue #48)

## Summary

펫 더블클릭이 지금은 "패널 표시·전면화"만 한다. 이미 열린 패널을 다시 더블클릭해도 닫히지 않아, 사용자는 반대 동작을 위해 패널의 `✕`까지 마우스를 옮겨야 한다. 더블클릭을 **토글**로 바꾼다 — 숨겨져 있으면 열고 포커스를 주고, 보이면 숨긴다(프로세스는 계속 실행).

토글 판단은 **네이티브에서** 패널의 실제 가시성으로 내린다. 렌더러가 자체 상태를 들고 있으면 `✕`(`hide_panel`)·전체화면 감시·hide/show 단축키가 패널을 숨겼을 때 즉시 어긋난다.

## User Story

As a CacheBite 사용자,
I want 펫을 다시 더블클릭해 사용량 패널을 닫을 수 있기를,
So that 패널을 열 때와 닫을 때 같은 제스처를 쓰고, 닫으려고 패널의 `✕`까지 마우스를 옮기지 않아도 된다.

## Problem → Solution

| | 현재 | 목표 |
|---|---|---|
| 숨김 상태에서 더블클릭 | 패널 표시 | 패널 표시 (동일) |
| 표시 상태에서 더블클릭 | **아무 일도 안 일어난 것처럼 보임** (전면화만) | 패널 숨김, CacheBite는 계속 실행 |
| 닫기 경로 | `✕` 단독 | `✕` + 펫 더블클릭 |
| 판단 주체 | 없음 (항상 show) | 네이티브 (`panel.is_visible()` + 대기 중 reveal) |

## Metadata

- **Complexity**: Medium
- **Source PRD**: N/A (GitHub issue #48)
- **PRD Phase**: standalone
- **Estimated Files**: 12 (네이티브 4, 렌더러 4, 테스트 4 — 문서 3 별도)
- **릴리스 맥락**: #30(hide/show 단축키)과 함께 `main` 머지 후 0.1.1 릴리스 예정. `docs/beta-testing.md`의 "닫기는 `✕`" 안내가 배포 시점에 사실과 달라지므로 문서 갱신은 **선택이 아니라 릴리스 차단 항목**이다.

---

## UX Design

### Before

```text
  숨김 상태                       표시 상태
┌───────────┐                  ┌───────────┐   ┌──────────────┐
│    🐱     │  ── 더블클릭 ──▶  │    🐱     │   │ 사용량   (✕) │
│           │                  │           │   │  Claude/Codex│
└───────────┘                  └───────────┘   └──────────────┘
                                      │
                                  더블클릭
                                      │
                                      ▼
                               ┌───────────┐   ┌──────────────┐
                               │    🐱     │   │ 사용량   (✕) │  ← 그대로
                               └───────────┘   └──────────────┘
                               "닫으려면 ✕까지 마우스를 옮겨야 한다"
```

### After

```text
  숨김 상태                       표시 상태
┌───────────┐   더블클릭 ──▶   ┌───────────┐   ┌──────────────┐
│    🐱     │                 │    🐱     │   │ 사용량   (✕) │
│           │   ◀── 더블클릭   │           │   │  Claude/Codex│
└───────────┘                 └───────────┘   └──────────────┘

  ✕ 도 그대로 동작한다. 두 경로 모두 같은 네이티브 가시성을 읽고 내린다.
  숨겨도 프로세스·수집 폴링은 계속 돈다.
```

### Interaction Changes

| Touchpoint | Before | After | Notes |
|---|---|---|---|
| 펫 더블클릭 (패널 숨김) | `show_panel` → 표시 | `toggle_panel` → 표시 | 동일 결과. 배치·레이아웃 대기 경로 그대로 |
| 펫 더블클릭 (패널 표시) | 전면화 + 포커스 | **숨김** | 계약 변경. §Risks 참조 |
| 펫 Enter/Space | `show_panel` | `toggle_panel` | 같은 핸들러(`onOpen`)를 공유하므로 함께 토글. aria-label도 갱신 |
| 패널 `✕` | `hide_panel` | `hide_panel` (변경 없음) | |
| 푸터 Quit | `quit` | `quit` (변경 없음) | 숨김과 혼용 금지 불변식 유지 |
| `Ctrl+Shift+H` | 오버레이+패널 숨김 | 동일 + **레이아웃 게이트 해제** | 150ms 유예 타이머가 숨긴 패널을 되살리는 창을 막는다 |

---

## Mandatory Reading

| Priority | File | Lines | Why |
|---|---|---|---|
| P0 | `src-tauri/src/refresh/ipc.rs` | 24-41, 309-365, 437-474 | 유예 게이트(`PanelLayoutGate`)·`reveal_panel`·`show_panel`·`resize_panel`·`hide_panel`. 이 계획의 거의 모든 변경이 여기 있다 |
| P0 | `src-tauri/src/window/mod.rs` | 62-135 | `NativeCommand`, `command_allowed`, `PanelReveal`/`panel_reveal` — 대체 대상 |
| P0 | `src/lib/api/gateway.ts` | 72-102, 250-261 | `AppGateway` 계약과 `tauriGateway` 구현. 와이어 DTO의 유일한 정의처 |
| P1 | `src-tauri/src/lib.rs` | 36-68, 134-152, 194-230, 249-284 | 커맨드 등록, `OverlayHideGate`, `toggle_overlay_visibility`, 전체화면 감시 |
| P1 | `src/App.svelte` | 695-723 | 오버레이 배선 (`onOpen`) |
| P1 | `src/lib/components/PetOverlay.svelte` | 1-57 | 더블클릭/키보드 진입점, aria-label |
| P1 | `src-tauri/src/window/tests.rs` | 337-371 | 권한 테이블 테스트와 `panel_reveal` 테스트 |
| P2 | `src/App.test.ts` | 140-192, 919-1007 | 게이트웨이 목 형태, 더블클릭/닫기 테스트 |
| P2 | `tests/e2e/native.spec.ts` | 27-134 | `invokeFromCurrentWindow`, `switchToCacheBiteWindow` 헬퍼와 권한 거부 테스트 |
| P2 | `docs/ui-contract.md` | 160-204 | §4.3 포인터 동작, §5 패널 규칙 — 계약 문구가 여기서 바뀐다 |
| P2 | `docs/beta-testing.md` | 84-101 | 베타 안내 문구 |

## External Documentation

| Topic | Source | Key Takeaway |
|---|---|---|
| `Manager::try_state` | `~/.cargo/registry/.../tauri-2.11.5/src/lib.rs:744` | `fn try_state<T>(&self) -> Option<State<'_, T>>` — 미등록 상태에서 패닉 대신 `None`. 훅 핸들러처럼 setup 완료 전에 불릴 수 있는 경로에서 필수 |
| `State::inner` / `Deref` | `tauri-2.11.5/src/state.rs:28,33` | `State<'r, T>`는 `Deref<Target = T>`이면서 `inner(&self) -> &'r T`도 제공. `&PanelLayoutGate`를 받는 함수에 넘길 때 `gate.inner()`를 쓴다 (함수 인자에는 자동 deref가 적용되지 않는다) |
| Tauri 2 capabilities | `src-tauri/capabilities/{overlay,panel}.json` | 앱이 `invoke_handler!`로 등록한 자체 커맨드는 capability 파일에 선언하지 **않는다**. 목록에 있는 것은 `core:*`/플러그인 권한뿐이므로 `toggle_panel` 추가 시 capability 변경 불필요 |

외부 라이브러리 신규 도입 없음. 나머지는 전부 내부 확립 패턴이다.

---

## Patterns to Mirror

### NATIVE_POLICY_FUNCTION — 순수 판단을 `window/mod.rs`로 분리

// SOURCE: `src-tauri/src/window/mod.rs:112-135`

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
/// another window while still reporting `is_visible() == true`. Returning early
/// on that state is what made a double-click on the pet a no-op, with no way
/// back other than the taskbar entry.
pub fn panel_reveal(visible: bool) -> PanelReveal {
    if visible {
        PanelReveal::RaiseExisting
    } else {
        PanelReveal::AwaitLayout
    }
}
```

판단은 인자만 받는 순수 함수, 호출부는 `match` 한 줄. **이 파일의 모든 정책이 이 형태다** — 새 `panel_toggle`도 정확히 이 틀을 복제한다.

### IPC_COMMAND — 인가 → 조회 → 실행

// SOURCE: `src-tauri/src/refresh/ipc.rs:465-481`

```rust
#[tauri::command]
pub fn hide_panel(
    window: tauri::WebviewWindow,
    gate: State<'_, PanelLayoutGate>,
) -> Result<(), IpcError> {
    authorize(&window, NativeCommand::HidePanel)?;
    // Stop a pending grace timer from resurrecting the panel just closed.
    gate.awaiting_layout.store(false, Ordering::SeqCst);
    window.hide().map_err(|_| IpcError::PanelUnavailable)
}

#[tauri::command]
pub fn quit(window: tauri::WebviewWindow, app: AppHandle) -> Result<(), IpcError> {
    authorize(&window, NativeCommand::Quit)?;
    app.exit(0);
    Ok(())
}
```

모든 커맨드가 `authorize(&window, NativeCommand::X)?`로 시작한다. 실패는 타입화된 `IpcError`이며 원시 에러를 절대 전파하지 않는다.

### BEST_EFFORT_SIDE_EFFECT — 헤드리스/Wayland에서 실패해도 에러로 승격하지 않는다

// SOURCE: `src-tauri/src/refresh/ipc.rs:313-326`

```rust
fn reveal_panel(panel: &tauri::WebviewWindow) -> Result<(), IpcError> {
    // A minimised panel still reports `is_visible() == true`, so restoring it has
    // to happen before the raise or the focus lands on nothing.
    if panel.is_minimized().unwrap_or(false) {
        let _ = panel.unminimize();
    }
    panel.show().map_err(|_| IpcError::PanelUnavailable)?;
    // Best-effort, mirroring `position_panel`: headless runners and restrictive
    // Wayland compositors refuse focus changes, and promoting that to an error
    // would fail the fixture jobs in native-smoke.yml for a panel that is in fact
    // on screen.
    let _ = panel.set_focus();
    Ok(())
}
```

`show()`는 에러, `set_focus()`/`unminimize()`는 `let _ =`. **이 구분을 그대로 유지한다** — `native-smoke.yml`의 headless Wayland 잡이 여기에 걸린다.

### ERROR_HANDLING / LOGGING — 네이티브

// SOURCE: `src-tauri/src/refresh/ipc.rs:300-305`

```rust
    // Persistence and OS integrations are committed at this point. Reporting
    // an event-delivery failure as a save failure would invite the caller to
    // retry a transaction that already succeeded.
    if app.emit("settings-updated", &settings).is_err() {
        eprintln!("failed to emit settings-updated after settings were committed");
    }
```

로그는 `eprintln!`, 소문자로 시작하는 서술문, 자격증명·경로·계정 식별자 없음. 이번 변경에서 **새 로그를 추가할 필요는 없다**(실패는 전부 타입화된 `IpcError`로 렌더러에 도달한다).

### WIRE_DTO — 네이티브 enum → 렌더러 유니온

// SOURCE: `src-tauri/src/refresh/ipc.rs:74-79` + `src/lib/api/gateway.ts:67`

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectorMode {
    Fixture,
    Production,
}
```

```typescript
export type CollectorMode = 'fixture' | 'production';
```

`rename_all = "snake_case"` + TS 문자열 리터럴 유니온. 새 `PanelVisibility`도 이 짝을 그대로 따른다.

### GATEWAY_METHOD — 인자 없는 커맨드

// SOURCE: `src/lib/api/gateway.ts:250-260`

```typescript
  showPanel: () => invokeNative('show_panel'),
  resizePanel: (height) => {
    if (!Number.isFinite(height) || height <= 0) {
      return Promise.reject(
        new Error('Panel height must be a positive finite number'),
      );
    }
    return invokeNative('resize_panel', { height: Math.ceil(height) });
  },
  hidePanel: () => invokeNative('hide_panel'),
```

인자 없는 커맨드는 한 줄 화살표 함수. 인터페이스 쪽에는 권한 제약을 doc comment로 남긴다:

```typescript
  /** Authorized for the `panel` window only (`window::command_allowed`). */
  hidePanel(): Promise<void>;
```

### RENDERER_COMPONENT_CALLBACK — 컴포넌트는 정책을 모른다

// SOURCE: `src/lib/components/PetOverlay.svelte:6-20,54`

```svelte
  /** @type {{ model: ...; onOpen?: () => void }} */
  let {
    model,
    onOpen = () => {},
  } = $props();
  /** @param {KeyboardEvent} event */
  const keydown = (event) => {
    if (event.key !== 'Enter' && event.key !== ' ') return;
    event.preventDefault();
    onOpen();
  };
...
    ondblclick={() => onOpen()}
```

`PetOverlay.svelte`는 **JSDoc 타입의 `.svelte` (lang="ts" 아님)**. props 타입은 파일 상단 한 줄 JSDoc 블록에 몰아 쓴다.

### TEST_STRUCTURE — Rust

// SOURCE: `src-tauri/src/window/tests.rs:337-371`

```rust
#[test]
fn native_commands_are_authorized_by_window_label() {
    assert!(command_allowed("overlay", NativeCommand::GetCollectorMode));
    assert!(command_allowed("overlay", NativeCommand::ShowPanel));
    assert!(!command_allowed("overlay", NativeCommand::ResizePanel));
    ...
    assert!(!command_allowed("unknown", NativeCommand::ShowPanel));
}

#[test]
fn a_visible_panel_is_raised_rather_than_left_behind_another_window() {
    assert_eq!(panel_reveal(true), PanelReveal::RaiseExisting);
    assert_eq!(panel_reveal(false), PanelReveal::AwaitLayout);
}
```

테스트 이름이 **왜 그런지를 서술**한다(`a_visible_panel_is_raised_rather_than_...`). `assert_eq!`로 판단 표를 통째로 고정한다.

### TEST_STRUCTURE — 렌더러

// SOURCE: `src/App.test.ts:170-192`

```typescript
  it('hydrates overlay and opens the panel only on a circular-surface double-click', async () => {
    const { gateway, emit } = fixture();
    render(App, { props: { gateway, notificationAdapter: notifications } });
    expect(await screen.findByLabelText('CacheBite pet status')).toBeTruthy();
    ...
    const overlay = screen.getByTestId('overlay-pointer-surface');
    await fireEvent.pointerDown(overlay, { clientX: 10, clientY: 10 });
    await fireEvent.pointerUp(overlay, { clientX: 12, clientY: 10 });
    expect(gateway.showPanel).not.toHaveBeenCalled();
    await fireEvent.dblClick(overlay);
    expect(gateway.showPanel).toHaveBeenCalledOnce();
  });
```

게이트웨이 전체를 `vi.fn()` 목으로 만든 `fixture()`를 쓰고, **네거티브 단언을 먼저** 둔다(단일 클릭으로는 안 열린다 → 더블클릭으로 열린다).

### TEST_STRUCTURE — 네이티브 E2E

// SOURCE: `tests/e2e/native.spec.ts:56-74, 114-134`

```typescript
  const invokeFromCurrentWindow = async <T = undefined>(command: string) =>
    browser.executeAsync(
      (requestedCommand: string, done: (result: InvokeResult<T>) => void) => {
        const internals = (window as Window & { __TAURI_INTERNALS__: {...} }).__TAURI_INTERNALS__;
        void internals
          .invoke<T>(requestedCommand)
          .then((value) => done({ status: 'resolved', value }))
          .catch((reason: unknown) => done({ status: 'rejected', reason: String(reason) }));
      },
      command,
    );

  it('authorizes history IPC only from the panel window', async () => {
    const overlayHistory = await invokeFromCurrentWindow('get_history');
    expect(overlayHistory).toEqual({ status: 'rejected', reason: 'forbidden' });
    await $('main[data-window-label="overlay"] [data-testid="overlay-pointer-surface"]').doubleClick();
    await switchToCacheBiteWindow('panel');
    ...
  });
```

권한 거부는 `{ status: 'rejected', reason: 'forbidden' }`로 정확히 단언한다.

---

## Files to Change

| File | Action | Justification |
|---|---|---|
| `src-tauri/src/window/mod.rs` | UPDATE | `PanelReveal`/`panel_reveal` → `PanelToggle`/`panel_toggle`, `NativeCommand::ShowPanel` → `TogglePanel`, `command_allowed` 권한 축소 |
| `src-tauri/src/window/tests.rs` | UPDATE | 권한 표 갱신 + 토글 판단 표(대기 중 reveal 포함) 테스트 |
| `src-tauri/src/refresh/ipc.rs` | UPDATE | `PanelLayoutGate::disarm`, `conceal_panel`/`begin_reveal` 추출, `show_panel` → `toggle_panel`, `PanelVisibility` DTO |
| `src-tauri/src/lib.rs` | UPDATE | 핸들러 등록 교체, 단축키·전체화면 숨김 경로에서 레이아웃 게이트 해제 |
| `src/lib/api/gateway.ts` | UPDATE | `showPanel` → `togglePanel(): Promise<PanelVisibility>`, `PanelVisibility` 타입 추가 |
| `src/lib/api/fixtureGateway.ts` | UPDATE | 픽스처 게이트웨이를 실제 계약과 동기화 |
| `src/App.svelte` | UPDATE | `onOpen` → `onToggle`, `gateway.togglePanel()` 호출 |
| `src/lib/components/PetOverlay.svelte` | UPDATE | prop 이름·aria-label을 토글 의미로 |
| `src/lib/components/PetOverlay.test.ts` | UPDATE | aria-label 단언 갱신 + 토글 콜백 테스트 |
| `src/App.test.ts` | UPDATE | 더블클릭/Enter → `togglePanel`, `✕` → `hidePanel` 유지 |
| `src/lib/api/gateway.test.ts` | UPDATE | `toggle_panel` invoke 단언 |
| `tests/e2e/native.spec.ts` | UPDATE | 열기→닫기→열기 순환, 패널 창에서의 권한 거부 |
| `docs/ui-contract.md` | UPDATE | §4.3, §5의 "토글이 아니라 표시·전면화" 문구 교체 (계약 변경) |
| `docs/beta-testing.md` | UPDATE | 더블클릭 안내와 "닫기는 ✕" 문구 |
| `CLAUDE.md` | UPDATE | "dismissed only by its explicit ✕ control" 불변식 갱신 |

## NOT Building

- **단일 클릭 토글.** 이슈는 더블클릭 제스처만 요구한다. 단일 클릭은 드래그 개시 경로이며 `DRAG_THRESHOLD_PX` 정책과 충돌한다.
- **패널 가시성 조회 IPC(`get_panel_visibility` 등).** 렌더러가 가시성을 캐시하면 `✕`·단축키·전체화면 감시와 즉시 어긋난다 — 이슈가 명시적으로 금지하는 드리프트다. `toggle_panel`의 반환값은 **호출 결과 보고**이지 조회 API가 아니다.
- **애니메이션·트랜지션.** 패널 표시/숨김은 지금과 같은 즉시 전환이다.
- **포커스 기반 3단 판단**(포커스 있으면 숨기고, 없으면 전면화). §Notes의 Alternatives 참조 — 기각.
- **트레이 아이콘·창 목록 등 다른 복구 경로 신설.**
- **설정 항목 추가.** 토글은 항상 켜져 있는 동작이며 스키마 변경이 없다(`schema_version`은 5 그대로).
- **`hide_panel`·`quit`의 의미 변경.** 푸터 Quit은 `app.exit(0)` 그대로다.

---

## Step-by-Step Tasks

> TDD: 각 Task는 RED(테스트 먼저) → GREEN(구현) 순서다. Rust와 렌더러를 번갈아 오가지 않도록 네이티브를 먼저 완성한 뒤 경계를 넘는다.

### Task 1: 토글 판단 테스트를 먼저 작성 (RED)

- **ACTION**: `src-tauri/src/window/tests.rs`의 `a_visible_panel_is_raised_rather_than_left_behind_another_window` 테스트를 토글 판단 테스트로 교체한다.
- **IMPLEMENT**:
  ```rust
  #[test]
  fn a_visible_or_pending_panel_is_hidden_and_a_hidden_one_is_shown() {
      assert_eq!(panel_toggle(false, false), PanelToggle::Show);
      assert_eq!(panel_toggle(true, false), PanelToggle::Hide);
      // Rapid double-clicks: the second one cancels the reveal the first armed
      // rather than arming a second one on a panel that is already on its way.
      assert_eq!(panel_toggle(false, true), PanelToggle::Hide);
      assert_eq!(panel_toggle(true, true), PanelToggle::Hide);
  }
  ```
- **MIRROR**: TEST_STRUCTURE — Rust (판단 표를 `assert_eq!`로 통째 고정, 이름이 이유를 서술)
- **IMPORTS**: `tests.rs` 상단 import에서 `panel_reveal, PanelReveal`을 `panel_toggle, PanelToggle`로 교체한다. 파일 상단 import 블록을 먼저 확인할 것.
- **GOTCHA**: 이 시점에는 컴파일이 실패한다(정상). `cargo test`가 아니라 `cargo check`로 "함수 없음" 에러만 확인하고 Task 2로 넘어간다.
- **VALIDATE**: `cargo check --manifest-path src-tauri/Cargo.toml` → `cannot find function panel_toggle` 계열 에러만 나온다.

### Task 2: `panel_toggle` 정책 구현 + `panel_reveal` 제거 (GREEN)

- **ACTION**: `src-tauri/src/window/mod.rs:112-135`의 `PanelReveal`/`panel_reveal`을 통째로 교체한다.
- **IMPLEMENT**:
  ```rust
  /// What a pet double-click must do for a panel in the given state.
  #[derive(Clone, Copy, Debug, Eq, PartialEq)]
  pub enum PanelToggle {
      /// On screen, or on its way there: hide it. CacheBite keeps running and
      /// keeps polling.
      Hide,
      /// Off screen: anchor it and arm the layout gate, so the reveal waits for
      /// the renderer to measure its content height.
      Show,
  }

  /// A reveal that is still waiting for the renderer counts as visible.
  ///
  /// `show()` is deferred until the renderer reports its height, or until the
  /// grace timer fires, so for a moment the panel reports `is_visible() == false`
  /// while already being on its way to the screen. Reading visibility alone would
  /// make a second double-click inside that window arm the reveal a second time
  /// instead of cancelling it — the panel would appear right after the gesture
  /// meant to dismiss it.
  pub fn panel_toggle(visible: bool, reveal_pending: bool) -> PanelToggle {
      if visible || reveal_pending {
          PanelToggle::Hide
      } else {
          PanelToggle::Show
      }
  }
  ```
- **MIRROR**: NATIVE_POLICY_FUNCTION (순수 함수 + 이유를 적은 doc comment)
- **IMPORTS**: 없음.
- **GOTCHA**: `PanelReveal`은 `refresh/ipc.rs:16-17`의 `use` 목록에도 있다. 제거하면 그쪽도 함께 고쳐야 컴파일된다(Task 6에서 처리).
- **VALIDATE**: `cargo test --manifest-path src-tauri/Cargo.toml window::tests::a_visible_or_pending` → 통과. (다른 모듈은 아직 깨져 있을 수 있다.)

### Task 3: `NativeCommand`와 권한 표 갱신

- **ACTION**: `src-tauri/src/window/mod.rs:62-110`에서 `ShowPanel`을 `TogglePanel`로 바꾸고, `overlay`에만 허용한다.
- **IMPLEMENT**:
  - `NativeCommand` enum: `ShowPanel` → `TogglePanel` (선언 순서 유지 — `UpdateSettings` 뒤, `ResizePanel` 앞).
  - `command_allowed`의 `"overlay"` 팔: `NativeCommand::ShowPanel` → `NativeCommand::TogglePanel`.
  - `command_allowed`의 `"panel"` 팔: `NativeCommand::ShowPanel`을 **삭제**한다(대체하지 않는다).
- **MIRROR**: 기존 `command_allowed` `matches!` 체인 형태 그대로.
- **IMPORTS**: 없음.
- **GOTCHA**: 패널 창의 `ShowPanel` 권한을 없애는 것은 의도된 축소다. 패널은 `show_panel`을 호출한 적이 없고(렌더러 전역 검색으로 확인: 유일한 호출부는 `App.svelte:708` 오버레이 경로), 만약 호출했다면 `position_panel(&window, &panel)`이 **패널을 자기 자신에 앵커**하는 잠재 버그였다. `toggle_panel`을 오버레이 전용으로 두면 이 경로가 구조적으로 사라진다.
- **VALIDATE**: Task 4의 권한 테스트에서 함께 검증.

### Task 4: 권한 테스트 갱신

- **ACTION**: `src-tauri/src/window/tests.rs:337-365`의 `native_commands_are_authorized_by_window_label`을 갱신한다.
- **IMPLEMENT**:
  ```rust
      assert!(command_allowed("overlay", NativeCommand::TogglePanel));
      ...
      // The pet gesture is the overlay's; the panel dismisses itself with `hide_panel`.
      assert!(!command_allowed("panel", NativeCommand::TogglePanel));
      ...
      assert!(!command_allowed("unknown", NativeCommand::TogglePanel));
  ```
  기존 `ShowPanel` 단언 3곳(`overlay` 허용 / `panel` 허용 / `unknown` 거부)을 위 형태로 교체한다. `panel`의 `HidePanel`·`Quit`·`ResizePanel` 단언은 그대로 둔다.
- **MIRROR**: TEST_STRUCTURE — Rust
- **IMPORTS**: 없음.
- **GOTCHA**: `panel`에 대한 단언을 "허용"에서 "거부"로 **뒤집는** 것이므로, 단순 치환이 아니라 `!`가 붙는다.
- **VALIDATE**: `cargo test --manifest-path src-tauri/Cargo.toml window::tests` → 통과.

### Task 5: `PanelLayoutGate`에 `disarm` 추가하고 숨김 경로를 하나로 모은다

- **ACTION**: `src-tauri/src/refresh/ipc.rs`의 `PanelLayoutGate`(38-41행)에 `impl` 블록을 추가하고, 패널을 숨기는 공통 헬퍼를 만든다.
- **IMPLEMENT**:
  ```rust
  impl PanelLayoutGate {
      /// Cancels a pending reveal.
      ///
      /// Exposed because `lib.rs` hides the panel too — the hide/show hotkey and
      /// the fullscreen monitor — and an armed grace timer would put the panel
      /// back on screen milliseconds after it was hidden.
      pub fn disarm(&self) {
          self.awaiting_layout.store(false, Ordering::SeqCst);
      }
  }

  /// Cancels any pending reveal, then hides the panel.
  ///
  /// Every in-process path that hides the panel goes through here, so the
  /// double-click and the `✕` cannot leave the gate in different states.
  fn conceal_panel(
      panel: &tauri::WebviewWindow,
      gate: &PanelLayoutGate,
  ) -> Result<(), IpcError> {
      gate.disarm();
      panel.hide().map_err(|_| IpcError::PanelUnavailable)
  }
  ```
  그리고 `hide_panel`(465-474행)의 본문을 `conceal_panel(&window, gate.inner())`로 교체한다. 주석 `// Stop a pending grace timer...`는 `conceal_panel`의 doc comment로 옮겨졌으므로 커맨드 쪽에서는 제거한다.
- **MIRROR**: IPC_COMMAND (인가 → 실행), BEST_EFFORT_SIDE_EFFECT (`hide()`는 에러로 승격)
- **IMPORTS**: 없음 (`Ordering`은 이미 `use std::sync::atomic::{AtomicBool, Ordering}`로 import되어 있다).
- **GOTCHA**: `State<'_, PanelLayoutGate>`를 `&PanelLayoutGate` 파라미터에 넘길 때 **`&gate`는 컴파일되지 않는다**(그건 `&State<...>`다). 함수 인자에는 자동 deref가 적용되지 않으므로 `gate.inner()`(또는 `&*gate`)를 쓴다. `inner()` 쪽이 의도가 명확하다.
- **GOTCHA**: `hide_panel`은 여전히 `window`(= 호출한 패널 창 자신)를 숨긴다. `toggle_panel`은 오버레이에서 호출되므로 **절대 `window`를 숨기면 안 된다** — `app.get_webview_window("panel")`로 얻은 창을 숨겨야 한다. 이걸 놓치면 더블클릭이 펫을 숨긴다.
- **VALIDATE**: `cargo check --manifest-path src-tauri/Cargo.toml` — `show_panel` 관련 에러만 남는다.

### Task 6: `show_panel`을 `toggle_panel`로 대체

- **ACTION**: `src-tauri/src/refresh/ipc.rs:328-365`의 `show_panel`을, reveal 본체를 분리한 뒤 `toggle_panel`로 교체한다.
- **IMPLEMENT**:
  ```rust
  /// The state the panel will be in once this toggle settles — and the state the
  /// next toggle will read.
  ///
  /// Returned so the native boundary can be exercised end to end without a
  /// visibility query command. The renderer must not cache it: the `✕`, the
  /// hide/show hotkey and the fullscreen monitor all change panel visibility
  /// without going through the renderer at all.
  #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
  #[serde(rename_all = "snake_case")]
  pub enum PanelVisibility {
      Shown,
      Hidden,
  }

  /// Anchors the panel beside the pet and arms the layout gate.
  ///
  /// The panel is placed before it is revealed even though the renderer may
  /// resize it in a moment: the grace timer below can reveal the panel without a
  /// second placement pass.
  fn begin_reveal(
      anchor: &tauri::WebviewWindow,
      panel: &tauri::WebviewWindow,
      app: &AppHandle,
      gate: &PanelLayoutGate,
  ) -> Result<(), IpcError> {
      position_panel(anchor, panel)?;
      gate.awaiting_layout.store(true, Ordering::SeqCst);

      let deadline_app = app.clone();
      tauri::async_runtime::spawn(async move {
          tokio::time::sleep(PANEL_LAYOUT_GRACE).await;
          if deadline_app
              .state::<PanelLayoutGate>()
              .awaiting_layout
              .swap(false, Ordering::SeqCst)
          {
              if let Some(panel) = deadline_app.get_webview_window("panel") {
                  let _ = reveal_panel(&panel);
              }
          }
      });
      Ok(())
  }

  /// Toggles the usage panel from the pet's double-click.
  ///
  /// The decision is made here rather than in the renderer because the panel's
  /// visibility is changed by paths the renderer never sees: the `✕`, the
  /// hide/show hotkey, and the fullscreen monitor.
  #[tauri::command]
  pub fn toggle_panel(
      window: tauri::WebviewWindow,
      app: AppHandle,
      gate: State<'_, PanelLayoutGate>,
  ) -> Result<PanelVisibility, IpcError> {
      authorize(&window, NativeCommand::TogglePanel)?;
      let panel = app
          .get_webview_window("panel")
          .ok_or(IpcError::PanelUnavailable)?;
      match panel_toggle(
          panel.is_visible().unwrap_or(false),
          gate.awaiting_layout.load(Ordering::SeqCst),
      ) {
          PanelToggle::Hide => {
              conceal_panel(&panel, gate.inner())?;
              Ok(PanelVisibility::Hidden)
          }
          PanelToggle::Show => {
              begin_reveal(&window, &panel, &app, gate.inner())?;
              Ok(PanelVisibility::Shown)
          }
      }
  }
  ```
  `use` 목록(16-17행)에서 `panel_reveal, PanelReveal`을 `panel_toggle, PanelToggle`로 교체한다.
- **MIRROR**: IPC_COMMAND, WIRE_DTO, NATIVE_POLICY_FUNCTION 호출부(`match` 한 줄)
- **IMPORTS**: `crate::window::{command_allowed, panel_toggle, CapabilityDiagnostic, HideShowHotkeyCapability, NativeCommand, PanelToggle, PlatformCapabilities}`.
- **GOTCHA**: `position_panel`은 **Show 분기 안에서만** 호출한다. 기존 `show_panel`은 가시성 검사보다 먼저 배치했는데(이미 열린 패널을 재앵커하기 위해), 토글에서는 Hide 분기에서 배치가 의미 없다.
- **GOTCHA**: `reveal_panel`(함수)은 삭제하지 않는다 — `begin_reveal`의 유예 타이머와 `resize_panel`(459-461행)이 여전히 쓴다.
- **GOTCHA**: `PANEL_LAYOUT_GRACE` 주석(30-33행)이 `show_panel`을 이름으로 가리킨다. `toggle_panel`로 갱신할 것. `PanelLayoutGate`의 doc comment(35-37행)도 마찬가지.
- **VALIDATE**: `cargo check --manifest-path src-tauri/Cargo.toml` → `lib.rs`의 핸들러 등록 에러만 남는다.

### Task 7: 핸들러 등록 교체 + 네이티브 숨김 경로에서 게이트 해제

- **ACTION**: `src-tauri/src/lib.rs`를 세 곳 고친다.
- **IMPLEMENT**:
  1. `invoke_handler!`(134-149행): `refresh::ipc::show_panel,` → `refresh::ipc::toggle_panel,`
  2. `toggle_overlay_visibility`(216-222행)의 패널 숨김:
     ```rust
      if hidden {
          let _ = overlay.hide();
          if let Some(panel) = app.get_webview_window("panel") {
              // A reveal armed moments ago would otherwise put the panel back on
              // screen while the pet it belongs to is hidden.
              if let Some(gate) = app.try_state::<refresh::ipc::PanelLayoutGate>() {
                  gate.disarm();
              }
              let _ = panel.hide();
          }
          return;
      }
     ```
  3. `start_fullscreen_monitor`(277-281행)의 패널 숨김에 같은 `try_state` + `disarm`을 적용한다.
- **MIRROR**: 기존 `let _ = panel.hide();` 베스트에포트 스타일 유지.
- **IMPORTS**: `toggle_overlay_visibility`와 `start_fullscreen_monitor` 모두 함수 안에서 이미 `use tauri::Manager;`를 한다. `try_state`도 `Manager` 트레이트 메서드이므로 추가 import 불필요.
- **GOTCHA**: **`state::<T>()`가 아니라 `try_state::<T>()`를 써야 한다.** `PanelLayoutGate`는 setup의 127행에서 관리 등록되는데, 전역 단축키 플러그인은 빌더 체인에서 이미 살아 있다. 단축키가 그 사이에 눌리면 `state::<PanelLayoutGate>()`는 **패닉**한다 — `lib.rs:55-58` 주석이 `OverlayHideGate`에 대해 경고하는 바로 그 상황이다. `try_state`는 `Option`을 돌려주므로 순서 변경 없이 안전하다.
- **VALIDATE**: `cargo test --manifest-path src-tauri/Cargo.toml --all-features` → 전체 통과. `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` 통과.

### Task 8: 게이트웨이 계약 갱신 (렌더러 경계)

- **ACTION**: `src/lib/api/gateway.ts`에서 `showPanel`을 `togglePanel`로 교체한다.
- **IMPLEMENT**:
  - `CollectorMode` 타입 선언 근처(67행 부근)에 추가:
    ```typescript
    /**
     * The state the panel settles into after a toggle. Reported for diagnostics
     * and tests only — panel visibility also changes through the `✕`, the
     * hide/show hotkey and the fullscreen monitor, so the renderer must never
     * cache this as its own copy of the panel state.
     */
    export type PanelVisibility = 'shown' | 'hidden';
    ```
  - `AppGateway`(95행)의 `showPanel(): Promise<void>;`를 교체:
    ```typescript
      /** Authorized for the `overlay` window only (`window::command_allowed`). */
      togglePanel(): Promise<PanelVisibility>;
    ```
  - `tauriGateway`(250행): `showPanel: () => invokeNative('show_panel'),` → `togglePanel: () => invokeNative('toggle_panel'),`
- **MIRROR**: GATEWAY_METHOD, WIRE_DTO
- **IMPORTS**: 없음.
- **GOTCHA**: `AppGateway` 안의 메서드 순서는 `showPanel` → `resizePanel` → `hidePanel` → `quit`이다. 같은 자리에 `togglePanel`을 둬서 diff를 최소화한다.
- **VALIDATE**: `pnpm check` → `App.svelte`, `fixtureGateway.ts`, 테스트 파일들에서만 에러(다음 Task에서 해소).

### Task 9: 픽스처 게이트웨이 동기화

- **ACTION**: `src/lib/api/fixtureGateway.ts:113`의 `showPanel: async () => undefined,`를 교체한다.
- **IMPLEMENT**: `togglePanel: async () => 'shown',`
- **MIRROR**: 파일 내 다른 async 화살표 스텁들과 동일한 형태.
- **IMPORTS**: 없음 (객체에 `AppGateway` 타입 주석이 붙어 있어 반환 리터럴이 `PanelVisibility`로 좁혀진다).
- **GOTCHA**: 이 픽스처는 렌더러 전용 E2E(`?fixture=e2e`)에서 쓰이며 네이티브 창이 없다. `'shown'` 고정으로 충분하다 — 여기서 상태를 흉내내면 실제 네이티브 판단과 다른 두 번째 진실이 생긴다.
- **VALIDATE**: `pnpm check`에서 이 파일 관련 에러 소멸.

### Task 10: `PetOverlay`의 prop과 접근성 라벨을 토글 의미로

- **ACTION**: `src/lib/components/PetOverlay.svelte`의 `onOpen`을 `onToggle`로 바꾸고 라벨을 갱신한다.
- **IMPLEMENT**:
  - 6행 JSDoc: `... onOpen?: () => void }` → `... onToggle?: () => void }`
  - 13행: `onOpen = () => {},` → `onToggle = () => {},`
  - 19행: `onOpen();` → `onToggle();`
  - 48행: `aria-label="Move pet; double-click or press Enter for usage"` → `aria-label="Move pet; double-click or press Enter to show or hide usage"`
  - 54행: `ondblclick={() => onOpen()}` → `ondblclick={() => onToggle()}`
- **MIRROR**: RENDERER_COMPONENT_CALLBACK
- **IMPORTS**: 없음.
- **GOTCHA**: 이 파일은 `<script lang="ts">`가 **아니다**. 타입은 상단 JSDoc 한 줄 블록에만 존재하므로 TS 문법을 넣지 말 것.
- **GOTCHA**: 키보드(`Enter`/`Space`) 경로도 같은 콜백을 공유하므로 자동으로 토글이 된다. 이것이 의도다 — 라벨을 바꾸는 이유이기도 하다.
- **VALIDATE**: Task 12의 컴포넌트 테스트에서 검증.

### Task 11: `App.svelte` 배선

- **ACTION**: `src/App.svelte:708`의 `onOpen={() => void gateway.showPanel()}`을 교체한다.
- **IMPLEMENT**:
  ```svelte
          onToggle={() => void gateway.togglePanel()}
  ```
- **MIRROR**: 같은 블록의 `onPointerDown` 등 다른 콜백과 동일한 `void` 처리.
- **IMPORTS**: 없음.
- **GOTCHA**: 반환값(`PanelVisibility`)을 **상태로 저장하지 않는다.** `void`로 버리는 것이 계약이다 — 저장하는 순간 `✕`/단축키가 바꾼 가시성과 어긋난다.
- **VALIDATE**: `pnpm check` 전체 통과 (테스트 파일 제외).

### Task 12: 컴포넌트 테스트 갱신

- **ACTION**: `src/lib/components/PetOverlay.test.ts`의 라벨 단언을 갱신하고, 토글 콜백 테스트를 추가한다.
- **IMPLEMENT**:
  - 43행: `name: 'Move pet; double-click or press Enter for usage',` → `name: 'Move pet; double-click or press Enter to show or hide usage',`
  - 새 테스트 추가(파일 끝, 마지막 `it.each` 뒤):
    ```typescript
    it('routes both the double-click and the Enter key to a single toggle request', async () => {
      const onToggle = vi.fn();
      render(PetOverlay, {
        props: {
          model: {
            system: 'active',
            stale: false,
            session: { usedPercent: 10, severity: 'ok' },
            weekly: { usedPercent: 10, severity: 'ok' },
            animation,
            petName: 'Geometric pet',
            size: 160,
          },
          onToggle,
        },
      });
      const surface = screen.getByTestId('overlay-pointer-surface');

      await fireEvent.dblClick(surface);
      expect(onToggle).toHaveBeenCalledOnce();
      await fireEvent.keyDown(surface, { key: 'Enter' });
      expect(onToggle).toHaveBeenCalledTimes(2);
    });
    ```
- **MIRROR**: TEST_STRUCTURE — 렌더러
- **IMPORTS**: 이 파일의 현재 import는 `{ cleanup, render, screen }`과 `{ afterEach, describe, expect, it }`뿐이다. `fireEvent`(@testing-library/svelte)와 `vi`(vitest)를 **추가해야 한다**.
- **GOTCHA**: 컴포넌트는 정책을 모른다 — 여기서 "패널이 열렸는지"를 단언하지 말 것. 콜백이 정확히 한 번 불리는지만 본다.
- **VALIDATE**: `pnpm vitest run src/lib/components/PetOverlay.test.ts` → 통과.

### Task 13: 게이트웨이 테스트 갱신

- **ACTION**: `src/lib/api/gateway.test.ts:147,153`의 `showPanel`/`show_panel` 단언을 교체한다.
- **IMPLEMENT**:
  - 147행: `await tauriGateway.showPanel();` → `await tauriGateway.togglePanel();`
  - 153행: `expect(invoked).toHaveBeenCalledWith('show_panel', {});` → `expect(invoked).toHaveBeenCalledWith('toggle_panel', {});`
- **MIRROR**: 같은 테스트의 `hide_panel`/`quit` 단언 형태 그대로.
- **IMPORTS**: 없음.
- **GOTCHA**: 이 테스트의 `mockIPC` 핸들러(109-128행)는 커맨드별 분기 뒤 `return null`로 끝난다. `toggle_panel`은 분기가 없어 `null`을 받지만, 반환값을 쓰지 않으므로 그대로 통과한다.
- **VALIDATE**: `pnpm vitest run src/lib/api/gateway.test.ts` → 통과.

### Task 14: `App.test.ts` 갱신 및 토글 왕복 테스트 추가

- **ACTION**: 목 게이트웨이와 관련 단언을 갱신하고, 순환 단언을 추가한다.
- **IMPLEMENT**:
  - 143행: `showPanel: vi.fn(async () => undefined),` → `togglePanel: vi.fn(async () => 'shown' as const),`
  - 170행 테스트 이름: `'hydrates overlay and opens the panel only on a circular-surface double-click'` → `'hydrates overlay and toggles the panel only on a circular-surface double-click'`
  - 185, 187행: `gateway.showPanel` → `gateway.togglePanel`
  - 933행: `expect(gateway.showPanel).not.toHaveBeenCalled();` → `expect(gateway.togglePanel).not.toHaveBeenCalled();`
  - 170행 테스트에서 187행 단언 뒤(191행 `getPlatformCapabilities` 단언 앞)에 왕복 단언을 덧붙인다:
    ```typescript
    // Open/close/open: the gesture is uniform, and the renderer keeps no copy of
    // the panel state — every double-click is one request to the same native
    // decision.
    await fireEvent.dblClick(overlay);
    await fireEvent.dblClick(overlay);
    expect(gateway.togglePanel).toHaveBeenCalledTimes(3);
    ```
  - 987행 `hides the panel through the close control without quitting CacheBite` 테스트에 드리프트 방지 단언을 추가한다:
    ```typescript
    expect(gateway.togglePanel).not.toHaveBeenCalled();
    ```
- **MIRROR**: TEST_STRUCTURE — 렌더러 (네거티브 단언 우선)
- **IMPORTS**: 없음.
- **GOTCHA**: 목의 반환 타입을 `'shown' as const`로 두지 않으면 `string`으로 넓어져 `AppGateway` 타입과 불일치한다.
- **VALIDATE**: `pnpm vitest run src/App.test.ts` → 통과.

### Task 15: 네이티브 E2E — 열기/닫기/열기 순환과 권한 경계

- **ACTION**: `tests/e2e/native.spec.ts`에 두 테스트를 추가한다. 기존 114행 테스트(`authorizes history IPC only from the panel window`)는 첫 더블클릭으로 패널을 여는 동작에 의존하므로 **그대로 통과하며 수정하지 않는다**.
- **IMPLEMENT**:
  ```typescript
  it('toggles the panel from the pet and returns to the state it started in', async () => {
    await switchToCacheBiteWindow('overlay');
    const toggle = async () => {
      const result = await invokeFromCurrentWindow<'shown' | 'hidden'>(
        'toggle_panel',
      );
      if (result.status !== 'resolved')
        throw new Error(`toggle_panel was rejected: ${result.reason}`);
      return result.value;
    };

    // Asserted as an alternating cycle rather than against fixed values: the
    // panel's starting visibility depends on what ran before, and a test that
    // assumed 'hidden' would be order-dependent.
    const first = await toggle();
    const second = await toggle();
    const third = await toggle();

    expect(second).not.toBe(first);
    expect(third).toBe(first);
  });

  it('authorizes the panel toggle only from the overlay window', async () => {
    await switchToCacheBiteWindow('overlay');
    await $(
      'main[data-window-label="overlay"] [data-testid="overlay-pointer-surface"]',
    ).doubleClick();
    await switchToCacheBiteWindow('panel');
    await expect($('section[aria-label="Usage panel"]')).toExist();

    // The panel dismisses itself with `hide_panel`; the toggle is the pet's.
    expect(await invokeFromCurrentWindow('toggle_panel')).toEqual({
      status: 'rejected',
      reason: 'forbidden',
    });
  });
  ```
- **MIRROR**: TEST_STRUCTURE — 네이티브 E2E (`invokeFromCurrentWindow`, `{ status: 'rejected', reason: 'forbidden' }`)
- **IMPORTS**: 없음 — 두 헬퍼 모두 같은 `describe` 스코프에 이미 있다.
- **GOTCHA**: 두 번째 테스트는 패널 창으로 스위치한 뒤 끝나므로, `beforeEach`의 `switchToCacheBiteWindow('overlay')`가 다음 테스트를 정상화한다. 첫 번째 테스트 안에서도 명시적으로 오버레이로 스위치해 순서 의존을 없앤다.
- **GOTCHA**: 첫 번째 테스트는 더블클릭이 아니라 IPC를 직접 호출한다. 제스처→IPC 배선은 렌더러 유닛 테스트(Task 14)와 기존 114행 E2E가 덮고, 여기서는 **네이티브 판단의 왕복**만 본다. 두 층을 한 테스트에 섞으면 실패 원인이 모호해진다.
- **VALIDATE**: `pnpm test:e2e` (로컬에서 네이티브 빌드 필요). CI에서는 `native-smoke.yml`이 픽스처 모드로 실행한다.

### Task 16: 계약 문서 갱신 (`docs/ui-contract.md`)

- **ACTION**: §4.3과 §5의 패널 가시성 문구를 토글 계약으로 바꾼다.
- **IMPLEMENT**:
  - 162행 `- 이동 거리 < \`DRAG_THRESHOLD_PX\`인 클릭(release) → 패널 토글.`을 실제 구현(더블클릭)과 맞춘다:
    ```markdown
    - 이동 거리 < `DRAG_THRESHOLD_PX`인 더블클릭 → 패널 토글(`toggle_panel`). 단일 클릭은 아무것도 열지 않는다.
    ```
  - 199행에서 `always-on-top을 거부하는 컴포지터에서는 ... 그 경우 펫 더블클릭의 전면화가 복구 경로다.` 구절을 삭제하고, 닫기 수단 문장을 다음으로 교체:
    ```markdown
    닫기 수단은 두 가지이며 둘 다 명시적 제스처다: 헤더 우측 상단의 `✕`, 그리고 펫 더블클릭. 둘 다 패널만 숨긴다(프로세스는 계속 실행된다). 포커스 상실이나 외부 클릭으로는 닫히지 않는다.
    ```
  - 203행(`- 펫 더블클릭은 **토글이 아니라 표시·전면화**다. ...`) 전체를 두 불릿으로 교체:
    ```markdown
    - 펫 더블클릭은 **토글**이다. 숨겨져 있으면 열고 포커스를 주며, 보이면 숨긴다. 판단은 네이티브가 패널의 실제 가시성으로 내린다(`panel_toggle`) — 렌더러는 패널 상태 사본을 갖지 않는다. 표시 요청 직후 렌더러 높이 측정을 기다리는 동안(`PANEL_LAYOUT_GRACE`)은 아직 화면에 없어도 "보이는 것"으로 취급한다. 그래야 연속 더블클릭의 두 번째가 방금 건 표시를 취소한다.
    - always-on-top을 거부하는 컴포지터에서 패널이 다른 창 뒤로 밀린 경우, 더블클릭 한 번은 그것을 **숨긴다**(전면화가 아니다). 복구는 한 번 더 더블클릭하는 것이며, 이때 표시 경로가 전면화와 포커스를 함께 수행한다. 토글의 예측 가능성을 포커스 기반 분기보다 우선한 결과다.
    ```
- **MIRROR**: 문서 전체가 한국어 서술 + 백틱 식별자 형식.
- **IMPORTS**: 해당 없음.
- **GOTCHA**: 199행의 "그 경우 펫 더블클릭의 전면화가 복구 경로다"는 이제 **틀린 문장**이 된다. 위 두 번째 불릿이 이를 대체하므로 199행에서 반드시 걷어낸다.
- **VALIDATE**: `pnpm lint` (prettier가 md도 검사한다) → 통과.

### Task 17: 베타 문서와 CLAUDE.md 불변식 갱신

- **ACTION**: `docs/beta-testing.md`와 `CLAUDE.md`를 새 동작에 맞춘다.
- **IMPLEMENT**:
  - `docs/beta-testing.md:86`:
    ```markdown
    - **Double-click** the pet to open the usage panel — and double-click it again
      to hide the panel. The same gesture does both.
    ```
  - `docs/beta-testing.md:97-99` 교체:
    ```markdown
    - The panel stays on top until you close it. Double-click the pet, or click the
      **×** at its top-right — either one hides it and CacheBite keeps running.
    ```
  - "What this beta is looking for" 목록(105-121행) 끝에 항목 추가:
    ```markdown
    6. **Panel toggle.** Double-click the pet repeatedly, including two clicks in
       quick succession. The panel must alternate open and closed every time and
       never end up in a state where the gesture stops responding.
    ```
  - `CLAUDE.md`의 "Panel visibility policy" 불변식 전체를 교체:
    ```markdown
    - **Panel visibility policy:** the panel is always-on-top and is dismissed only by explicit gestures — its `✕` control or a pet double-click — never by focus loss or an outside click. `toggle_panel` decides from the panel's real visibility plus any pending reveal, so the renderer never keeps its own copy of the panel state. The footer `Quit` is `app.exit(0)`, not a panel hide.
    ```
- **MIRROR**: 두 문서 모두 영문, 불릿, 백틱 식별자.
- **IMPORTS**: 해당 없음.
- **GOTCHA**: `CLAUDE.md`의 기존 문장에는 `show_panel must raise and focus an already-visible panel rather than returning early, or a double-click on the pet becomes a no-op`가 들어 있다. `show_panel`은 사라졌고 raise 동작도 토글로 대체됐으므로 **이 문장을 남겨두면 안 된다** — 위 교체문이 통째로 대신한다.
- **VALIDATE**: `pnpm lint` 통과. `git grep -n "show_panel"` → `docs/code-review/**`와 `.claude/PRPs/**`, `docs/superpowers/**`(과거 기록)만 남아야 한다.

### Task 18: 전체 게이트 실행 및 diff 감사

- **ACTION**: 전체 검증을 순서대로 돌리고 diff를 읽는다.
- **IMPLEMENT**: §Validation Commands 전부 실행.
- **MIRROR**: 해당 없음.
- **GOTCHA**: `git grep -n "showPanel\|show_panel\|onOpen"`으로 잔여 참조를 확인한다. `docs/superpowers/plans/2026-07-29-overlay-toast-notification.md:165`에도 `onOpen={() => void gateway.showPanel()}`이 있으나, 이는 **완료된 과거 계획 문서의 코드 인용**이므로 고치지 않는다(이력 기록).
- **VALIDATE**: 아래 전체 명령이 모두 통과.

---

## Testing Strategy

### Unit Tests

| Test | Layer | Input | Expected Output | Edge Case? |
|---|---|---|---|---|
| `a_visible_or_pending_panel_is_hidden_and_a_hidden_one_is_shown` | Rust 정책 | `(false, false)` | `PanelToggle::Show` | |
| ⟶ | | `(true, false)` | `PanelToggle::Hide` | |
| ⟶ | | `(false, true)` | `PanelToggle::Hide` | ✅ 연타 중 대기 reveal |
| ⟶ | | `(true, true)` | `PanelToggle::Hide` | ✅ 표시+대기 동시 |
| `native_commands_are_authorized_by_window_label` | Rust 권한 | `("overlay", TogglePanel)` | `true` | |
| ⟶ | | `("panel", TogglePanel)` | `false` | ✅ 권한 축소 |
| ⟶ | | `("unknown", TogglePanel)` | `false` | ✅ |
| `routes both the double-click and the Enter key to a single toggle request` | 컴포넌트 | dblclick, Enter | 콜백 2회 | ✅ 키보드 동등성 |
| `hydrates overlay and toggles the panel only on a circular-surface double-click` | 앱 | 단일 클릭 → dblclick ×3 | `togglePanel` 3회, 단일 클릭으로는 0회 | ✅ 열기/닫기/열기 |
| `hides the panel through the close control without quitting CacheBite` | 앱 | `✕` 클릭 | `hidePanel` 1회, `togglePanel` 0회, `quit` 0회 | ✅ 경로 분리 |
| `exits CacheBite through the footer Quit button` | 앱 | Quit 클릭 | `quit` 1회, `hidePanel` 0회 | |
| gateway invoke 단언 | 게이트웨이 | `togglePanel()` | `invoke('toggle_panel', {})` | |
| `toggles the panel from the pet and returns to the state it started in` | 네이티브 E2E | `toggle_panel` ×3 | 교대 후 원상복귀 | ✅ 완전 순환 |
| `authorizes the panel toggle only from the overlay window` | 네이티브 E2E | 패널에서 호출 | `forbidden` | ✅ |

### Edge Cases Checklist

- [x] **연타(150ms 이내 두 번째 더블클릭)** — `panel_toggle(false, true) == Hide`. `conceal_panel`이 게이트를 내려 유예 타이머가 부활시키지 못한다.
- [x] **드래그 중 더블클릭** — 드래그는 `pointermove` 경로이고 `dblclick`은 별개 이벤트다. `DRAG_THRESHOLD_PX`를 넘은 제스처는 네이티브 드래그 루프로 넘어가 `dblclick`을 만들지 않는다. 기존 `starts native dragging once when pointer movement crosses the threshold` 테스트가 회귀를 잡는다.
- [x] **최소화된 패널** — `is_visible() == true`를 보고하므로 Hide로 간다. 이후 더블클릭 시 `reveal_panel`이 `unminimize()` → `show()` → `set_focus()` 순으로 복구한다.
- [x] **`✕`로 닫은 직후 더블클릭** — `hide_panel`도 `conceal_panel`을 거쳐 게이트를 내리므로 다음 토글은 깨끗한 Show다.
- [x] **`Ctrl+Shift+H`로 숨긴 직후** — Task 7의 `disarm`이 유예 타이머를 끈다. 펫이 숨겨진 채 패널만 되살아나지 않는다.
- [x] **패널 창이 없음** — `get_webview_window("panel")`이 `None` → `IpcError::PanelUnavailable`. 렌더러는 `void`로 무시하고 오버레이는 계속 동작한다.
- [x] **헤드리스/Wayland에서 포커스 거부** — `set_focus()`/`unminimize()`는 `let _ =` 유지. `native-smoke.yml` 픽스처 잡이 깨지지 않는다.
- [x] **모니터 없음(헤드리스)** — `position_panel`은 `Ok(())`로 조용히 건너뛴다(기존 정책 유지).
- [x] **백그라운드 폴링** — 이번 변경은 `refresh` 액터를 건드리지 않는다. 숨김은 창 표시만 바꾼다.
- [ ] 네트워크 실패 — 해당 없음(창 정책 변경).
- [ ] 권한 거부 — capability 파일 변경 없음(자체 커맨드는 선언 대상이 아님).

---

## Validation Commands

### Static Analysis

```bash
pnpm check
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
```
EXPECT: 타입 에러 0, 포맷 차이 없음, clippy 경고 0.

### Unit Tests

```bash
cargo test --manifest-path src-tauri/Cargo.toml window::tests
pnpm vitest run src/lib/components/PetOverlay.test.ts src/lib/api/gateway.test.ts src/App.test.ts
```
EXPECT: 전부 통과.

### Full Test Suite

```bash
cargo test --manifest-path src-tauri/Cargo.toml --all-features
pnpm test:ci
```
EXPECT: 회귀 없음. `pnpm test:ci`는 svelte-check + eslint + prettier + vitest 커버리지(브랜치/함수/라인/구문 80%) + vite build를 모두 돌린다.

### Browser / E2E Validation

```bash
pnpm test:e2e:renderer
pnpm test:e2e
```
EXPECT: 렌더러 E2E는 기존 6개 그대로 통과(이 변경으로 영향 없음). 네이티브 E2E는 신규 2개 포함 통과.

### Manual Validation

- [ ] `pnpm tauri dev`로 실행 후 펫 더블클릭 → 패널이 펫 옆에 열리고 포커스를 받는다.
- [ ] 다시 더블클릭 → 패널이 사라지고 **펫과 프로세스는 그대로**다.
- [ ] 한 번 더 더블클릭 → 다시 열린다(완전 순환).
- [ ] 아주 빠르게 4회 더블클릭 → 최종 상태가 "닫힘"이고, 잠시 뒤 패널이 저절로 나타나지 않는다(유예 타이머 부활 없음).
- [ ] 패널이 열린 상태에서 `✕` → 닫힘. 이어서 더블클릭 → 열림(두 경로가 어긋나지 않는다).
- [ ] 패널이 열린 상태에서 `Ctrl+Shift+H` → 펫과 패널이 함께 사라진다. 다시 `Ctrl+Shift+H` → 펫만 돌아오고 패널은 닫힌 채다. 이어서 더블클릭 → 패널이 열린다.
- [ ] 펫에 포커스를 준 뒤 `Enter` 두 번 → 열림/닫힘.
- [ ] 펫을 드래그(임계값 초과) → 패널이 열리거나 닫히지 않는다.
- [ ] 패널을 열어둔 채 펫을 다른 디스플레이로 드래그 → 닫고 다시 열면 그 디스플레이 작업 영역 안에 앵커된다.

---

## Acceptance Criteria

- [ ] 숨김 → 더블클릭 → 표시
- [ ] 표시 → 더블클릭 → 숨김 (프로세스 종료 없음, 폴링 계속)
- [ ] 열기 → 닫기 → 열기 완전 순환이 유닛·E2E 두 층에서 검증됨
- [ ] 토글 판단이 네이티브의 실제 가시성 + 대기 중 reveal에서 나옴 (렌더러 상태 사본 없음)
- [ ] `✕`와 `Quit`의 기존 동작이 그대로임
- [ ] `toggle_panel`이 `overlay`에서만 허용되고 `panel`/`unknown`에서는 `forbidden`
- [ ] 모든 검증 명령 통과, 커버리지 80% 유지
- [ ] `docs/ui-contract.md`·`docs/beta-testing.md`·`CLAUDE.md`가 새 동작과 일치

## Completion Checklist

- [ ] 정책은 `window/mod.rs`의 순수 함수, 호출부는 `match` 한 줄 (기존 패턴 준수)
- [ ] 에러는 타입화된 `IpcError`, 원시 에러 미전파
- [ ] `show()`는 에러 승격, `set_focus()`/`unminimize()`는 베스트에포트 유지
- [ ] 새 로그 없음 / 프라이버시 계약 무관 (자격증명·계정 식별자·경로 미노출)
- [ ] 하드코딩 값 없음 (`PANEL_LAYOUT_GRACE` 등 기존 상수 재사용)
- [ ] `git grep "show_panel\|showPanel\|onOpen"` 잔여물 없음 (과거 기록 문서 제외)
- [ ] capability 파일 변경 없음 (자체 커맨드는 선언 불필요)
- [ ] 불필요한 범위 확장 없음 (§NOT Building 준수)

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| **다른 창에 가려진 패널을 더블클릭하면 "전면화" 대신 "숨김"** — `34d4677`이 넣은 raise 동작의 의도적 후퇴 | 중 (always-on-top 거부 컴포지터에서만) | 중 | 이슈가 요구한 계약이다. 복구는 더블클릭 한 번 더(그때 표시 경로가 raise+focus 수행). Windows/macOS에서는 always-on-top이 동작하므로 실질 노출이 낮다. `ui-contract.md`에 명시 |
| 연타 시 유예 타이머가 숨긴 패널을 되살림 | 높음 (대책 없을 경우) | 높음 | `panel_toggle`이 `reveal_pending`을 가시성으로 취급 + `conceal_panel`이 게이트 해제. Rust 유닛 테스트로 4가지 조합 고정 |
| `toggle_overlay_visibility`에서 `state::<PanelLayoutGate>()` 사용 시 패닉 | 낮음 (타이밍 의존) | 높음 | `try_state`를 쓴다. `lib.rs:55-58` 주석이 경고하는 동일 클래스 문제 |
| `toggle_panel`이 `window`(오버레이)를 숨겨 펫이 사라짐 | 낮음 | 높음 | Hide 분기는 반드시 `get_webview_window("panel")` 결과를 숨긴다. Task 5·6의 GOTCHA로 명시, 수동 검증 항목에 포함 |
| 렌더러가 반환값을 상태로 캐시해 드리프트 재발 | 낮음 | 중 | 타입 doc comment + `App.svelte`의 `void` 처리 + `✕` 테스트의 `togglePanel` 미호출 단언 |
| `panel` 창 권한 축소가 미지의 호출부를 깬다 | 매우 낮음 | 낮음 | 렌더러 전역 검색으로 호출부가 `App.svelte:708` 하나뿐임을 확인. E2E가 `forbidden`을 고정 |

## Notes

- **왜 `show_panel`을 남기지 않는가**: 유일한 호출부가 펫 더블클릭이었다. 토글로 바꾸면 `show_panel`은 호출자 없는 IPC 표면으로 남는다. 이름을 바꾸는 편이 죽은 커맨드를 유지하는 것보다 낫고, `panel` 창의 잉여 권한도 함께 정리된다.
- **Alternatives Considered**
  - *렌더러가 가시성을 들고 토글*: 이슈가 명시적으로 금지. `✕`·단축키·전체화면 감시가 렌더러를 거치지 않고 패널을 숨기므로 즉시 어긋난다. 기각.
  - *가시성 조회 IPC 추가 후 렌더러가 판단*: 조회와 실행 사이 경합이 남고, IPC 표면이 하나 더 는다. 기각.
  - *포커스 기반 3단 판단(포커스 있으면 숨김, 없으면 전면화)*: 가려진 패널 문제는 풀리지만, 플랫폼별 포커스 보고가 신뢰할 수 없고 같은 제스처가 상황에 따라 다른 일을 해 예측 가능성을 잃는다. 기각.
  - *`show_panel` 유지 + `toggle_panel` 신설*: 죽은 커맨드가 남는다. 기각.
- **`PanelVisibility` 반환의 성격**: 이것은 조회 API가 아니라 **방금 수행한 일의 보고**다. E2E가 가시성 조회 커맨드를 새로 만들지 않고도 왕복을 단언할 수 있게 해주는 것이 유일한 소비처다. 렌더러는 버린다.
- **릴리스 순서**: Task 7이 `toggle_overlay_visibility`(#30에서 도입)를 건드린다. #30이 아직 머지 전이면 `feat/pet-hide-show-hotkey` 위에 쌓고, 머지된 뒤라면 `main` 기준 새 브랜치로 시작한다. 어느 쪽이든 #48과 #30이 같은 0.1.1 릴리스에 함께 들어간다.
