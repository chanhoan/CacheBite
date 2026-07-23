# Plan: CacheBite 안정화 · 디자인 버그 수정 · 코드 품질 향상

## Summary

CacheBite 전체 코드베이스(Rust 7,302줄 / 렌더러 6,035줄)를 정적 탐색한 결과, **계약(`docs/ui-contract.md`)에 정의되어 있으나 배선되지 않은 기능 7건**, **디자인/표현 결함 6건**, **코드 품질 이슈 8건**을 확인했다. 핵심 패턴은 "순수 함수와 상수는 구현·테스트되어 있으나 `App.svelte`에서 호출되지 않는다"는 것이다 — `SNAPSHOT_TTL_MS`, `expiresAt`, `setRefreshing`, `setFullscreen`, `tickResetTimers`, `quit`, `defaultSize`가 모두 정의만 되고 참조가 0건이다.

## User Story

CacheBite 사용자로서, 나는 **오래된 사용량이 최신인 것처럼 표시되지 않고, 말풍선이 화면에 영구히 남지 않으며, 로그인이 필요할 때 그 사실을 패널에서 알 수 있고, 앱을 정상적으로 종료할 수 있기를** 원한다. 그래야 CacheBite가 상시 떠 있는 오버레이로서 신뢰할 수 있다.

## Problem → Solution

| 현재 상태 | 목표 상태 |
| --- | --- |
| 30분 지난 스냅샷이 영구히 `active`로 표시 | TTL 만료 시 `error`/`offline`으로 강등 (계약 §1.2) |
| 말풍선이 클릭할 때까지 사라지지 않음 | 8초 후 자동 소멸 (계약 §7.1-4) |
| 앱 종료 경로 없음 (강제 종료만 가능) | 패널 푸터에서 정상 종료 (계약 §5) |
| `auth_required`에도 안내 없이 "Unknown" 게이지 | 상태별 복구 안내 문구 표시 (계약 §4.2) |
| `resets in 2026-07-20T09:00:00Z` | `resets in 1h 12m` / `resets Mon 09:00` |
| 링 수치를 스크린리더가 못 읽음 | `role="img"`로 접근성 트리 노출 |

## Metadata

- **Complexity**: Large
- **Source PRD**: N/A (자유 서술 요청)
- **PRD Phase**: standalone
- **Estimated Files**: 16개 수정, 2개 신규
- **탐색 근거**: 이 계획의 모든 파일:줄 참조는 2026-07-22 작업 트리(`f5c56d7`) 기준 실측이다.

---

## 검증 환경 제약 (먼저 읽을 것)

| 항목 | 상태 | 근거 |
| --- | --- | --- |
| `pnpm install` | **실패** | WSL DrvFs(`/mnt/c`)에서 `ERR_PNPM_EACCES: rename` — `--node-linker=hoisted --package-import-method=copy`로도 재현. 661/656 패키지에서 중단 |
| `cargo` | **미설치** | `which cargo` → not found |
| 결과 | 로컬에서 `pnpm test:ci` / `cargo test` **실행 불가** | 이번 탐색은 100% 정적 분석 |

**Task 0을 먼저 수행하지 않으면 이 계획의 어떤 VALIDATE 단계도 실행할 수 없다.** 이전 리뷰(`docs/code-review/local-changes-2026-07-17.md:114`)에서도 Rust 검증이 같은 이유로 Blocked였고, `ui-design-pass-2026-07-17.md:66`에서는 "비용" 사유로 skip됐다 — 즉 이 저장소는 **로컬 검증이 반복적으로 건너뛰어져 왔다**. 이것이 아래 결함들이 누적된 구조적 원인이다.

---

## Mandatory Reading

| Priority | File | Lines | Why |
| --- | --- | --- | --- |
| P0 | `docs/ui-contract.md` | §1.2, §4.2, §5, §7.1 | 위반된 계약 조항의 원문 |
| P0 | `src/App.svelte` | 173-234, 324-394, 396-436 | 모든 미배선 결함의 수렴점 |
| P0 | `src/lib/state/engine.ts` | 1-123 | 만료/심각도 도출 순수 함수 |
| P1 | `src-tauri/src/refresh/actor.rs` | 287-387 | `expired`/`reset_pending` 발신부 |
| P1 | `src/lib/interaction/bubblePolicy.ts` | 29-61 | `expiresAt` 생성부 |
| P1 | `src/lib/components/UsagePanel.svelte` | 18-67 | 상태 안내·캡처시각 표시부 |
| P2 | `src/lib/api/gateway.ts` | 12-87 | 와이어 DTO 계약 |
| P2 | `src-tauri/src/window/mod.rs` | 100-300 | 죽은 추상화 판정 대상 |

## External Documentation

| Topic | Source | Key Takeaway |
| --- | --- | --- |
| Svelte 5 `$derived` 반응성 | Svelte 5 룬 문서 | `Date.now()`는 반응 의존성이 아니다 — 시간 기반 파생값은 `$state` 타이머 신호가 필요 |
| `Intl.RelativeTimeFormat` | MDN | 상대 시각 휴먼화의 표준 API, 의존성 추가 불필요 |
| SVG 접근성 | WAI-ARIA Graphics | `<path>`는 암묵 role이 없어 `aria-label`이 무시됨 — `role="img"` 필수 |

> 외부 라이브러리 추가는 없다. 모든 수정이 기존 내부 패턴과 웹 표준 API로 해결된다.

---

## Patterns to Mirror

### 순수 정책 함수 (모든 신규 로직이 따를 형태)

```ts
// SOURCE: src/lib/state/engine.ts:67-75
export function severity(usedPercent: number | null): Severity {
  if (usedPercent === null) return 'unknown';
  if (!Number.isFinite(usedPercent)) return 'unknown';
  const percent = Math.min(100, Math.max(0, usedPercent));
  if (percent >= 100) return 'exhausted';
  if (percent >= 90) return 'critical';
  if (percent >= 70) return 'warn';
  return 'ok';
}
```

