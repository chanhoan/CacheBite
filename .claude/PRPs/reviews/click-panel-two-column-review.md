# Code Review: 클릭 패널 2컬럼 전환 + 연결 provider 자동 감지

**Reviewed**: 2026-08-13
**Mode**: Local (uncommitted changes)
**Branch**: `feat/overlay-double-ring`
**Decision**: **APPROVE with comments** — 리뷰 중 발견한 HIGH 1건은 리뷰 과정에서 수정 완료

## Summary

패널 표시 계층 재배치로, 보안 표면은 건드리지 않는다. 리뷰에서 **HIGH 결함 1건**을 발견했다 — 네이티브 E2E가 제거된 탭 스트립을 계속 조작하고 있었고, `tsconfig.json`이 `tests/`를 포함하지 않아 `pnpm test:ci`가 이를 잡지 못했다. 수정 후 전 검증 통과.

## Findings

### CRITICAL
없음.

- 하드코딩된 자격증명·토큰 없음
- 사용자 입력 처리 없음 (props로 받은 정규화된 뷰모델만 렌더)
- SQL/명령 주입 표면 없음
- `innerHTML`/`{@html}` 사용 없음 — 모든 보간이 Svelte 텍스트 노드
- 프라이버시 계약 유지: freshness 문자열에 `oauth_api`/`cli_rpc`/`cached`가 새지 않음을 `ProviderColumn.test.ts`가 고정

### HIGH

#### H1. 네이티브 E2E가 제거된 탭 스트립을 조작 — **수정 완료**

**위치**: `tests/e2e/native.spec.ts:391, 416-418, 463-476, 484-491` / `src/nativeWorkflow.test.ts:190`

`ProviderTabs` 제거 후에도 네이티브 E2E가 탭을 클릭하고 `aria-selected`를 단언하고 있었다. 파손 지점 8개:

| 위치 | 문제 |
|---|---|
| `:391` | `button=Refresh now` — 픽스처 모드는 두 provider 모두 미연결 → 2컬럼 → 라벨이 `Refresh both` |
| `:416-418` | `button[role="tab"]=Claude` 클릭 + `aria-selected` — 요소 자체가 없음 |
| `:463-466` | `button[role="tab"]=Codex` 동일 |
| `:467-470` | 탭의 `aria-label='Claude (primary)'` — 없음 |
| `:471` | `button=Set as primary` — 이제 `Set Codex as primary` |
| `:476` | `waitUntil(!setPrimary.isEnabled())` — 후보가 상대편으로 뒤집혀 버튼이 **계속 활성** → 타임아웃까지 행 |
| `:484-491` | `finally` 블록에 같은 문제 반복 |

**왜 로컬 검증을 통과했는가**: `tsconfig.json`의 `include`가 `src/**`와 `vite.config.ts`뿐이라 `tests/`는 타입 검사를 받지 않고, `pnpm test:ci`는 네이티브 E2E를 실행하지 않으며(webdriver 피처를 켠 디버그 바이너리가 필요), eslint는 타입을 보지 않는다.

**다만 출시로 새어 나가지는 않았을 결함이다.** `native-smoke.yml`은 `on: pull_request`로 돌므로 머지 전에 잡혔다. 놓친 것은 **로컬 피드백 루프**이지 CI 안전망이 아니다 — M1의 우선순위를 이에 맞춰 낮췄다.

**수정**: 컬럼 기반으로 재작성 — `[data-provider="claude"]` 존재 확인, `aria-label="Claude usage (primary)"` 단언, `Set Codex as primary` → `Set Claude as primary` 왕복. `nativeWorkflow.test.ts`의 순서 가드 앵커도 새 첫 조회로 교체.

**검증 한계 (정직한 기록)**: `native.spec.ts`는 이 환경에서 **실행하지 못했다** — webdriver 피처가 켜진 디버그 바이너리가 필요하다. 확인한 것은 (a) eslint 통과, (b) 잔여 탭 셀렉터 0건 grep, (c) 두 실행 모드 모두 provider 2개가 미연결이므로 컬럼이 2개라는 상태 추론(`not_signed_in` → `auth_required`, `not_installed` → `unavailable`, 둘 다 blocking → `visiblePanelProviders`가 둘 다 반환, `panelModels.test.ts`가 이 조합을 직접 고정). `native-smoke.yml`에서 실제 확인이 필요하다.

### MEDIUM

