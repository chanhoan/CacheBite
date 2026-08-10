# Implementation Report: Double Ring — 원근 배치(Perspective Layout)

## Summary

`double` 링 모드의 작은 원을 큰 원과 겹친 우하단 뱃지 배치에서, **외접 공통접선이 수렴하는 원근
배치**로 옮겼다. 작은 원 안의 정적 provider 로고는 **작은 원 전용 궤도 워커**로 빠져나갔고, 비워진
내부에는 사용률 숫자가 들어갔다. 궤도는 두 원의 합집합 아웃라인에서 **각 원의 독립 원**으로 바뀌었다.

## Assessment vs Reality

| Metric | Predicted (Plan) | Actual |
| --- | --- | --- |
| Complexity | Medium | Medium |
| Confidence | 8/10 | 구현은 단일 패스, 다만 사용자 피드백으로 기하가 2회 재조정됨 |
| Files Changed | 10 | 10 |
| Tests | 신규 4~6건 | 신규 10건 (총 329 → 332) |

## Tasks Completed

| # | Task | Status | Notes |
| --- | --- | --- | --- |
| 1 | `orbitPath.ts` 기하 재정의 | Complete | 방위각 도입으로 계획 대비 확장 (아래 Deviations) |
| 2 | `models.ts` 공유 클램프 | Complete | `satelliteReading` 정책 함수도 추가 (아래 Deviations) |
| 3 | `SplitUsageRing.svelte` 헬퍼 전환 | Complete | 렌더 출력 불변 |
| 4 | `SatelliteRing.svelte` 로고 제거·숫자 삽입 | Complete | |
| 5 | `PetOverlay.svelte` 작은 워커·z-index | Complete | |
| 6 | `orbitPath.test.ts` 재작성 | Complete | 13 tests |
| 7 | `PetOverlay.test.ts` 갱신 | Complete | 29 tests |
| 8 | `App.test.ts` 배선 갱신 | Complete | 크기 테스트 재작성 필요 (아래 Issues) |
| 9 | `docs/ui-contract.md` §4.4·§4.5 | Complete | |
| 10 | `CLAUDE.md` 불변식 | Complete | |

## 최종 기하 상수

```
RING_OUTER_RADIUS         45.25     (불변)
WALKER_SIZE               14        (불변)
SATELLITE_SIZE            36        (40 → 36)
SATELLITE_WALKER_SIZE      9        (신규)
SATELLITE_DISTANCE        70        (신규)
SATELLITE_READOUT_RATIO    0.33     (신규)
WALK_DURATION_S           26        (불변)
SATELLITE_WALK_DURATION_S ≈ 11.20   (신규, 26 × SMALL_ORBIT/BIG_ORBIT)

TANGENT_HALF_ANGLE = asin((45.25 − 18) / 70)   = 22.897°
SATELLITE_BEARING  = TANGENT_HALF_ANGLE        = 22.897°
SATELLITE_CENTER   = (114.477, 77.25)

두 링 사이 틈    70 − 45.25 − 18            = 6.75
작은 원 최하단   77.25 + 18                 = 95.25
큰 원 최하단     50 + 45.25                 = 95.25   ← 정확히 일치
도달(가로)       114.477 + 22.5 + 4.5 − 50  = 91.477
OVERLAY_BOUNDS_FACTOR                       = 1.8295
펫 상한          240 / 1.8295               = 131.2px  (번들 펫 128 ≥ 통과)
```

**방위각 = 접선 반각인 이유**: 작은 원의 최하단이 큰 원의 최하단을 넘지 않으려면
`d·sin(bearing) ≤ R − r`이고, 우변을 `d`로 나눈 값이 정확히 `sin(TANGENT_HALF_ANGLE)`이다.
따라서 제약은 `bearing ≤ TANGENT_HALF_ANGLE`이며, 등호에서 두 원의 최하단이 하나의 수평선에
놓이고 아래쪽 외접 접선이 수평으로 떨어진다 — 같은 바닥에 선 두 원 중 먼 쪽이 물러난 구성.

## Validation Results