명명 상수 → 널/비유한값 조기 반환 → 클램프 → 경계 비교. **모든 신규 순수 함수는 `src/lib/state/` 또는 `src/lib/interaction/`에 놓고, Svelte 컴포넌트에는 로직을 두지 않는다.**

### 스토어 액션 (이벤트 반환 규약)

```ts
// SOURCE: src/lib/stores/providers.ts:97-124
markResetPending(provider: Provider, revision: number): readonly UsageTransitionEvent[] {
  let events: readonly UsageTransitionEvent[] = [];
  update((state) => {
    const current = state[provider];
    if (revision < current.revision) return state;
    /* ... */
    return { ...state, [provider]: { ...current, revision, resetWindows } };
  });
  return events;
}
```

`update()` 안에서 불변 갱신 → 지역 `events`에 수집 → 반환. **리비전 가드가 항상 먼저 온다.**

### 시스템 상태 → 표현 매핑 (배지 패턴)

```svelte
<!-- SOURCE: src/lib/components/SystemBadge.svelte:5-19 -->
/** @type {Record<import('./models').BadgeState, { color: string; label: string }>} */
const badges = {
  auth_required: { color: 'var(--badge-lock)', label: 'Authentication required' },
  unavailable: { color: 'var(--badge-slash)', label: 'Provider unavailable' },
  /* ... */
};
const badge = $derived(badges[system]);
```

상태 키 → 룩업 레코드 → `$derived`. **Task 6의 안내 문구도 정확히 이 형태를 따른다.**

### 심각도 색상 (CSS 셀렉터 분기, JS 색 계산 금지)

```svelte
<!-- SOURCE: src/lib/components/UsageGauge.svelte:85-99 -->
.gauge-fill[data-severity='ok'] { background: var(--sev-ok); }
.gauge-fill[data-severity='warn'] { background: var(--sev-warn); }
```

`data-severity` 속성 + CSS 셀렉터. **컴포넌트에 hex 하드코딩 금지, `tokens.css`가 단일 원본.**

### Rust 액터 상태 전이

```rust
// SOURCE: src-tauri/src/refresh/actor.rs:370-377
() = sleep_optional(ttl_deadline), if ttl_deadline.is_some() => {
    ttl_deadline = None;
    if !state.expired {
        state.revision = state.revision.saturating_add(1);
        state.expired = true;
        state_tx.send_replace(state.clone());
    }
}
```

`tokio::select!` 분기 → 리비전 증가 → `send_replace`. **상태 변경 시 리비전은 항상 증가한다.**

### 테스트 구조

```ts
// SOURCE: src/lib/state/domain.test.ts (구조 요약)
const NOW = Date.parse('2026-07-17T00:00:00Z');
it('설명적인 동작 문장', () => {
  const state = createProviderState('claude');
  const result = applyProviderUpdate(state, snapshot, NOW);
  expect(result.events).toEqual([...]);
});
```

고정 `NOW` 상수 + 순수 함수 직접 호출. **Svelte 컴포넌트가 아닌 정책 함수를 테스트한다.**

---

## Files to Change

| File | Action | Justification |
| --- | --- | --- |
| `src/lib/state/engine.ts` | UPDATE | TTL 만료 판정 추가 (A1) |
| `src/App.svelte` | UPDATE | `expired` 소비, 말풍선 타이머, 새로고침 플래그, 시계 신호 (A1·A2·A4·A7) |
| `src/lib/stores/providers.ts` | UPDATE | `applyExpiry` 액션, `setRefreshing` 배선 (A1·A4) |
| `src/lib/interaction/bubblePolicy.ts` | UPDATE | `expireBubble` 순수 함수 (A2) |
| `src/lib/stores/interaction.ts` | UPDATE | `expireBubble` 액션 (A2) |
| `src/lib/api/gateway.ts` | UPDATE | `quit()` 메서드 추가 (A3) |
| `src/lib/api/fixtureGateway.ts` | UPDATE | `quit()` 미러 (A3) |
| `src/lib/components/UsagePanel.svelte` | UPDATE | 상태 안내, 종료 버튼, 시각 휴먼화 (A3·B1·B2) |
| `src/lib/components/UsageGauge.svelte` | UPDATE | 리셋 시각 휴먼화 (B1) |
| `src/lib/format/time.ts` | **CREATE** | 상대/절대 시각 포맷 순수 함수 (B1) |
| `src/lib/components/systemGuidance.ts` | **CREATE** | 시스템 상태 → 안내 문구 룩업 (B2) |
| `src/lib/components/SplitUsageRing.svelte` | UPDATE | 접근성 라벨 합성 (B5) |
| `src/lib/components/HistoryGraph.svelte` | UPDATE | 포인트 `aria-hidden`, `aria-controls` (B5) |
| `src/lib/components/PetOverlay.svelte` | UPDATE | `defaultSize` 반영 (B3) |
| `src/lib/components/models.ts` | UPDATE | `size` 필드 (B3) |
| `src/lib/components/SystemBadge.svelte` | UPDATE | 아이콘 색 토큰화 (B6) |
| `src/lib/styles/global.css` | UPDATE | macOS 다크 유리 효과 (B4) |
| `src/lib/styles/tokens.css` | UPDATE | `--badge-icon` 토큰 (B6) |
| `src/lib/components/SettingsPanel.svelte` | UPDATE | `any` 제거 (C1) |
| `src/lib/state/domain.test.ts` | UPDATE | **잘못된 기대값 수정** (A1) |
| `src-tauri/src/lib.rs` | UPDATE | `HistoryRepository` 중복 제거 (C4) |

## NOT Building

명시적으로 이번 범위 밖:

