# Plan: 펫이 드래그 이후 `idle`에 고정되는 버그 수정

## Summary

오버레이 펫이 사용량 무드(`idle_warn`/`idle_critical`/`idle_exhausted`) 대신 기본 `idle`로 고정되는 버그를 수정한다. 근본 원인은 하나다 — Tauri 네이티브 창 드래그(`startDragging()`)가 시작되면 OS가 마우스 루프를 가져가면서 웹뷰가 `pointerup`을 영영 받지 못하고, `interactionStore.dragging`이 `true`로 래치된 채 남는다. 번들 펫에는 `dragging` 에셋이 없으므로 이 래치는 곧 "영구 `idle`"을 의미한다. 다음 클릭의 `pointerup`이 래치를 풀기 때문에 "한 번 클릭하면 고쳐지는" 증상이 나온다.

## User Story

As a CacheBite 사용자,
I want 펫을 드래그해 옮기거나 설정/주 provider를 바꾼 뒤에도 펫이 내 실제 사용량 상태를 계속 보여주기를,
So that 펫을 다시 클릭하지 않고도 한도에 얼마나 근접했는지 한눈에 알 수 있다.

## Problem → Solution

**현재:** 드래그(또는 4px 이상 흔들린 더블클릭) 한 번이면 `dragging` 래치가 세션 내내 `true`로 남는다 → 펫이 항상 `idle`, 말풍선도 영구 억제 → 펫을 한 번 클릭해야 복구된다.

**목표:** 드래그 종료를 웹뷰의 `pointerup` 하나에만 의존하지 않고 여러 신호로 확정한다. 추가로 패키지에 `dragging` 에셋이 없으면 `dragging` 키를 아예 요청하지 않아 드래그 중에도 무드가 유지된다.

## Metadata

- **Complexity**: Small
- **Source PRD**: N/A
- **PRD Phase**: N/A
- **Estimated Files**: 6 (소스 3, 테스트 3) + 문서 1

---

## Root Cause Analysis

### 증상 → 코드 경로

두 증상 모두 단일 원인에서 나온다.

**1. 래치가 걸리는 지점 — `src/App.svelte:555-561`**

```ts
const pointerMove = (event: PointerEvent) => {
  if (!pointer) return;
  const wasDragging = pointer.dragging;
  pointer = updatePointer(pointer, { x: event.clientX, y: event.clientY });
  interactionStore.setDragging(pointer.dragging);
  if (!wasDragging && pointer.dragging) void gateway.startDragging();
};
```

`DRAG_THRESHOLD_PX = 4` (`src/lib/interaction/petPointer.ts:1`)를 넘으면 `dragging = true`가 되고 `gateway.startDragging()`이 호출된다.

**2. 래치를 푸는 유일한 경로가 사라진다 — `src/lib/api/gateway.ts:198`**

```ts
startDragging: () => getCurrentWindow().startDragging(),
```

Tauri의 `startDragging()`은 플랫폼 네이티브 이동 드래그를 시작한다 (Windows: `ReleaseCapture()` + `WM_NCLBUTTONDOWN`, GTK: `begin_move_drag`, macOS: `performWindowDragWithEvent`). **이 시점부터 OS/WM이 마우스 이벤트 루프를 소유하므로 웹뷰는 `pointerup`을 받지 못한다.** Windows의 `ReleaseCapture()`는 `setPointerCapture()`로 잡아둔 캡처까지 해제하므로 요소로 향하던 이벤트 경로도 끊긴다.

→ `src/App.svelte:562-573`의 `pointerUp`이 실행되지 않는다 → `interactionStore.setDragging(false)`가 호출되지 않는다 → `dragging`이 `true`로 고정된다.

플랫폼/WM에 따라 `pointercancel`이 발화하기도 하고 안 하기도 해서, 사용자가 말한 **"간헐적"** 성격이 그대로 설명된다.

**3. 래치가 `idle`로 보이는 이유 — `src/lib/assets/resolver.ts:39` + 번들 매니페스트**

```ts
if (context.dragging) return 'dragging';
```

그런데 번들 펫 두 개 모두 `dragging` 상태를 선언하지 않는다:

```
src-tauri/resources/pets/tabby/manifest.json → states: {idle, idle_warn, idle_critical, idle_exhausted}
src-tauri/resources/pets/corgi/manifest.json → states: {idle, idle_warn, idle_critical, idle_exhausted}
```

`resolvePetAnimation` (`src/lib/assets/resolver.ts:84-89`)은 선언되지 않은 키를 `manifest.animations.idle`로 폴백한다. 따라서 **`dragging` 래치 = 영구 기본 `idle`**.

**4. 다시 클릭하면 고쳐지는 이유**

다음 `pointerdown` → 짧은 `pointerup`(4px 미만이라 네이티브 드래그 없음) → `setDragging(false)` → `resolvedAnimation`이 재계산되어 `idle_critical` 등으로 복귀. 사용자가 관찰한 "펫을 한번 클릭해야 사용량에 맞는 상태로 변경됨"과 정확히 일치한다.

### 증상 A (설정/Set as primary)가 같은 원인인 이유

`resolvedAnimation`은 `$derived` (`src/App.svelte:458-470`)이고 `$interactionStore.dragging`을 읽는다.

- 펫 교체: `listenSettings` → `petChanged` → `loadPetPackage()` → `petPackage` 갱신 → `resolvedAnimation` 재계산. 이때 `dragging`이 여전히 `true`면 새 펫도 곧바로 기본 `idle`로 그려진다.
- Set as primary: `appSettings.primaryProvider` 변경 → `primaryUi` 재계산 → 마찬가지.

**"간헐적"의 트리거:** 패널을 여는 조작이 펫 **더블클릭**(`src/lib/components/PetOverlay.svelte:54`)이다. 더블클릭 도중 손이 4px 이상 흔들리면 `startDragging()`이 발화하고 래치가 걸린다. 사용자는 그 상태로 패널에서 provider/pet을 바꾸고, 그래서 펫이 `idle`로 나온다.

