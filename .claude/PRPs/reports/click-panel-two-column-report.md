# Implementation Report: 클릭 패널 2컬럼 전환 + 연결 provider 자동 감지

## Summary

클릭 패널을 탭 기반 단일 provider 뷰에서 **2컬럼 동시 표시**로 바꿨다. 컬럼 수는 설정이 아니라 **연결된 provider 수에서 파생**된다 — 둘 다 연결이면 2컬럼, 하나만이면 그 하나만, 둘 다 미연결이면 둘 다. 패널 창 폭은 312px → 380px로 넓혔고, `ProviderTabs`와 스토어의 `selected`/`selectTab` 상태를 제거했다.

## Assessment vs Reality

| Metric | Predicted (Plan) | Actual |
|---|---|---|
| Complexity | Large | Large — 예측대로 |
| Confidence | 8/10 | 타당했음. 계획에 없던 이슈 1건(`noUncheckedIndexedAccess`), 예측 못 한 테스트 파급 1건(`findByText('Pro')` 다중 매치 13곳) |
| Files Changed | 22 (신규 4 / 수정 16 / 삭제 2) | **22** (신규 4 / 수정 16 / 삭제 2) — 정확히 일치 |
| Rust 기하 기대 좌표 | 4건 불변 예측 | **4건 전부 불변 확인** — 계획의 수기 계산이 맞았음 |

## Tasks Completed

| # | Task | Status | Notes |
|---|---|---|---|
| 1 | `panelModels.ts` 순수 판정 함수 (TDD) | 완료 | 17 tests. `primaryCandidate` 구현만 편차 (아래) |
| 2 | `UsageGauge` 헤딩 카피 축약 | 완료 | `{label} window` → `{label}`, `aria-label` 불변 |
| 3 | `ProviderColumn.svelte` 신규 (TDD) | 완료 | 8 tests |
| 4 | `UsagePanel.svelte` 재작성 | 완료 | 22 tests. `style:` 디렉티브 대신 속성 보간 사용 (아래) |
| 5 | 스토어 `selected`/`selectTab` 제거 | 완료 | `providers.test.ts`에서 탭 테스트 1건 삭제 |
| 6 | `App.svelte` 배선 + `App.test.ts` 갱신 | 완료 | 계획이 예상한 2건 외 **13곳 추가 수정** (아래) |
| 7 | `ProviderTabs` 삭제 | 완료 | ★ 색 리터럴·근거 주석을 `ProviderColumn`으로 이전 후 삭제 |
| 8 | 패널 폭 380px | 완료 | 상수 5곳 + 플립 분기 신규 Rust 테스트 1건 |
| 9 | `fixtureGateway` `?panel=single` | 완료 | |
| 10 | e2e 스펙 갱신 | 완료 | 3건 수정 / 1건 삭제 / 2건 신규 |
| 11 | 문서 갱신 | 완료 | `ui-contract.md` §5, README 2종 |

## Validation Results

| Level | Status | Notes |
|---|---|---|
| Static Analysis | 통과 | `svelte-check` 0 errors 0 warnings, `eslint` 무출력, `prettier --check` 통과 |
| Unit Tests | 통과 | 렌더러 **373 tests** (신규 47), Rust **147 tests** (신규 1) |
| Coverage Gate | 통과 | 80% branches/functions/lines/statements 유지 |
| Build | 통과 | `vite build` — 165 modules, 105.67 kB JS / 16.25 kB CSS |
| E2E (renderer) | 통과 | **11 passing** — 신규 컬럼 스펙 2건 포함 |
| Rust (all-features) | 통과 | 147 passed, 0 failed |
| Edge Cases | 통과 | 아래 체크리스트 참조 |

### Edge Case 검증

| 케이스 | 검증 위치 |
|---|---|
| 둘 다 연결 → 2컬럼 | `panelModels.test.ts`, `UsagePanel.test.ts`, e2e `renders one column per connected provider` |
| 하나만 연결 → 1컬럼, 폭 전체 | `UsagePanel.test.ts` ×2, e2e `collapses to a single column…` (셸 폭 일치 측정) |
| 둘 다 미연결 → 2컬럼 (빈 패널 방지) | `panelModels.test.ts`, `UsagePanel.test.ts` (안내문 2개 확인) |
| `loading` → 연결로 취급 | `panelModels.test.ts` |
| `error`/`offline` → 컬럼 유지 | `panelModels.test.ts` ×2, `UsagePanel.test.ts`, `ProviderColumn.test.ts` |
| primary가 숨겨진 provider일 때 후보 활성 | `panelModels.test.ts`, `UsagePanel.test.ts` |
| 보이는 게 primary 하나뿐 → 비활성, 콜백 없음 | `UsagePanel.test.ts` |
| 디바운스 중 하나라도 있으면 새로고침 비활성 | `UsagePanel.test.ts` |
| `planType === null` → chip 미렌더 | `ProviderColumn.test.ts` |
| freshness 프라이버시(source/cache 누출 없음) | `ProviderColumn.test.ts` |
| 넓어진 패널의 좌측 플립 분기 | `window/tests.rs` 신규 테스트 |