- **트레이 아이콘** — A3는 패널 종료 버튼으로 해결한다. 트레이는 별도 기능이며 플랫폼별 검증이 필요하다.
- **`window/mod.rs` 죽은 추상화 삭제** — Task 15에서 **인벤토리만 작성**하고 삭제는 하지 않는다. 사용자 승인이 필요한 별도 결정이다.
- **컴포넌트 8개의 `lang="ts"` 일괄 전환** — C1은 `any` 제거만 한다. 전면 전환은 별도 리팩터링.
- **`always_on_top` 실제 검증 구현** — CLAUDE.md 불변식("미검증 능력은 unavailable")을 그대로 유지한다.
- **상태별 GIF 에셋** — ui-contract §9에서 v1.1+ 범위로 정의됨.
- **Codex 실행파일 핸들 피닝** — `codex.rs:72-74`에 "승인된 개선 범위 밖"으로 명시적 문서화됨.
- **새 npm/cargo 의존성 추가.**

---

## Step-by-Step Tasks

### Phase 0 — 검증 가능성 확보 (선행 필수)

#### Task 0: 검증 환경 복구
- **ACTION**: 로컬에서 `pnpm test:ci`와 `cargo test`가 실행되도록 만든다.
- **IMPLEMENT**: 아래 중 하나를 선택.
  - (권장) 저장소를 WSL 네이티브 경로(`~/dev/CacheBite`)로 클론하고 거기서 작업. DrvFs rename 문제가 근본 해소된다.
  - 또는 Rust 툴체인 설치: `rustup toolchain install`(`rust-toolchain.toml` 핀 사용) + Linux Tauri 전제조건(WebKitGTK 4.1, AppIndicator, librsvg, patchelf).
- **GOTCHA**: `/mnt/c`에서는 `--node-linker=hoisted --package-import-method=copy`로도 실패한다(실측). Windows 측 파일 잠금이 원인이므로 pnpm 옵션으로는 우회할 수 없다.
- **VALIDATE**: `corepack pnpm test:ci` 및 `cargo test --manifest-path src-tauri/Cargo.toml --all-features`가 **끝까지 실행**될 것. 통과/실패 여부는 그 다음 문제다.

> **이 태스크를 건너뛰면 Task 1~16의 VALIDATE가 전부 무의미해진다.**

---

### Phase 1 — 계약 위반 수정 (HIGH)

#### Task 1: A1이 고착시킨 잘못된 테스트 수정 (RED 만들기)
- **ACTION**: `src/lib/state/domain.test.ts:167`의 `'keeps an expired snapshot visible as stale without a recorded failure'`를 계약에 맞게 고친다.
- **IMPLEMENT**: 이 테스트는 **버그를 사양으로 기록**하고 있다. ui-contract §1.2는 "만료: 나이 > `SNAPSHOT_TTL_MIN` — 스냅샷을 버리고 마지막 실패 원인에 따라 `error`/`offline`으로 강등"을 요구한다. 테스트 이름과 기대값을 `drops an expired snapshot and degrades to error when no network failure was recorded`로 교체.
- **MIRROR**: `domain.test.ts`의 기존 고정 `NOW` 상수 패턴
- **GOTCHA**: 이 태스크는 **의도적으로 테스트를 실패시킨다**. Task 2가 GREEN을 만든다. 순서를 바꾸면 TDD 사이클이 성립하지 않는다.
- **VALIDATE**: `pnpm vitest run src/lib/state/domain.test.ts` — 해당 케이스가 **실패**할 것.

#### Task 2: 스냅샷 TTL 만료 배선 (A1 — GREEN)
- **ACTION**: 백엔드가 이미 보내는 `expired` 플래그를 렌더러가 소비하게 한다.
- **IMPLEMENT**:
  1. `engine.ts`에 `StatusUpdate` 변형 추가: `{ kind: 'snapshot_expired'; revision: number }`.
  2. `applyProviderUpdate`에서 이 변형 처리 — 스냅샷을 `null`로 버리고, `lastFailure === 'network'`이면 `offline`, 아니면 `error`로 강등. `auth_required`/`unavailable`은 유지(계약 §2.2 전환표 `active` 행의 `SNAPSHOT_EXPIRED` 열 그대로).
  3. `providers.ts`에 `applyExpiry(provider, revision)` 액션 추가 — `markResetPending`의 리비전 가드 패턴을 그대로 따른다.
  4. `App.svelte:173` `statusUpdate()` **맨 앞**에 `if (wire.expired) return providersStore.applyExpiry(wire.provider, wire.revision);` 추가.
- **MIRROR**: `providers.ts:97-124`(스토어 액션), `engine.ts:160-171`(status 도출 삼항)
- **IMPORTS**: 없음 (기존 모듈 내 확장)
- **GOTCHA**:
  - `reset_pending` 검사보다 **먼저** 와야 한다. 만료된 스냅샷은 리셋 대기 대상이 아니다.
  - `SNAPSHOT_TTL_MS`(`engine.ts:8`)는 **백엔드가 권위**다(`actor.rs:143-149` `ttl: 30분`). 렌더러에서 나이를 재계산해 이중 판정하지 말 것 — 시계 드리프트로 두 계층이 어긋난다. 상수는 문서/테스트 용도로 유지하되 "백엔드 TTL과 동일해야 함"을 주석으로 명시.
- **VALIDATE**: `pnpm vitest run src/lib/state/domain.test.ts` — Task 1의 케이스가 통과하고, 신규 케이스 3건(만료→error, 만료→offline, 만료 중 auth_required 유지)도 통과.

#### Task 3: 말풍선 자동 소멸 (A2)
- **ACTION**: `expiresAt`(`bubblePolicy.ts:15,58`)이 실제로 말풍선을 제거하게 한다.
- **IMPLEMENT**:
  1. `bubblePolicy.ts`에 순수 함수 추가:
     ```ts
     export function expireBubble(state: BubblePolicyState, nowMs: number): BubblePolicyState {
       if (state.bubble === null || nowMs < state.bubble.expiresAt) return state;
       return { ...state, bubble: null };
     }
     ```
     **동일 참조 반환**으로 불필요한 스토어 알림을 막는다(`reduceBubble:48-51`의 기존 패턴과 동일).
  2. `interaction.ts`에 `expireBubble(nowMs)` 액션 추가.
  3. `App.svelte`: 말풍선이 생길 때 `setTimeout`을 걸고 소멸/교체 시 정리. `$effect`로 `$interactionStore.bubblePolicy.bubble`을 관찰하는 방식이 `PetAnimation.svelte:12-21`의 기존 타이머 패턴과 일치한다.