### 부수 피해 (같은 원인, 미보고)

`dragging: true`는 말풍선도 억제한다 — `src/lib/interaction/bubblePolicy.ts:51`:

```ts
() => !context.dragging && !context.fullscreen && (...)
```

→ 래치가 걸린 세션에서는 사용량 경고 말풍선이 **하나도** 뜨지 않는다. 본 수정으로 함께 해소된다.

### 배제한 가설

| 가설 | 판정 |
| --- | --- |
| `loadPetPackage()`가 `'idle'`로 프로브(`App.svelte:265`)해서 상태를 덮어씀 | ✗ 프로브는 검증용 호출이고 반환값을 버린다. `petPackage`에는 영향 없음 |
| `PetAnimation.svelte`의 `frameIndex = 0` 리셋 | ✗ 프레임 인덱스일 뿐 애니메이션 키와 무관 |
| `derivePetUiState`가 provider 전환 시 `loading`을 반환 | ✗ `engine.ts:123` — 스냅샷이 있으면 `active`. 두 provider 모두 부팅 시 `consume()`됨 |
| 다른 `setDragging` 호출처 존재 | ✗ `grep` 결과 `App.svelte:559`, `App.svelte:572` 두 곳뿐 |

### 관련 잠재 결함 (본 계획 범위 밖 — NOT Building 참조)

`system ∈ {unavailable, offline}`이면 `sleep` 키를 요청하는데(`resolver.ts:40-41`) 번들 펫에 `sleep` 에셋이 없어 기본 `idle`로 폴백된다. 다만 이 상태에서는 `petMood`가 항상 `'ok'`이므로(`engine.ts:125-134`) 보여줄 무드 자체가 없다 — 시각적으로 올바르며 사용자가 보고한 증상과 무관하다.

---

## UX Design

### Before

```
┌──────────────────────────────────────────────────────────┐
│ 1. 펫 더블클릭(손 4px 흔들림) 또는 드래그로 위치 이동      │
│      ↓  startDragging() → OS가 마우스 루프 인수           │
│ 2. pointerup 유실 → dragging = true 영구 래치             │
│      ↓                                                    │
│ 3. 펫: 사용량 91%인데도 기본 idle 표시                    │
│    말풍선: 영구 억제                                       │
│      ↓                                                    │
│ 4. 패널에서 pet 변경 / Set as primary → 여전히 idle       │
│      ↓                                                    │
│ 5. 펫을 한 번 클릭 → 그제서야 idle_critical 복귀          │
└──────────────────────────────────────────────────────────┘
```

### After

```
┌──────────────────────────────────────────────────────────┐
│ 1. 펫 더블클릭 또는 드래그로 위치 이동                     │
│      ↓  startDragging()                                   │
│ 2. 드래그 중: idle_critical 유지                          │
│    (패키지에 dragging 에셋이 있으면 dragging 재생)         │
│      ↓  버튼을 놓음                                        │
│ 3. pointerup / pointercancel / buttons==0 pointermove /   │
│    window blur / 다음 pointerdown 중 무엇이든 도달하면     │
│    래치 해제                                               │
│      ↓                                                    │
│ 4. 펫: idle_critical 계속 표시, 말풍선 정상 동작           │
│ 5. pet 변경 / Set as primary → 즉시 올바른 무드로 렌더     │
└──────────────────────────────────────────────────────────┘
```

### Interaction Changes

| Touchpoint | Before | After | Notes |
| --- | --- | --- | --- |
| 드래그 시작 | 기본 `idle`로 다운그레이드 | 무드 유지 (`dragging` 에셋 있으면 그것) | `AnimationContext.draggingAvailable` |
| 드래그 종료 (`pointerup` 도달) | 정상 복귀 | 변화 없음 | 해피 패스 유지 |
| 드래그 종료 (`pointerup` 유실) | **영구 `idle` 고정** | 창 레벨 신호로 복귀 | 핵심 수정 |
| 흔들린 더블클릭 | 래치 후 영구 `idle` | 정상 | 위와 동일 경로 |
| 펫/주 provider 변경 | 래치 상태면 `idle` | 항상 올바른 무드 | 래치 해소의 결과 |
| 말풍선 | 래치 후 영구 억제 | 정상 동작 | 래치 해소의 결과 |
| 다음 `pointerdown` | 래치 유지 | 무조건 래치 초기화 | 최후 방어선 |

---

## Mandatory Reading

| Priority | File | Lines | Why |
| --- | --- | --- | --- |
| P0 | `src/App.svelte` | 547-574 | 수정할 포인터 핸들러 4개 |
| P0 | `src/App.svelte` | 400-447 | `$effect`로 리스너를 등록/해제하는 기존 패턴 (미러 대상) |
| P0 | `src/App.svelte` | 449-499 | `$derived` 파생 체인 — `resolvedAnimation` 호출부 |
| P0 | `src/lib/assets/resolver.ts` | 11-43 | `RequestedAnimationKey` / `AnimationContext` / `requestedAnimationKey` |
| P0 | `src/lib/interaction/petPointer.ts` | 1-41 | 순수 리듀서 스타일 (미러 대상) |
| P1 | `src/lib/interaction/bubblePolicy.ts` | 40-54 | `dragging`의 두 번째 소비처 |
| P1 | `src/lib/assets/manifest.ts` | 6-15, 30-38 | `PET_STATES`, `PetManifest.states` 타입 |
| P1 | `src/lib/assets/manifest.test.ts` | 148-177 | 갱신할 `it.each` 테이블 |
| P1 | `src/App.test.ts` | 38-115 | 게이트웨이 픽스처 (`getPetPackage` 포함) |
| P1 | `src/App.test.ts` | 350-366 | 기존 드래그 테스트 — 확장 대상 |
| P2 | `docs/ui-contract.md` | 200-217 | §6 GIF 에셋 계약 — 갱신 필요 |
| P2 | `src/lib/components/PetOverlay.svelte` | 43-56 | 포인터 서피스 / `dblclick` |
| P2 | `src/lib/stores/interaction.ts` | 16-51 | `setDragging` 스토어 |

