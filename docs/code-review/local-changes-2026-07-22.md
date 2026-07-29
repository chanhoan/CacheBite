# Code Review: 로컬 미커밋 변경분 (stabilization-and-quality-pass)

**리뷰 일자**: 2026-07-22
**브랜치**: `develop`
**베이스**: `f5c56d7` (docs: add repository contributor guidance)
**범위**: 수정 29개 파일 (+742 / −89), 신규 모듈 3개 (`src/lib/format/time.ts`, `src/lib/components/systemGuidance.ts` + 각 테스트)
**결정**: **REQUEST CHANGES** — HIGH 1건 (재현 확인됨)

---

## Summary

렌더러 안정화·품질 패스로, 대부분의 변경이 실제 결함을 정확히 짚어 고쳤다. 특히 네이티브의 이중 `HistoryRepository` 인스턴스 제거, `SpeechBubble`의 mount-scoped 타이머를 정책 레이어로 이관, `selectedPetId` 기본값 오류(`'idle'` → `'cat'`), bare `<path>`에 걸려 무시되던 `aria-label`을 `<svg>` 단일 레이블로 통합한 부분은 근본 원인을 제대로 수정했다. 커버리지도 게이트(80%)를 크게 상회한다.

다만 새로 도입된 `snapshot_expired` 경로가 `unavailable_reason` / `failure_class`를 통째로 버리면서, **부팅 시 만료된 캐시 + 로그아웃 상태 조합에서 잘못된 복구 안내를 표시**하는 회귀가 생겼다. 이 조합은 백엔드 `hydrate` 경로에서 실제로 동시에 발생한다.

---

## Findings

### CRITICAL

없음.

- 하드코딩된 자격 증명·토큰·API 키 없음
- 신규 DTO(`quit`), 신규 문자열(`systemGuidance`)에 인증값·계정 식별자·자격 증명 경로 노출 없음 — 프라이버시 계약 유지
- 신규 `console.log` / 디버그 출력 없음, `TODO`/`FIXME` 없음
- `quit` 커맨드는 `NativeCommand::Quit`로 `panel` 윈도우에만 인가되며(`window/mod.rs:94`), `window/tests.rs:210,213`에 overlay 거부/panel 허용이 모두 테스트되어 있음. `gateway.ts:87`의 JSDoc 주장과 실제 인가 정책이 일치함

---

### HIGH

#### H-1. 만료 분기가 `unavailable_reason`/`failure_class`를 폐기해, 로그아웃 상태에서 잘못된 복구 안내가 표시됨

**위치**: `src/App.svelte:189-190`, `src/lib/state/engine.ts:179-200`

```ts
// App.svelte:186-190
const statusUpdate = (wire: ProviderBackendStateWire) => {
  if (wire.expired)
    return providersStore.applyExpiry(wire.provider, wire.revision);   // ← unavailable_reason / failure_class 유실
  ...
  if (wire.unavailable_reason === 'not_signed_in') ...                  // ← 도달 불가
```

`applyExpiry`는 `provider`와 `revision`만 전달한다. 따라서 `engine.ts`의 만료 분기는 **렌더러가 이전에 누적해 둔** `state.status` / `state.lastFailure`에만 의존해 강등 대상을 정한다.

```ts
// engine.ts:183-188
const degraded =
  state.status === 'auth_required' || state.status === 'unavailable'
    ? state.status
    : state.lastFailure === 'network' ? 'offline' : 'error';
```

문제는 백엔드가 `expired`와 `unavailable_reason`을 **동시에, 그리고 첫 이벤트로** 보낼 수 있다는 점이다. `src-tauri/src/refresh/actor.rs:450-472`의 `hydrate`는 영속화된 레코드를 복원하면서 두 값을 독립적으로 채운다:

```rust
let (failure_class, unavailable_reason) = match record.last_outcome {
    OutcomeMetadata::CredentialsMissing => (None, Some(UnavailableReason::NotSignedIn)),
    ...
};
let expired = snapshot.as_ref().is_some_and(|s|
    OffsetDateTime::now_utc() - s.captured_at > Duration::minutes(30));
```

이때 렌더러 상태는 `createProviderState`가 만든 초기값(`status: 'loading'`, `lastFailure: null`)이므로 → `degraded = 'error'`로 떨어진다.

