# Plan: Double Ring — 원근 배치(Perspective Layout) 개편

## Summary

`double` 링 모드의 작은 원을 우하단 대각선 배치에서 **우측 수평 접선 배치**로 옮긴다. 두 원은
외접 공통접선(outer common tangent)이 우측 소실점으로 수렴하는 관계에 놓여, 같은 원이 멀어지는
것을 제3자가 바라보는 원근으로 읽힌다. 작은 원 안에 있던 provider 로고는 **작은 원 전용 궤도
워커**로 빠져나가고, 비워진 내부에는 **5시간(5H) 사용률 숫자**가 들어간다(비-`active`일 때는
기존 시스템 배지로 교체).

## User Story

As a CacheBite 사용자,
I want 두 provider의 링이 원근을 가진 하나의 구성으로 읽히고 작은 원도 자기 수치를 숫자로 말해주기를,
So that 오버레이 한 번 보고 두 provider의 정체와 5시간 잔량을 동시에 파악할 수 있다.

## Problem → Solution

**현재**: 작은 원이 큰 원 우하단에 ⅓ 겹쳐 붙어 있어 "큰 원에 달라붙은 뱃지"처럼 읽힌다. 내부에는
정적 provider 로고가 있고, 궤도 워커는 두 원의 합집합 아웃라인을 도는 단일 마크다. 작은 원은
아크 색·길이 외에 읽을 수 있는 수치가 없다.

**목표**: 작은 원이 우측으로 분리돼 원근 관계를 이룬다. 각 원이 **자기 provider 마크를 자기 궤도에
태운** 대칭 구조가 되고, 작은 원 내부는 5H 숫자를 싣는다.

## Metadata

- **Complexity**: Medium
- **Source PRD**: N/A (자유 형식 요청 + 대화형 설계 확정)
- **PRD Phase**: N/A (standalone)
- **Estimated Files**: 10 (코드 5 · 테스트 3 · 문서 2)
- **Base branch**: `feat/overlay-double-ring` (커밋 `417be49` 위에 쌓음)
- **선행 플랜**: `.claude/PRPs/plans/completed/overlay-double-ring.plan.md` (최초 구현, 완료됨)

---

## 확정된 설계 결정 (대화에서 승인됨)

| # | 결정 | 근거 |
| --- | --- | --- |
| 1 | 작은 원을 **수평 우측**(y=50)으로 이동, 중심거리 `d = 64.5` | 요청 1·2 |
| 2 | 작은 원 지름 **36%**(기존 40%), 두 링 사이 틈 1.25%p | 예산 내에서 α≈25° 성립 |
| 3 | 수렴 접선은 **그리지 않는다** — 배치 규칙으로만 사용 | 사용자 선택, Apple HIG의 장식 최소화 |
| 4 | 작은 원 전용 워커 신설, 크기 **9%**(큰 워커 14%) | 링 대비 비율 유지 + 예산 절약 |
| 5 | 작은 원 내부: `active`면 **5H 숫자**, 그 외엔 기존 `SystemBadge` | 사용자 선택. 라벨 없이도 규칙이 고정이라 모호하지 않음 |
| 6 | 큰 워커가 작은 링 앞을 지나는 것은 **의도된 오클루전** | 가까운 것이 먼 것을 가림 = 원근 강화 |
| 7 | z-order: 큰 워커 3 > 작은 워커 2 > 표면 1 > 링 0 | 작은 워커가 큰 링 아크에 가려지지 않게 |
| 8 | 펫 크기 **128px 유지** (손실 0) | `OVERLAY_BOUNDS_FACTOR` 1.83 → 상한 131.1px |

### 기하 유도 (오버레이 % 공간, 큰 원 중심 = (50, 50))

```
R  = RING_OUTER_RADIUS      = 45.25   (아크 반지름 42 + 획 6.5의 절반 3.25)
r  = SATELLITE_SIZE / 2     = 18      (SATELLITE_SIZE = 36)
d  = SATELLITE_DISTANCE     = 64.5
SATELLITE_CENTER            = (50 + 64.5, 50) = (114.5, 50)

큰 워커   WALKER_SIZE            = 14  → 반지름 7
작은 워커 SATELLITE_WALKER_SIZE  = 9   → 반지름 4.5

BIG_ORBIT   = R + 7   = 52.25
SMALL_ORBIT = r + 4.5 = 22.5

수렴 반각 α = asin((R - r) / d) = asin(27.25 / 64.5) = asin(0.422481) = 24.995°
두 링 사이 틈 = d - R - r = 64.5 - 45.25 - 18 = 1.25

도달 범위 (오버레이 중심 기준):
  세로 = max(BIG_ORBIT + 7, SMALL_ORBIT + 4.5) = max(59.25, 27) = 59.25
  가로 = d + SMALL_ORBIT + 4.5 = 64.5 + 22.5 + 4.5 = 91.5
OVERLAY_BOUNDS_FACTOR = 2 × 91.5 / 100 = 1.83
펫 상한 = 240 / 1.83 = 131.1px  ≥  모든 매니페스트의 defaultSize 128  ✓
```

**접점 위치 (문서화용, 코드에서 렌더하지 않음)**: y가 아래를 향하므로 상단 접선의 접점은 두 원
모두 각도 `α - 90° = -65.0°`에 있다. 큰 원 (69.12, 8.99), 작은 원 (122.11, 33.69). 즉 접점은
각 원의 꼭대기에서 α만큼 우측으로 돌아간 지점이며, **정확히 최상단은 아니다**.

> **요청 2의 문자적 해석이 불가능한 이유**: 접선이 두 원의 *정확한* 최상단·최하단에 닿으려면
> 그 점에서의 접선이 수평이어야 하고, 그러면 두 선은 평행이 되어 "수렴"할 수 없다. 수렴하는
> 구성은 외접 공통접선 하나뿐이며, 그것이 요청하신 "제3자가 바라보는" 원근을 만든다.

### 애니메이션 주기

큰 워커 26s는 유지한다. 작은 워커는 **선속도를 맞춰** `26 × (SMALL_ORBIT / BIG_ORBIT) =
26 × 22.5 / 52.25 ≈ 11.196s`로 파생한다. 상수를 손으로 적지 않고 파생시키는 이유는 반지름을
튜닝했을 때 두 마크의 걸음 속도가 조용히 어긋나는 것을 막기 위해서다.

---

## UX Design

### Before

```
        ┌──── 오버레이 240px 창 ────┐
        │                          │
        │      ,-''''''-.          │
        │    ,'  큰 링    `.       │
        │   /    ( 펫 )     \      │
        │  |                 |     │
        │   \               /      │
        │    `.          ,'        │
        │      `-.....-'  ,---.    │
        │          (@)   ( logo )  │  ← 작은 원 40%, 우하단 ⅓ 겹침
        │           ↑     `---'    │     내부 = 정적 provider 로고
        │   워커 1개 (합집합 아웃라인)  │
        └──────────────────────────┘
```

### After

```
        ┌──────── 오버레이 240px 창 ────────┐
        │                                  │
        │      ,-''''''-.  ╌╌╌╌╌╌╌╌        │  ← 접선(보이지 않음)
        │    ,'    (C)   `.       ╌╌╌╌     │     α ≈ 25°로 우측 수렴
        │   /    큰 링      \      ,-''-.  │
        │  |     ( 펫 )      |    /  72  \ │  ← 작은 원 36%, 내부 = 5H 숫자
        │   \               /    |  (X)   |│  ← (X) = 작은 워커, 자기 궤도
        │    `.          ,'       \      / │
        │      `-......-'  ╌╌╌     `-..-'  │
        │                     ╌╌╌╌╌        │
        │  (C) = 큰 워커, primary 마크      │
        └──────────────────────────────────┘

    z-order:  큰 워커 3  >  작은 워커 2  >  표면 1  >  링 0