- **MIRROR**: `PetAnimation.svelte:12-21` (`$effect` + 타이머 + cleanup 반환)
- **IMPORTS**: `expireBubble`을 `bubblePolicy`에서 `interaction.ts`로
- **GOTCHA**:
  - 타이머는 overlay 창에서만 필요하다(`windowLabel === 'overlay'`). 패널에는 말풍선이 없다.
  - `dismissBubble()`(수동 클릭)과 타이머가 경합할 수 있다. `expireBubble`이 `nowMs < expiresAt`에서 no-op이므로 안전하지만, cleanup에서 `clearTimeout`을 반드시 호출할 것.
- **VALIDATE**: `pnpm vitest run src/lib/interaction/bubblePolicy.test.ts` — 만료 전 no-op(동일 참조), 만료 후 `bubble === null`. `App.test.ts`에 vitest fake timer로 8초 경과 후 말풍선 DOM 제거 검증 추가.

#### Task 4: 앱 종료 경로 (A3)
- **ACTION**: 등록되어 있으나 호출되지 않는 `quit` IPC를 UI에 연결한다.
- **IMPLEMENT**:
  1. `gateway.ts` `AppGateway`에 `quit(): Promise<void>` 추가, `tauriGateway`에 `quit: () => invokeNative('quit')`.
  2. `fixtureGateway.ts`에 `quit: async () => undefined` 미러.
  3. `UsagePanel.svelte` 푸터에 종료 버튼 추가 (ui-contract §5 `[설정] [종료]`).
  4. `App.svelte`에서 `onQuit={() => void gateway.quit()}` 전달.
- **MIRROR**: `gateway.ts:232` `showPanel: () => invokeNative('show_panel')`
- **IMPORTS**: 없음
- **GOTCHA**:
  - `quit`은 **panel 창에서만 인가**된다(`window/mod.rs:83-95` — `overlay` 목록에 `Quit` 없음). overlay에서 호출하면 `IpcError::Forbidden`. 버튼은 반드시 `UsagePanel`(panel 창)에만 둘 것.
  - `fixtureGateway`에 미러하지 않으면 렌더러 E2E(`wdio.browser.conf.ts`)가 타입 오류로 깨진다.
- **VALIDATE**: `pnpm check` 통과(AppGateway 구현체 2개 모두 만족). `UsagePanel.test.ts`에 종료 버튼 클릭 → `onQuit` 호출 검증 추가.

---

### Phase 2 — 디자인 버그 수정

#### Task 5: 시각 표기 휴먼화 (B1)
- **ACTION**: 원시 ISO 문자열 노출을 제거한다.
- **IMPLEMENT**: `src/lib/format/time.ts` 신규 생성.
  ```ts
  export function relativeFromNow(isoTimestamp: string, nowMs: number): string | null
  export function absoluteShort(isoTimestamp: string, nowMs: number): string | null
  export function capturedAgo(isoTimestamp: string, nowMs: number): string | null
  ```
  - `relativeFromNow`: 5시간 창용 → `"1h 12m"`, `"12m"`, 음수면 `"now"`.
  - `absoluteShort`: 주간 창용 → `Intl.DateTimeFormat`으로 `"Mon 09:00"`.
  - `capturedAgo`: 패널 freshness 줄용 → `"2 min ago"`.
  - **모두 파싱 실패 시 `null` 반환** (`Number.isNaN(Date.parse(...))` 검사).
- **적용**:
  - `UsageGauge.svelte:35-37` → `label === 'Weekly' ? \`resets ${absoluteShort(...)}\` : \`resets in ${relativeFromNow(...)}\``, `null`이면 `<time>` 자체를 렌더하지 않음.
  - `UsagePanel.svelte:47-51` → `captured {capturedAgo(...)}`.
- **MIRROR**: `engine.ts:67-75` (널 조기 반환 + 순수 함수)
- **IMPORTS**: 없음 — `Intl`은 브라우저 내장. **의존성 추가 금지.**
- **GOTCHA**:
  - `nowMs`를 **인자로 받는다**. 함수 내부에서 `Date.now()`를 부르면 테스트가 불안정해지고 Svelte 반응성이 깨진다(Task 8과 직결).
  - `datetime` 속성에는 **원본 ISO를 유지**한다 — 기계 판독 값이므로 휴먼화하면 안 된다.
  - jsdom의 `Intl` 타임존은 CI와 로컬이 다를 수 있다. 테스트는 `absoluteShort`에 UTC 고정 옵션을 넘기거나 상대 시각만 단언할 것.
  - `UsageGauge.svelte`는 현재 `lang="ts"`가 없다. TS 헬퍼를 import하려면 `lang="ts"`로 전환하거나 JSDoc으로 타입을 붙여야 한다 — **전자를 권장**(C1의 부분 해소).
- **VALIDATE**: `pnpm vitest run src/lib/format/time.test.ts` — 경계값(0분, 59분, 60분, 음수, 잘못된 ISO). `UsageGauge.test.ts:30`의 기존 케이스가 더 이상 원시 ISO를 기대하지 않도록 갱신.

#### Task 6: 패널 시스템 상태 안내 (B2)
- **ACTION**: `auth_required`/`unavailable`/`error`/`offline`에서 복구 안내를 표시한다.
- **IMPLEMENT**: `src/lib/components/systemGuidance.ts` 신규 생성 — provider별 안내 룩업.
  ```ts
  export function systemGuidance(system: SystemState, provider: Provider): string | null
  ```
  ui-contract §4.2 표를 그대로 옮긴다: `auth_required` → `"Sign in to the Claude CLI: claude login"` / `"Sign in to the Codex CLI: codex login"`, `unavailable` → `"The Codex CLI is not installed"`, `error` → `"Could not fetch usage. Retrying shortly."`, `offline` → `"Cannot reach the network"`. `active`/`loading` → `null`.
  `UsagePanel.svelte`에서 `{#if guidance}<p class="guidance" role="status">{guidance}</p>{/if}`.
