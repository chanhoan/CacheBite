# Plan: 클릭 패널 2컬럼 전환 + 연결 provider 자동 감지

## Summary

클릭 패널을 `docs/UI-plan-2.0/CacheBite v2.dc.html` §5 목업대로 **탭 기반 단일 provider 뷰 → 2컬럼 동시 표시**로 바꾼다. 컬럼 수는 사용자가 고르지 않고 **연결된 provider 수에서 자동으로 파생**된다: 둘 다 연결이면 2컬럼, 하나만 연결이면 그 하나만, 둘 다 미연결이면(숨길 기준이 없으므로) 둘 다. 패널 창 폭은 312px → 380px로 넓힌다.

## User Story

CacheBite 사용자로서,
Claude와 Codex 사용량을 **탭 전환 없이 한 화면에서** 보고 싶고, 한쪽만 연결했을 때는 쓰지도 않는 provider 자리가 화면을 차지하지 않기를 바란다.
그래야 패널을 여는 것만으로 지금 내 한도 상황 전부를 알 수 있다.

## Problem → Solution

| | 현재 | 목표 |
|---|---|---|
| 레이아웃 | `ProviderTabs`로 한 번에 한 provider만 (`UsagePanel.svelte:40`, `:52-77`) | 2컬럼 grid 동시 표시 |
| 컬럼 수 | 고정 1 (탭이 선택) | 연결 provider 수에서 자동 파생 |
| 폭 | 312px (`tauri.conf.json:38`) | 380px |
| 새로고침 | `Refresh now` (선택 탭) | `Refresh both` / `Refresh now` (보이는 것 전부) |
| primary 전환 | `Set as primary` (선택 탭) | `Set {상대} as primary` |
| freshness | 본문 하단 1개 | 컬럼마다 |

## Metadata

- **Complexity**: Large (렌더러 + 네이티브 상수 + e2e + 문서)
- **Source PRD**: `docs/UI-plan-2.0/CacheBite v2.dc.html` (§5 패널 목업) — PRD 형식은 아니지만 이 계획의 디자인 원본
- **선행 계획**: `.claude/PRPs/plans/completed/overlay-double-ring.plan.md` — 그 계획의 `Notes` (836-846행)가 **이 계획으로 넘긴 델타 표**를 이미 명시해 두었다. 이 계획은 그 표를 그대로 이행한다
- **PRD Phase**: standalone
- **Estimated Files**: 22 (신규 4 / 수정 16 / 삭제 2)

---

## 사용자 확정 결정 (이 계획의 전제)

| 항목 | 결정 | 근거 |
|---|---|---|
| 패널 폭 | **380px로 확장** | 목업 §5 그대로. 컬럼당 콘텐츠 폭 ~156px 확보 |
| 미연결 provider | **완전히 숨김** | 요청 문구("하나만 표시")에 충실 |
| "연결됨" 판정 | `auth_required` / `unavailable`만 **미연결** | `engine.ts:121-122`의 `isBlockingStatus`와 동일 기준. `error`·`offline`은 "연결됐지만 일시 실패"라 컬럼 유지 |
| 탭 스트립 | **제거** | 2컬럼 동시 표시에서 탭은 중복. 목업에도 없음 |

---

## UX Design

### Before (312px)

```text
┌──────────── 312px ────────────┐
│                          (✕)  │
│  [Claude ★] [Codex]           │  ← 탭. 한 번에 하나만 열람
├───────────────────────────────┤
│  Claude                 Pro   │
│  5-hour window          74%   │
│  ▓▓▓▓▓▓▓░░░                   │
│  Weekly window          20%   │
│  ▓▓░░░░░░░░                   │
│  ● Fresh · captured 2 min ago │
│  (guidance role=status)       │
├───────────────────────────────┤
│  [Refresh now][Set as primary]│
│  [Settings]        [Quit]     │
└───────────────────────────────┘
```

### After — 2개 연결 (380px)

```text
┌──────────────── 380px ─────────────────┐
│  Usage                            (✕)  │  ← header: 보이는 제목 + border-bottom
├──────────────────┬─────────────────────┤
│ Claude ★    Pro  │ Codex        Plus   │
│ 5-hour      82%  │ 5-hour        41%   │
│ ▓▓▓▓▓▓▓▓░░       │ ▓▓▓▓░░░░░░          │
│ resets in 1h 12m │ resets in 3h 40m    │
│ Weekly      94%  │ Weekly        58%   │
│ ▓▓▓▓▓▓▓▓▓░       │ ▓▓▓▓▓▓░░░░          │
│ ● Fresh · 2m     │ ● Fresh · 5m        │
│ (guidance)       │ (guidance)          │
├──────────────────┴─────────────────────┤
│  [Refresh both] [Set Codex as primary] │
│  [Settings]              [Quit]        │
└────────────────────────────────────────┘
                   ↑ 1px 세로 구분선 (.column + .column)
```

### After — 1개만 연결 (Codex 미설치)

```text
┌──────────────── 380px ─────────────────┐
│  Usage                            (✕)  │
├────────────────────────────────────────┤
│ Claude ★                        Pro    │  ← 컬럼 1개가 폭 전체를 차지
│ 5-hour                          82%    │
│ ▓▓▓▓▓▓▓▓░░                             │
│ resets in 1h 12m                       │
│ Weekly                          94%    │
│ ▓▓▓▓▓▓▓▓▓░                             │
│ ● Fresh · captured 2 min ago           │
├────────────────────────────────────────┤
│  [Refresh now] [Set as primary (비활성)]│
│  [Settings]              [Quit]        │
└────────────────────────────────────────┘
```

### Interaction Changes

| Touchpoint | Before | After | Notes |
|---|---|---|---|
| provider 전환 | 탭 클릭 | **없음** (둘 다 보임) | `selectTab` / `selected` 상태 자체를 제거 |
| 새로고침 | 선택된 하나 | 보이는 것 전부, 라벨 `Refresh both`/`Refresh now` | `onRefresh(provider)`를 보이는 provider 수만큼 호출 |
| primary 전환 | `Set as primary` (선택 탭 대상) | `Set {상대} as primary`, 대상 없으면 비활성 | 대상 = 보이는 컬럼 중 primary가 **아닌** 하나 |
| 헤더 | `h2.visually-hidden` | 보이는 `<h2>Usage</h2>` + border-bottom | ✕는 그대로 흐름 밖 절대 위치 |
| freshness | 본문 하단 1줄 | 컬럼마다 1줄 | |
| ✕ 겹침 | 두 번째 탭 위에 얹힘 (계약 §5) | **겹침 대상 소멸** | 관련 계약 문단·e2e 테스트 삭제 |
| 로딩 스켈레톤 | 본문 전체 1개 | 컬럼마다 1개 | 기동 시 2개 동시 등장 가능 |

### UX Edge Cases

- **기동 직후**: 두 provider 모두 `loading`(비-blocking) → 2컬럼 + 스켈레톤 2개. 판정이 끝나면 1컬럼으로 접힐 수 있다. 높이 변화는 기존 `ResizeObserver → resize_panel` 경로가 그대로 흡수한다(`App.svelte:460-481`). **`resizePanel`을 직접 호출하지 말 것** — 그 명령은 패널 노출 게이트를 겸한다(ui-contract §5.1).
- **둘 다 미연결**: 숨길 기준이 없으므로 2컬럼 유지. 그래야 두 provider의 `systemGuidance`(로그인 안내)가 모두 화면에 남는다.
- **primary가 미연결이고 상대만 연결**: 링 모드 불변식상 primary는 자동 강등되지 않으므로, 보이는 컬럼은 상대 하나뿐이고 `Set {상대} as primary` 버튼이 **활성**된다 — 이때 이 버튼이 가장 유용하다.

---

## Mandatory Reading

| Priority | File | Lines | Why |
|---|---|---|---|
| P0 | `src/lib/components/UsagePanel.svelte` | 전체 | 재작성 대상. 기존 CSS 토큰·푸터 구조를 그대로 이어받아야 함 |
| P0 | `src/lib/components/panelModels.ts` | 1-22 | `PanelProviderModel` 계약. 여기에 순수 헬퍼를 추가 |
| P0 | `src/lib/state/engine.ts` | 112-145 | `derivePetUiState` — `isBlockingStatus`(121-122)가 "연결됨" 판정의 기준선 |
| P0 | `src/App.svelte` | 527-534, 776-793 | `panelProviders` 파생과 `UsagePanel` prop 배선 |
| P1 | `src/lib/components/UsageGauge.svelte` | 30-38, 59-74 | 컬럼 안에서 그대로 재사용. 헤딩 카피만 변경 |
| P1 | `src/lib/components/systemGuidance.ts` | 전체 | 컬럼별 안내문 소스. 시그니처 변경 없음 |
| P1 | `src/lib/stores/providers.ts` | 18-33, 182-184 | `selected` / `selectTab` 제거 대상 |
| P1 | `src/lib/contracts/domain.ts` | 1-22 | `Provider`, `PROVIDER_NAME`, `secondaryProvider` |
| P1 | `src/lib/components/UsagePanel.test.ts` | 전체 | 테스트 픽스처 형태(`provider()`, `bothProviders()`)를 그대로 확장 |
| P2 | `tests/e2e/renderer.spec.ts` | 212-400 | 패널 e2e 3건 — 폭·패딩·탭 겹침 |
| P2 | `src-tauri/src/window/tests.rs` | 118-236 | 패널 앵커 기하 테스트 4건의 `width: 312.0` 리터럴 |
| P2 | `src-tauri/src/window/mod.rs` | 474-492 | `anchor_panel` / `clamp_to_rect` — 폭 변경 영향 계산의 근거 |
| P2 | `docs/ui-contract.md` | 326-366 | §5 클릭 패널 계약. 이 계획이 개정 |
| P2 | `.claude/PRPs/plans/completed/overlay-double-ring.plan.md` | 836-846 | 선행 계획이 남긴 델타 표 = 이 계획의 명세서 |