| Level | Status | Notes |
| --- | --- | --- |
| Static Analysis | Pass | svelte-check 0 errors / 0 warnings, eslint clean |
| Formatting | Pass | prettier (orbitPath.test.ts 1건 자동 수정) |
| Unit Tests | Pass | 332 passed / 25 files |
| Coverage | Pass | 96.27% stmts, 91.01% branches (게이트 80%) |
| Build | Pass | `vite build` 성공 |
| Rust Regression | Pass | `cargo test --all-features` exit 0 |
| Browser | **미실행** | 아래 Next Steps |

`orbitPath.ts`와 `SatelliteRing.svelte`는 라인/브랜치 100%.

## Files Changed

| File | Action | 요지 |
| --- | --- | --- |
| `src/lib/components/orbitPath.ts` | UPDATE | 기하 전면 재정의, 합집합 수학 제거, 2궤도 + 방위각 + 주기 |
| `src/lib/components/models.ts` | UPDATE | `clampPercent`, `SatelliteReading`, `satelliteReading` 추가 |
| `src/lib/components/SplitUsageRing.svelte` | UPDATE | 로컬 클램프 → 공유 헬퍼 (출력 불변) |
| `src/lib/components/SatelliteRing.svelte` | UPDATE | 로고 제거, readout 추가, 중심 기준 배치 |
| `src/lib/components/PetOverlay.svelte` | UPDATE | 작은 워커 추가, 프로퍼티 교체, z-index 재정렬 |
| `src/lib/components/orbitPath.test.ts` | UPDATE | 13 tests (접선 성립·바닥선·예산·비율·주기) |
| `src/lib/components/PetOverlay.test.ts` | UPDATE | 29 tests (readout 4건, a11y, 모드 불변) |
| `src/App.test.ts` | UPDATE | 배선 어서션 2건 + 크기 테스트 재작성 |
| `docs/ui-contract.md` | UPDATE | §4.4 기하·내부 숫자, §4.5 궤도·주기·z-order·상한 |
| `CLAUDE.md` | UPDATE | orbit walker 불변식 |

`src/App.svelte`와 Rust 전체는 **무변경** — `OVERLAY_BOUNDS_FACTOR`/`orbitDirection` 시그니처가
유지되어 조립 코드가 그대로 동작한다.

## Deviations from Plan

1. **`SATELLITE_BEARING` 도입 (계획에 없던 상수)**
   **WHAT**: 계획은 작은 원을 수평 중심선(y=50)에 두었으나, 사용자가 "우측 하단"을 의도했다고
   정정. 방위각 상수를 추가하고 `SATELLITE_CENTER`를 극좌표로 계산하도록 바꿨다.
   **WHY**: 요청의 "같은 선상"을 수평선으로 잘못 읽었다. 접선 구성은 중심선 방향에 대해 회전
   대칭이라 원근 자체는 손상 없이 이전됐다.

2. **`SATELLITE_DISTANCE` 64.5 → 70, 틈 1.25 → 6.75**
   **WHAT**: 계획보다 두 원을 더 벌렸다.
   **WHY**: 정사각형 창에서 대각선 배치는 도달거리를 가로·세로로 나눠 갖는다. 수평 배치가 쓰던
   예산이 풀리면서 같은 상한(131px) 안에서 간격을 5배 이상 벌릴 수 있었다.

3. **방위각을 접선 반각에 고정 (자유 파라미터 아님)**
   **WHAT**: "작은 원 최하단이 큰 원 최하단을 넘지 않게" 제약을 적용한 결과, 방위각이
   `TANGENT_HALF_ANGLE`과 같은 값으로 유도됐다.
   **WHY**: 제약식 `d·sin θ ≤ R − r`이 곧 `θ ≤ α`이고, 등호에서 두 원이 같은 수평 바닥선에 선다.
   자유 파라미터를 하나 없애면서 구성이 더 자기완결적이 됐다.

4. **`satelliteReading` 폴백 정책 추가 (계획은 "항상 5H")**
   **WHAT**: 5시간 창에 값이 아예 없으면 주간 창 값으로 떨어지도록 했다. 표시 창은
   `data-window` 속성으로 노출.
   **WHY**: provider가 프로모션으로 5시간 한도를 없애는 기간에 해당 창이 `unknown`이 된다(현재
   Codex가 그 상태). 5H 고정이면 이벤트 내내 퍽이 비어 버린다. **심각도로는 전환하지 않는다** —
   부재로만 전환해야 빈 상반원이 그 자체로 단서가 되고, 색이 표시 창의 severity를 따르므로
   라벨 없이도 구분된다.