#### M1. `tests/`가 모든 타입 검사 게이트 밖에 있다 — **의도적으로 미수정**

**위치**: `tsconfig.json`의 `include`

E2E 스펙은 wdio가 런타임에 트랜스파일만 하므로 타입 오류가 PR CI까지 살아남는다. `include`에 `tests/**/*.ts`를 넣으면 이런 부류가 `pnpm check`에서 즉시 잡힌다.

**수정하지 않은 이유 (측정 근거)**: 설정 한 줄로 끝나지 않는다. 임시 tsconfig로 실제로 돌려 보면:

```
error TS2688: Cannot find type definition file for '@wdio/globals'
error TS2688: Cannot find type definition file for 'expect-webdriverio'
```

두 패키지 모두 **transitive 의존이라 pnpm strict layout에서 루트로부터 해석되지 않는다** — `node_modules/@wdio/`에는 `cli`, `local-runner`, `mocha-framework`, `spec-reporter`, `tauri-service`만 있다. e2e 스펙은 `$`/`browser`/`expect`를 import 없이 전역으로 쓰므로, M1을 하려면 **devDependency 2개 추가가 선행**되어야 하고 이는 `audit:ci`·`licenses:check`·Dependabot 표면을 건드린다. 기능 브랜치에 섞을 변경이 아니다.

앰비언트 타입이 해석되지 않은 상태에서는 위 2건 외 오류가 0으로 나왔지만, **타입이 실제로 붙은 뒤의 오류 수는 여전히 미지수**다.

또한 우선순위 자체가 높지 않다: `native-smoke.yml`이 `on: pull_request`이므로 CI 안전망은 이미 있다. M1이 개선하는 것은 "PR CI 45분 뒤"를 "로컬 즉시"로 당기는 피드백 속도다.

#### M2. `region` 랜드마크가 3단으로 중첩된다 — **수정 완료**

**위치**: `src/lib/components/ProviderColumn.svelte`, `src/lib/components/UsageGauge.svelte`

`<section aria-label="Usage panel">` → `<section class="column" aria-label="Claude usage (primary)">` → `<section aria-label="5-hour usage">` (`UsageGauge`).

접근성 이름이 붙은 `<section>`은 `region` 랜드마크가 된다. 이번 변경이 한 단계를 더해, 2컬럼 × 게이지 2개 구성에서 380px 팝오버 하나에 랜드마크가 **7개**(패널 1 + 컬럼 2 + 게이지 4) 생겼다. 기존은 3개였다.

**수정**: 컬럼과 게이지 양쪽에 `role="group"`을 명시해 암묵 `region`을 대체했다. 랜드마크는 패널 1개로 줄고 접근성 이름은 그대로 유지된다. 게이지만 고치면 컬럼이, 컬럼만 고치면 게이지가 남으므로 **두 곳을 함께** 처리했다.

**테스트 영향 없음**: `getByLabelText`(접근명 기준), `getAllByTestId('usage-gauge')`, e2e의 `section[aria-label="Weekly usage"]`·`[data-provider="claude"]`(CSS 태그/속성 셀렉터) 모두 롤과 무관하다. 373 unit / 11 e2e 전건 통과로 확인.

### LOW

#### L1. `visiblePanelProviders`가 모듈 상수 배열을 그대로 반환 — **수정 완료**

**위치**: `src/lib/components/panelModels.ts:66`

0개 연결 경로에서 `PANEL_ORDER`를 그대로 돌려주고 있었다. `readonly`는 컴파일 타임 주장일 뿐이라, 한 번의 변형이 세션 내내 순서를 오염시킬 수 있다. `[...PANEL_ORDER]` 복사본 반환으로 수정.

#### L2. 테스트 이름이 실제 버튼 라벨과 불일치 — **수정 완료**

**위치**: `src/App.test.ts:612`

`'changes primary only when Set as primary is clicked'` → 버튼은 이제 `Set Codex as primary`다. 이름을 `'…when the named primary control is clicked'`로 교체.

#### L3. `main.panel` 폭의 rem↔px 결합 (기존 패턴)

`global.css`가 `min(23.75rem, 100%)`, 네이티브가 `380.0` 논리 픽셀이다. `:root`에 `font-size`가 없어 1rem = 16px이므로 현재는 일치한다. 다만 누군가 루트 폰트 크기를 지정하면 셸이 창보다 좁아져 투명 여백이 생긴다. 이전 값(19.5rem/312px)도 같은 결합이었으므로 회귀는 아니다.