## External Documentation

| Topic | Source | Key Takeaway |
|---|---|---|
| v2 패널 목업 | `docs/UI-plan-2.0/CacheBite v2.dc.html` 176-237행 | 380px, 2컬럼 `1fr 1fr`, 컬럼 사이 `border-right: 1px solid #eceef1`, 컬럼 padding 16px, 헤더 "Usage" + ✕, 푸터 `Refresh both` / `Set Codex as ★` |
| v2 문서의 명시적 범위 | 같은 문서 114행, 241행 | "This contract covers **only** the new double-ring overlay … the click panel and any of its changes … follow v1 as-is (out of scope here)". 즉 §5 목업은 **참고 렌더**이며 v2 계약문이 아니다 — 그래서 목업과 어긋나는 부분(아래 "목업 대비 의도적 편차")은 v1 계약을 따른다 |

**외부 라이브러리 리서치 불필요** — 전부 기존 내부 패턴(Svelte 5 runes, 순수 모델 헬퍼 + 컴포넌트 분리, vitest + @testing-library/svelte, WebdriverIO)으로 처리된다.

### 목업 대비 의도적 편차

| 목업 | 이 계획 | 근거 |
|---|---|---|
| 시스템 안내문 없음 | 컬럼마다 `role="status"` 안내문 유지 | 미연결은 숨기지만 `error`/`offline` 컬럼은 남는다. 안내문을 지우면 그 상태에서 복구 경로가 사라진다 (ui-contract §5) |
| `Set Codex as ★` | `Set Codex as primary` | 버튼 접근명에 ★ 글리프가 들어가면 스크린리더가 읽지 못한다. ★는 컬럼 헤딩의 `aria-hidden` 장식으로만 유지 |
| 컬럼 헤딩에 `Pro` / `Plus` 하드코딩 | `planType`이 있을 때만 chip 렌더 | 기존 `UsagePanel.svelte:54-55` 동작 유지 |

---

## Patterns to Mirror

### NAMING_CONVENTION — 컴포넌트 옆 순수 모델 헬퍼
```ts
// SOURCE: src/lib/components/systemGuidance.ts:19-35
export function systemGuidance(
  system: SystemState,
  provider: Provider,
): string | null {
  switch (system) {
    case 'auth_required':
      return `Sign in to the ${CLI_NAME[provider]} CLI: ${SIGN_IN_COMMAND[provider]}`;
    ...
    default:
      return null;
  }
}
```
> 컴포넌트 로직은 `.svelte`가 아니라 옆의 `.ts`에 두고 거기서 단위 테스트한다 (CLAUDE.md "components/ — Svelte views plus their non-visual model/copy helpers").

### TYPE_DEFINITION — 읽기 전용 뷰모델
```ts
// SOURCE: src/lib/components/panelModels.ts:4-22
export interface PanelProviderModel {
  readonly provider: Provider;
  readonly system: SystemState;
  readonly stale: boolean;
  readonly planType: string | null;
  readonly session: {
    readonly usedPercent: number | null;
    readonly severity: Severity;
    readonly resetsAt: string | null;
  };
  ...
}
```

### SVELTE_PROPS — Svelte 5 runes + JSDoc 타입
```svelte
<!-- SOURCE: src/lib/components/UsagePanel.svelte:7-22 -->
<script>
  /** @typedef {import('./panelModels').PanelProviderModel} PanelProvider */
  /** @type {{ providers: { claude: PanelProvider; codex: PanelProvider }; ... }} */
  let {
    providers,
    selected,
    primary = selected,
    refreshing,
    nowMs = Date.now(),
    onRefresh = () => {},
    ...
  } = $props();
  const current = $derived(providers[selected]);
</script>
```
> `UsagePanel.svelte`는 `lang="ts"` 없이 JSDoc으로 타입을 붙인다. `UsageGauge.svelte`는 `lang="ts"`를 쓴다 — **신규 `ProviderColumn.svelte`는 `lang="ts"`** 쪽을 따른다 (props가 단순 스칼라 위주라 JSDoc보다 읽기 쉽다).

### CSS_TOKENS — 하드코딩 색상 금지
```css
/* SOURCE: src/lib/components/UsagePanel.svelte:154-200 */
header { padding: var(--space-3) var(--space-4) 0; }
.body  { display: grid; gap: var(--space-4); padding: var(--space-4); }
footer {
  display: grid;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4) 0.875rem;
  border-top: 1px solid var(--color-border);
}
.footer-row { display: grid; grid-template-columns: 1fr 1fr; gap: var(--space-2); }
```
> 유일한 예외는 primary ★의 앰버색으로, 기존 `ProviderTabs.svelte:56-58`이 `#f59e0b`를 직접 쓴다. 이 리터럴과 그 위의 주석을 `ProviderColumn.svelte`로 **그대로 옮긴다**.

### TEST_STRUCTURE — 컴포넌트 테스트
```ts
// SOURCE: src/lib/components/UsagePanel.test.ts:1-25
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import UsagePanel from './UsagePanel.svelte';

const NOW = Date.parse('2026-07-16T12:02:00Z');

const provider = (system: SystemState, stale = false) => ({
  provider: 'claude' as const,
  system,
  stale,
  planType: 'pro',
  session: { usedPercent: 74, severity: 'warn' as const, resetsAt: null },
  weekly: { usedPercent: 20, severity: 'ok' as const, resetsAt: null },
  capturedAt: '2026-07-16T12:00:00Z',
  source: 'oauth_api' as const,
  isCached: false,
});

describe('UsagePanel', () => {
  afterEach(cleanup);
  ...
});
```

### TEST_STRUCTURE — 표 기반 케이스
```ts
// SOURCE: src/lib/components/UsagePanel.test.ts:228-245
it.each([
  ['auth_required' as const, 'Sign in to the Claude CLI: claude login'],
  ['unavailable' as const, 'The Claude CLI is not installed'],
])('shows recovery guidance for %s', (system, expected) => { ... });
```

### RUST_CONSTANT
```rust
// SOURCE: src-tauri/src/refresh/ipc.rs:29-30
/// Fixed panel width in logical pixels. Only the height tracks content.
const PANEL_WIDTH_LOGICAL: f64 = 312.0;
```

### RUST_TEST — 기하 테스트
```rust
// SOURCE: src-tauri/src/window/tests.rs:118-143
/// 1920x1080 with a 48px taskbar leaves a 1032px work area. A 312x520 panel
/// anchored to a pet parked at the bottom must not slide under the taskbar.
#[test]
fn panel_anchor_keeps_panel_above_taskbar_for_bottom_pet() {
    let work_area = Rect { x: 0.0, y: 0.0, width: 1920.0, height: 1032.0 };
    let pet = Rect { x: 100.0, y: 800.0, width: 240.0, height: 240.0 };
    let panel = Size { width: 312.0, height: 520.0 };

    let anchored = anchor_panel(pet, panel, work_area, 12.0);

    assert_eq!(anchored, Point { x: 352.0, y: 512.0 });
    assert!(anchored.y + panel.height <= work_area.y + work_area.height);
}
```

### E2E — 브라우저 측정
```ts
// SOURCE: tests/e2e/renderer.spec.ts:248-310
it('uses the unified 312px vibrancy panel shell', async () => {
  await browser.url('/?window=panel&fixture=e2e');
  await expect($('section[aria-label="Usage panel"]')).toBeDisplayed();
  const layout = await browser.execute(() => { /* getComputedStyle 측정 */ });
  expect(layout).toMatchObject({ outerWidth: 312, headerPadding: [...] });
});
```

### FIXTURE_VARIANT — 쿼리 파라미터로 e2e 변형
```ts
// SOURCE: src/lib/api/fixtureGateway.ts:56-59
ringMode:
  new URLSearchParams(window.location.search).get('ring') === 'double'
    ? 'double'
    : 'single',
```
> 기본값은 **기존 스펙이 쓰던 레이아웃 그대로**를 유지하고, 새 변형만 쿼리 파라미터로 연다.

---

## Files to Change

