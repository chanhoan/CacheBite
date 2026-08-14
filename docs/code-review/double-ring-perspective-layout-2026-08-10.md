# Code Review: Double Ring Perspective Layout (원근 배치 + 위성 리드아웃)

**Reviewed**: 2026-08-10
**Branch**: `feat/overlay-double-ring`
**Base**: `417be49 feat: add double ring overlay mode with satellite ring and orbit walker`
**Scope**: `HEAD` 대비 로컬 변경사항(수정 10개 파일, +759 / −244) + 미추적 신규 파일 2건(`.claude/PRPs/**`)
**Decision**: **APPROVE with comments** — CRITICAL 0건, HIGH 0건, MEDIUM 3건, LOW 4건. 전 검증 통과.

## Summary

이전 리뷰(`overlay-double-ring-2026-08-07.md`)에서 차단 사유였던 **HIGH — `.overlay :global(.ring)` 자손 결합자로 인한 위성 지오메트리 무효화**는 해결됐다. 빌드 산출물에서 `.overlay.svelte-ha4nt8>.ring`으로 자식 결합자가 확인되며, `.satellite .ring`이 더는 덮이지 않는다. 이전 MEDIUM 3건(말풍선 클램프의 단일 출처화, 조합 루트 배선 테스트, `Math.random()` 순수화)과 LOW 2건(`offset-path` 미지원 폴백, 워커 문서화)도 모두 반영됐다.

이번 변경의 본체는 두 가지다. 첫째, 위성을 **우하단으로 이동**하고 두 원을 분리해 **외접 공통접선이 α ≈ 22.9°로 수렴**하는 원근 배치로 재구성했다. 방위각을 접선 반각과 같게 잡아 두 원의 최하단이 하나의 수평선에 놓이는 것은 실제로 성립하며(`smallBottom = bigBottom = 95.25`), 테스트가 이를 좌표 스냅샷이 아니라 **접선 성질 자체**(각 중심에서 접선까지의 수직거리 = 각자의 반지름)로 검증한다. 합집합 아웃라인이 폐기되고 각 링이 자기 마크를 태우는 구조로 바뀌면서, 큰 원의 궤도가 링 모드와 무관하게 동일해진 것도 "표시 선호가 레이아웃을 바꾸지 않는다"는 불변식과 정확히 맞는다.

둘째, 위성 퍽 중앙에 **5시간 사용률 숫자**가 들어가고 provider 로고는 위성 궤도로 나갔다. 5H 창이 프로모션으로 사라졌을 때 주간 창으로 떨어지는 규칙(`satelliteReading`)은 **부재로만 전환하고 심각도로는 전환하지 않는다**는 선택이 옳다 — `5H`/`WK` 라벨이 숨겨진 크기에서 심각도 기반 전환은 어느 한도를 말하는지 알 방법을 없애는 반면, 부재 기반 전환은 "빈 상반원 + 하반원 색으로 물든 숫자"라는 시각적 단서를 남긴다. 색이 표시 중인 창의 severity를 따라가는 것까지가 이 규칙의 핵심이고, 테스트 4건이 이를 고정한다.

숫자/색 정합성도 확인했다: `presentation.ts:toProviderPresentation`이 `severity === 'unknown'`인 창의 `usedPercent`를 `null`로 눕히므로 `usedPercent === null ⟺ severity === 'unknown'` 불변식이 성립하고, 따라서 링의 `aria-label`("5-hour unknown")과 퍽의 숫자가 어긋나는 조합은 발생하지 않는다.

보안 결함, 자격증명 노출, 디버그 잔재는 없다. Rust/IPC 표면은 이번 변경에서 **전혀 건드리지 않았다**(`src-tauri/` 무변경) — 명령 권한 allowlist, 프라이버시 계약, 설정 스키마 모두 영향 없음.

차단 사유는 없다. 다만 **MEDIUM 3건은 병합 전 처리를 권한다** — 특히 이번 변경의 핵심 판단 로직이 커버리지 게이트에서 제외된 모듈로 들어간 건은 지금 고치는 편이 싸다.

## Findings

### CRITICAL

None.

### HIGH

None.

### MEDIUM

---

**[MEDIUM-1] 커버리지 게이트에서 제외된 모듈에 이번 변경의 핵심 판단 로직이 들어갔다**

`vite.config.ts:36-37`, `src/lib/components/models.ts:16-70`