## External Documentation

| Topic | Source | Key Takeaway |
| --- | --- | --- |
| `Window.startDragging()` | Tauri v2 `@tauri-apps/api/window` | 플랫폼 네이티브 이동 드래그를 시작한다. 이후 마우스 이벤트 루프는 OS/WM 소유이며 웹뷰로의 `pointerup` 전달이 보장되지 않는다 |
| Windows 구현 | `WM_NCLBUTTONDOWN` 앞의 `ReleaseCapture()` | 명시적 포인터 캡처를 해제하므로 `setPointerCapture()`로 잡아둔 요소 경로가 끊긴다 |
| `PointerEvent.buttons` | MDN | `0` = 눌린 버튼 없음. 드래그 종료를 판정하는 결정적 신호 |

> 별도 외부 라이브러리 도입 없음. 저장소의 기존 패턴만 사용한다.

---

## Patterns to Mirror

### NAMING_CONVENTION — 순수 정책 함수는 동사/술어형 camelCase, 상수는 UPPER_SNAKE

```ts
// SOURCE: src/lib/interaction/petPointer.ts:1-20
export const DRAG_THRESHOLD_PX = 4;

export function beginPointer(origin: PointerPoint): PetPointerState {
  return { origin: { ...origin }, current: { ...origin }, dragging: false };
}
```

### PURE_POLICY_REDUCER — 동작은 Svelte 컴포넌트가 아니라 순수 함수에서 테스트한다 (CLAUDE.md 불변식)

```ts
// SOURCE: src/lib/interaction/petPointer.ts:37-41
export function releasePointer(state: PetPointerState): PointerRelease {
  return state.dragging
    ? { kind: 'finish_drag', position: { ...state.current } }
    : { kind: 'toggle_panel' };
}
```

### EFFECT_LISTENER_LIFECYCLE — `$effect`에서 리스너를 등록하고 cleanup에서 해제한다

```ts
// SOURCE: src/App.svelte:400-405
$effect(() => {
  const ticker = window.setInterval(() => {
    nowMs = Date.now();
  }, CLOCK_TICK_MS);
  return () => window.clearInterval(ticker);
});
```

```ts
// SOURCE: src/App.svelte:407-428 — 윈도 라벨 가드 + 조기 반환 + cleanup
$effect(() => {
  if (
    windowLabel !== 'panel' ||
    !panelShell ||
    typeof ResizeObserver === 'undefined'
  ) {
    return;
  }
  // ...
  const observer = new ResizeObserver(resize);
  observer.observe(panelShell);
  return () => observer.disconnect();
});
```

### RATIONALE_COMMENT — 자명하지 않은 결정에는 "왜"를 남긴다 (저장소 전반의 스타일)

```ts
// SOURCE: src/App.svelte:113-119
// `Date.now()` is not a reactive dependency, so a `$derived` that calls it
// only recomputes when some unrelated `$state` happens to change. This ticker
// is that dependency: it drives fresh→stale transitions and relative-time
// labels when no snapshot arrives.
```

### TEST_STRUCTURE_UNIT — vitest + `describe`/`it`, 표 기반은 `it.each`

```ts
// SOURCE: src/lib/interaction/petPointer.test.ts:1-12
import { describe, expect, it } from 'vitest';
import { beginPointer, releasePointer, updatePointer } from './petPointer';

describe('pet pointer policy', () => {
  it('toggles the panel below the four pixel boundary', () => {
    const state = updatePointer(beginPointer({ x: 10, y: 10 }), {
      x: 13.99,
      y: 10,
    });
    expect(releasePointer(state)).toEqual({ kind: 'toggle_panel' });
  });
});
```

```ts
// SOURCE: src/lib/assets/manifest.test.ts:148-164
describe('v1.1 animation resolution', () => {
  it.each([
    [{ system: 'auth_required', mood: 'exhausted', dragging: true }, 'idle'],
    // ...
  ] as const)('selects priority for %o', (context, expected) => {
    expect(requestedAnimationKey(context)).toBe(expected);
  });
});
```

### TEST_STRUCTURE_COMPONENT — 합성 루트 테스트는 수동 PointerEvent 조립

```ts
// SOURCE: src/App.test.ts:350-366
it('starts native dragging once when pointer movement crosses the threshold', async () => {
  const { gateway } = fixture();
  render(App, { props: { gateway, notificationAdapter: notifications } });
  const overlay = await screen.findByTestId('overlay-pointer-surface');
  const pointer = (type: string, x: number, y: number) => {
    const event = new Event(type, { bubbles: true });
    Object.defineProperties(event, {
      clientX: { value: x },
      clientY: { value: y },
    });
    return event;
  };
  await fireEvent(overlay, pointer('pointerdown', 10, 10));
  await fireEvent(overlay, pointer('pointermove', 20, 10));
  await fireEvent(overlay, pointer('pointermove', 30, 10));
  expect(gateway.startDragging).toHaveBeenCalledOnce();
});
```

### TEST_GATEWAY_OVERRIDE — 공유 픽스처를 건드리지 않고 개별 메서드만 덮어쓴다

```ts
// SOURCE: src/App.test.ts:298
vi.mocked(gateway.listenPositionMoved).mockResolvedValue(positionCleanup);
```

---

## Files to Change

| File | Action | Justification |
| --- | --- | --- |
| `src/lib/interaction/petPointer.ts` | UPDATE | 드래그 종료 판정 술어를 순수 정책으로 추가 |
| `src/lib/interaction/petPointer.test.ts` | UPDATE | 새 술어의 단위 테스트 |
| `src/lib/assets/resolver.ts` | UPDATE | `dragging` 에셋이 없으면 `dragging` 키를 요청하지 않음 |
| `src/lib/assets/manifest.test.ts` | UPDATE | `it.each` 테이블에 `draggingAvailable` 반영 + 신규 케이스 |
| `src/App.svelte` | UPDATE | 포인터 종료 경로 통합 + 창 레벨 안전망 + `draggingAvailable` 전달 |
| `src/App.test.ts` | UPDATE | `pointerup` 유실 시나리오 회귀 테스트 |
| `docs/ui-contract.md` | UPDATE | §6 `dragging` 행의 재생 조건 명문화 |