| File | Action | Justification |
|---|---|---|
| `src/lib/components/panelModels.ts` | UPDATE | `isProviderConnected` / `visiblePanelProviders` / `primaryCandidate` 순수 헬퍼 추가 |
| `src/lib/components/panelModels.test.ts` | CREATE | 위 헬퍼의 단위 테스트 (현재 타입 전용 파일이라 테스트가 없음) |
| `src/lib/components/ProviderColumn.svelte` | CREATE | 컬럼 1개의 마크업. `UsagePanel`이 800줄 규칙에 안 걸리게 분리 |
| `src/lib/components/ProviderColumn.test.ts` | CREATE | 컬럼 렌더링·접근명·★ 테스트 |
| `src/lib/components/UsagePanel.svelte` | UPDATE | 헤더/본문/푸터 재작성 |
| `src/lib/components/UsagePanel.test.ts` | UPDATE | 탭 기반 단언 → 컬럼 기반으로 |
| `src/lib/components/UsageGauge.svelte` | UPDATE | 헤딩 카피 `{label} window` → `{label}` (좁은 컬럼) |
| `src/lib/components/ProviderTabs.svelte` | **DELETE** | 탭 제거 |
| `src/lib/components/ProviderTabs.test.ts` | **DELETE** | 위와 함께 |
| `src/lib/stores/providers.ts` | UPDATE | `selected` / `selectTab` 제거 |
| `src/lib/stores/providers.test.ts` | UPDATE | `selectTab` 테스트 삭제 |
| `src/App.svelte` | UPDATE | `UsagePanel` prop 배선 변경 |
| `src/App.test.ts` | UPDATE | 탭 클릭에 의존하는 테스트 3건 재작성 |
| `src/lib/api/fixtureGateway.ts` | UPDATE | `?panel=single` 변형 추가 |
| `src/lib/styles/global.css` | UPDATE | `main.panel` 폭 `19.5rem` → `23.75rem` |
| `src/securityConfig.test.ts` | UPDATE | 패널 창 폭 단언 312 → 380 |
| `src-tauri/tauri.conf.json` | UPDATE | 패널 창 `width` 312 → 380 |
| `src-tauri/src/refresh/ipc.rs` | UPDATE | `PANEL_WIDTH_LOGICAL` 312.0 → 380.0 |
| `src-tauri/src/window/tests.rs` | UPDATE | 기하 테스트 4건 리터럴 + 플립 분기 신규 테스트 1건 |
| `tests/e2e/renderer.spec.ts` | UPDATE | 폭·패딩 단언 갱신, 탭 겹침 테스트 삭제, 컬럼 수 스펙 2건 추가 |
| `docs/ui-contract.md` | UPDATE | §5 클릭 패널 계약 개정 |
| `README.md` / `README.en.md` | UPDATE | 76-77행(+ README.md 111행) "탭" 서술 갱신 |

## NOT Building

- **오버레이 변경 일체.** 링 모드, 위성 링, 궤도 워커, 펫 무드는 그대로다. `ring_mode`는 여전히 표시 전용이며 이 계획의 "연결 감지"와 **절대 연결되지 않는다** — 큰 원은 항상 primary이고 자동 강등도 스왑도 없다 (CLAUDE.md 불변식).
- **네이티브 수집/새로고침/알림 라우팅 변경.** 컬럼이 숨겨져도 그 provider의 수집은 계속된다. 숨김은 **표시 판정**일 뿐이다.
- **`primary_provider` 자동 변경.** 미연결 provider가 primary여도 자동으로 옮기지 않는다.
- **설정 화면(`SettingsPanel`) 변경.** 폭이 380px로 넓어지는 것 외 손대지 않는다.
- **`UpdateNotice` 배너 위치/동작 변경** (ui-contract §5.1).
- **히스토리 그래프 / `get_history`** — 이 패널에 아직 붙어 있지 않고 이번에도 붙이지 않는다.
- **패널 높이 정책 변경.** `resize_panel`은 계속 높이만 나른다.
- **애니메이션/트랜지션.** 컬럼이 1↔2로 바뀔 때 모션을 넣지 않는다.

---

## Step-by-Step Tasks

### Task 1: `panelModels.ts` — 표시 판정 순수 함수 (TDD)

- **ACTION**: `src/lib/components/panelModels.test.ts`를 **먼저** 작성해 실패시킨 뒤, `panelModels.ts`에 함수를 추가한다.
- **IMPLEMENT**:
  ```ts
  // src/lib/components/panelModels.ts 하단에 추가
  // (파일 상단 import에 `Provider`가 이미 있다 — type-only import이므로
  //  런타임 값이 필요 없는 이 코드에서는 그대로 쓸 수 있다.)

  /**
   * 패널 컬럼의 고정 순서. primary를 앞으로 끌어오지 않는 이유: `Set as primary`
   * 를 누르는 순간 두 컬럼이 자리를 바꿔 사용자가 방금 읽던 수치가 반대쪽으로
   * 튀어 버린다. ★가 어느 쪽이 primary인지 이미 말해 준다.
   */
  const PANEL_ORDER: readonly Provider[] = ['claude', 'codex'];

  /**
   * provider가 "연결됨"인지. `engine.ts`의 blocking status와 같은 기준이다 —
   * `error`/`offline`은 연결된 provider의 일시적 실패이므로 컬럼을 유지한다.
   * 그러지 않으면 네트워크가 잠깐 끊길 때마다 패널이 1↔2컬럼으로 튄다.
   */
  export const isProviderConnected = (system: SystemState): boolean =>
    system !== 'auth_required' && system !== 'unavailable';

  /**
   * 화면에 그릴 provider들. 연결된 것만 남기되, 하나도 연결되지 않았으면
   * 둘 다 남긴다 — 숨김은 "다른 쪽이 연결돼 있을 때"만 성립하는 판정이고,
   * 전부 숨기면 두 provider의 로그인 안내가 어디에도 없게 된다.
   */
  export function visiblePanelProviders(
    providers: Readonly<Record<Provider, PanelProviderModel>>,
  ): readonly Provider[] {
    const connected = PANEL_ORDER.filter((provider) =>
      isProviderConnected(providers[provider].system),
    );
    return connected.length === 0 ? PANEL_ORDER : connected;
  }

  /**
   * `Set … as primary`가 겨냥할 provider. 보이는 것 중 primary가 아닌 하나이며,
   * 보이는 것이 primary 하나뿐이면 대상이 없다(`null` → 버튼 비활성).
   */
  export function primaryCandidate(
    visible: readonly Provider[],
    primary: Provider,
  ): Provider | null {
    const others = visible.filter((provider) => provider !== primary);
    return others.length === 1 ? others[0] : null;
  }
  ```
- **MIRROR**: `systemGuidance.ts:19-35` (컴포넌트 옆 순수 함수 + JSDoc 근거 주석), `panelModels.ts:4-22` (`readonly` 뷰모델).
- **IMPORTS**: 파일 1-2행의 `import type { Provider } from '../contracts/domain';` 와 `import type { Severity, SystemState } from '../state/engine';` 를 그대로 쓴다. **새 import 불필요.**
- **GOTCHA**:
  - `panelModels.ts`는 `vite.config.ts:37-38`의 커버리지 exclude 목록에 **없다**(`models.ts`와 `contracts/domain.ts`만 제외). 지금까지 타입 전용이라 문제없었지만, 런타임 코드를 넣는 순간 커버리지 집계 대상이 된다 → 테스트 필수.
  - `PANEL_ORDER`를 `export` 하지 말 것. 순서는 내부 구현이고 외부에서 재정렬할 이유가 없다.
  - `visiblePanelProviders`는 `primary`를 받지 않는다. 순서가 고정이므로 필요 없다.
- **VALIDATE**: `pnpm vitest run src/lib/components/panelModels.test.ts`

**테스트 케이스** (`panelModels.test.ts`):
```ts
import { describe, expect, it } from 'vitest';
import {
  isProviderConnected,
  primaryCandidate,
  visiblePanelProviders,
  type PanelProviderModel,
} from './panelModels';
import type { Provider } from '../contracts/domain';
import type { SystemState } from '../state/engine';

const model = (provider: Provider, system: SystemState): PanelProviderModel => ({
  provider,
  system,
  stale: false,
  planType: null,
  session: { usedPercent: null, severity: 'unknown', resetsAt: null },
  weekly: { usedPercent: null, severity: 'unknown', resetsAt: null },
  capturedAt: null,
  source: provider === 'claude' ? 'oauth_api' : 'cli_rpc',
  isCached: false,
});

const pair = (claude: SystemState, codex: SystemState) => ({
  claude: model('claude', claude),
  codex: model('codex', codex),
});
```

| Test | Input | Expected |
|---|---|---|
| 둘 다 active → 둘 다 | `pair('active','active')` | `['claude','codex']` |
| Codex 미설치 → Claude만 | `pair('active','unavailable')` | `['claude']` |
| Claude 미로그인 → Codex만 | `pair('auth_required','active')` | `['codex']` |
| 둘 다 미연결 → 둘 다 (빈 패널 방지) | `pair('auth_required','unavailable')` | `['claude','codex']` |
| `error`는 연결로 취급 | `pair('active','error')` | `['claude','codex']` |
| `offline`은 연결로 취급 | `pair('offline','active')` | `['claude','codex']` |
| `loading`은 연결로 취급 (기동 시 깜빡임 방지) | `pair('loading','loading')` | `['claude','codex']` |
| 순서는 primary와 무관하게 고정 | `pair('active','active')` | 항상 `['claude','codex']` |
| 후보 = 보이는 것 중 비-primary | `['claude','codex']`, `'claude'` | `'codex'` |
| 후보 = 반대 방향도 동일 | `['claude','codex']`, `'codex'` | `'claude'` |
| 보이는 게 primary 하나뿐이면 없음 | `['claude']`, `'claude'` | `null` |
| 보이는 게 비-primary 하나면 그것 | `['claude']`, `'codex'` | `'claude'` |
| `isProviderConnected` 전 상태 표 | 6개 `SystemState` 각각 | `auth_required`/`unavailable`만 `false` |

---

### Task 2: `UsageGauge` 헤딩 카피 축약