```js
// vite.config.ts
exclude: [
  ...
  // Type-only view-model contracts produce no executable application code.
  'src/lib/components/models.ts',
  'src/lib/contracts/domain.ts',
],
```

주석의 전제("타입 전용, 실행 코드 없음")가 이번 변경으로 **거짓이 됐다**. `models.ts`에는 이제 두 개의 런타임 함수가 있다:

- `clampPercent` — `null` / 비유한수 / 범위 초과를 가르는 경계 로직. `SplitUsageRing`이 `?? 0`으로, `SatelliteRing`이 `—`로 서로 **다르게** 소비한다.
- `satelliteReading` — **5H → WK 폴백**. 이번 변경의 규칙 그 자체다.

즉 이 기능의 유일한 분기 로직이 80% 게이트에 잡히지 않는다. 커버리지 리포트에도 `models.ts`가 아예 등장하지 않는다(`src/lib/components` 그룹 아래 `orbitPath.ts`, `panelModels.ts`, `systemGuidance.ts`만 나옴).

추가로 이 프로젝트는 정책을 **순수 리듀서로 분리해 직접 단위 테스트**하는 규약을 갖고 있는데(`interaction/petPointer.ts`, `bubblePolicy.ts` 등 — CLAUDE.md의 "Behavior tested here, not in Svelte components"), `satelliteReading`은 그 규약을 따르지 않는다. 현재 검증은 전부 `PetOverlay.test.ts`의 렌더 결과 단언(`data-window`, `data-severity`, `textContent`)을 경유한다. 훌륭한 테스트지만, 컴포넌트 렌더링을 거치므로 규칙 자체의 계약이 아니라 규칙의 **표현**을 고정한다.

**Fix (택1)**

1. `vite.config.ts`의 `models.ts` 제외를 해제하고 `src/lib/components/models.test.ts`를 추가한다 — `satelliteReading`을 4개 케이스(session 있음 / session만 없음 / 둘 다 없음 / 클램프)로 직접 고정. `PetOverlay.test.ts`의 기존 4건은 "화면에 어떻게 나오는가"로 남겨 둔다.
2. 두 함수를 `orbitPath.ts` 옆의 별도 모듈(예: `satelliteReading.ts`)로 옮긴다. `orbitPath.ts`는 이미 100% 커버리지로 게이트 안에 있다.

1번을 권한다. `domain.ts`도 같은 제외 목록에 있고 `secondaryProvider()`라는 실행 코드를 이미 갖고 있으므로, 제외 주석 자체를 재검토할 시점이다.

---

**[MEDIUM-2] `OVERLAY_BOUNDS_FACTOR`가 `offset-rotate: auto`의 회전 바운딩 박스를 계산에 넣지 않는다 — 자기 상한에서 클리핑된다**

`src/lib/components/orbitPath.ts:146-152`

```ts
const SATELLITE_REACH = Math.max(
  SATELLITE_CENTER.x + SMALL_ORBIT + SATELLITE_WALKER_RADIUS - 50,
  SATELLITE_CENTER.y + SMALL_ORBIT + SATELLITE_WALKER_RADIUS - 50,
);
```

이 식은 마크를 **축에 정렬된 정사각형**으로 가정하고 x 반너비를 `SATELLITE_WALKER_RADIUS`(4.5)로 놓는다. 그러나 `.satellite-walker`에는 `offset-rotate: auto`가 걸려 있어 궤도를 도는 동안 정사각형이 **회전**한다. 회전한 정사각형의 x축 반너비는 `(s/2)(|sin θ| + |cos θ|)`이고, θ ≈ 9.5°에서 실제 도달거리가 최대가 된다.

| 항목 | 모델 값 | 실제 값 |
| --- | --- | --- |
| 우측 도달거리 (중심 기준 %) | 91.478 | **91.851** (θ ≈ 9.5°) |
| `OVERLAY_BOUNDS_FACTOR` | 1.82956 | 1.83701 |
| 240px 창의 상한 | 131.179 | 130.647 |
| 128px 펫 기준 여유 | 2.272 pt | **1.899 pt** |