```

### Interaction Changes

| Touchpoint | Before | After | Notes |
| --- | --- | --- | --- |
| 작은 원 위치 | 우하단 대각선, 큰 원과 ⅓ 겹침 | 수평 우측, 1.25%p 떨어짐 | 포인터 표면도 함께 이동 |
| 작은 원 내부 | 정적 provider 로고 | `active`: 5H 숫자 / 그 외: `SystemBadge` | 로고는 워커로 이동 |
| 궤도 워커 개수 | 1개 (합집합 아웃라인) | 2개 (각 원의 독립 원형 궤도) | primary는 큰 원, secondary는 작은 원 |
| 워커 주기 | 26s | 큰 26s / 작은 ≈11.2s | 선속도 일치 |
| 드래그·더블클릭·우클릭 | 큰 원 ∪ 작은 원 | **변경 없음** | 표면 좌표만 이동 |
| 접근성 트리 | 링 2개 + 워커 1개 이름 | **변경 없음** | 작은 워커는 `aria-hidden` (아래 근거) |
| 펫 렌더 크기 | 128px | 128px | 손실 없음 |

### 접근성 비대칭의 근거

큰 워커는 `role="img"` + `"Primary provider: Claude"`를 갖는다 — 큰 링의 이름이 `"Provider
usage: …"`로 일반적이라 primary의 정체를 아무도 말해주지 않기 때문이다. 반면 작은 링은 이미
`"Codex usage: 5-hour 41%, Weekly 58%"`로 자기 provider를 밝히므로, 작은 워커에 이름을 붙이면
**같은 사실이 세 번째로 읽힌다**. 따라서 작은 워커는 `aria-hidden="true"`이며 역할이 없다.
이는 §4.4가 이미 링 이름에 적용하고 있는 비대칭과 같은 원칙이다.

내부 5H 숫자도 같은 이유로 `aria-hidden="true"`다 — 링의 `aria-label`이 이미 `5-hour 41%`를
싣고 있다.

---

## Mandatory Reading

| Priority | File | Lines | Why |
| --- | --- | --- | --- |
| P0 | `src/lib/components/orbitPath.ts` | 1-151 | 전면 개편 대상. 모든 기하 상수의 단일 출처 |
| P0 | `src/lib/components/PetOverlay.svelte` | 1-212 | 워커 렌더·CSS 커스텀 프로퍼티·z-index |
| P0 | `src/lib/components/SatelliteRing.svelte` | 1-75 | 로고 제거 + 숫자 삽입 + 위치 규칙 |
| P0 | `src/lib/components/orbitPath.test.ts` | 1-161 | 합집합 검증 테스트가 통째로 무효화됨 |
| P1 | `src/lib/components/models.ts` | 1-55 | `SatelliteRingModel` / `OrbitModel` 형태 (변경 불필요 확인용) |
| P1 | `src/lib/components/SplitUsageRing.svelte` | 1-108 | `percent()` 클램프 · `data-severity` 패턴 · `.stale` 규칙 |
| P1 | `src/lib/components/PetOverlay.test.ts` | 228-484 | double ring / orbit walker 스위트 |
| P1 | `docs/ui-contract.md` | 167-248 | §4.4·§4.5 — 갱신 대상 |
| P2 | `src/App.svelte` | 551-632 | `overlayModel` 조립 (이번 변경에서 **수정 불필요**) |
| P2 | `src/App.test.ts` | 1211-1256 | 로고 위치를 가정한 어서션 2건 |
| P2 | `src/lib/styles/tokens.css` | 1-42 | `--sev-*`, `--font-mono`, `--satellite-puck`, `--overlay-stale-dim` |
| P2 | `src/lib/components/SystemBadge.svelte` | 1-110 | 배지 크기(2rem 고정) — 36% 원 안 여백 확인용 |

## External Documentation

| Topic | Source | Key Takeaway |
| --- | --- | --- |
| Apple HIG — Materials & Depth | developer.apple.com/design/human-interface-guidelines | 깊이는 그림자보다 **오클루전과 레이어 순서**로. 결정 6·7의 근거 |
| Apple HIG — Complications (watchOS) | 동상 | 작은 원형 표면에는 **단일 값 하나**만. 라벨+값 2줄은 이 크기에서 금물 → 결정 5 |
| Apple HIG — Typography (Rounded) | 동상 | 숫자 중심 소형 표면에는 SF Pro Rounded. `font-family: ui-rounded`로 **번들 없이** macOS 시스템 SF Rounded 사용 |
| SF Pro / SF Symbols 라이선스 | Apple Font License | 폰트·심볼 **재배포는 Apple 플랫폼 앱 한정**. CacheBite는 Windows/Linux도 타깃 → **번들 금지**. `ui-rounded` 제네릭 패밀리 + `--font-mono` 폴백으로 해결 |
| CSS `offset-path: path()` | MDN | 퍼센트를 받지 않음 → 경로는 **절대 픽셀**, `model.size`에서 재계산 필수 (기존 주석이 이미 경고) |

> **KEY_INSIGHT**: `ui-rounded`는 CSS 제네릭 패밀리라 폰트를 배포하지 않는다. macOS는 SF Pro
> Rounded로 해석하고 Windows/Linux는 다음 폴백(`--font-mono` = IBM Plex Mono)으로 내려간다.
> **APPLIES_TO**: Task 4의 `.readout` 스타일.
> **GOTCHA**: 플랫폼별 자간이 달라지므로 `font-variant-numeric: tabular-nums`를 함께 지정해
> 숫자 폭이 값에 따라 흔들리지 않게 한다.

---

## Patterns to Mirror

### NAMING_CONVENTION — 기하 상수는 `UPPER_SNAKE_CASE`, 순수 함수는 `camelCase`, 전부 named export
```ts
// SOURCE: src/lib/components/orbitPath.ts:29-46
/** Outer edge of the ring: arc radius 42 plus half of the 6.5 stroke. */
export const RING_OUTER_RADIUS = 45.25;
/** Edge length of the walking mark. */
export const WALKER_SIZE = 14;
const WALKER_RADIUS = WALKER_SIZE / 2;

export const SATELLITE_SIZE = 40;
```

### DERIVED_CONSTANT — 파생값은 손으로 적지 않고 식으로 남긴다
```ts
// SOURCE: src/lib/components/orbitPath.ts:58-64
const BIG_REACH = BIG_ORBIT + WALKER_RADIUS;
const SATELLITE_REACH = Math.max(
  SATELLITE_CENTER.x + SMALL_ORBIT + WALKER_RADIUS - 50,
  SATELLITE_CENTER.y + SMALL_ORBIT + WALKER_RADIUS - 50,
);
export const OVERLAY_BOUNDS_FACTOR =
  (2 * Math.max(BIG_REACH, SATELLITE_REACH)) / 100;
```

### COMMENT_STYLE — "무엇"이 아니라 **"이걸 바꾸면 무엇이 조용히 깨지는가"**
```ts
// SOURCE: src/lib/components/orbitPath.ts:71-77
/**
 * `Math.acos` outside [-1, 1] is `NaN`, and a `NaN` here would flow straight
 * into the path string, which browsers then drop in silence. The constants
 * above keep the two orbits intersecting, but they are exported to be tuned.
 */
```
```css
/* SOURCE: src/lib/components/PetOverlay.svelte:128-131 */
/* Child combinator, not descendant: the satellite's ring is one level deeper
   and owns its own geometry. A descendant selector here matches that one too,
   at equal specificity, so whichever rule the bundler happens to emit last
   would win — and `SatelliteRing`'s geometry would become dead code. */
```

### CSS_CUSTOM_PROPERTY_HANDOFF — 기하는 TS가 소유, CSS는 커스텀 프로퍼티로만 받는다
```svelte
<!-- SOURCE: src/lib/components/PetOverlay.svelte:44-51 -->
<section
  class="overlay"
  aria-label="CacheBite pet status"
  style:width={`${model.size}px`}
  style:--satellite-size={`${SATELLITE_SIZE}%`}
  style:--satellite-right={`${SATELLITE_RIGHT}%`}
  style:--satellite-bottom={`${SATELLITE_BOTTOM}%`}
>
```
```css
/* SOURCE: src/lib/components/SatelliteRing.svelte:32-38 */
.satellite {
  position: absolute;
  right: var(--satellite-right);
  bottom: var(--satellite-bottom);
  width: var(--satellite-size);
  height: var(--satellite-size);
}
```

### SEVERITY_STYLING — `data-severity` 속성 + `--sev-*` 토큰
```svelte
<!-- SOURCE: src/lib/components/SplitUsageRing.svelte:34-41, 85-99 -->
<path class="usage" data-severity={session.severity} … />
```
```css
.usage { stroke: var(--sev-unknown); }
.usage[data-severity='ok'] { stroke: var(--sev-ok); }
.usage[data-severity='warn'] { stroke: var(--sev-warn); }
.usage[data-severity='critical'] { stroke: var(--sev-critical); }
.usage[data-severity='exhausted'] { stroke: var(--sev-exhausted); }
```