- **ACTION**: `src/lib/components/UsageGauge.svelte:33`의 `<span>{label} window</span>`를 `<span>{label}</span>`으로 바꾼다.
- **IMPLEMENT**: 그 한 줄만. `aria-label={`${label} usage`}`(30행, 43행)는 **손대지 않는다**.
- **MIRROR**: 목업 196행 — 컬럼 헤딩은 `5-hour` / `weekly`뿐이고 "window"가 없다.
- **IMPORTS**: 변경 없음.
- **GOTCHA**:
  - `tests/e2e/renderer.spec.ts:235`가 `section[aria-label="Weekly usage"]`로 조회한다. `aria-label`은 그대로이므로 통과한다 — **`aria-label`을 같이 줄이면 e2e가 깨진다.**
  - `UsageGauge.test.ts`는 " window" 문자열을 단언하지 않는다(전체 확인 완료) → 수정 불필요.
  - 380px 2컬럼에서 컬럼 콘텐츠 폭은 약 156px다. `5-hour window`(13px, ≈85px) + `82%`(15px mono, ≈36px)는 줄바꿈 위험이 있고, `5-hour`(≈45px)면 여유가 생긴다.
- **VALIDATE**: `pnpm vitest run src/lib/components/UsageGauge.test.ts`

---

### Task 3: `ProviderColumn.svelte` 신규 (TDD)

- **ACTION**: 컬럼 1개를 렌더하는 컴포넌트를 새로 만든다. 기존 `UsagePanel.svelte:52-83`의 본문 마크업 + `ProviderTabs.svelte`의 ★ 처리를 합친다.
- **IMPLEMENT**:
  ```svelte
  <script lang="ts">
    import UsageGauge from './UsageGauge.svelte';
    import { capturedAgo } from '../format/time';
    import { PROVIDER_NAME } from '../contracts/domain';
    import { systemGuidance } from './systemGuidance';
    import type { PanelProviderModel } from './panelModels';

    let {
      model,
      isPrimary,
      nowMs = Date.now(),
    }: {
      model: PanelProviderModel;
      isPrimary: boolean;
      nowMs?: number;
    } = $props();

    const name = $derived(PROVIDER_NAME[model.provider]);
    const guidance = $derived(systemGuidance(model.system, model.provider));
    const captured = $derived(
      model.capturedAt === null ? null : capturedAgo(model.capturedAt, nowMs),
    );
  </script>

  <section
    class="column"
    data-provider={model.provider}
    aria-label={isPrimary ? `${name} usage (primary)` : `${name} usage`}
  >
    {#if model.system === 'loading'}
      <div class="skeleton" data-testid="usage-skeleton" aria-label="Loading usage">
        Loading…
      </div>
    {:else}
      <div class="provider-heading">
        <strong
          >{name}{#if isPrimary}<span class="primary-star" aria-hidden="true">
              ★</span
            >{/if}</strong
        >
        {#if model.planType}<span class="plan-chip">{model.planType}</span>{/if}
      </div>
      <UsageGauge label="5-hour" window={model.session} stale={model.stale} {nowMs} />
      <UsageGauge label="Weekly" window={model.weekly} stale={model.stale} {nowMs} />
      <small class:stale={model.stale} class="freshness"
        >● {model.stale
          ? 'Stale'
          : 'Fresh'}{#if model.capturedAt && captured}<span
            >&nbsp;· captured <time datetime={model.capturedAt}>{captured}</time></span
          >{/if}</small
      >
    {/if}
    <!-- 상태가 바뀔 때 "다시 선언"이 아니라 "변경 안내"가 되도록 항상 마운트해
         둔다. 내용만 바뀌며, 빈 경우 line box가 없어 높이 0으로 접힌다. -->
    <p class="guidance" role="status">{guidance ?? ''}</p>
  </section>
  ```
  스타일은 `UsagePanel.svelte`에서 **그대로 이동**한다 (`.provider-heading` 162-166, `.plan-chip` 167-174, `.freshness` 175-184, `.skeleton` 185-189, `.guidance` 269-275) + 신규:
  ```css
  .column {
    display: grid;
    align-content: start;
    gap: var(--space-4);
    padding: var(--space-4);
  }
  /* UI-plan 스펙: primary ★는 앰버 고정이며 텍스트 색(near-black)과 독립이다. */
  .primary-star { color: #f59e0b; }
  .guidance { padding: 0; }   /* 컬럼이 이미 패딩을 갖는다 */
  ```
- **MIRROR**: `UsagePanel.svelte:52-83` (본문 마크업 + guidance 주석), `ProviderTabs.svelte:54-58` (★ 색 리터럴 + 주석).
- **IMPORTS**: 위 스크립트 블록 그대로. `SystemBadge`는 이 컴포넌트에서 쓰지 않으므로 **import 하지 말 것** (eslint no-unused).
- **GOTCHA**:
  - `.guidance`의 원래 `padding: 0 var(--space-4)`를 그대로 가져오면 컬럼 패딩과 이중으로 먹어 32px가 된다. `padding: 0`으로 덮어쓴다.
  - `align-content: start`가 없으면 짧은 컬럼(예: 스켈레톤 1개)이 긴 컬럼 높이에 맞춰 세로로 늘어져 게이지가 벌어진다.
  - import 경로에 `.js` 확장자를 붙이지 말 것 — `UsagePanel.svelte`(JSDoc)는 `'../format/time.js'`를 쓰지만 `lang="ts"` 컴포넌트(`UsageGauge.svelte:2`)는 확장자 없이 쓴다.
  - `data-provider` 속성은 e2e에서 컬럼 수를 세는 셀렉터다. 빠뜨리면 Task 10이 실패한다.
- **VALIDATE**: `pnpm vitest run src/lib/components/ProviderColumn.test.ts`

**테스트 케이스** (`ProviderColumn.test.ts`):

| Test | Input | Expected |
|---|---|---|
| primary는 접근명에 `(primary)`가 붙고 ★가 장식으로만 존재 | `isPrimary: true`, provider `claude` | `getByLabelText('Claude usage (primary)')` 존재, `.primary-star`가 `aria-hidden="true"` |
| 비-primary는 ★ 없음 | `isPrimary: false`, provider `codex` | `getByLabelText('Codex usage')` 존재, `.primary-star` 없음 |
| `loading`만 스켈레톤 | `system: 'loading'` | `getByTestId('usage-skeleton')` 존재, 게이지 0개 |
| 비-`loading`은 스켈레톤 없이 게이지 2개 | `system: 'offline'` | 스켈레톤 없음, `getAllByTestId('usage-gauge').length === 2` |
| freshness에 source/cache 문자열 누출 없음 | `stale: true, isCached: true` | `● Stale · captured 2 min ago`, `/oauth_api\|cli_rpc\|cached/` 불일치 |
| 컬럼별 안내문이 자기 provider 이름을 쓴다 | `system: 'auth_required'`, `codex` | `getByRole('status').textContent === 'Sign in to the Codex CLI: codex login'` |
| `active`면 안내문 비어 있음 | `system: 'active'` | `getByRole('status').textContent === ''` |
| `planType` 없으면 chip 미렌더 | `planType: null` | `.plan-chip` 없음 |

---

### Task 4: `UsagePanel.svelte` 재작성

- **ACTION**: 헤더/본문/푸터를 2컬럼 구조로 바꾸고 `ProviderTabs` 의존을 끊는다.
- **IMPLEMENT** (스크립트):
  ```svelte
  <script>
    import ProviderColumn from './ProviderColumn.svelte';
    import { PROVIDER_NAME } from '../contracts/domain';
    import { primaryCandidate, visiblePanelProviders } from './panelModels.js';
    /** @typedef {import('./panelModels').PanelProviderModel} PanelProvider */
    /** @typedef {import('../contracts/domain').Provider} Provider */
    /** @type {{ providers: { claude: PanelProvider; codex: PanelProvider }; primary: Provider; refreshing: Readonly<Record<Provider, boolean>>; nowMs?: number; updateAvailable?: boolean; onRefresh?: (provider: Provider) => void; onPrimary?: (provider: Provider) => void; onSettings?: () => void; onClose?: () => void; onQuit?: () => void }} */
    let {
      providers,
      primary,
      refreshing,
      nowMs = Date.now(),
      updateAvailable = false,
      onRefresh = () => {},
      onPrimary = () => {},
      onSettings = () => {},
      onClose = () => {},
      onQuit = () => {},
    } = $props();

    const visible = $derived(visiblePanelProviders(providers));
    const candidate = $derived(primaryCandidate(visible, primary));
    // 보이는 것 전부를 새로 읽는 하나의 버튼이다. 그중 하나라도 디바운스 중이면
    // 비활성 — 나머지에 요청을 보내 봐야 네이티브 디바운스가 그대로 흘려버린다.
    const refreshBusy = $derived(visible.some((provider) => refreshing[provider]));
    const refreshLabel = $derived(visible.length > 1 ? 'Refresh both' : 'Refresh now');
    const primaryLabel = $derived(
      candidate === null
        ? 'Set as primary'
        : `Set ${PROVIDER_NAME[candidate]} as primary`,
    );
  </script>
  ```