결과: **정확히 상한(131.18px)을 요구하는 패키지는 우측 가장자리에서 약 0.49px 잘린다**(`120 + 0.91851 × 131.18 = 240.49` > 창 240px). 번들 펫 3종은 모두 128px이라 2.4px 여유가 남으므로 **출시물에는 영향이 없다**. 다만 `OVERLAY_BOUNDS_FACTOR`의 존재 이유가 "아무것도 잘리지 않게 한다"인데 자기 경계에서 그 보장이 깨지는 것은 계약 위반이다.

부수적으로 `docs/ui-contract.md`와 `CLAUDE.md`가 명시한 **"여유는 약 2.3%p"** 수치도 1.9%p로 정정이 필요하다(과대 표기 방향이라 튜닝 시 위험하다).

**Fix**

```ts
/**
 * `offset-rotate: auto` turns the mark with the path, so a square of side `s`
 * spans `(s/2)(|sin θ| + |cos θ|)` on the x axis rather than `s/2`. The peak is
 * not at the orbit's own extreme — it sits back at tan θ = r_walker / r_orbit.
 */
const rotatedReach = (cx: number, orbit: number, half: number): number => {
  const theta = Math.atan(half / orbit);
  return (
    cx + orbit * Math.cos(theta) + half * (Math.sin(theta) + Math.cos(theta)) - 50
  );
};
```

`Math.SQRT2 * SATELLITE_WALKER_RADIUS`로 보수적으로 잡는 방법도 있으나(반너비 6.364), 그러면 상한이 128.56px이 되어 번들 펫 128px과의 여유가 0.56px밖에 남지 않는다. 위의 정확한 최댓값(130.65px)을 권한다. 어느 쪽이든 `orbitPath.test.ts:191-212`의 `expect(240 / OVERLAY_BOUNDS_FACTOR).toBeGreaterThanOrEqual(128)`는 계속 통과하며, 회전 항을 테스트에도 반영해 두면 향후 튜닝에서 다시 놓치지 않는다.

---

**[MEDIUM-3] 브라우저 E2E가 이중 링을 전혀 밟지 않는다 — `offset-path`는 jsdom이 검증할 수 없는 유일한 축이다**

`src/lib/api/fixtureGateway.ts:50`, `tests/e2e/renderer.spec.ts`

```ts
ringMode: 'single',
```

렌더러 E2E 픽스처가 `single`로 고정돼 있어, 실제 Chrome에서 검증되는 것은 큰 원 하나뿐이다. 결과적으로 다음이 **어떤 브라우저에서도 확인되지 않는다**:

- `offset-path: path(...)` + `offset-anchor` + `offset-rotate: auto`가 마크를 실제로 궤도 위에 놓는가 — jsdom은 모션 패스를 계산하지 않으므로 단위 테스트는 **경로 문자열만** 검증한다. 즉 지오메트리 전체가 "수학은 맞다"까지만 증명돼 있고 "브라우저가 그 수학대로 그린다"는 미검증이다.
- z-order로 원근을 싣는 규칙(`z 3` 큰 마크 / `z 0` 위성 그룹). 이번 변경 문서가 가장 길게 설명한 항목인데, 스택 컨텍스트는 `.satellite`의 `transform`이 만들고 `.satellite-walker`는 `z-index: 0`으로 트리 순서에 의존한다 — 계산된 스타일이 아니라 **합성 결과**라 단위 테스트가 닿지 않는다.
- `@supports not (offset-path: ...)` 폴백과 `prefers-reduced-motion` 처리.
- 위성 히트 영역(`overlay-satellite-pointer-surface`)의 실제 좌표. `limits overlay hit testing to the circular surface` 스펙이 이미 존재하지만 `single` 픽스처라 위성을 지나가지 않는다.

`App.test.ts`의 `ring mode wiring` 6건이 배선은 잘 덮고 있어 **회귀 위험은 낮다**. 그러나 이번 변경에서 가장 깨지기 쉬운 부분이 정확히 "브라우저 레이아웃"이라는 점에서 공백의 위치가 나쁘다.

**Fix**: E2E에서 링 모드를 전환 가능하게 만든다. 픽스처 상수를 바꾸는 대신 쿼리 파라미터나 `CACHEBITE_E2E_RING_MODE` 환경변수로 주입해 기존 `single` 스펙을 유지한 채 `double` 스펙을 추가하는 편이 안전하다. 최소한 다음 두 건이면 공백이 메워진다:

1. `double`에서 `orbit-walker`와 `satellite-orbit-walker`의 `getBoundingClientRect()` 중심이 각자의 궤도 반지름 위에 있고, 둘 다 오버레이 창(240px) 안에 있다.
2. 위성 퍽 중앙 좌표에서의 포인터 다운이 드래그 핸들러에 도달한다.

### LOW

---

**[LOW-1] 말풍선 클램프(`TOAST_OVERLAY_PX`)가 영구히 도달 불가 분기가 됐다**

`src/App.svelte:565-578`

```ts
const TOAST_OVERLAY_PX = 152;
const sizeCeiling = $derived(
  $interactionStore.bubblePolicy.bubble ? TOAST_OVERLAY_PX : OVERLAY_WINDOW_PX,
);
const overlaySize = $derived(
  Math.min(OVERLAY_WINDOW_PX / OVERLAY_BOUNDS_FACTOR, /* 131.18 */ ..., sizeCeiling),
);
```

`240 / 1.82956 = 131.18 < 152`이므로 `sizeCeiling`은 `Math.min`에서 **절대 이길 수 없다**. `App.test.ts`가 `expect(bounded).toBeLessThan(152)`로 이 사실을 명시적으로 고정하고, 주석이 "기하가 152 안쪽으로 되돌아오면 이 단언이 깨지고 말풍선 경로를 다시 덮어야 한다"고 남긴 것은 **올바른 처리**다. 삭제하지 않고 남긴 판단에 이견 없다.

다만 `App.svelte:568-573`의 긴 주석("The bubble's clamp belongs here rather than in CSS…")은 현재 **작동하지 않는 메커니즘**을 현재형으로 설명한다. 다음 독자가 말풍선을 띄워 놓고 폭 변화를 찾다가 시간을 쓴다. 주석 한 줄 추가를 권한다: "현재 기하에서는 `OVERLAY_BOUNDS_FACTOR`가 먼저 걸리므로 이 천장은 걸리지 않는다 — `App.test.ts`가 그 사실을 단언으로 지키고 있다."

---

**[LOW-2] `arcTo`의 `from`/`to` 일반화가 합집합 아웃라인 폐기 후 남은 잔재다**

`src/lib/components/orbitPath.ts:164-179`

```ts
const segments = Math.max(2, Math.ceil(Math.abs(to - from) / 120));
```

호출부는 `circleOrbit` 하나뿐이고 항상 `arcTo(cx, cy, r, 0, 360, scale)`을 넘긴다. 따라서 `from`/`to`는 상수이고, `Math.max(2, …)`의 하한도 절대 걸리지 않는다(`ceil(360/120) = 3`). 두 원이 하나의 아웃라인을 이루던 이전 설계에서는 부분 호(arc)가 필요했지만, 이제는 전원(全圓)만 그린다.

기능적 문제는 없고 `orbitPath.ts`는 100% 커버리지다. 다만 KISS/YAGNI 관점에서, 고정 3분할 원 이미터로 좁히면 `from`/`to`/`segments` 세 개념이 사라지고 주석("Split into segments below a half turn so the large-arc flag is always 0")도 자명해진다. 향후 부분 호가 다시 필요해지면 그때 되돌리는 편이 싸다.

---

**[LOW-3] 두 마크 모두 `pointer-events: none`이라 로고 위는 드래그 사각지대다**

`src/lib/components/PetOverlay.svelte:216-227`

`.interaction-surface`는 `clip-path: circle(50%)`(반경 50%)인데 큰 마크의 중심선은 52.25%에 있다. 마크 몸통은 반경 45.25~59.25 구간을 차지하므로 **약 2/3가 드래그 표면 밖**이다. 위성도 마찬가지로 `.satellite-surface`가 반경 18%인 반면 작은 마크의 중심선은 22.5%다.

`pointer-events: none`은 "장식적 중복이 드래그를 가로채면 안 된다"는 올바른 결정이고, 이는 큰 마크에 대해 이미 존재하던 성질이다. 다만 사용자 입장에서는 화면에 뚜렷하게 보이는 로고를 잡았는데 아무 일도 일어나지 않는다. 히트 영역을 마크 궤도까지 넓히는 것(반경 `BIG_ORBIT + WALKER_RADIUS`)은 오히려 펫 바깥의 투명 영역을 클릭 가능하게 만들어 오버레이 창 전체가 드래그 대상이 되므로 권하지 않는다. **의도된 트레이드오프로 기록**만 해 두면 충분하다 — `docs/ui-contract.md` §4.5의 "포인터" 행에 한 문장 추가를 제안한다.