5. **`SATELLITE_READOUT_RATIO` 도입**
   **WHAT**: 계획은 PetOverlay에서 `0.33`을 인라인으로 곱했다. 상수로 승격해 `orbitPath.ts`에 뒀다.
   **WHY**: 매직 넘버 회피 + 퍽 내부 여백 대비 3자리 숫자가 들어가는지 테스트로 고정하기 위해서.

## Issues Encountered

1. **`App.test.ts`의 펫 크기 테스트 실패 → 재작성**
   테스트 픽스처 펫은 `defaultSize 160`(번들 펫은 128)이다. 새 `OVERLAY_BOUNDS_FACTOR`가 상한을
   172px → 131px로 낮추면서 160px 픽스처가 클램프됐고, 결과적으로 **152px 말풍선 천장이 더 이상
   걸리지 않게 됐다**(기본 상한이 이미 그보다 작음). 테스트를 "모델이 폭을 정한다"는 원래 의도는
   유지하되 실제 동작을 인코딩하도록 다시 썼고, `bounded < 152` 어서션을 넣어 기하가 되돌아가면
   말풍선 경로 커버리지가 다시 필요하다는 신호가 나게 했다. 문서에도 명시.

2. **`getAllByRole('img', { name: /provider/i })`가 Svelte `rune_outside_svelte` 오류 유발**
   정규식 이름 질의가 전체 role 스캔을 타면서 Svelte 런타임과 충돌했다. 정확 문자열 질의 +
   `queryByLabelText`로 같은 의도를 표현해 해결.

3. **PowerShell PATH에 `pnpm` 없음** → Git Bash에서 `corepack pnpm`으로 실행.

4. **z-order가 원근과 어긋나 있었음 (사용자 지적으로 발견, 수정 완료)**
   최초 구현은 `큰 마크 3 > 작은 마크 2 > 표면 1 > 링 0`이었다. 큰 마크에 대해서는 원근이 맞지만
   **작은 마크(먼 쪽)가 큰 링(가까운 쪽) 위에** 그려져 정반대였다. "작은 마크가 아크에 잘리지 않게"
   라는 **가독성 논거**를 원근 논거와 섞은 결과다. 그룹 단위로 통일해
   `큰 마크 3 > 표면 2 > 큰 링·펫·배지 1 > 작은 링·숫자·작은 마크 0`으로 바로잡았다.
   비용은 거의 없다 — 작은 마크의 궤도 중심선은 오버레이 중심에서 최소 47.5까지만 접근하고 아크
   바깥은 45.25라, 몸통의 약 1/4만 획 뒤로 들어간다. 펫에는 닿지 않는다(최소 x 87.5 vs 박스 84).

## Tests Written

| Test File | Tests | 커버 영역 |
| --- | --- | --- |
| `orbitPath.test.ts` | 13 | 두 궤도 반경 일정·크기 무관성·우하단 배치·바닥선 일치·두 원 분리·접선 성립·선속도 일치·마크 비율·readout 여백·크기 예산·방향 롤·NaN 부재 |
| `PetOverlay.test.ts` | 29 (신규 6) | readout 5H/WK 폴백·심각도 무전환·3자리 클램프·양쪽 부재 시 em dash·작은 워커 a11y·single 모드 누출·큰 궤도 모드 불변 |
| `App.test.ts` | 51 (수정 3) | 배선(작은 워커 provider 파생), single 모드 누출, 모델 기반 크기 결정 |

## Next Steps

- [ ] **브라우저 육안 검증** — `pnpm tauri dev` 후 설정에서 ring mode `double`:
  - 두 원의 최하단이 같은 수평선에 놓이는지
  - 3자리(`100`)일 때 숫자가 퍽을 넘지 않는지
  - 작은 워커가 큰 링 아크를 지날 때 사라지지 않는지 / 큰 워커가 작은 링 **앞**을 지나는지
  - 라이트·다크 양쪽에서 고정 어두운 퍽 위 숫자 대비
  - 감축 모션에서 두 마크가 궤도 위에 정지하는지
  - Codex의 5H가 `unknown`인 현재 상태에서 실제로 WK 값이 뜨는지
- [ ] `/code-review`
- [ ] `/prp-commit` → `/prp-pr`