### PERCENT_CLAMP — null·비유한값 방어는 표시 직전에
```svelte
<!-- SOURCE: src/lib/components/SplitUsageRing.svelte:5-9 -->
/** @param {import('./models').RingWindowModel} window */
const percent = (window) =>
  window.usedPercent === null || !Number.isFinite(window.usedPercent)
    ? 0
    : Math.min(100, Math.max(0, window.usedPercent));
```

### MOTION_DEGRADATION — 감축 모션과 미지원 엔진을 **둘 다** 처리
```css
/* SOURCE: src/lib/components/PetOverlay.svelte:197-211 */
@media (prefers-reduced-motion: reduce) {
  .walker { animation: none; }
}
@supports not (offset-path: path('M 0 0')) {
  .walker { display: none; }
}
```

### TEST_STRUCTURE — 기하는 경로 문자열을 파싱해 **좌표 불변식**으로 검증
```ts
// SOURCE: src/lib/components/orbitPath.test.ts:46-55
describe('orbitPath', () => {
  it('keeps the walker a constant step outside the ring in single mode', () => {
    const path = orbitPath(160, false);

    for (const p of points(path, 160)) {
      // Feet on the ring's outer edge, all the way round.
      expect(distance(p, CENTRE)).toBeCloseTo(BIG_ORBIT, 2);
    }
    expect(path.endsWith('Z')).toBe(true);
  });
```

### COMPONENT_TEST_STRUCTURE — 모델 팩토리 + `screen.getByRole` 이름 질의
```ts
// SOURCE: src/lib/components/PetOverlay.test.ts:229-252
const satellite = (
  overrides: Partial<SatelliteRingModel> = {},
): SatelliteRingModel => ({
  provider: 'codex',
  providerName: 'Codex',
  system: 'active',
  stale: false,
  session: { usedPercent: 41, severity: 'ok' },
  weekly: { usedPercent: 58, severity: 'ok' },
  ...overrides,
});
```

---

## Files to Change

| File | Action | Justification |
| --- | --- | --- |
| `src/lib/components/orbitPath.ts` | UPDATE | 상수 재정의, 합집합 기하 제거, 작은 원 궤도·주기·접선각 추가 |
| `src/lib/components/models.ts` | UPDATE | 공유 `clampPercent` 헬퍼 추가 (형태 변경 없음) |
| `src/lib/components/SplitUsageRing.svelte` | UPDATE | 로컬 `percent()`를 공유 헬퍼로 교체 (동작 동일) |
| `src/lib/components/SatelliteRing.svelte` | UPDATE | 로고 제거, 5H 숫자 추가, 중심 기준 배치로 전환 |
| `src/lib/components/PetOverlay.svelte` | UPDATE | 작은 워커 렌더, 커스텀 프로퍼티 교체, z-index 재정렬 |
| `src/lib/components/orbitPath.test.ts` | UPDATE | 합집합 테스트 2건 폐기 → 2궤도 + 접선 + 예산 테스트 |
| `src/lib/components/PetOverlay.test.ts` | UPDATE | 로고 위치·워커 개수·경로 어서션 갱신 + readout 신규 |
| `src/App.test.ts` | UPDATE | 위성 내부 로고를 가정한 어서션 2건 (~L1223, ~L1249) |
| `docs/ui-contract.md` | UPDATE | §4.4 기하 문단, §4.5 궤도/주기/상한 표 |
| `CLAUDE.md` | UPDATE | "orbit walker is bound to the primary" 불변식 문단 |

**변경 없음 (확인 완료)**: `src/App.svelte` — `OVERLAY_BOUNDS_FACTOR`와 `orbitDirection`만
import하며 두 시그니처 모두 유지된다. `overlayModel` 조립도 그대로다.
**Rust 변경 없음** — `ring_mode` 스키마 v6, 설정, IPC 모두 무관.

## NOT Building

- 접선 자체의 렌더링 (사용자가 "안 그림"을 선택)
- `ring_mode` 스키마·설정 UI 변경 — 여전히 `single` / `double` 둘뿐
- 작은 원 provider를 사용자가 직접 고르는 UI — `secondaryProvider(primary)` 파생 유지
- 작은 원 크기·거리의 사용자 설정화 — 상수로 고정
- 큰 원 내부(펫), 말풍선, 패널, 설정 화면 변경
- SF Pro / SF Symbols **폰트 파일 번들** — 라이선스상 불가 (External Documentation 참조)
- 두 워커의 충돌 회피 로직 — 오클루전은 의도된 동작 (결정 6)

---

## Step-by-Step Tasks

### Task 1: `orbitPath.ts` 기하 재정의

- **ACTION**: 상수를 새 값으로 교체하고, 원-원 교차 수학을 제거한 뒤 두 개의 독립 원형 궤도를 노출한다.
- **IMPLEMENT**:
  - 교체: `SATELLITE_SIZE = 40` → `36`
  - **삭제**: `SATELLITE_RIGHT`, `SATELLITE_BOTTOM` (배치가 우하단 오프셋에서 중심거리로 바뀜)
  - **신규**: `SATELLITE_DISTANCE = 64.5`, `SATELLITE_WALKER_SIZE = 9`
  - `SATELLITE_CENTER`를 `{ x: 50 + SATELLITE_DISTANCE, y: 50 }`로 재정의하고 **export** (PetOverlay가 CSS 프로퍼티로 넘겨야 함)
  - `SMALL_ORBIT = SATELLITE_SIZE / 2 + SATELLITE_WALKER_SIZE / 2`
  - `SATELLITE_REACH`는 두 축을 모두 `Math.max`에 넣어 대칭성을 남기되, 작은 워커 반지름을 쓴다:
    `SATELLITE_CENTER.{x,y} + SMALL_ORBIT + SATELLITE_WALKER_SIZE / 2 - 50`
  - **신규 export** `TANGENT_HALF_ANGLE = Math.asin((RING_OUTER_RADIUS - SATELLITE_SIZE / 2) / SATELLITE_DISTANCE) / RADIANS`
  - **신규 export** `WALK_DURATION_S = 26`, `SATELLITE_WALK_DURATION_S = WALK_DURATION_S * (SMALL_ORBIT / BIG_ORBIT)`
  - `arcTo`/`point`는 그대로 유지. **삭제**: `acosDegrees`와 `orbitPath` 안의 `dx/dy/distance/along/bearing/halfBig/halfSmall/leaveBig/rejoinBig/enterSmall/leaveSmall` 블록 전체
  - 내부 헬퍼 신설 후 두 공개 함수로 감싼다:
    ```ts
    const circleOrbit = (size: number, cx: number, cy: number, radius: number): string => {
      const scale = size / 100;
      const px = (value: number) => (value * scale).toFixed(3);
      const start = point(cx, cy, radius, 0);
      return `M ${px(start.x)} ${px(start.y)} ${arcTo(cx, cy, radius, 0, 360, scale)} Z`;
    };
    export const orbitPath = (size: number): string => circleOrbit(size, 50, 50, BIG_ORBIT);
    export const satelliteOrbitPath = (size: number): string =>
      circleOrbit(size, SATELLITE_CENTER.x, SATELLITE_CENTER.y, SMALL_ORBIT);
    ```
  - `orbitPath`에서 **`hasSatellite` 파라미터를 제거**한다 (큰 궤도는 이제 모드와 무관하게 완전한 원)
  - `orbitDirection`은 손대지 않는다
- **MIRROR**: `NAMING_CONVENTION`, `DERIVED_CONSTANT`, `COMMENT_STYLE`
- **IMPORTS**: 기존 `import type { OrbitDirection } from './models';` 유지. 신규 import 없음
- **GOTCHA**:
  - 파일 상단 독스트링이 "합집합 아웃라인"과 "작은 원이 우하단"을 전제로 쓰여 있다. **본문과 함께 반드시 고쳐라** — 이 파일의 주석은 다음 사람이 상수를 튜닝할 때 읽는 유일한 근거다.
  - `acosDegrees`를 지우면 그것을 정당화하던 "never emits NaN" 테스트의 근거도 바뀐다. 테스트는 남기되 사유를 갱신한다(Task 6).
  - `SATELLITE_DISTANCE`를 키우면 `OVERLAY_BOUNDS_FACTOR`가 1.875를 넘어 **펫이 128px 아래로 줄어든다**. 여유는 단 `93.75 - 91.5 = 2.25`%p뿐이라는 주석을 남겨라.