---

**[LOW-4] `TANGENT_HALF_ANGLE`의 `NaN` 전파 범위가 넓어졌다**

`src/lib/components/orbitPath.ts:78-102`

```ts
export const TANGENT_HALF_ANGLE =
  Math.asin((RING_OUTER_RADIUS - SATELLITE_RADIUS) / SATELLITE_DISTANCE) / RADIANS;
export const SATELLITE_BEARING = TANGENT_HALF_ANGLE;
```

`SATELLITE_DISTANCE < RING_OUTER_RADIUS - SATELLITE_RADIUS`(= 27.25)이면 `asin` 인자가 1을 넘어 `NaN`이 된다. 이전 설계의 `acos` 기반 교점 계산과 달리, 이제 `NaN`은 `SATELLITE_CENTER` → `SATELLITE_REACH` → `OVERLAY_BOUNDS_FACTOR` → **`App.svelte:overlaySize`** 까지 전파된다. 즉 마크가 사라지는 데 그치지 않고 **펫의 렌더 폭 자체가 `NaN`이 되어 `style:width`가 무효화**된다.

`orbitPath.test.ts:226-231`의 `never emits NaN coordinates`와 `App.test.ts`의 `overlay.style.width` 단언이 둘 다 실패하므로 CI에서 반드시 잡힌다 — **실질 위험은 없다**. 다만 두 테스트 모두 "왜 `NaN`인지"는 말해 주지 않는다. 상수 정의부를 비율로 한 번 끊고

```ts
// asin의 정의역을 벗어나면 SATELLITE_CENTER부터 overlaySize까지 전부 NaN이 된다.
// 두 원이 외접 공통접선을 가지려면 중심거리가 반지름 차보다 커야 한다.
const TANGENT_RATIO = (RING_OUTER_RADIUS - SATELLITE_RADIUS) / SATELLITE_DISTANCE;
```

여기에 대응하는 단언(`expect(Math.abs(TANGENT_RATIO)).toBeLessThan(1)`)을 추가하면 실패 지점이 증상이 아니라 원인 위치로 옮겨 간다.

## Validation Results

| Check | Result |
| --- | --- |
| `svelte-check --tsconfig ./tsconfig.json` | **Pass** — 0 errors, 0 warnings |
| `eslint .` | **Pass** — exit 0 |
| `prettier --check .` | **Pass** — All matched files use Prettier code style |
| `vitest run --coverage` | **Pass** — 25 files, **332 tests** (이전 312 → +20) |
| Coverage | **Pass** — statements 96.27% / branches 91.01% / functions 91.3% / lines 96.27% (게이트 80%) |
| `vite build` | **Pass** — 4.34s, JS 104.86 kB / gzip 35.54 kB, CSS 16.09 kB / gzip 3.69 kB |
| `wdio run ./wdio.browser.conf.ts` | **Pass** — 2 specs, 8 tests |
| `cargo test` / `clippy` / `fmt` | **Not run — 해당 없음.** 이번 변경에 `src-tauri/` 파일이 하나도 없다 |
| `pnpm test:e2e` (native) | **Not run** — webdriver 피처 디버그 빌드가 선행되어야 하는 별도 절차 |

> 실행 환경 메모: `pnpm`이 PATH에 없어 `corepack pnpm`(`C:\Program Files\nodejs`)으로 구동했다. `pnpm@10.15.1`, Node v24.18.0.

컴포넌트별 커버리지: `orbitPath.ts` 100/100/100/100, `SatelliteRing.svelte` 100/100/100/100, `PetOverlay.svelte` 100/81.81/100/100(미커버 분기는 `keydown`의 Enter/Space 외 키 조기 반환 — 무해).

## 확인한 불변식