- **MIRROR**: `SystemBadge.svelte:5-19` (상태 키 → 룩업 레코드 → `$derived`)
- **IMPORTS**: `SystemState`(`state/engine`), `Provider`(`contracts/domain`)
- **GOTCHA**:
  - 기존 UI 문자열은 **영어**다(`SystemBadge.svelte:7-18`, `UsagePanel.svelte`). 계약 문서만 한국어다. **UI 문자열은 영어를 유지**해 일관성을 지킬 것.
  - `role="status"`는 라이브 리전이다. 탭 전환마다 재선언되지 않도록 조건부 렌더 대신 내용만 바꾸는 편이 나은지 검토.
- **VALIDATE**: `UsagePanel.test.ts`에 상태 4종 × 안내 문구 존재 검증 추가.

#### Task 7: 접근성 결함 (B5)
- **ACTION**: 스크린리더가 링 수치를 읽고, 히스토리 그래프에서 폭주하지 않게 한다.
- **IMPLEMENT**:
  1. `SplitUsageRing.svelte:24-40`: `<path>`의 `aria-label`은 무시된다(암묵 role 없음). `<svg>`의 `aria-label`에 두 창 수치를 합성한다 — `"5-hour 68%, Weekly 31%"`. path 2개를 개별 노출하는 것보다 한 줄이 낫다.
  2. `HistoryGraph.svelte:90-99`: `<circle role="img" aria-label>` 최대 240개 → `aria-hidden="true"`로 변경. `<svg role="img" aria-label>`(76행)이 이미 그래프 전체를 대표하며, `<desc>`(78-80행)로 요약이 제공된다.
  3. `HistoryGraph.svelte:55-72`: `role="tab"`에 `aria-controls`를 추가하고 그래프 컨테이너에 `role="tabpanel"`+`id` 부여. 현재는 tablist/tab만 있고 tabpanel이 없어 ARIA 패턴이 불완전하다.
- **MIRROR**: `UsageGauge.svelte:20-28` (`role="progressbar"` + `aria-valuenow`)
- **GOTCHA**: `SplitUsageRing.test.ts`가 없다. `PetOverlay.test.ts`가 링을 간접 검증하므로 거기 단언이 깨지는지 확인할 것.
- **VALIDATE**: `pnpm vitest run src/lib/components/` 전체 통과. `PetOverlay.test.ts`에 합성 aria-label 검증 추가.

#### Task 8: 시간 경과 반영 (A7)
- **ACTION**: 스냅샷 갱신이 없어도 fresh→stale 전환과 상대 시각이 갱신되게 한다.
- **IMPLEMENT**: `App.svelte`에 `let nowMs = $state(Date.now())`를 두고 `$effect`에서 60초 간격 `setInterval`로 갱신. `panelProviders`/`primaryPresentation`/`primaryUi`(396-404행)의 `Date.now()`를 `nowMs`로 교체.
- **MIRROR**: `PetAnimation.svelte:12-21` (`$effect` + 타이머 + cleanup)
- **IMPORTS**: 없음
- **GOTCHA**:
  - **근본 원인**: `$derived` 안의 `Date.now()`는 반응 의존성이 아니다. 다른 `$state`가 바뀔 때만 우연히 재계산된다.
  - 60초 간격은 stale 경계(20분)와 상대 시각 표시("2 min ago") 양쪽에 충분하다. 더 짧게 하면 오버레이가 상시 재렌더되어 배터리를 먹는다.
  - `consume()` 안의 `Date.now()`(App.svelte:182,193,221)는 **그대로 둔다** — 이벤트 발생 시각이므로 실시간 값이 맞다.
- **VALIDATE**: `App.test.ts`에 fake timer로 21분 경과 후 stale 표시 전환 검증 추가.

#### Task 9: 새로고침 진행 표시 (A4)
- **ACTION**: `setRefreshing`(`providers.ts:131`, 호출처 0건)을 배선한다.
- **IMPLEMENT**: `createProvidersStore`의 `requestRefresh`에서 `setRefreshing(provider, true)`, 해당 provider의 다음 상태 이벤트 수신 시(`consume`) `false`.
- **MIRROR**: `providers.ts:125-136` (기존 액션 형태)
- **GOTCHA**: `refresh_provider` IPC는 **디바운스만 트리거**하고 즉시 반환한다(`actor.rs:355` — `debounce_deadline` 설정 후 종료). Promise resolve가 수집 완료를 의미하지 않는다. **상태 이벤트 기반으로 해제**해야 한다. 수집이 영영 오지 않는 경우를 대비해 타임아웃 상한(예: 30초)을 둘 것.
- **VALIDATE**: `providers.test.ts`에 refreshing 토글 검증. `UsagePanel.test.ts`에서 `refreshing=true`일 때 버튼 `disabled` 확인.

#### Task 10: macOS 다크 유리 효과 (B4)
- **ACTION**: `global.css:53-57`이 macOS 유리 효과를 덮어쓰는 문제를 고친다.
- **IMPLEMENT**: 다크 모드 블록에서 `main.panel[data-platform]` 일괄 덮어쓰기를 플랫폼별로 분리. macOS는 `rgb(28 31 36 / 92%)` 같은 반투명 다크 배경으로 `backdrop-filter`를 살린다. macOS accent도 정의한다(이전 리뷰 L2 미해결 — `ui-design-pass-2026-07-17.md:50`).
- **MIRROR**: `global.css:39-51` (플랫폼별 accent 재정의 패턴)
- **GOTCHA**: `backdrop-filter`는 Tauri의 투명 창에서 플랫폼별로 다르게 동작한다. 실제 macOS 확인 전까지는 **불투명 폴백이 보기 흉하지 않은지**를 기준으로 삼을 것.
- **VALIDATE**: 시각 확인 필요 — `pnpm tauri dev` 후 macOS 라이트/다크 양쪽. macOS 미보유 시 이 태스크를 **보류**하고 `docs/`에 미검증으로 기록.