## NOT Building

- `sleep` 상태의 폴백 변경 — 해당 상태에서 `petMood`는 항상 `ok`이므로 기본 `idle`이 이미 정답이다.
- 번들 펫에 `dragging`/`sleep` 에셋 신규 제작 (`docs/UI-plan/` 아트 작업 + `scripts/build-pet-packages.py` 재생성).
- `resolvePetAnimation`의 폴백 체인 변경 — 계약 §6의 "요청 키 → `idle`, 중간 폴백 없음" 규칙은 그대로 유지한다. 바뀌는 것은 *어떤 키를 요청하는가*뿐이다.
- `DRAG_THRESHOLD_PX` 조정이나 더블클릭 판정 로직 변경.
- Rust/네이티브 측 변경 — 이 버그는 전적으로 렌더러 상태 관리 문제다.
- `PetOverlay` 컴포넌트의 prop 시그니처 변경 — 요소 레벨 핸들러는 그대로 유지한다.

---

## Step-by-Step Tasks

### Task 1: 드래그 종료 판정 술어 추가 (`petPointer.ts`)

- **ACTION**: `src/lib/interaction/petPointer.ts` 하단에 순수 술어를 추가한다.
- **IMPLEMENT**:
  ```ts
  /**
   * A native window drag (`startDragging()`) hands the mouse loop to the OS, so
   * the webview may never receive `pointerup` for that gesture. Any later
   * pointer event with no button held is proof the gesture ended, and is the
   * signal that releases the drag latch when `pointerup` was swallowed.
   */
  export function pointerButtonsReleased(buttons: number): boolean {
    return buttons === 0;
  }
  ```
- **MIRROR**: `PURE_POLICY_REDUCER` + `NAMING_CONVENTION` + `RATIONALE_COMMENT`.
- **IMPORTS**: 없음.
- **GOTCHA**: `buttons`는 `button`과 다르다. `button`은 어떤 버튼이 이벤트를 유발했는지(이동 중엔 `-1`), `buttons`는 현재 눌린 버튼 비트마스크다. 반드시 `buttons`를 쓸 것.
- **VALIDATE**: `pnpm vitest run src/lib/interaction/petPointer.test.ts`

### Task 2: 술어 단위 테스트 (`petPointer.test.ts`)

- **ACTION**: 기존 `describe('pet pointer policy')` 블록에 케이스를 추가한다.
- **IMPLEMENT**:
  ```ts
  it('treats a button-free pointer event as the end of the gesture', () => {
    expect(pointerButtonsReleased(0)).toBe(true);
    expect(pointerButtonsReleased(1)).toBe(false);
    expect(pointerButtonsReleased(3)).toBe(false);
  });
  ```
  import 문에 `pointerButtonsReleased`를 추가한다.
- **MIRROR**: `TEST_STRUCTURE_UNIT`.
- **IMPORTS**: `import { beginPointer, pointerButtonsReleased, releasePointer, updatePointer } from './petPointer';`
- **GOTCHA**: import 항목은 위 순서 그대로 두면 prettier 재정렬이 발생하지 않는다.
- **VALIDATE**: `pnpm vitest run src/lib/interaction/petPointer.test.ts`

### Task 3: `dragging` 키를 선언된 경우에만 요청 (`resolver.ts`)

- **ACTION**: `AnimationContext`에 필수 필드를 추가하고 `requestedAnimationKey`의 분기를 좁힌다.
- **IMPLEMENT**: `src/lib/assets/resolver.ts:18-43`을 다음으로 교체한다.
  ```ts
  export interface AnimationContext {
    readonly system:
      | 'active'
      | 'auth_required'
      | 'unavailable'
      | 'error'
      | 'offline'
      | 'loading';
    readonly mood: 'ok' | 'warn' | 'critical' | 'exhausted';
    readonly dragging: boolean;
    /**
     * Whether the loaded package declares a `dragging` state. Requesting a key
     * the package does not declare resolves to bare `idle`, which would erase
     * the usage mood for the whole gesture — so ask for it only when the
     * package can actually render it.
     */
    readonly draggingAvailable: boolean;
  }

  export function requestedAnimationKey(
    context: AnimationContext,
  ): RequestedAnimationKey {
    if (
      context.system === 'auth_required' ||
      context.system === 'error' ||
      context.system === 'loading'
    )
      return 'idle';
    if (context.dragging && context.draggingAvailable) return 'dragging';
    if (context.system === 'unavailable' || context.system === 'offline')
      return 'sleep';
    return context.mood === 'ok' ? 'idle' : `idle_${context.mood}`;
  }
  ```
- **MIRROR**: `RATIONALE_COMMENT`. 폴백 체인(`resolvePetAnimation`)은 **손대지 않는다**.
- **IMPORTS**: 변경 없음.
- **GOTCHA**: `draggingAvailable`을 선택 필드가 아닌 **필수**로 둔다 — 호출부가 한 곳뿐이고, 누락 시 조용히 예전 동작으로 되돌아가는 것을 타입으로 막는다. 부수 효과: `system === 'unavailable' && dragging && !draggingAvailable`이면 이제 `'sleep'`을 요청한다 (이전엔 `'dragging'`). 번들 펫에는 `sleep`도 없으므로 시각적으로 동일하게 `idle`이며, 의미상 더 정확하다.
- **VALIDATE**: `pnpm vitest run src/lib/assets/manifest.test.ts` (Task 4 이후 통과)

### Task 4: 애니메이션 키 테이블 갱신 (`manifest.test.ts`)