| 불변식 | 결과 |
| --- | --- |
| 이전 리뷰 HIGH(`.ring` 특이도 충돌) 해소 | **Pass** — 빌드 산출물에 `.overlay.svelte-ha4nt8>.ring` 자식 결합자 확인, `.satellite .ring`이 독립 적용됨 |
| 큰 원의 궤도가 링 모드와 무관하게 동일 | **Pass** — `orbitPath(size)`가 모드 인자를 받지 않음. `PetOverlay.test.ts`가 `toBe(singlePath)`로 고정 |
| 작은 마크는 `double`에서만 렌더 | **Pass** — `App.test.ts` + `PetOverlay.test.ts` 양쪽에서 `queryByTestId('satellite-orbit-walker')).toBeNull()` |
| 작은 마크 provider = `secondaryProvider(primary)`, 저장 안 함 | **Pass** — `model.satellite.provider`를 그대로 사용, `updateSettings` 페이로드 미포함 |
| 두 마크가 같은 방향 롤 공유 | **Pass** — 둘 다 `model.orbit.direction` 소비, 난수는 `App.svelte:564`에서 1회만 |
| 방위각 = 접선 반각, 두 원 최하단 동일선 | **Pass** — `smallBottom = bigBottom = 95.25`, 오차 1e-9 이하. 접선 성질을 수직거리로 직접 검증 |
| 두 원이 겹치지 않음 | **Pass** — 간격 6.75%p (`70 − 45.25 − 18`) |
| 선속도 매칭 | **Pass** — `11.196s / 26s = 22.5 / 52.25 = 0.43062` |
| 5H → WK 폴백이 **부재**로만 발동, 심각도로는 발동 안 함 | **Pass** — `prefers the 5-hour figure even when the weekly one is worse` 테스트 |
| 숫자 색 = 표시 중인 창의 severity | **Pass** — `data-severity`가 `reading.severity`를 따름. `presentation.ts`가 `usedPercent === null ⟺ severity === 'unknown'`을 보장하므로 라벨/숫자 불일치 조합은 발생 불가 |
| 두 창 모두 부재 시 `0`이 아닌 `—` | **Pass** — `clampPercent`가 `null`을 보존하고 소비처가 각자 해석 |
| 접근성 이름 중복 회피 | **Pass** — 큰 마크만 `role="img"` + 이름, 작은 마크·히트 영역은 `aria-hidden` |
| 링 모드가 수집·새로고침·알림·펫 무드에 안 닿음 | **Pass** — `does not let ring mode reach notification routing` 테스트 유지 |
| `ring_mode` 표시 전용(모드 전환이 펫 크기 불변) | **Pass** — `OVERLAY_BOUNDS_FACTOR`가 두 모드에 동일 적용 |
| 프라이버시 계약 / 명령 권한 | **Pass** — `src-tauri/` 무변경, 신규 IPC 없음, 자격증명 경로 없음 |
| 디버그 잔재 | **Pass** — `console.*` / `TODO` / `FIXME` / `debugger` 0건 |

### 문서 수치 대조 (한 건 제외 모두 코드와 일치)

| 문서 주장 | 실측 |
| --- | --- |
| `TANGENT_HALF_ANGLE ≈ 22.9°` | 22.9101° ✓ |
| 두 원 간격 6.75%p | 6.750 ✓ |
| `SATELLITE_CENTER` 우하단 | (114.478, 77.250) ✓ |
| 작은 마크 주기 ≈ 11.2s | 11.196s ✓ |
| `OVERLAY_BOUNDS_FACTOR` 1.83 | 1.82956 ✓ |
| 240px 창 상한 ≈ 131px | 131.179 ✓ |
| 작은 마크 궤도 최근접 47.5 vs 아크 외곽 45.25 → 몸통 약 1/4 잠김 | 2.25 / 9 = 25% ✓ |
| 작은 마크 최소 x 87.5 vs 펫 박스 84 | 87.478 ✓ (접촉 없음) |
| 여유 약 2.3%p | **1.90%p** — [MEDIUM-2] 참조 |

## Files Reviewed

### Source (Modified)

- `src/lib/components/orbitPath.ts` — 합집합 아웃라인 폐기, 원 2개 독립 궤도. `TANGENT_HALF_ANGLE` / `SATELLITE_BEARING` / `SATELLITE_CENTER` / `SATELLITE_WALKER_SIZE` / `SATELLITE_WALK_DURATION_S` / `SATELLITE_READOUT_RATIO` 신설, `OVERLAY_BOUNDS_FACTOR` 재산출
- `src/lib/components/models.ts` — `clampPercent`, `SatelliteReading`, `satelliteReading` 추가 → **[MEDIUM-1]**
- `src/lib/components/PetOverlay.svelte` — 위성 마크 렌더, z-order 재편(`z 3` 큰 마크 / `z 0` 위성 그룹), `--satellite-*` 커스텀 프로퍼티 확장, `.overlay > :global(.ring)` 자식 결합자
- `src/lib/components/SatelliteRing.svelte` — 로고 → 숫자 리드아웃 교체, severity별 색, `ui-rounded` 폴백 스택
- `src/lib/components/SplitUsageRing.svelte` — 인라인 클램프를 `clampPercent`로 대체