- **VALIDATE**: `pnpm check`로 타입 오류 0 확인. `pnpm vitest run src/lib/components/orbitPath.test.ts`는 이 시점에 실패가 정상(Task 6에서 정합).

### Task 2: `models.ts`에 공유 퍼센트 클램프 추가

- **ACTION**: `SplitUsageRing`의 로컬 `percent()`를 두 컴포넌트가 함께 쓸 수 있게 승격한다.
- **IMPLEMENT**:
  ```ts
  /**
   * A window's usage as a 0-100 number, or `null` when the provider did not
   * report one. Callers decide what absence looks like: an arc draws 0 length,
   * the satellite's readout draws an em dash. Collapsing both into 0 here would
   * make "no data" and "0% used" indistinguishable on screen.
   */
  export const clampPercent = (window: RingWindowModel): number | null =>
    window.usedPercent === null || !Number.isFinite(window.usedPercent)
      ? null
      : Math.min(100, Math.max(0, window.usedPercent));
  ```
- **MIRROR**: `PERCENT_CLAMP`
- **IMPORTS**: `RingWindowModel`은 이미 `models.ts` 안에 있으므로 추가 import 없음
- **GOTCHA**: 기존 함수는 null을 **0으로** 반환했다. 새 함수는 **null**을 반환한다 — 호출부에서 `?? 0`을 붙이지 않으면 `stroke-dasharray`가 `null 100`이 되어 아크가 사라진다.
- **VALIDATE**: `pnpm check`

### Task 3: `SplitUsageRing.svelte`를 공유 헬퍼로 전환

- **ACTION**: 로컬 `percent()`를 지우고 `clampPercent`를 쓰되 **렌더 결과는 완전히 동일**하게 유지한다.
- **IMPLEMENT**:
  - `import { clampPercent } from './models';` 추가
  - `const percent = (window) => clampPercent(window) ?? 0;` — `?? 0`이 기존 동작을 보존한다
  - `label()`과 `stroke-dasharray` 사용처는 그대로 둔다
- **MIRROR**: `PERCENT_CLAMP`
- **IMPORTS**: `./models`
- **GOTCHA**: 이 컴포넌트는 **큰 링과 작은 링이 공유**한다. 여기서 동작이 바뀌면 두 링이 같이 깨진다. `label()`이 `severity === 'unknown'`을 먼저 보므로 `null`이 라벨 텍스트로 새지는 않지만, `percent()`의 `?? 0`을 빠뜨리면 아크가 조용히 사라진다.
- **VALIDATE**: `pnpm vitest run src/lib/components/PetOverlay.test.ts -t "renders an unknown window as a neutral unfilled track"` — `stroke-dasharray`가 `'0 100'`으로 유지되어야 한다.

### Task 4: `SatelliteRing.svelte` — 로고 제거, 5H 숫자 삽입, 중심 기준 배치

- **ACTION**: 내부 콘텐츠를 교체하고 위치 규칙을 `right/bottom`에서 중심 좌표 + `translate`로 바꾼다.
- **IMPLEMENT**:
  - **삭제**: `import ProviderLogo`, `<div class="logo">…</div>`, `.logo` CSS 규칙
  - **추가**: `import { clampPercent } from './models';`
  - 표시 텍스트 파생:
    ```js
    const sessionPercent = $derived(clampPercent(model.session));
    // An em dash, not `0` — the ring is already drawing an empty track for the
    // unknown case, and a `0` there would read as "nothing used yet".
    const sessionText = $derived(
      sessionPercent === null ? '—' : String(Math.round(sessionPercent)),
    );
    ```
  - `active`일 때 링 **뒤(DOM상 나중)** 에 숫자를 얹어 아크 위에 그려지게 한다:
    ```svelte
    {#if model.system === 'active'}
      <SplitUsageRing … />
      <!-- The ring's own aria-label already carries `5-hour 41%`; announcing
           the same number again would read it twice. This is the visual
           duplicate, not a second fact. -->
      <div
        class="readout"
        class:stale={model.stale}
        data-testid="satellite-readout"
        data-severity={model.session.severity}
        aria-hidden="true"
      >
        {sessionText}
      </div>
    {/if}
    ```
  - `.satellite` 위치 규칙 교체:
    ```css
    .satellite {
      position: absolute;
      left: var(--satellite-center-x);
      top: var(--satellite-center-y);
      width: var(--satellite-size);
      height: var(--satellite-size);
      transform: translate(-50%, -50%);
    }
    ```
  - `.readout` 스타일:
    ```css
    .readout {
      position: absolute;
      display: grid;
      inset: 0;
      place-items: center;
      color: var(--sev-unknown);
      font-family: ui-rounded, var(--font-mono);
      font-size: var(--satellite-readout-size);
      font-weight: 600;
      font-variant-numeric: tabular-nums;
      letter-spacing: -0.02em;
      line-height: 1;
    }
    .readout[data-severity='ok'] { color: var(--sev-ok); }
    .readout[data-severity='warn'] { color: var(--sev-warn); }
    .readout[data-severity='critical'] { color: var(--sev-critical); }
    .readout[data-severity='exhausted'] { color: var(--sev-exhausted); }
    .readout.stale { opacity: var(--overlay-stale-dim); }
    ```
  - `.puck`, `.satellite :global(.ring)`, `.satellite :global(.ring-label)`, `.satellite-badge`는 **그대로 둔다**
- **MIRROR**: `SEVERITY_STYLING`, `CSS_CUSTOM_PROPERTY_HANDOFF`, `PERCENT_CLAMP`
- **IMPORTS**: `./models` (신규), `ProviderLogo` (제거)
- **GOTCHA**:
  - `--satellite-readout-size`는 **px 값**이어야 한다. `font-size`에 `%`를 쓰면 부모 폰트 크기 기준이 되어 오버레이 크기와 무관해진다. PetOverlay가 `model.size`에서 계산해 넘긴다(Task 5).
  - `--satellite-puck`이 고정 어두운색(`#1c1f24`)이므로 숫자는 **라이트 모드에서도 어두운 배경 위**에 놓인다. `--sev-*`의 라이트 값 대비는 실제 화면에서 확인해야 한다(Manual Validation).
  - 파일 상단 주석이 "40% of the big ring, pinned lower-right and overlapping it by about a third"라고 되어 있다. **반드시 새 배치로 고쳐라.**
- **VALIDATE**: `pnpm vitest run src/lib/components/PetOverlay.test.ts` (Task 7 이후 통과), 브라우저에서 숫자가 퍽 안에 들어오는지 육안 확인.

### Task 5: `PetOverlay.svelte` — 작은 워커 추가, 프로퍼티 교체, z-index 재정렬