- **ACTION**: `describe('v1.1 animation resolution')`의 `it.each` 테이블 8행 전부에 `draggingAvailable`을 추가하고, 신규 케이스 2건을 넣는다.
- **IMPLEMENT**: `src/lib/assets/manifest.test.ts:148-164`의 테이블을 다음으로 교체한다.
  ```ts
  it.each([
    [
      { system: 'auth_required', mood: 'exhausted', dragging: true, draggingAvailable: true },
      'idle',
    ],
    [
      { system: 'error', mood: 'critical', dragging: true, draggingAvailable: true },
      'idle',
    ],
    [
      { system: 'loading', mood: 'warn', dragging: true, draggingAvailable: true },
      'idle',
    ],
    [
      { system: 'offline', mood: 'critical', dragging: false, draggingAvailable: true },
      'sleep',
    ],
    [
      { system: 'unavailable', mood: 'warn', dragging: true, draggingAvailable: true },
      'dragging',
    ],
    // A package without a `dragging` asset must keep signalling usage instead of
    // silently falling back to bare `idle` for the whole gesture.
    [
      { system: 'active', mood: 'critical', dragging: true, draggingAvailable: false },
      'idle_critical',
    ],
    [
      { system: 'active', mood: 'critical', dragging: true, draggingAvailable: true },
      'dragging',
    ],
    [
      { system: 'active', mood: 'exhausted', dragging: false, draggingAvailable: false },
      'idle_exhausted',
    ],
    [
      { system: 'active', mood: 'critical', dragging: false, draggingAvailable: false },
      'idle_critical',
    ],
    [
      { system: 'active', mood: 'warn', dragging: false, draggingAvailable: false },
      'idle_warn',
    ],
    [
      { system: 'active', mood: 'ok', dragging: false, draggingAvailable: false },
      'idle',
    ],
  ] as const)('selects priority for %o', (context, expected) => {
    expect(requestedAnimationKey(context)).toBe(expected);
  });
  ```
- **MIRROR**: `TEST_STRUCTURE_UNIT` (`it.each` 테이블).
- **IMPORTS**: 변경 없음.
- **GOTCHA**: `as const` 때문에 테이블 리터럴이 그대로 `AnimationContext`에 대입 가능해야 한다 — 필드명 오타는 컴파일 에러로 잡힌다. prettier가 줄바꿈을 재배치하므로 `pnpm prettier --write src/lib/assets/manifest.test.ts`로 정리한다.
- **VALIDATE**: `pnpm vitest run src/lib/assets/manifest.test.ts`

### Task 5: 포인터 종료 경로 통합 + 창 레벨 안전망 (`App.svelte`)

- **ACTION**: `src/App.svelte:547-574`의 핸들러 4개를 교체하고, 그 바로 위(또는 다른 `$effect` 인근)에 창 레벨 리스너 `$effect`를 추가한다.
- **IMPLEMENT**:

  (a) import에 `pointerButtonsReleased`를 추가한다 (`src/App.svelte:34-38`):
  ```ts
  import {
    beginPointer,
    pointerButtonsReleased,
    updatePointer,
    type PetPointerState,
  } from './lib/interaction/petPointer';
  ```

  (b) 핸들러 블록을 교체한다:
  ```ts
  // The single place the drag latch is released. `startDragging()` hands the
  // mouse loop to the OS, so `pointerup` on the surface is not guaranteed to
  // arrive — every signal that proves the gesture ended routes through here.
  const endPointerInteraction = () => {
    pointer = null;
    interactionStore.setDragging(false);
  };
  const pointerDown = (event: PointerEvent) => {
    // Last-resort recovery: if every release signal was swallowed by the OS
    // drag loop, the next gesture still starts from a clean latch.
    endPointerInteraction();
    (
      event.currentTarget as EventTarget & {
        setPointerCapture?: (pointerId: number) => void;
      }
    ).setPointerCapture?.(event.pointerId);
    pointer = beginPointer({ x: event.clientX, y: event.clientY });
  };
  const pointerMove = (event: PointerEvent) => {
    if (!pointer) return;
    if (pointerButtonsReleased(event.buttons)) return endPointerInteraction();
    const wasDragging = pointer.dragging;
    pointer = updatePointer(pointer, { x: event.clientX, y: event.clientY });
    interactionStore.setDragging(pointer.dragging);
    if (!wasDragging && pointer.dragging) void gateway.startDragging();
  };
  const pointerUp = (event: PointerEvent) => {
    if (!pointer) return;
    const surface = event.currentTarget as EventTarget & {
      hasPointerCapture?: (pointerId: number) => boolean;
      releasePointerCapture?: (pointerId: number) => void;
    };
    if (surface.hasPointerCapture?.(event.pointerId)) {
      surface.releasePointerCapture?.(event.pointerId);
    }
    endPointerInteraction();
  };
  const pointerCancel = (event: PointerEvent) => pointerUp(event);
  ```

  (c) 창 레벨 안전망 `$effect`를 추가한다 (`$effect` 블록들이 모여 있는 `src/App.svelte:434-447` 뒤가 적당하다):
  ```ts
  // The pet surface can lose the end of a gesture: `startDragging()` gives the
  // mouse loop to the window manager, and on Windows the accompanying
  // `ReleaseCapture()` breaks the captured-element path outright. Without this
  // net the drag latch stays set for the rest of the session, pinning the pet
  // to bare `idle` and suppressing every bubble. Armed only while a gesture is
  // open, so idle overlays add no listeners.
  $effect(() => {
    if (windowLabel !== 'overlay' || !pointer) return;
    const end = () => endPointerInteraction();
    const moved = (event: PointerEvent) => {
      if (pointerButtonsReleased(event.buttons)) endPointerInteraction();
    };
    window.addEventListener('pointerup', end);
    window.addEventListener('pointercancel', end);
    window.addEventListener('pointermove', moved);
    window.addEventListener('blur', end);
    return () => {
      window.removeEventListener('pointerup', end);
      window.removeEventListener('pointercancel', end);
      window.removeEventListener('pointermove', moved);
      window.removeEventListener('blur', end);
    };
  });
  ```

  (d) `resolvedAnimation`의 `requestedAnimationKey` 호출에 `draggingAvailable`을 넘긴다 (`src/App.svelte:458-470`):
  ```ts
  const resolvedAnimation = $derived(
    petPackage
      ? resolvePetAnimation(
          petPackage.manifest,
          petPackage.assetBaseUrl,
          requestedAnimationKey({
            system: primaryUi.system,
            mood: primaryUi.petMood,
            dragging: $interactionStore.dragging,
            draggingAvailable: petPackage.manifest.states.dragging !== undefined,
          }),
        )
      : null,
  );
  ```