- **IMPLEMENT** (마크업 — 변경 지점만):
  ```svelte
  <section class="usage-panel" aria-label="Usage panel">
    <button class="close-panel" type="button" aria-label="Close usage panel"
      title="Close usage panel" onclick={() => onClose()}>×</button>
    <header><h2>Usage</h2></header>
    <div class="body" style:--panel-columns={visible.length}>
      {#each visible as provider (provider)}
        <ProviderColumn
          model={providers[provider]}
          isPrimary={provider === primary}
          {nowMs}
        />
      {/each}
    </div>
    <footer>
      <div class="footer-row">
        <button class="primary-action" disabled={refreshBusy}
          onclick={() => { for (const provider of visible) onRefresh(provider); }}
          >{refreshLabel}</button>
        <button class="secondary-action" disabled={candidate === null}
          onclick={() => { if (candidate !== null) onPrimary(candidate); }}
          >{primaryLabel}</button>
      </div>
      <div class="footer-row"><!-- Settings / Quit: 84-116행 그대로 --></div>
    </footer>
  </section>
  ```
- **IMPLEMENT** (스타일 변경분):
  ```css
  header {
    padding: var(--space-3) var(--space-4);
    border-bottom: 1px solid var(--color-border);
  }
  header h2 {
    margin: 0;
    font-size: 0.8125rem;
    font-weight: 700;
  }
  /* 패딩은 컬럼이 갖는다. 그래야 구분선이 본문 전체 높이를 가른다. */
  .body {
    display: grid;
    grid-template-columns: repeat(var(--panel-columns, 1), minmax(0, 1fr));
    padding: 0;
  }
  .body :global(.column + .column) {
    border-left: 1px solid var(--color-border);
  }
  ```
  `.provider-heading` / `.plan-chip` / `.freshness` / `.skeleton` / `.guidance` 규칙은 `ProviderColumn.svelte`로 **이동했으므로 여기서 삭제**한다. `.close-panel`(129-153), `footer`(190-195), `.footer-row`(196-200), `button`(201-211), `.primary-action`(215-223), `.secondary-action`(224-232), `.ghost-action`(233-239), `.settings-*`(240-260), `.ghost-action.quit`(265-268) 규칙은 **전부 그대로 둔다**.
- **MIRROR**: `UsagePanel.svelte:84-117` (푸터 구조는 손대지 않는다), `:127-129` (`.close-panel`의 "흐름 밖" 주석 유지).
- **IMPORTS**: 위 스크립트 블록 그대로. `ProviderTabs` / `UsageGauge` / `capturedAgo` / `systemGuidance` import는 **삭제**한다 (컬럼으로 이동).
- **GOTCHA**:
  - `h2`가 이제 보인다. `class="visually-hidden"`을 **제거**하되 `<section aria-label="Usage panel">`은 유지 — e2e 3건과 `App.test.ts:579`가 이 이름으로 패널을 찾는다.
  - `.body`의 자식은 `ProviderColumn`이라 부모 컴포넌트의 스코프드 CSS가 닿지 않는다. 구분선 규칙에 **`:global()`이 필수**다.
  - `--panel-columns`를 `style:` 디렉티브로 넘기는 이유: `grid-template-columns`를 클래스로 분기하면 컬럼 수가 3이 될 때 또 클래스를 늘려야 한다. `repeat(var(...))`는 값 하나만 바꾸면 된다.
  - `refreshing` prop이 **boolean → Record로 타입이 바뀐다.** 호출부(`App.svelte:782`)를 같은 커밋에서 고치지 않으면 `svelte-check`가 잡는다.
  - `{#each visible as provider (provider)}` — **키를 반드시 붙일 것.** 키가 없으면 1↔2컬럼 전환 시 Svelte가 DOM 노드를 재사용해 `claude` 컬럼 자리에 `codex` 데이터가 잠깐 그려질 수 있다.
  - `onRefresh`를 루프로 호출하는 건 의도적이다. prop 시그니처 `(provider) => void`가 유지되므로 `App.svelte`의 `providersStore.requestRefresh` 배선은 그대로다.
- **VALIDATE**: `pnpm vitest run src/lib/components/UsagePanel.test.ts`

**테스트 케이스** (`UsagePanel.test.ts` 재작성 — 기존 `provider()` 픽스처 헬퍼는 유지하고, `bothProviders`가 provider별 `system`을 각각 받도록 확장):

| Test | Input | Expected |
|---|---|---|
| 둘 다 연결이면 컬럼 2개 | 둘 다 `active` | `container.querySelectorAll('[data-provider]').length === 2` |
| Codex 미설치면 Claude 컬럼만 | claude `active`, codex `unavailable` | 컬럼 1개, `[data-provider="claude"]`만, `queryByLabelText('Codex usage')` 없음 |
| Claude 미로그인이면 Codex 컬럼만 | claude `auth_required`, codex `active` | 컬럼 1개, `[data-provider="codex"]` |
| 둘 다 미연결이면 둘 다 남는다 | `auth_required` / `unavailable` | 컬럼 2개, `getAllByRole('status')`에 두 안내문이 모두 존재 |
| `error`는 컬럼을 지우지 않는다 | claude `active`, codex `error` | 컬럼 2개 |
| 2컬럼이면 라벨이 `Refresh both` | 둘 다 `active` | `getByRole('button', { name: 'Refresh both' })` |
| 1컬럼이면 `Refresh now` | codex `unavailable` | `getByRole('button', { name: 'Refresh now' })` |
| Refresh는 보이는 것 전부를 호출 | 둘 다 `active` | `onRefresh` 2회, 인자 `'claude'`·`'codex'` |
| Refresh는 1컬럼일 때 1회만 | codex `unavailable` | `onRefresh` 1회, `'claude'` |
| 하나라도 디바운스 중이면 비활성 | `refreshing: { claude: false, codex: true }` | 버튼 `disabled === true` |
| primary 버튼이 상대를 겨냥 | primary `claude`, 둘 다 보임 | 라벨 `Set Codex as primary`, 클릭 시 `onPrimary('codex')` |
| 보이는 게 primary 하나면 비활성 | primary `claude`, codex `unavailable` | 라벨 `Set as primary`, `disabled === true`, 클릭해도 `onPrimary` 미호출 |
| primary가 숨겨지고 상대만 보이면 활성 | primary `codex`, codex `unavailable` | 라벨 `Set Claude as primary`, `disabled === false` |
| 탭이 더 이상 없다 | 둘 다 `active` | `queryAllByRole('tab').length === 0` |
| 헤더 제목이 보인다 | 둘 다 `active` | `getByRole('heading', { name: 'Usage' })` |
| ✕ / Quit / Settings 콜백 | 기존 130-174행 테스트 | 동작 변경 없음 (props에서 `selected`/`onSelect`만 제거) |
| 업데이트 점 | 기존 176-226행 테스트 | 동작 변경 없음 |

---

### Task 5: `providers` 스토어에서 `selected` 제거

- **ACTION**: `src/lib/stores/providers.ts`에서 `selected` 필드와 `selectTab` 메서드를 제거하고, `providers.test.ts:32-38`의 테스트를 삭제한다.
- **IMPLEMENT**:
  - `ProvidersStoreState`(18-23행)에서 `readonly selected: Provider;` 삭제
  - `initial`(28-33행)에서 `selected: 'claude',` 삭제
  - `selectTab(selected)`(182-184행) 메서드 삭제
  - `Provider` import는 다른 시그니처가 계속 쓰므로 유지
- **MIRROR**: 기존 스토어 구조 그대로. `refreshing` 관련 코드(36-58행, 185-197행)는 손대지 않는다.
- **IMPORTS**: 변경 없음.
- **GOTCHA**:
  - `providers.test.ts:32-38` (`'switches the viewed tab locally without refreshing'`)을 **삭제**해야 한다. 이 테스트가 `refresh` 미호출도 함께 단언하지만, 같은 보장을 40-50행의 `requestRefresh` 테스트가 이미 제공한다.
  - `selected`를 읽는 곳은 `App.svelte:780`, `:782`뿐이다 (grep 확인 완료). Task 6에서 함께 제거된다.
- **VALIDATE**: `pnpm vitest run src/lib/stores/providers.test.ts && pnpm check`

---

### Task 6: `App.svelte` 배선 + `App.test.ts` 갱신

- **ACTION**: `UsagePanel` prop 3개를 바꾸고, 탭 클릭에 의존하던 테스트를 재작성한다.
- **IMPLEMENT** (`App.svelte:777-793` 교체):
  ```svelte
  <UsagePanel
    updateAvailable={availableUpdateVersion !== null}
    providers={panelProviders}
    primary={$settingsStore.primaryProvider}
    refreshing={$providersStore.refreshing}
    {nowMs}
    onClose={() => void gateway.hidePanel()}
    onQuit={() => void gateway.quit()}
    onSettings={() => (showSettings = true)}
    onRefresh={(provider) => providersStore.requestRefresh(provider)}
    onPrimary={(provider) =>
      void changeSettings({ ...$settingsStore, primaryProvider: provider })}
  />
  ```
  → `selected={...}`(780) 와 `onSelect={...}`(787-789) 제거, `refreshing`(782)을 record 전체로 교체.