- **ACTION**: 두 번째 워커를 렌더하고 위치용 커스텀 프로퍼티를 새 상수에 맞춘다.
- **IMPLEMENT**:
  - import 갱신:
    ```js
    import {
      orbitPath,
      satelliteOrbitPath,
      SATELLITE_CENTER,
      SATELLITE_SIZE,
      SATELLITE_WALKER_SIZE,
      SATELLITE_WALK_DURATION_S,
      WALK_DURATION_S,
      WALKER_SIZE,
    } from './orbitPath';
    ```
  - 파생값 (`hasSatellite` 인자가 사라졌다):
    ```js
    const walkPath = $derived(orbitPath(model.size));
    const satelliteWalkPath = $derived(satelliteOrbitPath(model.size));
    ```
  - `<section>`의 `style:` 목록 교체 (`--satellite-right` / `--satellite-bottom` 삭제):
    ```svelte
    style:--satellite-size={`${SATELLITE_SIZE}%`}
    style:--satellite-center-x={`${SATELLITE_CENTER.x}%`}
    style:--satellite-center-y={`${SATELLITE_CENTER.y}%`}
    style:--satellite-readout-size={`${(model.size * SATELLITE_SIZE * 0.33) / 100}px`}
    style:--walk-duration={`${WALK_DURATION_S}s`}
    style:--satellite-walk-duration={`${SATELLITE_WALK_DURATION_S.toFixed(3)}s`}
    ```
  - `{#if model.satellite}` 블록 안, `.satellite-surface` 뒤에 작은 워커를 추가한다. **`.overlay`의 직계 자식이어야 한다** — `offset-path`가 오버레이 상자 기준 절대 픽셀이므로 `.satellite`(36% 크기 + translate) 안에 넣으면 좌표가 전부 어긋난다:
    ```svelte
    <!-- Aria-hidden on purpose: the satellite's own ring label already says
         "Codex usage: …", so naming this would read the same provider a third
         time. The big walker is named because the big ring's label is generic. -->
    <div
      class="satellite-walker"
      class:reverse={model.orbit.direction === 'reverse'}
      data-testid="satellite-orbit-walker"
      data-direction={model.orbit.direction}
      aria-hidden="true"
      style:offset-path={`path("${satelliteWalkPath}")`}
      style:width={`${SATELLITE_WALKER_SIZE}%`}
    >
      <ProviderLogo provider={model.satellite.provider} />
    </div>
    ```
  - `.satellite-surface` 위치 규칙을 `.satellite`와 동일하게 맞춘다:
    ```css
    .satellite-surface {
      position: absolute;
      z-index: 1;
      left: var(--satellite-center-x);
      top: var(--satellite-center-y);
      width: var(--satellite-size);
      height: var(--satellite-size);
      border-radius: 50%;
      transform: translate(-50%, -50%);
      clip-path: circle(50% at 50% 50%);
      cursor: grab;
      touch-action: none;
    }
    ```
  - 워커 규칙을 공유 셀렉터로 묶고 z-index를 재배치한다:
    ```css
    /* Depth order encodes the perspective: the big ring is the near end of the
       cone, so its walker passes in front of the far ring; the satellite's own
       walker still sits above the big ring's arc so it never vanishes into it. */
    .walker,
    .satellite-walker {
      position: absolute;
      top: 0;
      left: 0;
      aspect-ratio: 1;
      pointer-events: none;
      offset-anchor: 50% 50%;
      offset-rotate: auto;
    }
    .walker { z-index: 3; animation: walk-orbit var(--walk-duration) linear infinite; }
    .satellite-walker {
      z-index: 2;
      animation: walk-orbit var(--satellite-walk-duration) linear infinite;
    }
    .walker.reverse,
    .satellite-walker.reverse { animation-direction: reverse; }
    ```
  - **감축 모션과 미지원 엔진 규칙에 새 셀렉터를 반드시 추가한다**:
    ```css
    @media (prefers-reduced-motion: reduce) {
      .walker, .satellite-walker { animation: none; }
    }
    @supports not (offset-path: path('M 0 0')) {
      .walker, .satellite-walker { display: none; }
    }
    ```
- **MIRROR**: `CSS_CUSTOM_PROPERTY_HANDOFF`, `MOTION_DEGRADATION`, `COMMENT_STYLE`
- **IMPORTS**: `ProviderLogo`는 이미 import되어 있다 (큰 워커가 쓰는 중)
- **GOTCHA**:
  - `@supports` 폴백을 빠뜨리면 WebKitGTK 2.38 미만에서 **두 로고가 오버레이 좌상단 모서리에 겹쳐 박제**된다 — 기존 주석이 경고하는 그 실패다.
  - `.overlay > :global(.ring)` 자식 결합자를 건드리지 마라. 작은 링의 기하가 조용히 죽는다(기존 주석 참조).
  - 작은 워커의 방향은 `model.orbit.direction`을 **공유**한다. 두 마크가 같은 쪽으로 도는 것이 하나의 장면으로 읽히고, 난수 소비도 오버레이당 1회로 유지된다.
  - 작은 워커는 `model.satellite.system`과 **무관하게 렌더**한다 — 연결 실패한 provider일수록 어느 쪽인지 알아야 한다(`PetOverlay.test.ts:349-369`가 이미 이 규칙을 검증 중).
- **VALIDATE**: `pnpm vitest run src/lib/components/PetOverlay.test.ts`, `pnpm check`

### Task 6: `orbitPath.test.ts` 재작성

- **ACTION**: 합집합 아웃라인을 검증하던 테스트를 폐기하고, 두 독립 궤도 + 접선 관계 + 예산을 검증한다.
- **IMPLEMENT**:
  - import 갱신: `SATELLITE_RIGHT`/`SATELLITE_BOTTOM` 제거, `satelliteOrbitPath`, `SATELLITE_CENTER`, `SATELLITE_DISTANCE`, `SATELLITE_WALKER_SIZE`, `TANGENT_HALF_ANGLE`, `WALK_DURATION_S`, `SATELLITE_WALK_DURATION_S` 추가
  - `points()`, `distance()` 헬퍼는 **그대로 재사용**
  - 로컬 상수 갱신 — 모듈이 export하는 `SATELLITE_CENTER`를 **직접 쓴다**(재계산 금지):
    ```ts
    const BIG_ORBIT = RING_OUTER_RADIUS + WALKER_SIZE / 2;
    const SMALL_ORBIT = SATELLITE_SIZE / 2 + SATELLITE_WALKER_SIZE / 2;
    ```
  - **폐기**: `'walks the union outline of both circles in double mode'`, `'hands off between the circles without a jump'`
  - **유지·수정**: `'keeps the walker a constant step outside the ring in single mode'`, `'scales with the overlay so the loop is size-independent'` → `orbitPath(size)` 인자 1개로 조정
  - **신규**:
    ```ts
    it('keeps the satellite walker a constant step outside the small ring', () => {
      for (const p of points(satelliteOrbitPath(160), 160)) {
        expect(distance(p, SATELLITE_CENTER)).toBeCloseTo(SMALL_ORBIT, 2);
      }
    });

    it('places the satellite on the horizontal centre line, to the right', () => {
      // Requirement 1: the small ring moved off the lower-right diagonal onto
      // the axis. A y other than 50 would tilt the tangent construction and the
      // cone would stop reading as a recede.
      expect(SATELLITE_CENTER.y).toBe(50);
      expect(SATELLITE_CENTER.x).toBeGreaterThan(50);
    });

    it('separates the two rings so neither swallows the other', () => {
      const gap = SATELLITE_DISTANCE - RING_OUTER_RADIUS - SATELLITE_SIZE / 2;
      expect(gap).toBeGreaterThan(0);
    });

    it('converges the two outer tangents on both circles at the same angle', () => {
      // The design constraint from the request: one line off the big circle and
      // one off the small one, converging at equal angles and touching both.
      // Tangency means the perpendicular distance from each centre equals its
      // own radius — check that directly rather than trusting the arcsin.
      const alpha = (TANGENT_HALF_ANGLE * Math.PI) / 180;
      const r = SATELLITE_SIZE / 2;
      // Upper tangent, y pointing down: the line direction is (cos a, sin a),
      // so the contact point sits along the normal (sin a, -cos a).
      const contact = (cx: number, cy: number, radius: number) => ({
        x: cx + radius * Math.sin(alpha),
        y: cy - radius * Math.cos(alpha),
      });
      const onBig = contact(50, 50, RING_OUTER_RADIUS);
      const onSmall = contact(SATELLITE_CENTER.x, SATELLITE_CENTER.y, r);
      const perpendicular = (cx: number, cy: number) =>
        Math.abs(
          (onSmall.y - onBig.y) * cx -
            (onSmall.x - onBig.x) * cy +
            onSmall.x * onBig.y -
            onSmall.y * onBig.x,
        ) / Math.hypot(onSmall.x - onBig.x, onSmall.y - onBig.y);

      expect(perpendicular(50, 50)).toBeCloseTo(RING_OUTER_RADIUS, 6);
      expect(perpendicular(SATELLITE_CENTER.x, SATELLITE_CENTER.y)).toBeCloseTo(r, 6);
      // And the angle is one a person would call a recede, not a megaphone.
      expect(TANGENT_HALF_ANGLE).toBeGreaterThan(20);
      expect(TANGENT_HALF_ANGLE).toBeLessThan(30);
    });

    it('matches the two walkers on linear speed, not angular speed', () => {
      // The same period on a smaller circle would make the satellite's mark crawl.
      expect(SATELLITE_WALK_DURATION_S / WALK_DURATION_S).toBeCloseTo(
        SMALL_ORBIT / BIG_ORBIT,
        6,
      );
      expect(SATELLITE_WALK_DURATION_S).toBeLessThan(WALK_DURATION_S);
    });
    ```
  - **`'reserves enough room for the walker, the outermost element'` 갱신** — 작은 워커가 자기 크기를 갖게 됐으므로 계산을 분리한다:
    ```ts
    const reach = Math.max(
      BIG_ORBIT + WALKER_SIZE / 2,
      SATELLITE_CENTER.x + SMALL_ORBIT + SATELLITE_WALKER_SIZE / 2 - 50,
    );
    expect(OVERLAY_BOUNDS_FACTOR).toBeCloseTo((2 * reach) / 100, 5);
    expect((reach / 100) * (240 / OVERLAY_BOUNDS_FACTOR) * 2).toBeLessThanOrEqual(240);
    // The consequence the design was tuned for: every shipped pet declares
    // defaultSize 128, and the clamp must not drop below it or a display
    // preference would silently shrink the pet.
    expect(240 / OVERLAY_BOUNDS_FACTOR).toBeGreaterThanOrEqual(128);
    ```
  - **`'never emits NaN coordinates'` 유지·수정** — 두 함수 모두 검사하고, 사유 주석을 "acos 도메인"에서 "상수 튜닝 시 경로가 조용히 사라지는 것 방지"로 갱신
  - `'splits the direction roll evenly across the unit interval'`은 그대로