- **MIRROR**: `EFFECT_LISTENER_LIFECYCLE` (윈도 라벨 가드 → 조기 반환 → cleanup 반환), `RATIONALE_COMMENT`.
- **IMPORTS**: `pointerButtonsReleased` 하나 추가. 그 외 없음.
- **GOTCHA**:
  - `$effect`가 `pointer`(`$state`)를 읽으므로 `pointer`가 `null`이 되는 순간 자동으로 재실행되어 리스너가 해제된다. 별도 정리 코드가 필요 없다.
  - 요소 핸들러가 버블링으로 창 핸들러보다 **먼저** 실행된다. `pointerMove`에 `buttons` 가드가 없으면 드래그 후 첫 이동에서 요소 핸들러가 `dragging`을 다시 `true`로 올린 뒤 창 핸들러가 내리는 플리커가 생긴다 — (b)의 가드가 이를 막는다.
  - `blur`를 종료 신호로 쓰면 일부 WM에서 네이티브 드래그 도중 래치가 먼저 풀릴 수 있다. `startDragging()`은 이미 호출된 뒤이므로 창 이동은 정상 계속되고, Task 3 덕분에 시각적 차이도 없다.
  - `endPointerInteraction()`은 멱등이다. 여러 신호가 중복 도달해도 안전하다.
  - `pointerDown`의 `endPointerInteraction()`이 `pointer`를 `null`로 만든 직후 `beginPointer`로 다시 채우므로 `$effect`가 한 번 tear-down/re-arm 될 수 있다 — 무해하다.
- **VALIDATE**: `pnpm check && pnpm vitest run src/App.test.ts`

### Task 6: `pointerup` 유실 회귀 테스트 (`App.test.ts`)

- **ACTION**: 기존 `'starts native dragging once when pointer movement crosses the threshold'` 테스트 바로 뒤에 회귀 테스트 2건을 추가한다.
- **IMPLEMENT**:
  ```ts
  it('restores the usage animation when the native drag swallows pointerup', async () => {
    const { gateway } = fixture();
    // A package with a distinct asset per mood: the shared fixture declares
    // only `idle`, so every key would resolve to the same src and the
    // assertion could not fail.
    vi.mocked(gateway.getPetPackage).mockResolvedValue({
      manifest: {
        id: 'fixture-pet',
        displayName: 'Fixture Pet',
        defaultSize: { width: 160, height: 160 },
        animations: {
          idle: { type: 'image', source: 'idle.svg' },
          idle_critical: { type: 'image', source: 'idle_critical.svg' },
        },
        states: { idle: 'idle', idle_critical: 'idle_critical' },
      },
      assetBaseUrl: 'asset://localhost/pets/fixture-pet/',
    });
    render(App, { props: { gateway, notificationAdapter: notifications } });
    const overlay = await screen.findByTestId('overlay-pointer-surface');
    expect(
      (screen.getByAltText('Fixture Pet') as HTMLImageElement).src,
    ).toContain('idle_critical.svg');

    const pointer = (type: string, x: number, y: number, buttons = 1) => {
      const event = new Event(type, { bubbles: true });
      Object.defineProperties(event, {
        clientX: { value: x },
        clientY: { value: y },
        buttons: { value: buttons },
      });
      return event;
    };
    await fireEvent(overlay, pointer('pointerdown', 10, 10));
    await fireEvent(overlay, pointer('pointermove', 40, 10));
    expect(gateway.startDragging).toHaveBeenCalledOnce();

    // The OS drag loop owns the gesture from here: no pointerup ever reaches
    // the surface. The first button-free move after the drop must release the
    // latch on its own.
    await fireEvent(window, pointer('pointermove', 60, 10, 0));
    await waitFor(() =>
      expect(
        (screen.getByAltText('Fixture Pet') as HTMLImageElement).src,
      ).toContain('idle_critical.svg'),
    );
  });

  it('keeps the usage animation during a drag when the package has no dragging asset', async () => {
    const { gateway } = fixture();
    vi.mocked(gateway.getPetPackage).mockResolvedValue({
      manifest: {
        id: 'fixture-pet',
        displayName: 'Fixture Pet',
        defaultSize: { width: 160, height: 160 },
        animations: {
          idle: { type: 'image', source: 'idle.svg' },
          idle_critical: { type: 'image', source: 'idle_critical.svg' },
        },
        states: { idle: 'idle', idle_critical: 'idle_critical' },
      },
      assetBaseUrl: 'asset://localhost/pets/fixture-pet/',
    });
    render(App, { props: { gateway, notificationAdapter: notifications } });
    const overlay = await screen.findByTestId('overlay-pointer-surface');
    const pointer = (type: string, x: number, y: number, buttons = 1) => {
      const event = new Event(type, { bubbles: true });
      Object.defineProperties(event, {
        clientX: { value: x },
        clientY: { value: y },
        buttons: { value: buttons },
      });
      return event;
    };
    await fireEvent(overlay, pointer('pointerdown', 10, 10));
    await fireEvent(overlay, pointer('pointermove', 40, 10));
    expect(
      (screen.getByAltText('Fixture Pet') as HTMLImageElement).src,
    ).toContain('idle_critical.svg');
  });
  ```