- **MIRROR**: `App.svelte:776-793` 기존 배선 형태.
- **IMPORTS**: 변경 없음.
- **GOTCHA**:
  - `App.test.ts` 2건이 `findByRole('tab', …)`에 의존한다. 재작성 방향:
    - `'changes primary only when Set as primary is clicked'` (612-641): 617행 탭 클릭 삭제. 픽스처 기본값이 둘 다 `active`이므로 버튼 이름은 `Set Codex as primary`가 된다. `updateSettings({ primaryProvider: 'codex', selectedPetId: 'tabby' })` 단언은 유지. **클릭 후 단언이 달라진다** — 기존 640행 `setPrimary.disabled === true` 대신, 버튼 라벨이 `Set Claude as primary`로 뒤집히고 여전히 활성인지 확인한다 (후보가 상대편으로 옮겨갔을 뿐 사라지지 않는다).
    - `'restores the previous primary when saving a primary change fails'` (687-707): 695행 탭 클릭 삭제. 실패 후 `Settings could not be saved`가 뜨고 버튼 라벨이 `Set Codex as primary`로 되돌아오는지 단언.
    - 619, 639, 702-705행의 `getByRole('tab', { name: 'Claude (primary)' })` / `aria-selected` 단언은 `getByLabelText('Claude usage (primary)')`로 치환.
  - **`role="status"` 요소 수가 늘어난다.** `App.svelte`는 이미 설정 저장 실패·capability 안내로 `role="status"` 문단을 여럿 렌더한다(795-809행). `App.test.ts`에서 `getByRole('status')`를 쓰는 곳이 있으면 `getAllByRole` 또는 `findByText`로 바꿔야 한다 — `pnpm test` 실패 메시지가 정확한 위치를 알려 준다.
  - `App.test.ts:608`의 `expect(resizePanel).toHaveBeenLastCalledWith(385)`는 **높이**라 폭 변경과 무관하다. 그 테스트가 `getBoundingClientRect`를 스텁하는지 먼저 확인할 것 — 스텁이면 레이아웃 변경과 무관하게 통과한다.
- **VALIDATE**: `pnpm vitest run src/App.test.ts`

---

### Task 7: `ProviderTabs` 삭제

- **ACTION**: `src/lib/components/ProviderTabs.svelte`와 `src/lib/components/ProviderTabs.test.ts`를 삭제한다.
- **GOTCHA**: Task 3에서 ★ 색 리터럴 `#f59e0b`와 그 위 주석("UI-plan spec: the primary-provider star is amber…", 54-58행)이 `ProviderColumn.svelte`로 **옮겨졌는지 먼저 확인**하고 삭제할 것. 이 파일이 그 색 선택의 유일한 근거 기록이다.
- **VALIDATE**: `pnpm lint && pnpm check` — 남은 import가 있으면 여기서 잡힌다.

---

### Task 8: 패널 폭 312px → 380px

- **ACTION**: 폭 상수를 5곳에서 바꾸고, Rust 기하 테스트를 갱신 + 신규 1건 추가한다.
- **IMPLEMENT**:
  1. `src-tauri/tauri.conf.json:38` — `"width": 312` → `"width": 380`
  2. `src-tauri/src/refresh/ipc.rs:30` — `const PANEL_WIDTH_LOGICAL: f64 = 312.0;` → `380.0`
  3. `src/lib/styles/global.css:25` — `width: min(19.5rem, 100%);` → `width: min(23.75rem, 100%);` (380 ÷ 16)
  4. `src/securityConfig.test.ts:38` — `width: 312` → `width: 380`
  5. `src-tauri/src/window/tests.rs` — 118-236행의 `width: 312.0` 4곳 → `380.0`, 118-119행 doc 주석의 `312x520` → `380x520`
  6. `src-tauri/src/window/tests.rs`에 플립 분기 신규 테스트 추가:
     ```rust
     /// 넓어진 패널은 이전 폭에서는 오른쪽에 들어가던 자리에서 왼쪽으로 뒤집힌다.
     /// 펫 우측 여백이 312는 수용하고 380은 수용하지 못하는 구간(right + 312 <=
     /// 1920 < right + 380)을 고른다 — 폭 변경이 실제로 배치 분기를 옮겼음을
     /// 고정한다.
     #[test]
     fn panel_anchor_flips_left_when_widened_panel_no_longer_fits_right() {
         let work_area = Rect { x: 0.0, y: 0.0, width: 1920.0, height: 1032.0 };
         let pet = Rect { x: 1300.0, y: 400.0, width: 240.0, height: 240.0 };
         let panel = Size { width: 380.0, height: 520.0 };

         // right = 1300 + 240 + 12 = 1552; 1552 + 380 = 1932 > 1920 → 왼쪽으로.
         assert_eq!(
             anchor_panel(pet, panel, work_area, 12.0),
             Point { x: 908.0, y: 260.0 }
         );
     }
     ```
- **MIRROR**: `window/tests.rs:118-143` (doc comment + `assert_eq!(anchored, Point{..})` 스타일).
- **IMPORTS**: 없음 — `Rect`/`Size`/`Point`/`anchor_panel`은 이미 이 테스트 모듈이 쓰고 있다.
- **GOTCHA**:
  - **기존 4건의 기대 좌표는 바뀌지 않는다.** `anchor_panel`(`window/mod.rs:474-483`) 정의로 직접 계산해 확인했다:

    | 테스트 | 계산 | 기대값 |
    |---|---|---|
    | `..._above_taskbar_for_bottom_pet` | right=352, 352+380=732≤1920 → x=352; y=660→clamp 512 | `{352, 512}` (동일) |
    | `..._centers_vertically_on_pet...` | right=1052, 1052+380=1432≤1920 → x=1052; y=260 | `{1052, 260}` (동일) |
    | `..._clamps_to_work_area_top...` | right=292, 292+380=672≤1440 → x=292; y=−110→clamp 25 | `{292, 25}` (동일) |
    | `..._pins_oversized_panel...` | right=352 → x=352; max_y=max(1032−1200,0)=0 → y=0 | `{352, 0}` (동일) |

    즉 리터럴만 바꾸면 그대로 통과한다. **그래서 신규 플립 테스트가 필요하다** — 그것 없이는 폭 변경이 배치에 아무 영향도 주지 않은 것처럼 보인다.
  - `PANEL_WIDTH_LOGICAL`은 **논리 픽셀**이다. `position_panel`은 `outer_size()`(물리 픽셀)를 읽으므로 HiDPI에서 두 값이 다르다 — 혼동 금지 (`panel-anchor-work-area.plan.md:617`의 기존 주의사항).
  - `tauri.conf.json`의 `"height": 520`은 **건드리지 않는다**. 높이는 `resize_panel`이 계속 관리한다.
  - `main.panel`은 `width: min(23.75rem, 100%)`이라 네이티브 창이 380px일 때 100%로 붙는다. rem 값과 창 폭이 어긋나면 셸이 창보다 좁아져 투명 여백이 생긴다 — 두 값을 반드시 함께 바꿀 것.
- **VALIDATE**:
  ```bash
  cargo test --manifest-path src-tauri/Cargo.toml window::tests
  pnpm vitest run src/securityConfig.test.ts
  ```

---

### Task 9: `fixtureGateway`에 단일-provider 변형 추가

- **ACTION**: `?panel=single` 쿼리 파라미터로 Codex를 미설치 상태로 만드는 변형을 추가한다.
- **IMPLEMENT** (`src/lib/api/fixtureGateway.ts`):
  ```ts
  /**
   * 미연결(=CLI 미설치) provider. 스냅샷 없이 `not_installed`만 실으면
   * 렌더러가 `unavailable`로 파생하고, 패널이 그 컬럼을 감춘다.
   */
  const disconnected = (name: 'claude' | 'codex'): ProviderBackendStateWire => ({
    provider: name,
    revision: 1,
    snapshot: null,
    failure_class: null,
    unavailable_reason: 'not_installed',
    expired: false,
    reset_pending: false,
  });

  getProviderStates: async () => ({
    claude: provider('claude'),
    // `?panel=single`로만 열리는 변형. 기본값은 두 provider 모두 연결이라
    // 기존 스펙이 쓰던 2컬럼 레이아웃이 그대로 유지된다.
    codex:
      new URLSearchParams(window.location.search).get('panel') === 'single'
        ? disconnected('codex')
        : provider('codex'),
  }),
  ```
- **MIRROR**: `fixtureGateway.ts:50-59` (`?ring=double` 변형의 주석 형식과 "기본값은 기존 스펙 유지" 원칙).
- **IMPORTS**: 변경 없음 — `ProviderBackendStateWire`는 1행에서 이미 import 중이다.
- **GOTCHA**:
  - `fixtureGateway.ts`는 커버리지 exclude 대상(`vite.config.ts:35`)이라 단위 테스트가 필요 없다. e2e로만 검증된다.
  - `provider()` 헬퍼는 그대로 두고 `disconnected()`를 별도로 만든다 — 기존 함수에 분기를 넣으면 스냅샷 형태가 두 갈래가 되어 읽기 어려워진다.
- **VALIDATE**: Task 10과 함께 e2e로.

---

### Task 10: e2e 스펙 갱신