- **MIRROR**: `TEST_STRUCTURE`
- **IMPORTS**: `vitest`의 `describe/expect/it` 유지
- **GOTCHA**: 기존 파일은 `SATELLITE_CENTRE`(영국식 철자)를 `SATELLITE_RIGHT/BOTTOM`에서 **로컬로 재구성**했다. 이제는 export된 상수를 직접 쓴다 — 테스트가 상수를 재계산하면 소스와 어긋나도 초록으로 남는다.
- **VALIDATE**: `pnpm vitest run src/lib/components/orbitPath.test.ts` — 전부 통과

### Task 7: `PetOverlay.test.ts` 갱신

- **ACTION**: 로고 위치·워커 개수·경로 변화를 새 구조에 맞추고 readout 테스트를 추가한다.
- **IMPLEMENT**:
  - `'names the satellite by its provider while the big ring stays generic'` — 로고가 이제 **작은 워커 안**이므로 위치를 명시:
    ```ts
    expect(
      screen
        .getByTestId('satellite-orbit-walker')
        .querySelector('[data-testid="provider-logo-codex"]'),
    ).not.toBeNull();
    expect(
      screen
        .getByTestId('satellite-ring')
        .querySelector('[data-testid="provider-logo-codex"]'),
    ).toBeNull();
    ```
  - `'renders the Claude mark when Claude is the satellite'` — 개수 2는 유지되지만(큰 워커 + 작은 워커) 의미가 바뀌었으므로 주석을 고친다
  - `'walks in both ring modes, on a longer loop once there is a satellite'` — **제목과 내용을 교체**한다. 큰 궤도는 이제 모드와 무관하다:
    ```ts
    it('keeps the big loop identical in both ring modes and adds a second walker', () => {
      const single = render(PetOverlay, { props: { model: model() } });
      const singlePath = screen.getByTestId('orbit-walker').style.offsetPath;
      expect(singlePath.startsWith('path(')).toBe(true);
      expect(screen.queryByTestId('satellite-orbit-walker')).toBeNull();

      single.unmount();
      render(PetOverlay, { props: { model: model({ satellite: /* 기존 팩토리 */ }) } });

      // The big ring is always the primary's own circle; only a second mark
      // appears. A changed big path would mean the mode resized the pet.
      expect(screen.getByTestId('orbit-walker').style.offsetPath).toBe(singlePath);
      const satelliteWalker = screen.getByTestId('satellite-orbit-walker');
      expect(satelliteWalker.style.offsetPath).not.toBe(singlePath);
      expect(satelliteWalker.style.offsetPath.startsWith('path(')).toBe(true);
    });
    ```
  - **신규 테스트**:
    ```ts
    it('reads out the satellite 5-hour figure without announcing it twice', () => {
      render(PetOverlay, { props: { model: model(satellite()) } });

      const readout = screen.getByTestId('satellite-readout');
      expect(readout.textContent?.trim()).toBe('41');
      expect(readout.getAttribute('data-severity')).toBe('ok');
      // The ring's label already carries the number; a second announcement
      // would read the same fact twice.
      expect(readout.getAttribute('aria-hidden')).toBe('true');
    });

    it('draws an em dash rather than a zero when the window is unknown', () => {
      render(PetOverlay, {
        props: {
          model: model(
            satellite({ session: { usedPercent: null, severity: 'unknown' } }),
          ),
        },
      });

      expect(screen.getByTestId('satellite-readout').textContent?.trim()).toBe('—');
    });

    it('clamps a satellite reading above the window limit', () => {
      render(PetOverlay, {
        props: {
          model: model(
            satellite({ session: { usedPercent: 137, severity: 'exhausted' } }),
          ),
        },
      });

      expect(screen.getByTestId('satellite-readout').textContent?.trim()).toBe('100');
    });

    it('replaces the readout with a badge when the satellite is not active', () => {
      render(PetOverlay, {
        props: { model: model(satellite({ system: 'auth_required' })) },
      });

      expect(screen.queryByTestId('satellite-readout')).toBeNull();
      expect(screen.getByRole('status').getAttribute('aria-label')).toBe(
        'Authentication required',
      );
      // Identity matters most when the provider is failing.
      expect(
        screen
          .getByTestId('satellite-orbit-walker')
          .querySelector('[data-testid="provider-logo-codex"]'),
      ).not.toBeNull();
    });

    it('keeps the satellite walker out of the accessibility tree', () => {
      render(PetOverlay, { props: { model: model(satellite()) } });

      // Only the big walker is named: the big ring's label is generic, the
      // satellite's already says "Codex".
      expect(screen.getAllByRole('img', { name: /provider/i })).toHaveLength(1);
      expect(
        screen.getByTestId('satellite-orbit-walker').getAttribute('aria-hidden'),
      ).toBe('true');
    });
    ```
- **MIRROR**: `COMPONENT_TEST_STRUCTURE`
- **IMPORTS**: 변경 없음
- **GOTCHA**: `getAllByRole('img')`는 펫 애니메이션(`name: 'Geometric pet'`)과 링(`name: '… usage: …'`)도 잡는다. 위처럼 `{ name: /provider/i }`로 좁혀야 한다.
- **VALIDATE**: `pnpm vitest run src/lib/components/PetOverlay.test.ts`

### Task 8: `App.test.ts` 배선 테스트 갱신

- **ACTION**: 위성 **내부**에서 로고를 찾던 어서션 2건을 워커로 옮긴다.
- **IMPLEMENT**:
  - `~L1219-1224` (`'gives the satellite the other provider and its own usage'`):
    ```ts
    const satellite = await screen.findByTestId('satellite-ring');
    // The logo now rides the satellite's own orbit; the ring itself carries
    // the numbers.
    expect(
      screen
        .getByTestId('satellite-orbit-walker')
        .querySelector('[data-testid="provider-logo-codex"]'),
    ).not.toBeNull();
    expect(
      satellite.querySelector('[data-testid="usage-ring"]')?.getAttribute('aria-label'),
    ).toBe('Codex usage: 5-hour 20%, Weekly 40%');
    expect(
      satellite.querySelector('[data-testid="satellite-readout"]')?.textContent?.trim(),
    ).toBe('20');
    ```
  - `~L1245-1251` (`'keeps the walker on the primary and follows a primary change'`):
    ```ts
    // The satellite is derived, never stored, so flipping the primary flips it.
    expect(
      screen
        .getByTestId('satellite-orbit-walker')
        .querySelector('[data-testid="provider-logo-claude"]'),
    ).not.toBeNull();
    ```
  - `'draws no satellite in single mode but still walks the primary'`(~L1253)에 **추가**:
    ```ts
    expect(screen.queryByTestId('satellite-orbit-walker')).toBeNull();
    ```
- **MIRROR**: 기존 파일의 `findByTestId` + `querySelector` 조합
- **IMPORTS**: 변경 없음
- **GOTCHA**: single 모드 누출 어서션을 빠뜨리면 작은 워커가 새어 나와도 CI가 조용하다.
- **VALIDATE**: `pnpm vitest run src/App.test.ts -t "ring mode wiring"`