#### Task 11: 매니페스트 `defaultSize` 반영 (B3)
- **ACTION**: 검증만 되고 버려지는 `defaultSize`를 렌더링에 쓴다.
- **IMPLEMENT**: `PetOverlay.svelte:31`의 `width: 10rem` 하드코딩을 모델에서 받은 크기로 교체. `PetOverlayViewModel`(`models.ts`)에 `size` 필드 추가, `App.svelte:418-436`에서 `petPackage.manifest.defaultSize` 전달.
- **MIRROR**: `App.svelte:418-436` (overlayModel 조립부)
- **GOTCHA**: overlay 창 크기는 `tauri.conf.json:25-26`에서 240×240 고정이다. 매니페스트가 더 큰 값을 선언하면 잘린다. `defaultSize`를 창 크기 내로 클램프하거나, 이 태스크의 범위를 "창 크기에 맞춘 비율 적용"으로 한정할 것.
- **VALIDATE**: `PetOverlay.test.ts`에 크기 반영 검증.

#### Task 12: 배지 아이콘 토큰화 (B6)
- **ACTION**: `SystemBadge.svelte:81` `color: #fff` 하드코딩 제거.
- **IMPLEMENT**: `tokens.css`에 `--badge-icon: #fff` 추가(다크 모드 값 포함), 컴포넌트에서 `var(--badge-icon)` 참조.
- **MIRROR**: `tokens.css:16-20` (기존 `--badge-*` 토큰 그룹)
- **GOTCHA**: `UsagePanel.svelte:128` `.primary-action { color: #fff }`도 같은 위반이다. 함께 처리할 것.
- **VALIDATE**: `pnpm lint` 통과, 시각 회귀 없음.

---

### Phase 3 — 코드 품질

#### Task 13: `any` 제거 (C1)
- **ACTION**: `SettingsPanel.svelte:2`의 `onChange?: (settings: any) => void`.
- **IMPLEMENT**: `SettingsStoreState`(`presentation.ts:5-12`)를 참조해 `onChange?: (settings: SettingsStoreState) => void`로 교체.
- **MIRROR**: `HistoryGraph.svelte:12-20` (`lang="ts"` + 명시 prop 타입)
- **GOTCHA**: JSDoc에서 타입을 참조하려면 `import('../state/presentation').SettingsStoreState` 형태를 쓰거나 `lang="ts"`로 전환해야 한다.
- **VALIDATE**: `pnpm check` 0 errors, `pnpm lint` 통과.

#### Task 14: 이벤트 타입 캐스팅 제거 (C3)
- **ACTION**: `providers.ts:45`의 `as readonly UsageTransitionEvent[]` 강제 캐스팅.
- **IMPLEMENT**: `engine.ts:47-53` `DomainEvent`의 `severity`를 `Exclude<Severity, 'ok' | 'unknown'>`로 좁힌다. `applyProviderUpdate:151`은 `rank[after] > rank[before] && before !== 'unknown'` 조건이므로 `after`가 `ok`/`unknown`일 수 없다 — 타입 가드를 추가해 캐스팅 없이 증명한다.
- **MIRROR**: `engine.ts:106-108` (기존 타입 술어 `(value): value is Exclude<Severity,'unknown'>`)
- **GOTCHA**: 타입을 좁히면 `domain.test.ts`의 일부 단언이 타입 오류가 날 수 있다.
- **VALIDATE**: `pnpm check` 0 errors, `domain.test.ts` 통과.

#### Task 15: 죽은 코드 인벤토리 작성 (C2·C5·C7·A5·A6 — **삭제 없음**)
- **ACTION**: 미사용 추상화를 문서화만 한다.
- **IMPLEMENT**: `docs/dead-code-inventory.md` 생성. 확인된 항목:
  - `src-tauri/src/window/mod.rs`: `PlatformWindowAdapter`, `AutostartAdapter`, `UnsupportedAutostart`, `WindowCommand`, `RuntimeState`, `apply_fullscreen`, `synchronize_fullscreen`, `recover_position`, `panel_position`, `set_autostart`, `logical_to_physical`, `physical_to_logical`, `PlatformCapabilities::linux_wayland` — 프로덕션 호출처 0건, `window/tests.rs`에서만 사용. `lib.rs`가 실제 쓰는 건 `clamp_window`/`anchor_panel`/`platform_os`/`foreground_window_is_fullscreen`뿐.
  - `manifest.ts:1-12` `PET_STATES`의 `ok`/`warn`/`critical`/`exhausted` — `resolver.ts:11-17` `RequestedAnimationKey`에 없어 절대 요청되지 않음.
  - `ipc.rs:175-177` `always_on_top` — 항상 `Unavailable`이고 UI 표시 경로 없음.
  - `engine.ts:181` `tickResetTimers` — 백엔드 `reset_pending`이 대체, 테스트에서만 사용(A5).
  - `interaction.ts:26` `setFullscreen` — 호출처 0건(A6). 창이 숨겨지므로 실질 영향은 작으나 계약 §7.1-5는 미작동.
- **GOTCHA**: **삭제하지 말 것.** 각 항목에 "왜 남아있는가"(설계 의도 / 미래 계획 / 진짜 죽은 코드)를 사용자와 확인한 뒤에 별도 태스크로 처리한다. CLAUDE.md 가이드라인: "unrelated dead code는 언급하되 삭제하지 않는다."
- **VALIDATE**: 문서 존재. 코드 변경 0.