- **ACTION**: `tests/e2e/renderer.spec.ts`의 패널 스펙 3건을 고치고, 컬럼 수 스펙 2건을 새로 넣는다.
- **IMPLEMENT**:
  1. **`'uses the unified 312px vibrancy panel shell'`** (248-310행) → 제목을 `'uses the unified 380px vibrancy panel shell'`로, 측정 블록에 `const column = panel?.querySelector<HTMLElement>('.usage-panel .column');` + `columnPadding` 수집을 추가하고 기대값을:
     ```ts
     expect(layout).toMatchObject({
       platform: 'linux',
       outerWidth: 380,
       minHeight: '0px',
       borderRadius: '14px',
       backdropFilter: 'blur(20px)',
       shellPadding: ['0px', '0px', '0px', '0px'],
       // 헤더는 이제 눈에 보이는 제목을 담고 아래 테두리를 갖는다.
       headerPadding: ['12px', '16px', '12px', '16px'],
       // 본문 패딩은 컬럼으로 내려갔다 — 구분선이 본문 전체 높이를 가르려면
       // 본문 자신은 여백을 가질 수 없다.
       bodyPadding: ['0px', '0px', '0px', '0px'],
       columnPadding: ['16px', '16px', '16px', '16px'],
       footerPadding: ['12px', '16px', '14px', '16px'],
     });
     ```
  2. **`'keeps the close control overlap over the second tab within contract'`** (354-399행) → **테스트와 그 위 설명 주석(354-358행)을 함께 삭제**. 탭이 없어져 겹침 대상 자체가 사라졌다. 삭제 사유를 커밋 메시지에 남긴다.
  3. **`'hydrates provider panel and reaches settings via the footer button'`** (212-246행) → 238행의 탭 클릭 삭제. 239-242행을 다음으로 교체:
     ```ts
     const primaryButton = await $('button=Set Codex as primary');
     expect(await primaryButton.isEnabled()).toBe(true);
     await primaryButton.click();
     // 후보가 사라지는 게 아니라 상대편으로 뒤집힌다.
     await browser.waitUntil(async () =>
       $('button=Set Claude as primary').isExisting(),
     );
     ```
  4. **신규**: 2컬럼 스펙
     ```ts
     it('renders one column per connected provider', async () => {
       await browser.url('/?window=panel&fixture=e2e');
       await expect($('section[aria-label="Usage panel"]')).toBeDisplayed();
       await expect($$('[data-provider]')).toBeElementsArrayOfSize(2);
       await expect($('[data-provider="claude"]')).toBeDisplayed();
       await expect($('[data-provider="codex"]')).toBeDisplayed();
       // 나란히 놓였는지 — 세로로 쌓였다면 grid가 무너진 것이다.
       const sideBySide = await browser.execute(() => {
         const boxes = [...document.querySelectorAll('[data-provider]')].map((el) =>
           el.getBoundingClientRect(),
         );
         return boxes[0].right <= boxes[1].left + 1 && boxes[0].top === boxes[1].top;
       });
       expect(sideBySide).toBe(true);
     });
     ```
  5. **신규**: 1컬럼 스펙
     ```ts
     it('collapses to a single column when only one provider is connected', async () => {
       await browser.url('/?window=panel&fixture=e2e&panel=single');
       await expect($('section[aria-label="Usage panel"]')).toBeDisplayed();
       await expect($$('[data-provider]')).toBeElementsArrayOfSize(1);
       await expect($('[data-provider="claude"]')).toBeDisplayed();
       // 남은 컬럼이 셸 안쪽 폭을 다 쓴다 — 반쪽만 차지하면 grid가 접히지 않은 것.
       const fills = await browser.execute(() => {
         const panel = document.querySelector<HTMLElement>('main.panel');
         const column = document.querySelector<HTMLElement>('[data-provider]');
         if (!panel || !column) throw new Error('single column missing');
         return Math.abs(column.getBoundingClientRect().width - panel.clientWidth) <= 1;
       });
       expect(fills).toBe(true);
       await expect($('button=Refresh now')).toBeDisplayed();
     });
     ```
- **MIRROR**: `renderer.spec.ts:248-310` (browser.execute로 계산해 `toMatchObject`), `:344-350` (기하 허용오차 주석).
- **IMPORTS**: 없음 — `$`, `$$`, `browser`, `expect`는 WebdriverIO 전역이다.
- **GOTCHA**:
  - e2e는 **렌더러 전용** 실행이다: `pnpm test:e2e:renderer` (`wdio.browser.conf.ts`). 네이티브 바이너리를 빌드하지 않으므로 `tauri.conf.json` 폭 변경은 여기 반영되지 않는다 — `outerWidth: 380`은 `global.css`의 rem 값이 만든다.
  - `getBoundingClientRect()`는 분수 폭을 돌려주고 `clientWidth`는 정수다. 스케일 디스플레이에서 정확 비교가 깨지므로 **반드시 `<= 1` 허용오차**를 쓴다 (기존 344-350행과 같은 이유).
  - `outerHeight < 520` 단언(309행)은 유지한다. 2컬럼은 세로 콘텐츠를 줄이므로 여유가 더 생긴다.
- **VALIDATE**: `pnpm test:e2e:renderer`

---

### Task 11: 문서 갱신

- **ACTION**: `docs/ui-contract.md` §5와 두 README를 실제 구현에 맞춘다.
- **IMPLEMENT**:
  1. **`docs/ui-contract.md` 326-366행** — 다이어그램을 2컬럼으로 교체하고 규칙을 개정:
     - 헤더 줄: `[Claude ★] [Codex] 탭` → `Usage` 제목 + ✕
     - 본문: 2컬럼 다이어그램 (위 "After — 2개 연결" 그림 사용)
     - 352행 규칙("두 provider 모두 항상 탭으로 열람 가능")을 다음으로 교체:
       > - 컬럼 수는 **연결된 provider 수에서 파생**된다. `auth_required` / `unavailable`만 미연결로 보며, `error` / `offline` / `loading`은 연결된 provider의 상태이므로 컬럼을 유지한다. 한쪽의 실패가 다른 쪽 표시를 막지 않는다는 provider 독립 원칙은 그대로다.
       > - 하나도 연결되지 않았으면 **둘 다** 표시한다. 숨김은 "다른 쪽이 연결돼 있을 때"만 성립하는 판정이며, 전부 숨기면 로그인 안내가 화면에서 사라진다.
       > - 컬럼 순서는 `claude`, `codex`로 **고정**이다. primary를 앞으로 끌어오지 않는다 — `주 provider로 설정`을 누른 순간 두 컬럼이 자리를 바꿔 방금 읽던 수치가 반대편으로 튄다. ★가 primary를 이미 표시한다.
       > - 폭은 **380px 고정**이고 높이만 콘텐츠를 따른다. 컬럼이 1개로 접혀도 창 폭은 그대로이며 남은 컬럼이 폭 전체를 차지한다.
       > - `지금 새로고침`은 **보이는 provider 전부**를 대상으로 하며, 그중 하나라도 디바운스 중이면 비활성이다. 라벨은 컬럼 2개면 `Refresh both`, 1개면 `Refresh now`.
       > - `주 provider로 설정`은 **보이는 컬럼 중 primary가 아닌 하나**를 겨냥한다(`Set Codex as primary`). 대상이 없으면 비활성이다.
       > - freshness 줄과 상태 안내문은 **컬럼마다** 있다.
     - 361행(`✕`가 두 번째 탭을 덮는 계약) **문단 전체 삭제** — 탭이 사라져 근거가 소멸했다. 360행(`✕`가 흐름 밖 절대 위치)은 유지.
  2. **`README.md` 76-77행** →
     > 5. 패널에는 연결된 provider가 나란히 표시됩니다. 각 컬럼에서 5시간 사용량과 주간 사용량을 확인할 수 있습니다.
     > 6. provider를 하나만 연결했다면 그 하나만 표시됩니다 — 전환 조작이 필요 없습니다.
  3. **`README.md` 111행** — "탭 패널로 보여주며" → "나란히 보여주며"
  4. **`README.en.md` 76-77행** →
     > 5. The panel shows every connected provider side by side. Each column carries a 5-hour bar and a weekly bar.
     > 6. If only one provider is connected, only that column is shown — there is nothing to switch between.
- **MIRROR**: `docs/ui-contract.md` 기존 서술 톤(한국어 규칙 목록 + 근거 문장).
- **IMPORTS**: 해당 없음.
- **GOTCHA**: `docs/ui-contract.md` §4.4/§4.5(오버레이 링)는 **건드리지 않는다**. 이 계획은 패널만 다룬다.
- **VALIDATE**: `pnpm lint` (prettier가 마크다운도 검사한다)

---

## Testing Strategy

### Unit Tests

Task별 표를 참조. 요약:

| 파일 | 신규/변경 테스트 수 | 핵심 커버리지 |
|---|---|---|
| `panelModels.test.ts` | 13 (신규) | 연결 판정 6상태 + 표시 조합 8가지 + primary 후보 4가지 |
| `ProviderColumn.test.ts` | 8 (신규) | 접근명, ★, 스켈레톤, freshness 프라이버시, 컬럼별 안내문 |
| `UsagePanel.test.ts` | 15 (재작성 + 기존 유지) | 컬럼 수 자동 파생, 푸터 라벨/대상/비활성 |
| `providers.test.ts` | −1 (삭제) | — |
| `App.test.ts` | 2 (재작성) | primary 전환 경로가 탭 없이 동작 |
| `securityConfig.test.ts` | 1 (값 변경) | 창 폭 380 |
| `window/tests.rs` | 4 (값) + 1 (신규) | 넓어진 패널의 플립 분기 |

### Edge Cases Checklist

- [ ] 둘 다 연결 → 2컬럼
- [ ] 하나만 연결 → 1컬럼, 폭 전체 사용
- [ ] 둘 다 미연결 → 2컬럼 (빈 패널 방지)
- [ ] 기동 중(`loading`) → 2컬럼 + 스켈레톤 2개, 이후 접힘
- [ ] `error` / `offline` → 컬럼 유지 (연결로 판정)
- [ ] primary가 숨겨진 provider → `Set {보이는 것} as primary` 활성
- [ ] 보이는 게 primary 하나뿐 → 버튼 비활성, 클릭해도 콜백 없음
- [ ] 새로고침 디바운스 중 하나라도 있으면 비활성
- [ ] `planType === null` → chip 미렌더
- [ ] `capturedAt === null` → freshness에 captured 절 없음
- [ ] 패널 높이 변화 시 `resize_panel` 재보고 — 기존 `ResizeObserver` 경로가 처리하며 **새 코드가 없다**. 회귀 확인만

