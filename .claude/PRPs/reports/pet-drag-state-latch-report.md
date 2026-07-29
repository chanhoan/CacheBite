# Implementation Report: 펫이 드래그 이후 `idle`에 고정되는 버그 수정

## Summary

Tauri 네이티브 창 드래그(`startDragging()`)가 시작되면 OS가 마우스 루프를 인수해 웹뷰가 `pointerup`을 받지 못하고, `interactionStore.dragging`이 `true`로 영구 래치되던 버그를 수정했다. 래치는 곧 "영구 기본 `idle`"(번들 펫에 `dragging` 에셋 없음) + "말풍선 영구 억제"를 의미했다.

수정은 3중 방어다:

1. **요청 단계 필터링** — 패키지가 `dragging` 상태를 선언하지 않으면 `dragging` 키를 아예 요청하지 않고 사용량 무드 키를 유지한다.
2. **종료 신호 통합** — `pointerup` / `pointercancel` / `buttons === 0` `pointermove` / `blur`를 모두 단일 `endPointerInteraction()`으로 라우팅하고, 제스처가 열려 있는 동안만 창 레벨 리스너를 등록한다.
3. **최후 방어선** — 다음 `pointerdown`이 무조건 래치를 초기화한다.

## Assessment vs Reality

| Metric | Predicted (Plan) | Actual |
| --- | --- | --- |
| Complexity | Small | Small — 일치 |
| Confidence | 근본 원인 단일 (계획의 배제 가설 표로 확정) | 일치. 배제된 가설을 다시 검토할 필요 없었음 |
| Files Changed | 6 (소스 3, 테스트 3) + 문서 1 = 7 | 7 (동일) |

## Tasks Completed

| # | Task | Status | Notes |
| --- | --- | --- | --- |
| 1 | 드래그 종료 판정 술어 추가 (`petPointer.ts`) | 완료 | `pointerButtonsReleased(buttons)` 순수 술어 |
| 2 | 술어 단위 테스트 (`petPointer.test.ts`) | 완료 | `0`/`1`/`3` 케이스 |
| 3 | `dragging` 키를 선언된 경우에만 요청 (`resolver.ts`) | 완료 | `AnimationContext.draggingAvailable` 필수 필드 |
| 4 | 애니메이션 키 테이블 갱신 (`manifest.test.ts`) | 완료 — 편차 | 9행 → 12행 (신규 3건). 아래 Deviations 참조 |
| 5 | 포인터 종료 경로 통합 + 창 레벨 안전망 (`App.svelte`) | 완료 | `endPointerInteraction()` + `$effect` 안전망 |
| 6 | `pointerup` 유실 회귀 테스트 (`App.test.ts`) | 완료 — 편차 | 아래 Deviations 참조 |
| 7 | 계약 문서 갱신 (`docs/ui-contract.md`) | 완료 | §6 `dragging` 행 + 예외 문단 |

## Validation Results

| Level | Status | Notes |
| --- | --- | --- |
| Static Analysis (`pnpm check`) | 통과 | svelte-check 0 errors / 0 warnings |
| Lint (`eslint` + `prettier --check`) | 통과 | `test:ci` 체인 내 실행, 위반 0건 |
| Unit Tests | 통과 | 대상 3파일 66건 전부 통과 |
| Coverage Gate (80%) | 통과 | `petPointer.ts` 100%, `resolver.ts` 88.73% stmts / 85.36% branches |
| Build (`vite build`) | 통과 | 156 modules, 8.25s |
| Full CI (`pnpm test:ci`) | 통과 | exit 0 |
| Native (`cargo test`) | **미실행** | 이 환경에 `pkg-config`/GTK 개발 헤더 부재로 `gdk-sys` 빌드 실패. 본 변경은 Rust 파일을 하나도 건드리지 않으므로 회귀 위험 없음 — CI(`ci.yml`)에서 검증됨 |
| Manual / Browser | **미실행** | `pnpm tauri dev` 필요. 계획의 수동 체크리스트는 아래 Next Steps에 남김 |

## Files Changed

| File | Action | Lines |
| --- | --- | --- |
| `src/lib/interaction/petPointer.ts` | UPDATED | +10 |
| `src/lib/interaction/petPointer.test.ts` | UPDATED | +12 / -1 |
| `src/lib/assets/resolver.ts` | UPDATED | +8 / -1 |
| `src/lib/assets/manifest.test.ts` | UPDATED | +109 / -7 |
| `src/App.svelte` | UPDATED | +38 / -3 |
| `src/App.test.ts` | UPDATED | +93 |
| `docs/ui-contract.md` | UPDATED | +3 / -1 |

합계: 7 files, +272 / -14 (강화 편집 이전 스냅샷 기준)

## Deviations from Plan

### 1. Task 6 회귀 테스트 1의 픽스처에 `dragging` 에셋 추가