**실패 시나리오**: 사용자가 CLI에서 로그아웃한 상태로 30분 이상 지난 뒤 CacheBite를 재실행한다. 패널에는 `"Sign in to the Claude CLI: claude login"` 대신 `"Could not fetch usage. Retrying shortly."`가 뜨고, 펫 배지도 `auth_required`가 아닌 `error`로 표시된다. 사용자는 스스로 복구할 방법을 안내받지 못한 채 자동 재시도만 기다리게 된다. `not_installed`(→ `unavailable`이어야 함), `failure_class: 'network'`(→ `offline`이어야 함)도 동일하게 무너진다.

**재현 확인 (CONFIRMED)**: `getProviderStates`가 `{ expired: true, unavailable_reason: 'not_signed_in', snapshot: <45분 전> }`을 반환하도록 한 임시 프로브 테스트를 App 합성 루트에 실행한 결과:

```
RENDERED GUIDANCE >>> "Could not fetch usage. Retrying shortly."
Expected: "Sign in to the Claude CLI: claude login"
```

기존 테스트가 이 조합을 놓친 이유는, `App.test.ts`의 만료 케이스가 `{ ...active('claude', 3), snapshot: null, expired: true }`로 `unavailable_reason`이 `null`인 경우만 검증하고, `domain.test.ts`의 `keeps auth_required when a snapshot expires` 역시 `credentials_missing`을 **선행 이벤트로 따로 적용한 뒤** 만료를 보내는 순서를 전제하기 때문이다. 동시 도착 경로가 비어 있다.

**제안 수정**: 만료 업데이트에 백엔드가 보고한 사유를 함께 실어 보내고, 엔진에서 자체 상태보다 우선 적용한다.

```ts
// engine.ts — StatusUpdate
| {
    readonly kind: 'snapshot_expired';
    readonly revision: number;
    readonly unavailableReason?: 'not_signed_in' | 'not_installed' | null;
    readonly failureClass?: FailureClass | null;
  }

// 만료 분기
const degraded =
  update.unavailableReason === 'not_signed_in' ? 'auth_required'
  : update.unavailableReason === 'not_installed' ? 'unavailable'
  : state.status === 'auth_required' || state.status === 'unavailable' ? state.status
  : (update.failureClass ?? state.lastFailure) === 'network' ? 'offline'
  : 'error';
```

`App.svelte`에서 `providersStore.applyExpiry(wire.provider, wire.revision, wire.unavailable_reason, wire.failure_class)`로 전달하고, 위 프로브 시나리오를 회귀 테스트로 고정할 것.

---

### MEDIUM

#### M-1. 버블 만료 타이머가 1 ms만 일찍 발화해도 버블이 영구히 남음

**위치**: `src/App.svelte:426-432`

```ts
const timer = window.setTimeout(
  () => interactionStore.expireBubble(Date.now()),
  Math.max(0, bubble.expiresAt - Date.now()),
);
```

`expireBubble`은 `nowMs < state.bubble.expiresAt`이면 **동일 참조를 그대로 반환**하고(`bubblePolicy.ts:33-39`), 스토어 쪽도 참조가 같으면 업데이트를 건너뛴다(`interaction.ts:36-43`). 즉 스토어가 변하지 않으므로 `$effect`가 재실행되지 않고 타이머는 재무장되지 않는다. 타이머가 조기 발화하거나(Node/libuv는 역사적으로 1 ms 조기 발화 사례가 있음) 시스템 시계가 뒤로 점프하면 버블은 다음 이벤트가 올 때까지 화면에 남는다. 항상 위에 떠 있는 오버레이라 체감 비용이 크다.

**수정**: 발화 시점 벽시계 대신 만료 기준값을 그대로 넘긴다 — `() => interactionStore.expireBubble(bubble.expiresAt)`. 조건이 `nowMs < expiresAt`이므로 항상 거짓이 되어 결정론적으로 정리된다.

#### M-2. `setRefreshing`이 호출자 없는 공개 API로 남음

**위치**: `src/lib/stores/providers.ts:186`