`Concurrent access` / `Network failure` / `Permission denied`는 이 계획의 범위(순수 표시 계층)에 해당 사항 없음.

---

## Validation Commands

### Static Analysis
```bash
pnpm check
```
EXPECT: 타입 오류 0건. `refreshing` prop 타입 변경, `selected` 제거, `ProviderTabs` 삭제가 여기서 전부 걸린다.

```bash
pnpm lint
```
EXPECT: eslint + prettier 통과. 미사용 import(`ProviderTabs`, `capturedAgo`, `systemGuidance`, `UsageGauge`)가 여기서 잡힌다.

### Unit Tests
```bash
pnpm vitest run src/lib/components/panelModels.test.ts src/lib/components/ProviderColumn.test.ts src/lib/components/UsagePanel.test.ts
```
EXPECT: 전건 통과

### Rust
```bash
cargo test --manifest-path src-tauri/Cargo.toml window::tests
cargo test --manifest-path src-tauri/Cargo.toml --all-features
```
EXPECT: 기하 테스트 5건(기존 4 + 신규 1) 포함 전건 통과

### Full Test Suite
```bash
pnpm test:ci
```
EXPECT: svelte-check + eslint + prettier + vitest 커버리지(≥80% branches/functions/lines/statements) + vite build 전부 통과

### E2E
```bash
pnpm test:e2e:renderer
```
EXPECT: 패널 스펙 4건(폭/셸, ✕ 레이어, 2컬럼, 1컬럼) + 나머지 회귀 통과

### Manual Validation
```bash
pnpm tauri dev
```
- [ ] 펫 더블클릭 → 패널이 380px 폭으로 열린다
- [ ] Claude/Codex 둘 다 연결된 환경: 두 컬럼이 나란히, 사이에 1px 구분선, primary에 ★
- [ ] Codex CLI를 PATH에서 잠시 치운 뒤 재기동: Codex 컬럼이 사라지고 Claude가 폭 전체, 푸터가 `Refresh now` + `Set as primary`(비활성)
- [ ] `Set Codex as primary` 클릭 → ★가 옮겨가고 버튼이 `Set Claude as primary`로 뒤집힌다. **오버레이의 큰 원이 그 provider로 바뀌는지도 확인** (primary 설정의 기존 효과)
- [ ] `Refresh both` 클릭 → 두 컬럼 모두 갱신, 버튼이 잠시 비활성
- [ ] `Settings` → `← Back` 왕복 시 패널 폭이 380px로 유지되고 높이만 바뀐다
- [ ] 다크/라이트 테마 모두에서 구분선과 ★가 보인다
- [ ] 컬럼이 1↔2로 바뀔 때 창이 화면 밖으로 밀리지 않는다 (앵커/클램프)
- [ ] `resets in 3d 21h 0m` 같은 최장 라벨이 컬럼 안에서 줄바꿈 없이 들어간다

---

## Acceptance Criteria

- [ ] 두 provider 모두 연결 시 사용량 그래프가 2개, 나란히 표시된다
- [ ] 한쪽만 연결 시 그래프가 1개만 표시되며 **사용자 조작 없이 자동으로** 그렇게 된다
- [ ] `error` / `offline` provider는 컬럼이 유지된다
- [ ] 둘 다 미연결이면 패널이 비지 않는다
- [ ] 탭 스트립이 코드베이스에서 사라졌다 (`ProviderTabs.*` 파일 없음, `role="tab"` 0개)
- [ ] 패널 창 폭이 380px이며 렌더러 셸 폭과 일치한다
- [ ] 모든 검증 명령 통과, 커버리지 ≥80% 유지
- [ ] `docs/ui-contract.md` §5가 구현과 일치한다

## Completion Checklist

- [ ] 순수 판정 로직이 `.svelte`가 아니라 `panelModels.ts`에 있고 단위 테스트가 붙어 있다
- [ ] 색·간격이 전부 CSS 토큰이다 (예외: 옮겨 온 ★ `#f59e0b` 1건, 주석으로 근거 유지)
- [ ] freshness 문자열에 `oauth_api` / `cli_rpc` / `cached`가 새지 않는다 (프라이버시 계약)
- [ ] `ring_mode` / 오버레이 / 수집 / 알림 경로에 **변경이 없다** (`git diff --stat`로 확인)
- [ ] 파일당 800줄 이하, 함수당 50줄 이하
- [ ] 삭제한 e2e 테스트의 사유가 커밋 메시지에 남아 있다
- [ ] README 2종과 ui-contract가 갱신되었다

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| 미연결 provider를 완전히 숨겨, 사용자가 두 번째 provider를 연결할 수 있다는 사실을 패널에서 알 수 없게 됨 | **높음** (설계상 확실) | 중간 | 사용자 확정 결정. 완화책: 둘 다 미연결이면 둘 다 보이므로 최초 온보딩 경로는 살아 있다. 후속으로 Settings에 provider 연결 안내를 넣는 것을 **별도 계획**으로 남긴다 |
| 기동 시 2컬럼 → 1컬럼으로 접히며 패널 크기가 튄다 | 중간 | 낮음 | `loading`을 연결로 취급하는 선택의 대가. 반대(=`loading`을 미연결로) 선택은 1→2로 튀므로 더 나쁘다. `PANEL_LAYOUT_GRACE`(150ms)가 최초 노출까지의 측정을 이미 지연시킨다 |
| 380px 2컬럼에서 `resets in 3d 21h 0m` 같은 긴 라벨이 줄바꿈 | 중간 | 낮음 | 컬럼 콘텐츠 폭 ~156px, 해당 라벨은 mono 11px로 ≈100px. 수동 검증 항목으로 고정 |
| 패널 폭 변경이 앵커/클램프 회귀를 부름 | 낮음 | 높음 | 계산상 기존 4건 기대값 불변임을 확인했고, 플립 분기 신규 테스트로 실제 이동을 고정한다 |
| `App.test.ts`에서 `getByRole('status')` 다중 매치로 예기치 못한 실패 | 중간 | 낮음 | 컬럼당 안내문이 생기며 확실히 늘어난다. `pnpm test` 실패 메시지가 위치를 특정해 주므로 `getAllByRole`로 전환 |
| 커버리지 80% 게이트 하락 | 낮음 | 중간 | 신규 코드 대부분이 순수 함수(테스트 13건)와 컴포넌트(테스트 8건). `ProviderTabs` 삭제로 분모도 줄어든다 |

## Notes

- **이 계획의 성격**: 표시 계층 재배치다. 네이티브에서 바뀌는 것은 상수 하나(`PANEL_WIDTH_LOGICAL`)와 창 설정 하나뿐이며, **수집·새로고침·알림·오버레이 경로는 한 줄도 바뀌지 않는다.** 검증의 절반은 "새 UI가 맞는가"이고 나머지 절반은 "나머지가 그대로인가"다 — 후자는 `cargo test --all-features` 전건 통과와 `git diff --stat` 검토로 확인한다.

- **v2 문서의 자기 모순 처리**: `docs/UI-plan-2.0/CacheBite v2.dc.html`은 §5를 "unchanged / for reference"라고 쓰면서 정작 현행 구현(탭)과 다른 2컬럼을 그려 놓았다. 사용자 요청이 "이 문서의 클릭 패널 디자인을 적용"이므로 **그림 쪽을 명세로 채택**했고, 그림이 v1 계약과 충돌하는 지점(안내문 삭제, ★ 버튼 라벨)은 v1 계약을 우선했다. 그 판단 근거는 "목업 대비 의도적 편차" 표에 남겼다.

- **선행 계획의 예고**: `overlay-double-ring.plan.md:308`이 "클릭 패널 변경 일체 … **다음 계획**으로 넘긴다"고 명시했고, 같은 파일 836-846행에 델타 표를 남겼다. 이 계획이 그 후속이며 표의 7개 항목을 전부 소화한다.

- **`selected` 제거가 안전한 이유**: 이 필드는 스토어에 있지만 오직 `App.svelte:780`, `:782`만 읽는다(grep 확인). 네이티브로 나가지도, 설정에 저장되지도 않는 순수 렌더러 세션 상태다. 제거해도 마이그레이션이 필요 없다.

- **컬럼 순서를 primary 기준으로 정렬하지 않은 이유**를 계약문에 남기는 것이 중요하다. "primary가 왼쪽"이 직관적으로 들리므로, 근거 없이 두면 다음 사람이 되돌리기 쉽다. 근거: `Set … as primary` 클릭이 두 컬럼의 자리를 맞바꿔, 사용자가 방금 읽던 수치가 반대편으로 튄다.

- **`Refresh both`의 디바운스 정책**: 하나라도 진행 중이면 버튼 전체를 비활성으로 둔다. "안 바쁜 쪽만 보내기"는 버튼 하나가 상황에 따라 다른 개수의 요청을 내보내게 만들어 예측 가능성을 잃고, 네이티브 디바운스가 어차피 중복 요청을 흘려버리므로 얻는 것도 없다.