#### Task 16: 자잘한 정합성 (C4·C6)
- **ACTION**: 중복/불일치 정리.
- **IMPLEMENT**:
  - `lib.rs:85`와 `lib.rs:101`이 같은 경로로 `HistoryRepository`를 두 번 만든다. 85행의 `history`를 `Arc`로 공유하거나 101행을 제거.
  - `App.svelte:81` `selectedPetId: 'idle'` → `'cat'`로 교체. Rust 기본값(`settings.rs:47`)과 일치시킨다. 현재는 `getSettings()` 실패 시 존재하지 않는 `'idle'` 패키지를 로드해 반드시 `petPackageError`가 된다.
- **MIRROR**: `lib.rs:84-86` (Arc 공유 패턴)
- **GOTCHA**: `app.manage()`는 타입당 하나만 등록된다. 두 번째 `manage`가 첫 번째를 대체하는지 확인 후 제거할 것.
- **VALIDATE**: `cargo test --manifest-path src-tauri/Cargo.toml`, `pnpm test`.

---

## Testing Strategy

### 신규 단위 테스트

| Test | Input | Expected Output | Edge Case? |
| --- | --- | --- | --- |
| TTL 만료 → error | `expired:true`, `lastFailure:'parse'` | `system==='error'`, `snapshot===null` | — |
| TTL 만료 → offline | `expired:true`, `lastFailure:'network'` | `system==='offline'` | — |
| TTL 만료 중 auth_required | `expired:true`, `status:'auth_required'` | `auth_required` 유지 | ✅ |
| 만료 리비전 역행 | `revision` < 현재 | 상태 불변 | ✅ |
| 말풍선 만료 전 | `nowMs < expiresAt` | **동일 참조** 반환 | ✅ |
| 말풍선 만료 후 | `nowMs >= expiresAt` | `bubble===null` | — |
| `relativeFromNow` 경계 | 0분/59분/60분/음수 | `"now"`/`"59m"`/`"1h 0m"`/`"now"` | ✅ |
| `relativeFromNow` 잘못된 ISO | `"not-a-date"` | `null` | ✅ |
| `capturedAgo` 잘못된 ISO | `""` | `null` | ✅ |
| `systemGuidance` | 상태 6종 × provider 2종 | 계약 §4.2 문구 / `null` | — |
| 종료 버튼 | 클릭 | `gateway.quit()` 호출 | — |
| refreshing 토글 | 새로고침 요청 → 상태 수신 | `true` → `false` | — |
| 21분 경과 | fake timer | stale 표시 전환 | ✅ |

### Edge Cases Checklist
- [ ] 빈 입력 (`""`, `null` 타임스탬프)
- [ ] 잘못된 타입 (숫자 아닌 percent, 파싱 불가 ISO)
- [ ] 리비전 역행/중복 (백엔드 이벤트 재전송)
- [ ] 타이머 경합 (수동 dismiss vs 자동 만료)
- [ ] 창 라벨 분기 (overlay에서 quit 호출 시 Forbidden)
- [ ] 두 provider 독립성 (한쪽 만료가 다른 쪽에 영향 없음)

---

## Validation Commands

### 0. 환경 확인 (Task 0 완료 후에만 아래가 유효)
```bash
corepack pnpm install --frozen-lockfile
cargo --version
```
EXPECT: 설치 완료, cargo 버전 출력

### 1. 정적 분석
```bash
corepack pnpm check
corepack pnpm lint
```
EXPECT: svelte-check 0 errors / 0 warnings, eslint+prettier 통과

### 2. 단위 테스트 + 커버리지
```bash
corepack pnpm vitest run --coverage
```
EXPECT: 전체 통과, branches/functions/lines/statements ≥ 80% (`vite.config.ts:34-39`)

### 3. Rust
```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-features
```
EXPECT: 전부 통과

### 4. 전체 CI 게이트
```bash
corepack pnpm test:ci
```
EXPECT: svelte-check + eslint + prettier + vitest coverage + vite build 전부 통과

### 5. E2E
```bash
corepack pnpm test:e2e:renderer
```
EXPECT: 렌더러 픽스처 E2E 통과

### 6. 수동 검증 (실제 앱)
```bash
corepack pnpm tauri dev
```
- [ ] 패널 종료 버튼으로 앱이 정상 종료되는가
- [ ] 말풍선이 8초 후 사라지는가
- [ ] 자격 증명 없는 provider 탭에 안내 문구가 뜨는가
- [ ] 리셋 시각이 `"resets in 1h 12m"` / `"resets Mon 09:00"`로 표시되는가
- [ ] 캡처 시각이 `"captured 2 min ago"`로 표시되는가
- [ ] 라이트/다크 양쪽에서 패널이 의도대로 보이는가

---

## Acceptance Criteria
- [ ] Task 0 완료 — 로컬에서 검증 스위트가 실행됨
- [ ] Phase 1 (A1~A3) 전부 수정 + 회귀 테스트
- [ ] Phase 2 디자인 결함 수정 (B4는 macOS 미보유 시 보류 가능)
- [ ] `pnpm test:ci` 통과, 커버리지 ≥ 80%
- [ ] `cargo clippy -- -D warnings` 통과
- [ ] `docs/dead-code-inventory.md` 작성 (삭제는 미수행)
- [ ] `SNAPSHOT_TTL_MS`, `expiresAt`, `setRefreshing`, `quit`, `defaultSize`의 참조 건수가 모두 0 초과

## Completion Checklist
- [ ] 신규 순수 함수가 `state/`·`interaction/`·`format/`에 위치 (컴포넌트에 로직 없음)
- [ ] 색상이 전부 `tokens.css` 참조 (hex 하드코딩 0건)
- [ ] `any` 0건, 강제 캐스팅 0건
- [ ] UI 문자열 영어 유지 (기존 일관성)
- [ ] 새 의존성 0개
- [ ] `docs/ui-contract.md` §10 검증 기준의 테스트 표면 충족
- [ ] 죽은 코드 삭제 없음 (인벤토리만)