## 품질 체크리스트

| 항목 | 결과 |
|---|---|
| 함수 50줄 초과 | 없음 (최대 `visiblePanelProviders` 6줄) |
| 파일 800줄 초과 | 없음 (`UsagePanel.svelte` 226줄, `ProviderColumn.svelte` 121줄) |
| 중첩 4단 초과 | 없음 |
| `console.log`/디버그문 | 없음 |
| `TODO`/`FIXME` | 없음 |
| 공개 API JSDoc | `panelModels.ts` 3개 함수 모두 근거 주석 포함 |
| 불변 패턴 | 유지 — `filter`/스프레드만 사용, 제자리 변형 없음 |
| 매직 넘버 | 없음 — 색·간격 전부 CSS 토큰. 예외 1건(★ `#f59e0b`)은 `ProviderTabs`에서 근거 주석과 함께 이관 |
| 신규 코드 테스트 | 47건 신규 (17 + 8 + 22) |
| 죽은 코드 | `ProviderTabs.*` 및 스토어 `selected`/`selectTab` 제거 완료, 잔여 참조 0건 |

## Validation Results

| Check | Result |
|---|---|
| Type check (`svelte-check`) | **Pass** — 0 errors, 0 warnings |
| Lint (`eslint` + `prettier`) | **Pass** |
| Unit tests (renderer) | **Pass** — 373/373, 27 files |
| Coverage gate (80%) | **Pass** |
| Build (`vite build`) | **Pass** |
| E2E renderer | **Pass** — 11 passing |
| Rust tests (`--all-features`) | **Pass** — 147/147 |
| **E2E native** | **Skipped** — webdriver 피처 디버그 빌드 필요. H1 수정본은 미실행 |

## Files Reviewed

| File | Change |
|---|---|
| `src/lib/components/panelModels.ts` | Modified (+리뷰 중 L1 수정) |
| `src/lib/components/panelModels.test.ts` | Added |
| `src/lib/components/ProviderColumn.svelte` | Added |
| `src/lib/components/ProviderColumn.test.ts` | Added |
| `src/lib/components/UsagePanel.svelte` | Modified |
| `src/lib/components/UsagePanel.test.ts` | Modified |
| `src/lib/components/UsageGauge.svelte` | Modified |
| `src/lib/components/ProviderTabs.svelte` | Deleted |
| `src/lib/components/ProviderTabs.test.ts` | Deleted |
| `src/lib/stores/providers.ts` | Modified |
| `src/lib/stores/providers.test.ts` | Modified |
| `src/App.svelte` | Modified |
| `src/App.test.ts` | Modified (+리뷰 중 L2 수정) |
| `src/lib/api/fixtureGateway.ts` | Modified |
| `src/lib/styles/global.css` | Modified |
| `src/securityConfig.test.ts` | Modified |
| `src/nativeWorkflow.test.ts` | **Modified (리뷰 중 H1 수정)** |
| `src-tauri/tauri.conf.json` | Modified |
| `src-tauri/src/refresh/ipc.rs` | Modified |
| `src-tauri/src/window/tests.rs` | Modified |
| `tests/e2e/renderer.spec.ts` | Modified |
| `tests/e2e/native.spec.ts` | **Modified (리뷰 중 H1 수정)** |
| `docs/ui-contract.md` | Modified |
| `README.md` / `README.en.md` | Modified |

## 커밋 전 확인 사항

1. **`native-smoke.yml`에서 H1 수정본이 실제로 통과하는지 확인.** 로컬에서 검증 불가한 유일한 항목이다.
2. **작업 트리가 섞여 있다.** 이 브랜치에는 이전 세션의 오버레이 더블링 작업(`PetOverlay.svelte`, `SatelliteRing.svelte`, `SystemBadge.svelte`, `orbitPath.ts`, `orbitPath.test.ts`)이 커밋되지 않은 채 남아 있다. `docs/ui-contract.md`는 양쪽이 건드렸으나 섹션이 다르다(이전 §4.4/§4.5, 이번 §5).
3. **M1은 남겨 둔 유일한 항목이다.** 별도 브랜치에서 `@wdio/globals`·`expect-webdriverio`를 devDependency로 추가한 뒤 `tsconfig.json`의 `include`를 넓히는 순서로 진행할 것. 급하지 않다 — PR CI가 이미 막아 준다.