리팩터링으로 refreshing 플래그 수명이 `requestRefresh` → 타이머/`settleRefresh` 경로로 일원화되었는데, `setRefreshing`은 여전히 스토어 외부로 노출된다. 저장소 전체 grep 결과 외부 호출자는 0건이다. 남겨두면 `refreshTimers` 장부를 우회해 플래그를 켜는 호출이 생길 수 있고, 그 경우 `settleRefresh`는 타이머가 없어 조기 반환하므로 플래그가 영구히 `true`로 고착된다. 노출을 제거할 것.

#### M-3. 신규 `tabpanel`에 키보드 접근 경로가 없음

**위치**: `src/lib/components/HistoryGraph.svelte:78`

```svelte
<div id={PANEL_ID} role="tabpanel" aria-label={`${label} usage history`}>
```

WAI-ARIA APG의 tabs 패턴은 tabpanel 내부에 포커스 가능한 요소가 없으면 패널 자체에 `tabindex="0"`을 부여하도록 요구한다. 이 패널의 내용물은 `role="img"` SVG뿐이라 포커스 대상이 없고, 결과적으로 탭 버튼에서 Tab을 눌러도 패널로 진입할 수 없다. `tabindex="0"` 추가 필요.

#### M-4. 표시 문자열로 포맷 동작을 분기함

**위치**: `src/lib/components/UsageGauge.svelte:31`, `:62`

```svelte
: label === 'Weekly' ? absoluteShort(...) : relativeFromNow(...)
```

`label`은 화면에 그대로 출력되는 카피다. 문구를 `"Weekly"` → `"7-day"`로 바꾸는 순간 주간 리셋이 조용히 카운트다운 포맷으로 바뀐다. 컴파일러도 테스트도 잡아주지 못한다. `resetFormat: 'absolute' | 'relative'` 같은 명시적 prop으로 분리할 것.

---

### LOW

#### L-1. tabpanel과 SVG의 레이블이 중복 낭독됨
`HistoryGraph.svelte:78`의 `aria-label`과 `:83`의 SVG `aria-label`이 동일 문구(`"5-hour usage history"`)다. 스크린 리더가 같은 문구를 두 번 읽는다. tabpanel은 `aria-labelledby`로 해당 탭 버튼을 가리키는 편이 APG 권장에 부합한다.

#### L-2. `applyExpiry`만 시계를 내부에서 읽음
`src/lib/stores/providers.ts:126-140`. 형제 메서드 `apply` / `markResetPending`은 `nowMs`를 주입받는데 `applyExpiry`만 내부에서 `Date.now()`를 호출한다. 엔진의 만료 분기가 `nowMs`를 사용하지 않아 현재는 무해하지만, 저장소 전반의 "시계는 주입한다" 규약(`format/time.ts:3-7`에 명시)에서 벗어난다.

#### L-3. `refreshTimers`가 스토어 폐기 시 정리되지 않음
`src/lib/stores/providers.ts:30`. 스토어 수명이 앱 수명과 같아 실제 누수는 아니지만, `destroy()`가 있으면 모듈이 자기완결적이 된다.

#### L-4. `absoluteShort`만 시그니처가 다름
`src/lib/format/time.ts:37-40`. `(iso, timeZone?)`로 형제 함수 `(iso, nowMs)`와 두 번째 인자의 의미가 다르다. 주석에 이유가 문서화되어 있으나(lint의 미사용 파라미터 규칙 회피), 호출부가 헷갈릴 여지가 있다.

#### L-5. `PetOverlay.svelte`에 신규 필수 prop의 타입 강제가 없음
`models.ts:19`에서 `size`를 필수로 선언했지만 `PetOverlay.svelte:7`은 타입 주석 없는 `let { model } = $props();`라 컴포넌트 내부에서는 강제되지 않는다. 호출부(`App.svelte`)가 항상 채워주므로 현재는 안전하다.

---

## 잘 고친 부분