## Risks

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| Task 0 실패 — WSL 환경에서 끝내 검증 불가 | **높음** | **높음** | WSL 네이티브 경로로 클론 이전. 최후엔 CI(PR)에서만 검증하되 반복 실패 비용을 감수 |
| Task 2가 기존 테스트를 깨뜨림 | 확실 | 중간 | Task 1을 **먼저** 수행 (TDD RED→GREEN) |
| macOS 미보유로 B4 시각 검증 불가 | 높음 | 낮음 | 보류하고 미검증으로 기록. CI 스크린샷 잡 추가는 별도 |
| 커버리지 80% 게이트가 신규 코드로 하락 | 중간 | 중간 | 각 태스크가 테스트를 동반. `App.svelte` 분기 증가에 주의 |
| `refresh_provider` 디바운스로 refreshing 플래그 미해제 | 중간 | 낮음 | 타임아웃 상한 필수 (Task 9 GOTCHA) |
| 백엔드/렌더러 TTL 이중 판정으로 상태 진동 | 중간 | 높음 | 백엔드를 단일 권위로 (Task 2 GOTCHA) |

## Notes

### 근본 원인 관찰

세 가지 구조적 패턴이 이 결함들을 만들었다.

1. **순수 함수는 만들고 배선은 잊었다.** `SNAPSHOT_TTL_MS`, `expiresAt`, `setRefreshing`, `setFullscreen`, `tickResetTimers`, `quit`, `defaultSize` — 전부 정의·테스트되어 있으나 `App.svelte`에서 참조가 0건이다. 단위 테스트가 순수 함수를 통과시키므로 **테스트 그린이 기능 동작을 보증하지 못했다**.

2. **로컬 검증이 반복적으로 건너뛰어졌다.** `local-changes-2026-07-17.md:114`(Rust Blocked), `ui-design-pass-2026-07-17.md:66`(비용 사유 skip) — 그리고 이번 탐색에서도 환경 제약으로 불가능했다. Task 0이 최우선인 이유다.

3. **테스트가 버그를 사양으로 고착시켰다.** `domain.test.ts:167`은 계약이 금지하는 동작을 "keeps an expired snapshot visible"로 기록해 두었다.

### 통합 발견 요약

| ID | 심각도 | 결함 | 근거 |
| --- | --- | --- | --- |
| A1 | HIGH | TTL 만료 무시 | `actor.rs:370-377` 발신 ↔ `App.svelte:173` 미소비, `engine.ts:8` 참조 0 |
| A2 | HIGH | 말풍선 영구 표시 | `bubblePolicy.ts:15,58` 생성 ↔ 참조 0 |
| A3 | HIGH | 종료 경로 없음 | `lib.rs:122` 등록 ↔ 렌더러 호출 0, 트레이 없음 |
| A4 | MED | 새로고침 비활성화 안 됨 | `providers.ts:131` 호출처 0 |
| A5 | MED | 렌더러 리셋 타이머 미작동 | `engine.ts:181` 테스트 전용 |
| A6 | LOW | fullscreen 미반영 | `interaction.ts:26` 호출처 0 |
| A7 | MED | 시간 경과 미반영 | `App.svelte:396-404` `Date.now()` 비반응 |
| B1 | HIGH | 원시 ISO 노출 | `UsageGauge.svelte:36`, `UsagePanel.svelte:48` |
| B2 | HIGH | 패널 상태 안내 없음 | `UsagePanel.svelte:22` loading 분기만 |
| B3 | MED | `defaultSize` 무시 | `PetOverlay.svelte:31` 하드코딩 |
| B4 | MED | macOS 다크 유리 깨짐 | `global.css:33-37` ↔ `53-57` |
| B5 | MED | 링/그래프 접근성 | `SplitUsageRing.svelte:30,39`, `HistoryGraph.svelte:90-99` |
| B6 | LOW | 배지 색 하드코딩 | `SystemBadge.svelte:81`, `UsagePanel.svelte:128` |
| C1 | MED | `any` + 컴포넌트 8/10 무타입 | `SettingsPanel.svelte:2` |
| C2 | MED | `window/mod.rs` 죽은 추상화 | 13개 심볼, 프로덕션 호출 0 |
| C3 | LOW | 강제 캐스팅 | `providers.ts:45` |
| C4 | LOW | Repository 중복 생성 | `lib.rs:85` ↔ `101` |
| C5 | LOW | 사용 불가 상태 키 | `manifest.ts:1-12` ↔ `resolver.ts:11-17` |
| C6 | LOW | 기본 설정 불일치 | `App.svelte:81` ↔ `settings.rs:47` |
| C7 | LOW | `always_on_top` 죽은 데이터 | `ipc.rs:175-177` |
| C8 | LOW | 알림 실패 시 dedupe 유실 | `App.svelte:136-145` |

### 이전 리뷰 대비 진척

`docs/code-review/local-changes-2026-07-17.md`의 HIGH 4건 중 3건(reset_pending, startup IPC, 네이티브 스모크)과 MEDIUM 대부분은 **해결됐다**. `ui-design-pass-2026-07-17.md`의 M1(캡처 시각)·L1(eslint ignore)도 해결됐고, M2(resets 접두사)는 **부분 해결**(접두사만 분기, 휴먼화 미완 → B1), L2(macOS accent)·L3(배지 색)은 **미해결**(→ B4·B6).

### 보안 검토 결과

이번 탐색에서 **하드코딩된 비밀, IPC 인가 우회, 경로 순회 취약점은 발견되지 않았다.** 확인한 방어 장치:
- `pets.rs:50-88` — 펫 패키지 canonicalize + 루트 이탈 검사 + 매니페스트 크기 상한
- `resolver.ts:45-77` — asset 프로토콜 화이트리스트 + `..` 거부
- `claude.rs:151-158` — `https_only(true)` + 리다이렉트 금지 + 10초 타임아웃
- `codex.rs:26-45` — 절대경로만 허용, 상대 PATH 항목 건너뜀 (이전 리뷰 HIGH 3 해결)
- `window/mod.rs:71-98` — 창 라벨별 IPC 커맨드 인가
- `ipc.rs:307-316` — 디버그 로그가 사용률/상태만 출력, 자격 증명·본문 제외