### Task 9: `docs/ui-contract.md` §4.4 · §4.5 갱신

- **ACTION**: 문서가 코드보다 오래된 상태로 남지 않게 두 절을 새 기하로 다시 쓴다.
- **IMPLEMENT**:
  - **§4.4 표(L172-178)**: `작은 원 중앙` 행을 `해당 provider의 로고` → `` `active`면 5시간 사용률 숫자, 그 외에는 §4.2 배지 ``로 교체
  - **§4.4 기하 문단(L180-184)** 전면 교체:
    - 작은 원 지름 36%, 중심을 큰 원 중심에서 **수평 우측 64.5%** 지점에 둔다
    - 두 원의 외접 공통접선이 **α ≈ 25°**로 우측 수렴하며, 접점은 각 원의 꼭대기에서 α만큼 우측으로 돌아간 지점이다 (정확한 최상단이 아니다 — 최상단끼리 이으면 두 선이 평행이 되어 수렴이 성립하지 않는다)
    - 두 링 사이 틈 1.25%p. 접선은 **렌더하지 않는다**
    - `5H`/`WK` 라벨 숨김 규칙은 그대로 유지
  - **§4.4 신규 문단** — 내부 숫자 규칙:
    - 항상 **5시간 창**의 값이다. "더 나쁜 쪽"을 고르면 라벨 없는 숫자가 어느 창인지 알 수 없게 되므로 규칙을 고정한다
    - 색은 `session.severity`의 `--sev-*` — 숫자와 상반원 아크가 같은 사실을 가리킨다
    - 데이터 없음은 `0`이 아니라 `—`. `0`은 "아직 안 썼음"으로 읽힌다
    - `aria-hidden` — 링의 `aria-label`이 이미 같은 수치를 싣는다
  - **§4.5 표(L216 기하 행)**: `SATELLITE_WALKER_SIZE` = 오버레이의 9% 추가. 큰 마크보다 작게 잡는 이유는 각 링 대비 비율을 맞추기 위해서라는 사유 포함
  - **§4.5 표(L217 궤도 행)** 교체 → `` 각 원이 자기 마크를 자기 궤도에 태운다. 큰 원 = primary, 작은 원 = secondary. 합집합 아웃라인은 폐기(두 원이 이제 떨어져 있다) ``
  - **§4.5 표(L220 주기 행)** → `큰 워커 26s. 작은 워커는 선속도를 맞춰 26 × (SMALL_ORBIT / BIG_ORBIT) ≈ 11.2s로 파생. 같은 주기를 주면 작은 마크가 기어간다`
  - **§4.5 표(L215 provider 행)** 아래에 secondary 워커 행 신설 — `aria-hidden`인 이유(작은 링이 이미 자기 이름을 밝힘) 명시
  - **§4.5 L223-228 문단**: `≈1.39` → `1.83`, `최대 약 172px` → `최대 약 131px`. 그리고 **실질적으로 중요한 사실을 추가**: 번들된 모든 펫 매니페스트가 `defaultSize 128`이므로 상한은 아직 구속력이 없고, 여유는 `93.75 - 91.5 = 2.25`%p뿐이다 — `SATELLITE_DISTANCE`를 더 밀면 펫이 실제로 줄어든다
  - **§4.5 신규 문단** — z-order와 오클루전:
    - `큰 워커 3 > 작은 워커 2 > 포인터 표면 1 > 링 0`
    - 큰 워커가 3시 방향에서 작은 링의 퍽 위를 지나는 것은 **버그가 아니라 원근 표현**이다. 큰 원이 가까운 쪽이므로 먼 것을 가리는 게 맞다
    - 작은 워커는 큰 링의 아크 획(6.5%p) 위를 지나되 가려지지 않는다. 불투명한 요소는 펫(`inset: 16%`)과 퍽뿐인데 둘 다 작은 워커 궤도(x 최소 92)에 닿지 않는다
    - 완전 분리를 원하면 `SATELLITE_DISTANCE`를 77.25로 올려야 하지만 **펫이 115px로 축소**된다 — 이 트레이드오프를 기록해 둔다
- **MIRROR**: 문서의 기존 톤 — 규칙 + **"이걸 바꾸면 무엇이 깨지는가"**
- **GOTCHA**: 이 문서는 CLAUDE.md가 "source of truth"로 지정한 파일이다. 코드와 어긋나면 다음 세션이 잘못된 전제로 작업한다.
- **VALIDATE**: 육안 리뷰 — §4.4·§4.5의 모든 수치를 `orbitPath.ts` 상수와 1:1 대조

### Task 10: `CLAUDE.md` 불변식 갱신

- **ACTION**: "The orbit walker is bound to the primary…" 문단을 새 구조로 고친다.
- **IMPLEMENT**: 해당 불변식 항목을 아래 취지로 다시 쓴다:
  - 워커는 이제 **두 개**다. 큰 원의 마크는 항상 `primary_provider`이고 **두 모드 모두에서** 렌더된다. 작은 원의 마크는 `double`에서만 나타나며 `secondaryProvider(primary)`를 태운다
  - 두 궤도는 이제 **독립된 원**이다 — 합집합 아웃라인은 두 원이 떨어지면서 폐기되었다
  - `OVERLAY_BOUNDS_FACTOR`는 여전히 `overlaySize`를 클램프하고 **두 모드에 동일하게** 적용된다. 상한을 정하는 것은 작은 원 궤도의 **우측** 바깥이다
  - `offset-path`가 절대 픽셀이라는 제약과 말풍선 152px 상한 관련 문장은 **그대로 유지**한다
  - 링 모드 불변식("Ring mode is display-only")의 "the satellite is always `secondaryProvider(primary)`"는 그대로다 — 작은 워커도 같은 파생값을 쓴다
- **GOTCHA**: `ring_mode`가 표시 전용이라는 불변식을 약화시키지 마라. 워커가 두 개가 되어도 수집·새로고침·알림·펫 무드에는 여전히 닿지 않는다.
- **VALIDATE**: 육안 리뷰

---

## Testing Strategy

### Unit Tests

| Test | Input | Expected Output | Edge Case? |
| --- | --- | --- | --- |
| 작은 궤도 반지름 일정 | `satelliteOrbitPath(160)` | 모든 앵커가 `SATELLITE_CENTER`에서 `SMALL_ORBIT` 거리 | — |
| 큰 궤도 모드 불변 | `orbitPath(160)` | single·double에서 동일 문자열 | ✔ 모드 전환이 펫을 리사이즈하지 않음 |
| 수평 배치 | `SATELLITE_CENTER` | `y === 50`, `x > 50` | — |
| 두 링 분리 | `d - R - r` | `> 0` (= 1.25) | ✔ 겹침 회귀 방지 |
| 접선 성립 | 접점 2개를 잇는 직선 | 각 중심까지의 수직거리 = 각 반지름 | ✔ 상수 튜닝 시 원근 붕괴 감지 |
| 수렴각 범위 | `TANGENT_HALF_ANGLE` | `20 < α < 30` | — |
| 선속도 일치 | 두 주기의 비 | `SMALL_ORBIT / BIG_ORBIT` | — |
| 크기 예산 | `OVERLAY_BOUNDS_FACTOR` | `240 / factor ≥ 128` | ✔ 펫 축소 회귀 방지 |
| NaN 없음 | size ∈ {64,128,172,240} × 두 함수 | `/NaN/` 미매치 | ✔ 브라우저가 조용히 드롭 |
| 5H 숫자 | `session.usedPercent = 41` | `'41'`, `data-severity='ok'` | — |
| 데이터 없음 | `usedPercent: null` | `'—'` (0이 아님) | ✔ |
| 한도 초과 | `usedPercent: 137` | `'100'` (클램프) | ✔ |
| 비-active | `system: 'auth_required'` | readout 없음 + `role=status` 배지 + 워커 유지 | ✔ |
| 작은 워커 a11y | double 모드 | `aria-hidden="true"`, 이름 있는 provider img는 1개 | ✔ 중복 낭독 방지 |
| single 모드 누출 | `satellite: null` | `satellite-orbit-walker` 부재 | ✔ |
| stale 분리 | `satellite.stale = true` | 링과 readout만 dim, 펫은 아님 | ✔ |
| 배선 (App) | primary 전환 | 작은 워커의 로고가 따라 뒤집힘 | ✔ |

### Edge Cases Checklist