- **WHAT**: 계획의 `'restores the usage animation when the native drag swallows pointerup'` 테스트는 `states: { idle, idle_critical }`만 선언한 픽스처를 쓰고, 드래그 중과 해제 후 모두 `idle_critical.svg`를 단언한다. 이를 `dragging` 애니메이션/상태를 선언한 픽스처로 바꾸고, 드래그 중 `dragging.svg` → 버튼 해제 후 `idle_critical.svg` 전이를 단언하도록 변경했다.
- **WHY**: 계획대로면 `draggingAvailable === false`이므로 요청 키가 드래그 중에도 `idle_critical`로 고정된다. 즉 **래치가 걸린 채로도 테스트가 통과**한다 — Task 3만으로 통과하고 Task 5(래치 해제)를 전혀 검증하지 못한다. `dragging`을 선언해야 래치 상태가 렌더된 `src`에 드러나고, 해제 경로가 실제로 회귀 테스트된다.
- **부수 효과**: 이 테스트가 Acceptance Criteria "`dragging` 에셋을 선언한 패키지는 드래그 중 `dragging`을 재생한다(기존 동작 보존)"도 함께 커버한다. 계획의 두 번째 테스트(`dragging` 에셋 없는 패키지에서 무드 유지)는 원안 그대로 유지했다.

### 2. `manifest.test.ts` 테이블에 `unavailable + dragging + !draggingAvailable → 'sleep'` 행 추가

- **WHAT**: 계획 테이블 11행에 더해 이 케이스를 1행 추가했다.
- **WHY**: 계획의 GOTCHA와 Risks 표가 이 동작 변경(`'dragging'` → `'sleep'`)을 "의도된 변경, 테이블 테스트에 명시"라고 적었으나 제시된 테이블에는 해당 행이 없었다. Testing Strategy 표에는 있었다 — 둘의 불일치를 Testing Strategy 쪽으로 맞췄다.

### 3. `pnpm check` 최초 실행 결과 폐기 후 재실행

- **WHAT**: 첫 `pnpm check`가 마지막 `App.svelte` 편집(창 레벨 `$effect` + `draggingAvailable` 전달) 이전에 시작되어 결과를 버리고 재실행했다.
- **WHY**: 검증 명령이 검증하려는 코드보다 앞선 스냅샷을 보면 안 된다.

## Issues Encountered

| Issue | Resolution |
| --- | --- |
| `cargo`가 PATH에 없음 | `~/.cargo/bin/cargo`로 실행 — 그러나 `pkg-config`/GTK 헤더 부재로 `gdk-sys` 빌드 실패. 환경 제약으로 판단하고 미실행 처리 (Rust 무변경) |
| WSL2 `/mnt/c`에서 vitest jsdom 환경 셋업이 ~90s | 검증을 백그라운드 실행 + 배치로 묶어 진행 |

## Tests Written

| Test File | Tests | Coverage |
| --- | --- | --- |
| `src/lib/interaction/petPointer.test.ts` | +1 | `pointerButtonsReleased` 술어 (0 / 1 / 3) |
| `src/lib/assets/manifest.test.ts` | +3 (테이블 행) | `draggingAvailable` false일 때 무드 유지, true일 때 `dragging`, `unavailable`+드래그+에셋없음 → `sleep` |
| `src/App.test.ts` | +4 | `pointerup` 유실 후 창 레벨 `buttons===0` 신호로 래치 해제 / 드롭 후 `blur`로 해제 / 창 신호 전무 시 다음 `pointerdown`이 초기화 / `dragging` 에셋 없는 패키지의 드래그 중 무드 유지 |

## Edge Cases Checklist

- [x] `buttons === 0`인 `pointermove`가 창에 도달 — `App.test.ts` 회귀 테스트로 검증
- [x] `pointerup`/`pointercancel` 정상 도달 (해피 패스 회귀 없음) — 기존 34건 통과
- [x] 드롭 이후 `blur`만 도달 — 리뷰 후속 회귀 테스트(`releases the drag latch when the overlay loses focus after the drop`)
- [x] 다음 `pointerdown`이 래치 초기화 — 리뷰 후속 회귀 테스트(`starts the next gesture from a clean latch...`), 창 신호가 하나도 오지 않는 조건에서 단언
- [x] `endPointerInteraction()` 멱등 — 반복 호출해도 `pointer = null` / `dragging = false`
- [x] `pointer === null`이면 창 리스너 0개 — `$effect`의 `!gestureOpen` 조기 반환
- [x] `windowLabel === 'panel'`에서 창 리스너 미등록 — `$effect`의 라벨 가드
- [x] `petPackage === null`일 때 `draggingAvailable` 미접근 — `$derived` 삼항 가드
- [x] `dragging` 선언 패키지의 기존 동작 유지 — 강화한 회귀 테스트가 `dragging.svg` 재생을 단언
- [ ] `blur`가 드래그 도중 발화 (조기 해제 — 시각적 무해) — 수동 검증 필요 (플랫폼 의존)
- [ ] Windows `ReleaseCapture()` 경로에서 드래그 진행 중 `buttons === 0` `pointermove`가 흘러 무드 애니메이션으로 조기 복귀하는지 — 수동 검증 필요 (코드 리뷰 L-2)

## Next Steps

- [ ] `pnpm tauri dev`로 계획의 Manual Validation 체크리스트 수행 (특히 Windows / Linux X11 양쪽, **위 L-2 경로 포함**)
- [x] Code review via `/code-review` — `docs/code-review/pet-drag-state-latch-2026-07-29.md` (APPROVE with comments). MEDIUM 4건 + LOW 3건 반영
- [ ] Create PR via `/prp-pr` (base: `develop`)