| 변경 | 평가 |
|---|---|
| `lib.rs:86,101` — 동일 파일에 대한 `HistoryRepository` 이중 인스턴스 제거, `Arc` 공유로 전환 | 실제 정합성 결함 수정. `ipc.rs:141`의 `State<'_, Arc<HistoryRepository>>` 전환도 누락 없이 완료(다른 `State<HistoryRepository>` 사용처 없음 확인) |
| `SpeechBubble.svelte` — mount-scoped 타이머 제거 | 교체된 버블이 이전 버블의 잔여 시간을 물려받던 실제 버그 수정. 근본 원인을 정확히 짚음 |
| `App.svelte:80` — `selectedPetId` 기본값 `'idle'` → `'cat'` | `getSettings()` 실패 시 펫 로드가 반드시 실패하던 경로 제거 |
| `SplitUsageRing.svelte` — bare `<path>`의 `aria-label` → `<svg>` 단일 합성 레이블 | 정확한 진단. 암묵적 role이 없는 요소의 `aria-label`은 접근성 트리에 반영되지 않음 |
| `global.css:54-63` — 다크 모드 `[data-platform]` 일괄 오버라이드 분리 | macOS `backdrop-filter`를 조용히 무력화하던 문제 해결 |
| `engine.ts:53-60` — `RaisedSeverity` 술어 도입 | 캐스트 없이 타입 수준에서 불변식 표현 |
| `providers.test.ts` / `domain.test.ts` 확장 | 타임아웃 상한, 프로바이더 간 플래그 격리 등 경계 조건을 실제로 검증 |

---

## Validation Results

| Check | Result | 비고 |
|---|---|---|
| Type check (`pnpm check` / svelte-check) | **Pass** | 0 errors, 0 warnings |
| Lint (`pnpm lint` — eslint + prettier) | **Pass** | |
| Tests (`pnpm vitest run --coverage`) | **Pass** | 23 files / 206 tests |
| Coverage (게이트 80%) | **Pass** | Stmts 94.95 / Branch 88.93 / Funcs 90.07 / Lines 94.95 |
| Build (`pnpm vite build`) | **Pass** | JS 92.24 kB (gzip 31.52 kB), CSS 10.73 kB |
| Rust fmt (`cargo fmt --check`) | **Pass** | |
| Rust clippy / test | **Skipped** | 이 WSL 환경에 `pkg-config` 및 glib 개발 헤더 부재로 `glib-sys` 빌드 실패. 변경된 Rust 파일 2개는 수동 리뷰함 — CI(`ci.yml`)에서 반드시 확인 필요 |

> 참고: 커버리지 표의 `panelModels.ts` 0%는 타입 전용 모듈이라 실행 구문이 없기 때문이며 결함이 아니다.

---

## Files Reviewed

**Modified — Native (2)**
`src-tauri/src/lib.rs`, `src-tauri/src/refresh/ipc.rs`

**Modified — Renderer 소스 (13)**
`src/App.svelte`, `src/lib/api/gateway.ts`, `src/lib/api/fixtureGateway.ts`, `src/lib/state/engine.ts`, `src/lib/stores/providers.ts`, `src/lib/stores/interaction.ts`, `src/lib/interaction/bubblePolicy.ts`, `src/lib/components/{HistoryGraph,PetOverlay,SettingsPanel,SpeechBubble,SplitUsageRing,SystemBadge,UsageGauge,UsagePanel}.svelte`, `src/lib/components/models.ts`

**Modified — 스타일 (2)**
`src/lib/styles/global.css`, `src/lib/styles/tokens.css`

**Modified — 테스트 (9)**
`src/App.test.ts`, `src/lib/state/domain.test.ts`, `src/lib/stores/providers.test.ts`, `src/lib/interaction/bubblePolicy.test.ts`, `src/lib/components/{HistoryGraph,PetOverlay,SpeechBubble,UsageGauge,UsagePanel}.test.ts`

**Added (5)**
`src/lib/format/time.ts`, `src/lib/format/time.test.ts`, `src/lib/components/systemGuidance.ts`, `src/lib/components/systemGuidance.test.ts`, `docs/dead-code-inventory.md`

---

## 다음 단계

1. **H-1 수정** — `applyExpiry` 시그니처에 `unavailable_reason` / `failure_class` 추가, 엔진 만료 분기에서 우선 적용. 부팅 시 `expired + not_signed_in` 동시 도착 시나리오를 `App.test.ts` 회귀 테스트로 고정.
2. **M-1 수정** — `expireBubble(bubble.expiresAt)`으로 전환 (1줄).
3. **M-2 ~ M-4** — 병합 전 처리 권장. 각각 독립적이고 변경 범위가 작음.
4. **Rust 검증** — 이 환경에서 clippy/test 실행 불가. `Arc<HistoryRepository>` 전환이 CI의 Rust 잡을 통과하는지 확인할 것.