### Tests

- `src/lib/components/orbitPath.test.ts` — 5 → **13 tests**. 접선 수렴/수평선 정렬/분리/선속도/마크 비율/리드아웃 여백/경계 여유/NaN 방어
- `src/lib/components/PetOverlay.test.ts` — 리드아웃 4건, 위성 마크 접근성 1건 추가, 궤도 불변성 단언 반전(`not.toBe` → `toBe`)
- `src/App.test.ts` — 위성 마크 배선 + 리드아웃 값 검증, 말풍선 클램프 테스트를 `OVERLAY_BOUNDS_FACTOR` 기반으로 재작성

### Docs

- `docs/ui-contract.md` — §4.4 위성 기하/리드아웃 규칙, §4.5 이중 마크 + z-order 원근 규칙 전면 개정
- `CLAUDE.md` — 불변식 항목을 "Each ring carries its own orbiting mark…"로 교체

### Untracked (미커밋)

- `.claude/PRPs/plans/completed/double-ring-perspective-layout.plan.md`
- `.claude/PRPs/reports/double-ring-perspective-layout-report.md`

두 파일 모두 `git ls-files .claude/PRPs`에 동종 파일이 이미 추적되고 있으므로(`completed/overlay-double-ring.plan.md` 등) 같은 커밋에 포함하는 것이 저장소 관례에 맞다.

## 잘한 점

- **접선 성질을 좌표가 아니라 성질로 테스트했다.** `converges the two outer tangents…`는 각 중심에서 접선까지의 수직거리가 각자의 반지름과 같은지를 직접 계산해, `asin`이 만들어낸 값을 그대로 믿지 않는다. 나아가 "접점은 극점이 아니다"(극점끼리 이으면 평행선)를 반증 단언으로 남긴 것은 설계 의도를 코드가 스스로 방어하게 만든 좋은 예다.
- **`SATELLITE_BEARING = TANGENT_HALF_ANGLE`이 우연이 아님을 대수적으로 유도하고, 그 유도를 테스트가 재확인한다.** `rests both circles on one horizontal line`이 부등식(제약)과 등식(선택) 양쪽을 동시에 단언한다.
- **선속도 매칭을 상수로 쓰지 않고 파생시켰다.** `WALK_DURATION_S * (SMALL_ORBIT / BIG_ORBIT)` — 반지름 튜닝이 자동으로 속도에 반영되고, 테스트가 비율 자체를 고정한다.
- **`clampPercent`의 `null` 보존이 옳다.** "데이터 없음"과 "0% 사용"을 한 값으로 뭉개지 않고, 소비처가 각자 `?? 0`(빈 트랙)과 `—`(리드아웃)로 해석한다. 주석이 `stroke-dasharray: null 100`이 경로를 통째로 날린다는 구체적 이유까지 남겼다.
- **폴백 규칙의 선택 근거가 검증 가능한 형태로 남았다.** "심각도가 아니라 부재로만 전환"이라는 판단을 `prefers the 5-hour figure even when the weekly one is worse`가 직접 고정한다. 나중에 "더 나쁜 쪽을 보여주자"는 리팩터링이 들어오면 즉시 실패한다.
- **원근을 z-order로 싣되, 가독성 논거와 원근 논거를 섞지 말라고 문서에 못 박았다.** "작은 마크를 큰 링 위로 올리고 싶어지는데 그러면 먼 것이 가까운 것을 가린다"는 경고는 정확히 다음 사람이 저지를 실수다. 비용(몸통 1/4 잠김)까지 숫자로 남겼다.
- **이전 리뷰의 지적 5건이 모두 반영됐다** — CSS 특이도(HIGH), 말풍선 클램프 단일 출처화, 조합 루트 배선 테스트, `Math.random()` 순수화(`orbitDirection(roll)` + overlay 창 한정), `@supports` 폴백. 특히 폴백 주석이 "애니메이션만 빠지는 게 아니라 좌상단에 박제된다"는 실제 증상을 기술한 점이 좋다.