- **MIRROR**: `TEST_STRUCTURE_COMPONENT` + `TEST_GATEWAY_OVERRIDE`.
- **IMPORTS**: 파일 상단에 이미 `fireEvent`, `render`, `screen`, `waitFor`, `vi`가 있다. 추가 import 없음.
- **GOTCHA**:
  - 기존 헬퍼 `pointer(type, x, y)`에는 `buttons`가 없다. 새 테스트에서는 반드시 `buttons`를 정의해야 한다 — 정의하지 않으면 `undefined`가 되고 `pointerButtonsReleased(undefined)`가 `false`를 반환해 가드가 조용히 무력화된다.
  - 기본 픽스처의 claude는 `usedPercent: 91` → session severity `critical` → `petMood === 'critical'` → 요청 키 `idle_critical`. weekly는 40(ok)이라 최고 무드는 critical이다.
  - `<img>`의 `src`는 브라우저가 절대 URL로 정규화하므로 `toBe`가 아니라 `toContain`으로 비교한다.
  - `getByAltText`는 `PetAnimation.svelte:24`의 `alt={label}`을 타고, `label`은 `model.petName` = `manifest.displayName`이다.
  - `fireEvent(window, ...)`로 창 레벨 리스너를 직접 때린다. 요소에 쏘면 버블링되어 같은 결과지만, 유실 시나리오를 명시적으로 표현하려면 `window`가 낫다.
  - `render` 직후 `<img>`가 아직 없을 수 있으므로 `findByTestId('overlay-pointer-surface')`로 마운트를 먼저 기다린 뒤 `getByAltText`를 호출한다.
- **VALIDATE**: `pnpm vitest run src/App.test.ts`

### Task 7: 계약 문서 갱신 (`docs/ui-contract.md`)

- **ACTION**: §6 표의 `dragging` 행과 폴백 설명을 갱신한다.
- **IMPLEMENT**:
  - `docs/ui-contract.md:211`
    ```
    | `dragging` | 선택 | 드래그 중. 패키지가 이 키를 선언한 경우에만 요청한다 |
    ```
  - `docs/ui-contract.md:213`(폴백 체인 문단) 바로 뒤에 한 줄 추가:
    ```
    `dragging`은 예외적으로 요청 단계에서 걸러진다. 선언되지 않은 `dragging`을 요청하면 드래그가 끝날 때까지 무드 신호가 기본 `idle`로 덮여 사라지므로, 선언되지 않았다면 애초에 요청하지 않고 사용량 기반 키를 유지한다. 폴백 체인 자체는 그대로다.
    ```
- **MIRROR**: 기존 문서의 한국어 서술 톤 (근거를 문장으로 남기는 방식).
- **IMPORTS**: 해당 없음.
- **GOTCHA**: `docs/ui-contract.md:275`의 로드맵 항목은 에셋 지원 범위를 말하는 것이므로 손대지 않는다.
- **VALIDATE**: `pnpm prettier --check docs/ui-contract.md`

---

## Testing Strategy

### Unit Tests

| Test | Input | Expected Output | Edge Case? |
| --- | --- | --- | --- |
| `pointerButtonsReleased` — 버튼 없음 | `0` | `true` | — |
| `pointerButtonsReleased` — 주 버튼 | `1` | `false` | — |
| `pointerButtonsReleased` — 복수 버튼 | `3` | `false` | ✓ |
| `requestedAnimationKey` — 드래그 중, 에셋 있음 | `{active, critical, dragging:true, draggingAvailable:true}` | `'dragging'` | — |
| `requestedAnimationKey` — 드래그 중, 에셋 없음 | `{active, critical, dragging:true, draggingAvailable:false}` | `'idle_critical'` | ✓ 핵심 |
| `requestedAnimationKey` — 블로킹 상태 우선순위 | `{auth_required, exhausted, dragging:true, draggingAvailable:true}` | `'idle'` | ✓ |
| `requestedAnimationKey` — unavailable + 드래그, 에셋 없음 | `{unavailable, warn, dragging:true, draggingAvailable:false}` | `'sleep'` | ✓ 동작 변경 |
| App — `pointerup` 유실 후 복구 | pointerdown → pointermove(40px) → window pointermove(buttons=0) | `idle_critical.svg` | ✓ 회귀 |
| App — 드래그 중 무드 유지 | pointerdown → pointermove(40px) | `idle_critical.svg` | ✓ 회귀 |
| App — 기존 드래그 시작 (회귀 없음) | pointerdown → pointermove ×2 | `startDragging` 1회 | — |

### Edge Cases Checklist

- [ ] `buttons === 0`인 `pointermove`가 요소가 아니라 창에 도달 (창 레벨 리스너)
- [ ] `pointerup`/`pointercancel`이 정상 도달 (해피 패스 회귀 없음)
- [ ] 종료 신호가 하나도 오지 않고 다음 `pointerdown`이 바로 옴 (최후 방어선)
- [ ] `blur`가 드래그 도중 발화 (조기 해제 — 시각적 무해)
- [ ] 종료 신호 중복 도달 (`endPointerInteraction` 멱등)
- [ ] `pointer === null` 상태에서 창 리스너가 붙지 않음 (유휴 오버레이 리스너 0개)
- [ ] `windowLabel === 'panel'`에서 창 리스너가 붙지 않음
- [ ] `petPackage === null`(로드 실패)일 때 `draggingAvailable` 접근이 발생하지 않음 — `$derived`의 삼항 가드로 보장
- [ ] `dragging` 에셋을 선언한 패키지는 기존 동작 유지

---

## Validation Commands

### Static Analysis

```bash
pnpm check
```
EXPECT: 타입 에러 0건. `AnimationContext.draggingAvailable`을 필수로 만들었으므로 누락된 호출부가 있으면 여기서 잡힌다.

```bash
pnpm lint
```
EXPECT: eslint/prettier 위반 0건.

### Unit Tests

```bash
pnpm vitest run src/lib/interaction/petPointer.test.ts src/lib/assets/manifest.test.ts src/App.test.ts
```
EXPECT: 전부 통과. 신규 케이스 포함.

### Full Test Suite

```bash
pnpm test:ci
```
EXPECT: svelte-check + eslint + prettier + vitest 커버리지(branches/functions/lines/statements 80% 게이트) + vite build 전부 통과.

### Native Tests (회귀 확인)