## Files Changed

| File | Action | Lines |
|---|---|---|
| `src/lib/components/panelModels.test.ts` | CREATED | +117 |
| `src/lib/components/ProviderColumn.svelte` | CREATED | +121 |
| `src/lib/components/ProviderColumn.test.ts` | CREATED | +124 |
| `.claude/PRPs/plans/click-panel-two-column.plan.md` | CREATED | +740 |
| `src/lib/components/ProviderTabs.svelte` | DELETED | −59 |
| `src/lib/components/ProviderTabs.test.ts` | DELETED | −14 |
| `src/lib/components/UsagePanel.test.ts` | UPDATED | +345 (전면 재작성) |
| `src/lib/components/UsagePanel.svelte` | UPDATED | +149 (전면 재작성) |
| `tests/e2e/renderer.spec.ts` | UPDATED | +111 |
| `src/App.test.ts` | UPDATED | +95 |
| `docs/ui-contract.md` | UPDATED | +54 |
| `src/lib/components/panelModels.ts` | UPDATED | +45 |
| `src-tauri/src/window/tests.rs` | UPDATED | +42 |
| `src/lib/api/fixtureGateway.ts` | UPDATED | +22 |
| `README.md` | UPDATED | +8 |
| `src/lib/stores/providers.test.ts` | UPDATED | −8 |
| `README.en.md` | UPDATED | +6 |
| `src/App.svelte` | UPDATED | +6 |
| `src/lib/stores/providers.ts` | UPDATED | −5 |
| `src/lib/components/UsageGauge.svelte` | UPDATED | +1 / −1 |
| `src-tauri/tauri.conf.json` | UPDATED | +1 / −1 |
| `src-tauri/src/refresh/ipc.rs` | UPDATED | +1 / −1 |
| `src/lib/styles/global.css` | UPDATED | +1 / −1 |
| `src/securityConfig.test.ts` | UPDATED | +1 / −1 |

## Deviations from Plan

### 1. `primaryCandidate` 구현 — 인덱스 접근 대신 구조 분해

- **WHAT**: 계획의 `return others.length === 1 ? others[0] : null;` 대신
  `const [only, ...rest] = …; return rest.length === 0 ? (only ?? null) : null;`
- **WHY**: `tsconfig`의 `noUncheckedIndexedAccess`가 `others[0]`을 `Provider | undefined`로 좁혀 `svelte-check`가 실패했다. 계획이 이 컴파일러 옵션을 고려하지 못했다.

### 2. `--panel-columns` 전달 방식 — `style:` 디렉티브 대신 속성 보간

- **WHAT**: `style:--panel-columns={visible.length}` 대신 `style="--panel-columns: {visible.length};"`
- **WHY**: 커스텀 프로퍼티에 대한 `style:` 디렉티브 지원 여부를 런타임에 확인하는 대신, 모든 Svelte 버전에서 동작이 보장되는 속성 보간을 택했다. 결과는 동일하다.

### 3. `App.test.ts` 수정 범위 — 예상 2건 → 실제 15건

- **WHAT**: 계획은 탭 클릭에 의존하는 2건만 예상했으나, 실제로는 18건이 실패했다.
- **WHY**: 픽스처가 두 provider 모두 `active`라 컬럼이 2개가 되면서 **DOM 요소가 전부 2배**가 됐다.
  - `findByText('Pro')` (plan chip) 13곳 → `findAllByText` (그중 3곳은 `toHaveLength(2)`로 강화)
  - `findByText(/Fresh/)` / `getByText(/Stale/)` → `findAllByText` / `getAllByText`, 길이 2 단언
  - `Refresh now` → `Refresh both`
  - 탭 클릭 2건 → 이름이 붙은 primary 버튼 클릭으로 재작성

### 4. `keeps sign-in guidance when an expired snapshot arrives with the reason` — 시나리오 확장

- **WHAT**: Claude만 로그아웃시키던 테스트에, Codex도 로그아웃시키는 두 번째 단계를 추가했다.
- **WHY**: **새 동작이 기존 단언을 무효화했기 때문이다.** Claude가 `auth_required`가 되면 Codex가 연결된 동안에는 Claude 컬럼이 **숨겨져** 로그인 안내가 화면에서 사라진다. 원래 단언(`findByText('Sign in to the Claude CLI…')`)은 성립할 수 없다.
  단순히 단언을 지우는 대신 테스트를 두 단계로 만들었다:
  1. Claude만 로그아웃 → 컬럼이 사라지는 것을 **명시적으로 단언** (자동 접힘의 대가를 테스트로 못박음)
  2. Codex도 로그아웃 → 둘 다 미연결이 되어 컬럼이 돌아오고, 원래 검증하려던 "degradation 사유가 `error`가 아니라 `auth_required`" 단언이 다시 성립

