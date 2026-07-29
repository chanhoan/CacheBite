# Code Review: 로컬 미커밋 변경분 (pet-drag-state-latch)

**리뷰 일자**: 2026-07-29
**브랜치**: `fix/pet-drag-state-latch`
**베이스**: `0848c77` (Merge pull request #9 from chanhoan/feat/user-selectable-pet)
**범위**: 수정 7개 파일 (+272 / −14) — 소스 3, 테스트 3, 문서 1. 네이티브(Rust) 변경 없음
**결정**: **APPROVE with comments** — CRITICAL 0, HIGH 0, MEDIUM 4, LOW 6

---

## Summary

Tauri `startDragging()`이 마우스 루프를 OS에 넘기면서 웹뷰가 `pointerup`을 받지 못해 `interactionStore.dragging`이 영구 래치되던 버그를 3중 방어로 수정한 변경. **근본 원인 진단과 방어 설계는 정확하고, 계약 문서까지 함께 갱신했다.** 머지를 막을 사유는 없다.

특히 잘 된 판단:

- **`draggingAvailable`를 optional이 아닌 필수 필드로 추가**(`resolver.ts:34`)해, 호출부 누락이 런타임 무증상이 아니라 컴파일 에러가 되게 했다.
- 요청 단계 필터링이 **실제로 안전하다는 것이 매니페스트 검증으로 뒷받침된다**: `validatePetManifest`가 `states`의 각 값이 존재하는 애니메이션 id인지 검사하므로(`manifest.ts:125-130`), `draggingAvailable === true`인데 렌더가 `idle`로 떨어지는 조합은 성립할 수 없다. 필터가 우회 가능한 휴리스틱이 아니라 계약에 근거한다.
- **폴백 체인 자체는 건드리지 않고** 요청 단계에서만 걸러낸 점(`ui-contract.md:214`). "요청 상태 키 → `idle`" 단일 규칙이 유지되므로 제작자 관점의 예측 가능성이 보존된다.
- 동작이 실제로 바뀌는 조합(`unavailable` + 드래그 + `dragging` 미선언 → `'dragging'`에서 `'sleep'`으로)을 테이블 테스트에 **명시적 행으로 추가**했다(`manifest.test.ts`). 조용한 동작 변경이 아니다.
- 해제 경로를 `endPointerInteraction()` 한 곳으로 모은 리팩터링은 이 클래스의 버그(경로별 부분 정리)를 구조적으로 차단한다.
- 프라이버시 계약과 무관하다 — 자격 증명 경로, authorization 값, 계정 식별자, 원본 provider 응답을 다루는 코드가 전혀 바뀌지 않았고 신규 로그도 없다.

다만 **창 레벨 `$effect`가 `pointer` 상태 전체를 구독**해 드래그 중 매 프레임 리스너를 재등록하고(실측 확인), **3중 방어 중 회귀 테스트가 붙은 것은 1.5개뿐**이다. 나머지는 스타일·테스트 위생 수준이다.

---

## Findings

### CRITICAL

없음.

- 하드코딩된 자격 증명·토큰·API 키 없음
- 신규 `console.log` / 디버그 출력 없음, `TODO` / `FIXME` 없음
- 렌더러 DTO·로그 표면 변경 없음 → 프라이버시 계약 유지
- 사용자 입력 파싱·경로 조작·외부 요청 경로 무변경
- 신규 함수 3개 모두 50줄 미만, 파일 800줄 미만, 중첩 4단계 이하

---

### HIGH

없음.

---

### MEDIUM

#### M-1. 창 레벨 `$effect`가 `pointer` 객체 전체를 구독해, 드래그 중 매 `pointermove`마다 리스너 4종을 해제·재등록

**위치**: `src/App.svelte:456-472`

```ts
$effect(() => {
  if (windowLabel !== 'overlay' || !pointer) return;   // ← pointer 신호 전체를 구독
  ...
  window.addEventListener('pointerup', end);
  window.addEventListener('pointercancel', end);
  window.addEventListener('pointermove', moved);
  window.addEventListener('blur', end);
  return () => { /* 4종 해제 */ };
});
```

`pointer`는 `$state`이고 `pointerMove`가 매번 **새 객체**를 대입한다(`:596` `pointer = updatePointer(...)`). 이펙트가 필요한 정보는 `pointer !== null` 하나뿐인데 참조 변경 전부에 반응하므로, 제스처가 열려 있는 동안 계속 teardown → re-register가 돈다.

**실측 확인** — 임시 스펙에서 `window.addEventListener` / `removeEventListener`를 스파이하고 `pointerdown` 1회 + `pointermove` 10회를 발생시킨 결과:

```
SCRATCH added=11 removed=10        // 'pointermove' 리스너만 집계
```

리스너 4종 합계로는 **44회 등록 / 40회 해제**다. (검증 후 임시 파일은 삭제했고 작업 트리는 원상 복구됨.)

정확성 문제는 아니다 — Svelte 5는 이펙트를 이벤트 디스패치 이후 마이크로태스크에서 플러시하므로 이벤트가 유실되는 구간은 없다. 그러나 드래그는 이 앱에서 가장 이벤트가 촘촘한 경로이고, 주석이 "Armed only while a gesture is open, so idle overlays add no listeners"라고 비용을 이미 의식하고 있다. 의도한 비용은 "제스처당 1회"였을 것이다.

**수정안** (2줄):

```ts
const gestureOpen = $derived(pointer !== null);
...
$effect(() => {
  if (windowLabel !== 'overlay' || !gestureOpen) return;
  ...
});
```

`$derived`는 값이 실제로 변할 때만 의존자를 깨우므로, 등록/해제가 제스처 경계에서 정확히 1회씩만 발생한다.

#### M-2. 3중 방어 중 회귀 테스트가 붙은 것은 1.5개 — 나머지 해제 경로는 전부 미검증

**위치**: `src/App.test.ts:368-441`, `src/App.svelte:456-472`, `:584`

보고서(`.claude/PRPs/reports/pet-drag-state-latch-report.md:7-11`)는 3중 방어를 명시하지만, 신규 테스트가 실제로 커버하는 경로는:

| 방어 계층 | 신호 | 회귀 테스트 |
|---|---|---|
| 1. 요청 단계 필터 | `draggingAvailable` | ✅ `manifest.test.ts` 12행 + `App.test.ts:443` |
| 2. 종료 신호 통합 | 창 `pointermove` (`buttons===0`) | ✅ `App.test.ts:420` |
| 2. 종료 신호 통합 | 창 `pointerup` / `pointercancel` | ❌ |
| 2. 종료 신호 통합 | 창 `blur` | ❌ |
| 3. 최후 방어선 | 다음 `pointerdown`의 초기화 | ❌ |

특히 계층 3은 **이 변경에서 유일하게 "무슨 일이 있어도 복구된다"를 보장하는 장치**인데 단언이 하나도 없다. 창 리스너 없이도(즉 `$effect`가 통째로 사라져도) 계층 3만으로 다음 제스처가 정상화되는지가 회귀에서 지켜지지 않는다.

커버리지 리포트도 같은 지점을 가리킨다 — `App.svelte` 미커버 라인에 `607-608`(`hasPointerCapture` / `releasePointerCapture` 분기)이 그대로 남아 있다.

`blur`는 jsdom에서 `fireEvent(window, new Event('blur'))`로, 계층 3은 "래치가 걸린 상태에서 `pointerdown` → 즉시 `dragging` 해제" 단언으로 각각 3~5줄이면 붙는다.

#### M-3. 두 번째 신규 테스트가 드래그가 실제로 시작됐음을 단언하지 않아, 무동작으로도 통과함

**위치**: `src/App.test.ts:443-465`

```ts
it('keeps the usage animation during a drag when the package has no dragging asset', ...)
  await fireEvent(overlay, pointer('pointerdown', 10, 10));
  await fireEvent(overlay, pointer('pointermove', 40, 10));
  expect(... .src).toContain('idle_critical.svg');   // ← 유일한 단언
```

이 테스트는 `pointerdown` / `pointermove` 핸들러가 **아무 일도 하지 않아도** 통과한다(초기 상태가 이미 `idle_critical.svg`이므로). 검증하려는 명제는 "드래그 중임에도 무드가 유지된다"인데, "드래그 중"이라는 전제가 단언되지 않는다.

같은 파일 `:412`가 이미 쓰고 있는 한 줄을 추가하면 전제가 고정된다:

```ts
expect(gateway.startDragging).toHaveBeenCalledOnce();
```

보고서가 Deviation 1에서 첫 번째 테스트에 대해 정확히 이 함정("래치가 걸린 채로도 통과")을 지적하고 픽스처를 강화했는데, 두 번째 테스트에는 같은 잣대가 적용되지 않았다.

#### M-4. 이벤트 팩토리 3중 중복 + 기존 팩토리는 `buttons`를 설정하지 않아 실제 브라우저 이벤트와 다름

**위치**: `src/App.test.ts:354-361`, `:398-406`, `:456-464`

동일한 `pointer(type, x, y)` 헬퍼가 세 테스트에 복사돼 있다. 그중 **기존 팩토리(`:354-361`)만 `buttons`를 정의하지 않는다**:

```ts
const pointer = (type: string, x: number, y: number) => {
  const event = new Event(type, { bubbles: true });
  Object.defineProperties(event, {
    clientX: { value: x },
    clientY: { value: y },     // ← buttons 없음 → event.buttons === undefined
  });
```

이 테스트가 여전히 통과하는 이유는 `pointerButtonsReleased(undefined)`가 `undefined === 0` → `false`로 빠지기 때문이다. 즉 **실제 브라우저에는 존재하지 않는 이벤트 형태**(PointerEvent의 `buttons`는 항상 숫자)에 의존해 통과하고 있으며, `pointerButtonsReleased(buttons: number)`의 타입 계약도 런타임에서 어긴다. TypeScript는 App.svelte 쪽 `event`가 `PointerEvent`로 타입돼 있어 이를 잡지 못한다.

세 곳을 `buttons = 1` 기본값을 가진 모듈 레벨 헬퍼 하나로 합치면 중복과 이 불일치가 동시에 해소된다.

---

### LOW

#### L-1. 래치 해제가 "다음 포인터 이벤트가 오버레이 창에 도달"에 의존 — 시간 기반 백스톱 없음

**위치**: `src/App.svelte:456-472`

네 신호 모두 오버레이 창이 이벤트를 받아야 발화한다. 드롭 직후 커서가 오버레이 밖으로 나가고 창이 포커스도 잃지 않는 경로에서는 다음 제스처(`pointerdown`)까지 래치가 유지된다. 실사용에서는 드래그 종료 시 커서가 펫 위에 있으므로 1px만 움직여도 해제되고, 다른 창을 클릭하면 `blur`가 잡는다 — 그래서 실질 영향은 작다. 그러나 "무조건 복구"는 계층 3에만 의존한다는 점은 인지해 둘 가치가 있다.

네이티브 쪽에 이미 `listenPositionMoved`가 있으므로, 필요해지면 창 이동 정지 감지를 권위 있는 종료 신호로 승격할 수 있다.

#### L-2. Windows `ReleaseCapture()` 경로에서 드래그 도중 조기 해제 가능 — 보고서에 미기록

**위치**: `src/App.svelte:594`, `:459-461`

`:450-455` 주석 자체가 "on Windows the accompanying `ReleaseCapture()` breaks the captured-element path outright"라고 적고 있다. 그 상황에서 웹뷰가 **드래그가 아직 진행 중인데** `buttons===0`인 `pointermove`를 흘릴 수 있고, 그러면 드래그 중 무드 애니메이션으로 조기 복귀한다.

영향은 시각적일 뿐이다 — 창 이동은 이미 OS가 소유하고 있고 위치 저장도 네이티브 경로다. 영구 래치보다 명백히 나은 트레이드오프지만, 보고서의 Edge Cases 체크리스트에는 `blur` 조기 해제만 적혀 있고 이 경로는 없다. 수동 검증 항목에 추가할 것.

#### L-3. 창 레벨 신호가 먼저 래치를 지우면 `releasePointerCapture`가 호출되지 않음

**위치**: `src/App.svelte:600-609`

```ts
const pointerUp = (event: PointerEvent) => {
  if (!pointer) return;          // ← 창 리스너가 먼저 지웠으면 여기서 종료
  ...
  if (surface.hasPointerCapture?.(event.pointerId)) {
    surface.releasePointerCapture?.(event.pointerId);
  }
```

포인터 캡처 해제를 브라우저의 암묵 해제에 맡기게 된다. 다음 `pointerdown`이 어차피 `setPointerCapture`를 다시 호출하므로 실질 영향은 없지만, 캡처 해제를 `pointer` 존재 여부와 무관하게 수행하도록 순서를 바꾸면 의도가 더 분명해진다. 커버리지 미커버 라인 `607-608`과 같은 지점이다.

#### L-4. `releasePointer`는 export + 단위 테스트만 있고 프로덕션에서 호출되지 않음

**위치**: `src/lib/interaction/petPointer.ts:37-41`, `src/lib/interaction/petPointer.test.ts:15`, `:21`

패널 열기는 `PetOverlay.svelte:54`의 `ondblclick`이 담당하므로 `PointerRelease` 정책은 아무 데서도 소비되지 않는다(`grep`으로 확인 — `App.svelte`에는 `releasePointerCapture`만 존재). 이번 변경이 만든 죽은 코드는 아니지만, 같은 파일에 새 술어를 추가하는 김에 정리 여부를 정하는 편이 좋다. 남긴다면 "향후 클릭 정책용"이라는 근거를 주석으로 붙일 것.

#### L-5. `setDragging`이 값이 같아도 항상 새 상태 객체를 만들어 구독자를 깨움

**위치**: `src/lib/stores/interaction.ts:24-26`

```ts
setDragging(dragging: boolean) {
  update((state) => ({ ...state, dragging }));   // 동일 값에도 새 객체
}
```

이번 변경으로 제스처 종료 시 `endPointerInteraction()`이 2~3회 호출될 수 있게 됐다(서피스 `pointerup` + 창 `pointerup` + 창 `pointermove`). 각 호출이 스토어 갱신 → `resolvedAnimation` 재계산 → 재렌더를 유발한다. 같은 파일의 `expireBubble`(`:36-43`)은 이미 "변경 없으면 같은 참조 반환" 패턴을 쓰고 있으므로, 일관성 차원에서도 조기 반환을 붙일 만하다.

#### L-6. `return endPointerInteraction();` — void 반환값을 return하는 관용구

**위치**: `src/App.svelte:594`

```ts
if (pointerButtonsReleased(event.buttons)) return endPointerInteraction();
```

동작에는 문제가 없고 lint도 통과하지만, `endPointerInteraction`이 값을 반환하는 것처럼 읽힌다. `{ endPointerInteraction(); return; }`가 의도에 더 가깝다.

---

## Validation Results

이 환경(Windows PowerShell)에는 `pnpm`이 PATH에 없어, `test:ci` 체인과 동일한 명령을 `node_modules/.bin`에서 직접 실행했다.

| Check | Result | 비고 |
|---|---|---|
| Type check (`svelte-check --tsconfig ./tsconfig.json`) | **Pass** | 0 errors, 0 warnings |
| Lint (`eslint .`) | **Pass** | exit 0 |
| Format (`prettier --check .`) | **Pass** | All matched files use Prettier code style |
| Tests (`vitest run --coverage`) | **Pass** | 23 files / **237 tests** 전부 통과 |
| Coverage (게이트 80%) | **Pass** | Stmts 95.52 / Branch 90.37 / Funcs 92.08 / Lines 95.52 |
| Build (`vite build`) | **Pass** | 1.03s, `index-*.js` 92.60 kB (gzip 31.39 kB) |
| Rust (`cargo test` / `clippy`) | **N/A** | 이번 변경에 Rust 파일 없음. CI(`ci.yml`)가 계속 검증 |
| Manual (`pnpm tauri dev`) | **미실행** | 아래 참조 |

변경 파일별 커버리지:

| 파일 | Stmts | Branch |
|---|---|---|
| `petPointer.ts` | 100% | 100% |
| `resolver.ts` | 88.73% | 85.36% |
| `App.svelte` | 95.53% | 88.38% (미커버: 322-324, **607-608**) |

> **수동 검증은 이 리뷰에서 수행되지 않았다.** 이 버그의 근본 원인(OS 드래그 루프의 `pointerup` 삼킴)은 jsdom으로 재현할 수 없는 플랫폼 동작이므로, `pnpm tauri dev` 기준 Windows / Linux X11 확인은 여전히 필수다. 보고서의 Next Steps에 남아 있는 항목이며, L-2 경로를 체크리스트에 추가할 것을 권한다.

---

## Files Reviewed

| 파일 | 변경 | 내용 |
|---|---|---|
| `src/lib/interaction/petPointer.ts` | Modified | `pointerButtonsReleased(buttons)` 순수 술어 추가 (+10) |
| `src/lib/interaction/petPointer.test.ts` | Modified | 술어 단위 테스트 (0 / 1 / 3) |
| `src/lib/assets/resolver.ts` | Modified | `AnimationContext.draggingAvailable` 필수 필드 + 요청 필터 |
| `src/lib/assets/manifest.test.ts` | Modified | 키 우선순위 테이블 9행 → 12행 |
| `src/App.svelte` | Modified | `endPointerInteraction()` 통합, 창 레벨 `$effect` 안전망, `pointerdown` 초기화, `draggingAvailable` 전달 |
| `src/App.test.ts` | Modified | 회귀 테스트 2건 |
| `docs/ui-contract.md` | Modified | §6 `dragging` 행 + 예외 문단 |
| `.claude/PRPs/plans/completed/pet-drag-state-latch.plan.md` | Added (untracked) | 계획 문서 |
| `.claude/PRPs/reports/pet-drag-state-latch-report.md` | Added (untracked) | 구현 보고서 |

---

## 권장 조치 순서

머지 차단 항목은 없다. 아래는 이 브랜치에서 마무리하면 좋은 순서다.

1. **M-1** — `$derived(pointer !== null)`로 이펙트 의존성 축소 (2줄)
2. **M-3** — 두 번째 회귀 테스트에 `startDragging` 단언 추가 (1줄)
3. **M-2** — `blur` 경로 + `pointerdown` 최후 방어선 회귀 테스트 추가 (~10줄)
4. **M-4** — 이벤트 팩토리를 모듈 레벨 헬퍼 하나로 통합, `buttons` 기본값 1
5. **L-5 / L-6 / L-3** — 스토어 조기 반환, `return` 관용구, 캡처 해제 순서
6. **L-4** — `releasePointer` 제거 또는 존치 근거 주석
7. `pnpm tauri dev`로 Windows / Linux X11 수동 검증 (**L-2 경로 포함**) — 커밋 전 필수