```bash
cargo test --manifest-path src-tauri/Cargo.toml --all-features
```
EXPECT: 통과. 본 수정은 Rust를 건드리지 않으므로 변화 없어야 한다.

### Browser Validation

```bash
pnpm tauri dev
```
EXPECT: 아래 수동 체크리스트 통과.

### Manual Validation

- [ ] Claude 사용량이 70% 이상인 상태에서 펫이 `idle_warn`/`idle_critical`로 보인다.
- [ ] 펫을 잡고 화면 반대편으로 드래그한다 → **드래그 중에도** 무드 애니메이션이 유지된다.
- [ ] 마우스를 놓는다 → 펫이 무드 애니메이션 그대로다 (기본 `idle`로 떨어지지 않는다).
- [ ] 드래그 직후 펫을 클릭하지 않고 그대로 더블클릭 → 패널이 열리고, 패널을 닫아도 펫 상태 유지.
- [ ] 패널 → Settings → 펫을 corgi로 변경 → 오버레이 펫이 **곧바로** 무드 애니메이션으로 나온다 (기본 `idle` 아님).
- [ ] 패널 → Codex 탭 → "Set as primary" → 링과 펫이 codex 사용량 기준으로 즉시 갱신된다.
- [ ] 드래그를 여러 번 반복한 뒤에도 사용량 경고 말풍선이 정상적으로 뜬다.
- [ ] 펫 위에서 손을 살짝 떨며 더블클릭(4px 초과) → 패널이 열리고 펫 상태가 망가지지 않는다.
- [ ] Windows / Linux(X11) 양쪽에서 위 항목을 확인한다 — `startDragging`의 이벤트 유실 양상이 플랫폼마다 다르다.

---

## Acceptance Criteria

- [ ] 드래그 후 마우스를 놓으면 펫이 사용량 무드 애니메이션으로 돌아온다 (추가 클릭 불필요).
- [ ] 드래그 중에도 `dragging` 에셋이 없는 패키지는 무드 애니메이션을 유지한다.
- [ ] 펫 변경 / Set as primary 직후 펫이 올바른 무드로 렌더링된다.
- [ ] 드래그 이후에도 말풍선이 계속 동작한다.
- [ ] `dragging` 에셋을 선언한 패키지는 드래그 중 `dragging`을 재생한다 (기존 동작 보존).
- [ ] 모든 검증 명령이 통과한다.
- [ ] 커버리지 80% 게이트 유지.
- [ ] 타입/린트 에러 0건.

## Completion Checklist

- [ ] 순수 정책은 `interaction/`에, 뷰 배선은 `App.svelte`에 — 기존 레이어 경계 준수
- [ ] `$effect` 리스너에 cleanup 반환 존재
- [ ] 자명하지 않은 결정에 "왜" 주석 존재
- [ ] 테스트가 저장소의 `it.each` / 수동 PointerEvent 패턴을 따름
- [ ] 하드코딩된 매직 넘버 없음 (`buttons === 0`은 명명된 술어 뒤로 캡슐화)
- [ ] `docs/ui-contract.md` 갱신
- [ ] 요청 범위 밖 변경 없음 (Rust, 에셋, 임계값 미변경)
- [ ] 프라이버시 계약 무영향 (자격 증명/엔드포인트 미접촉)

## Risks

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| `blur`가 네이티브 드래그 도중 발화해 래치가 조기 해제 | 중 | 낮음 | `startDragging()`은 이미 호출된 뒤라 창 이동은 계속된다. Task 3 덕분에 시각적 차이 없음 |
| 창 레벨 `pointermove` 리스너의 성능 부담 | 낮음 | 낮음 | 제스처가 열려 있는 동안(`pointer !== null`)에만 등록. 유휴 오버레이는 리스너 0개 |
| jsdom의 PointerEvent 시뮬레이션이 실제 브라우저와 다름 | 중 | 중 | 단위 테스트로 정책을 고정하고, 플랫폼별 실제 검증은 수동 체크리스트로 커버 |
| `unavailable + dragging` 케이스의 키가 `'dragging'`→`'sleep'`으로 변경 | 확실 | 낮음 | 의도된 변경. 번들 펫에는 `sleep`도 없어 시각적으로 동일. 테이블 테스트에 명시 |
| `AnimationContext` 필수 필드 추가로 외부 호출부가 깨짐 | 낮음 | 낮음 | 호출부는 `App.svelte` 1곳 + 테스트 1곳. `pnpm check`가 전수 검출 |
| 사용자가 드래그 후 마우스를 전혀 움직이지 않는 경우 | 낮음 | 낮음 | `pointerup`/`pointercancel`/`blur`가 먼저 잡을 가능성이 높고, 최악의 경우에도 다음 `pointerdown`에서 반드시 복구 |

## Notes

- **왜 방어를 3중으로 두는가:** 원인이 OS 레벨 이벤트 유실이고, 플랫폼(Windows `ReleaseCapture` vs GTK `begin_move_drag` vs macOS `performWindowDragWithEvent`)마다 어떤 이벤트가 남는지 다르다. 단일 신호에 의존하면 어떤 플랫폼에서는 여전히 래치가 걸린다. 세 신호(창 레벨 이벤트 / `buttons === 0` 가드 / 다음 `pointerdown` 초기화)는 각각 다른 실패 모드를 덮는다.
- **`releasePointer`는 손대지 않는다.** `petPointer.ts:37-41`의 이 함수는 현재 `App.svelte`에서 호출되지 않는다 (단위 테스트에서만 사용). 기존 데드코드로 판단되지만 본 요청 범위 밖이므로 삭제하지 않고 여기 기록만 남긴다.
- **Rust 변경 불필요.** `save_position`은 `onMoved` 디바운스로 이미 독립 동작하며, 드래그 래치와 무관하다.
- **번들 펫에 `dragging`/`sleep` 에셋을 추가하는 것**은 이 버그의 대안 해결책이 아니다 — 래치가 걸리면 `dragging` 애니메이션이 영구 재생될 뿐 문제는 그대로다. 에셋 추가는 별개의 UX 개선 과제다.