### 5. freshness 프라이버시 테스트 이동

- **WHAT**: 계획에 명시되지 않았으나, freshness 프라이버시 테스트(`oauth_api`/`cli_rpc`/`cached` 누출 금지)를 `UsagePanel.test.ts`에서 `ProviderColumn.test.ts`로 옮겼다.
- **WHY**: 해당 마크업이 `ProviderColumn`으로 이동했으므로 테스트도 함께 따라가는 것이 맞다. 커버리지 손실은 없다.

## Issues Encountered

| 이슈 | 해결 |
|---|---|
| PowerShell에 `pnpm`이 PATH에 없음 | `corepack pnpm …`으로 실행 (레포의 `audit:ci` 스크립트가 쓰는 것과 동일한 경로) |
| `noUncheckedIndexedAccess`로 타입 오류 4건 | `panelModels.ts`는 구조 분해로, 테스트 3곳은 옵셔널 체이닝으로 해결 |
| `prettier --check` 실패 1건 | `panelModels.test.ts`에 `--write` 적용 |
| `App.test.ts` 18건 실패 | 위 편차 3·4번 참조. 최종 51/51 통과 |

## Tests Written

| Test File | Tests | Coverage |
|---|---|---|
| `src/lib/components/panelModels.test.ts` | 17 (신규) | 연결 판정 6상태 표 + 표시 조합 7가지 + primary 후보 4가지 |
| `src/lib/components/ProviderColumn.test.ts` | 8 (신규) | 접근명·★ 장식성·스켈레톤·게이지 2개·freshness 프라이버시·컬럼별 안내문·plan chip |
| `src/lib/components/UsagePanel.test.ts` | 22 (재작성, 기존 11 → 22) | 컬럼 자동 파생 6가지 + 푸터 라벨/대상/비활성 6가지 + 기존 회귀 |
| `src-tauri/src/window/tests.rs` | 1 (신규) | 넓어진 패널의 좌측 플립 분기 |
| `tests/e2e/renderer.spec.ts` | 2 (신규), 3 수정, 1 삭제 | 실제 엔진에서의 2컬럼 나란함 / 1컬럼 셸 폭 점유 |

## 작업 트리에 관한 주의

이 구현은 브랜치 `feat/overlay-double-ring`에서 수행되었고, 착수 시점에 **이전 세션의 오버레이 더블링 작업이 커밋되지 않은 채** 남아 있었다. 따라서 `git diff`에는 이 계획과 무관한 5개 파일이 섞여 있다:

- `src/lib/components/PetOverlay.svelte`
- `src/lib/components/SatelliteRing.svelte`
- `src/lib/components/SystemBadge.svelte`
- `src/lib/components/orbitPath.ts`
- `src/lib/components/orbitPath.test.ts`

`docs/ui-contract.md`는 양쪽이 모두 건드렸지만 섹션이 다르다 — 이전 작업은 §4.4/§4.5(오버레이 링), 이번 작업은 §5(클릭 패널). 커밋을 분리하려면 이 목록으로 스테이징을 나눌 수 있다.

## 불변식 확인

- **`ring_mode`는 손대지 않았다.** 이 계획의 "연결 감지"는 패널 표시에만 작용하며 오버레이 링 모드·큰 원 바인딩·펫 무드와 연결되지 않는다.
- **수집·새로고침 일정·알림 라우팅에 변경 없음.** 컬럼이 숨겨져도 해당 provider의 수집은 계속된다.
- **`primary_provider` 자동 변경 없음.** 미연결 provider가 primary여도 강등하지 않는다.
- **프라이버시 계약 유지.** freshness 문자열에 `oauth_api`/`cli_rpc`/`cached`가 새지 않음을 테스트로 고정.
- 네이티브 변경은 상수 1개(`PANEL_WIDTH_LOGICAL`)와 창 설정 1개(`width`)뿐이며, Rust 147건 전건 통과로 회귀 없음을 확인했다.

## Next Steps

- [ ] `/code-review`로 변경 검토
- [ ] 커밋 (이전 세션의 오버레이 변경과 분리 여부 결정 필요)
- [ ] 수동 검증: `pnpm tauri dev` — 계획서의 Manual Validation 체크리스트 9항목 (특히 다크/라이트 구분선, `resets in 3d 21h 0m` 최장 라벨 줄바꿈, 1↔2컬럼 전환 시 창 앵커)
- [ ] 후속 계획 후보: 미연결 provider를 숨기면서 잃은 "두 번째 provider 연결 가능성" 발견 경로를 Settings에 추가