- [x] `usedPercent === null` → `—`
- [x] `usedPercent > 100` → `100`으로 클램프
- [x] `usedPercent` 비유한값(`NaN`/`Infinity`) → `clampPercent`가 `null` 반환
- [x] `severity === 'unknown'` → `--sev-unknown` 색
- [x] 작은 원이 비-`active` → 숫자 대신 배지, 워커는 유지
- [x] 큰 원이 비-`active`, 작은 원 `active` → 서로 독립 (기존 테스트 유지)
- [x] `single` 모드 → 작은 원·작은 워커 모두 부재, 큰 궤도는 동일
- [x] 말풍선 표시 중 (`size` 152 → 128) → 두 경로 모두 재계산
- [ ] `prefers-reduced-motion` → 수동 검증
- [ ] `offset-path` 미지원 엔진 → 수동 검증 (Linux WebKitGTK)
- [ ] 라이트/다크 양쪽에서 퍽 위 숫자 대비 → 수동 검증

> 네트워크 실패·동시성·권한은 이 변경의 범위 밖이다 — 렌더러 표시 계층만 건드리며 수집·IPC·저장소는 손대지 않는다.

---

## Validation Commands

### Static Analysis
```bash
pnpm check
```
EXPECT: svelte-check 오류 0. 특히 `orbitPath(size, hasSatellite)` 호출부가 남아 있으면 여기서 걸린다.

```bash
pnpm lint
```
EXPECT: eslint + prettier 통과

### Unit Tests (영향 범위)
```bash
pnpm vitest run src/lib/components/orbitPath.test.ts
pnpm vitest run src/lib/components/PetOverlay.test.ts
pnpm vitest run src/App.test.ts -t "ring mode wiring"
```
EXPECT: 전부 통과

### Full Test Suite
```bash
pnpm test:ci
```
EXPECT: svelte-check + eslint + prettier + vitest 커버리지(브랜치/함수/라인/구문 80%) + vite build 전부 통과. 회귀 없음.

### Native Tests
```bash
cargo test --manifest-path src-tauri/Cargo.toml --all-features
```
EXPECT: 통과 (Rust는 이번 변경과 무관하므로 회귀 확인용)

### Browser Validation
```bash
pnpm tauri dev
```
EXPECT: 설정에서 ring mode를 `double`로 바꾸면 작은 원이 우측 접선 위치에 나타나고, 두 마크가 각자의 궤도를 돈다.

### Manual Validation

- [ ] `double` 모드에서 작은 원이 **수평 우측**에 있고 두 원이 겹치지 않는다
- [ ] 두 원의 크기 관계가 우측으로 멀어지는 원근으로 읽힌다 (α ≈ 25°)
- [ ] 작은 원 안의 숫자가 퍽 밖으로 넘치지 않는다 — **`100`(3자리)일 때 특히 확인**
- [ ] 숫자 색이 상반원 아크 색과 일치한다
- [ ] 작은 워커가 큰 링 아크를 지날 때 **사라지지 않는다**
- [ ] 큰 워커가 작은 링 앞을 지난다 (뒤가 아니라)
- [ ] 두 워커의 걸음 속도가 비슷하게 보인다 (작은 쪽이 기어가지 않음)
- [ ] `single` 모드로 되돌리면 펫 크기가 **변하지 않는다**
- [ ] 말풍선을 띄웠다 내려도 두 궤도가 링을 정확히 따라간다
- [ ] 라이트/다크 양쪽에서 퍽 위 숫자 대비가 충분하다
- [ ] 작은 원 위에서 드래그·더블클릭·우클릭이 §4.3대로 동작한다
- [ ] OS 감축 모션을 켜면 두 마크가 궤도 위에 **정지**한다(사라지지 않음)
- [ ] 오버레이가 240px 창 밖으로 잘리지 않는다 (우측 끝 특히)

---

## Acceptance Criteria

- [ ] Task 1-10 완료
- [ ] `pnpm test:ci` 통과 (커버리지 80% 게이트 포함)
- [ ] `cargo test` 회귀 없음
- [ ] 타입 오류 0, lint 오류 0
- [ ] 신규 테스트: 접선 성립, 크기 예산, 5H readout, 작은 워커 a11y
- [ ] `docs/ui-contract.md` §4.4·§4.5가 코드 상수와 일치
- [ ] `CLAUDE.md` 워커 불변식이 두 워커 구조를 반영
- [ ] Manual Validation 체크리스트 전 항목 확인

## Completion Checklist

- [ ] 기하 상수가 `orbitPath.ts` **한 곳**에만 있다 (컴포넌트·테스트가 재계산하지 않음)
- [ ] 주석이 "무엇"이 아니라 "바꾸면 무엇이 깨지는가"를 말한다
- [ ] `@supports` / `prefers-reduced-motion` 규칙이 **두 워커 모두**를 덮는다
- [ ] 하드코딩된 픽셀 없음 — 전부 `model.size`에서 파생
- [ ] 폰트 파일을 새로 번들하지 않았다 (`ui-rounded`는 제네릭 패밀리)
- [ ] 접근성 트리에 중복 낭독이 추가되지 않았다
- [ ] `ring_mode`가 여전히 **표시 전용** — 수집·새로고침·알림·펫 무드 무관
- [ ] 범위 밖 리팩터링 없음

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| 크기 예산 여유가 2.25%p뿐이라 이후 튜닝이 조용히 펫을 줄임 | 중 | 높음 | `orbitPath.test.ts`에 `240 / factor ≥ 128` 어서션 추가(Task 6). 상수 옆 주석에 여유 명시 |
| `100`(3자리)이 36% 원 안에서 빡빡함 | 중 | 중 | 내부 여유 ≈35.7px, 15.2px tabular 3자리 ≈27.7px로 계산상 통과. 수동 검증 필수 항목 |
| 큰 워커의 작은 링 통과가 실제 화면에서 지저분해 보임 | 중 | 중 | 의도된 오클루전으로 문서화. 거슬리면 `SATELLITE_DISTANCE`를 77.25로 올려 완전 분리 가능하나 **펫이 115px로 축소** — 트레이드오프를 §4.5에 기록 |
| `ui-rounded`가 Windows/Linux에서 IBM Plex Mono로 폴백되어 플랫폼 간 숫자 모양이 다름 | 높 | 낮 | `tabular-nums`로 폭은 안정. 단일 숫자라 시각적 영향 미미 |
| 라이트 모드 `--sev-ok`(#22c55e)가 고정 어두운 퍽(#1c1f24) 위에서 대비 부족 | 낮 | 중 | 수동 검증 항목. 부족하면 readout 전용으로 다크 팔레트 값을 쓴다 (`--satellite-puck`처럼 테마 불변) |
| 합집합 기하 삭제로 `orbitPath.ts` 주석의 근거가 통째로 낡음 | 높 | 중 | Task 1에 독스트링 갱신을 명시적 단계로 포함 |
| 커버리지 80% 게이트가 삭제된 브랜치 때문에 흔들림 | 낮 | 중 | 삭제되는 코드는 대부분 분기 없는 수식. 신규 테스트가 순증이므로 게이트 상승 방향 |

---

## Notes

**Apple 가이드라인 적용 지점**

1. **Depth via occlusion** — 그림자를 더하지 않고 레이어 순서로 원근을 만든다 (결정 6·7)
2. **Deference** — 접선을 그리지 않고 배치만으로 관계를 전달 (결정 3)
3. **Progressive disclosure** — 정상일 때 숫자, 문제일 때 배지. 두 상태가 같은 자리를 공유 (결정 5)
4. **Complication legibility** — 소형 원형 표면에는 값 하나만. 라벨+값 2줄을 배제한 근거
5. **System fonts, not bundled fonts** — `ui-rounded`로 macOS의 SF Pro Rounded를 라이선스 문제 없이 사용

**의도적으로 하지 않은 것**: 워커 방향을 각각 무작위로 굴리지 않는다. 오버레이당 난수 1회 소비를
유지하고(기존 주석의 "randomness stops here"), 두 마크가 같은 방향으로 돌 때 하나의 장면으로 읽힌다.

**후속 여지 (이번 범위 아님)**: 접선을 실제로 그리는 옵션, 작은 원의 WK 값 노출 방식, 두 워커의
교차 순간 미세 페이드. 전부 이 플랜의 상수를 건드리지 않고 추가 가능하다.
