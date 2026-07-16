# Plan: CacheBite MVP + v1.1

## Summary

문서·디자인 프로토타입 단계인 CacheBite를 Tauri 2 + Rust + Svelte/TypeScript 데스크톱 앱으로 구축한다. MVP는 Claude/Codex 사용량 수집, provider별 독립 상태, 투명 always-on-top 펫, 1a 분할 링, 패널, 말풍선, 드래그/복원, 전체화면 숨김, 네이티브 패키징까지 포함한다. v1.1은 선택 애니메이션, opt-in OS 알림, bounded 히스토리 그래프, 보조 provider 알림을 MVP 계약 위에 추가한다.

## User Story

As a Claude/Codex 구독 사용자, I want 데스크톱 펫에서 5시간·주간 사용량과 리셋/오류 상태를 즉시 확인하고, so that 별도 웹 페이지나 CLI를 열지 않고 한도 소진을 예방할 수 있다.

## Problem → Solution

현재는 아키텍처, UI 상태 계약, 정적 Design Canvas 시안만 있고 실행 가능한 앱/테스트/패키지가 없다 → credential을 renderer에 노출하지 않는 로컬 Rust core와 결정적 Svelte 상태 UI를 단계적으로 구축하고, 각 OS의 창 기능 차이는 adapter와 명시적 capability 진단으로 격리한다.

## Metadata

- **Complexity**: XL
- **Source PRD**: N/A — `docs/UI-plan/`, `docs/architecture.md`, `docs/ui-contract.md` 참조
- **PRD Phase**: standalone MVP + v1.1 roadmap
- **Estimated Files**: 약 55–70개 생성/수정(에셋 프레임 제외)
- **Estimated Tasks**: 13개(Foundation 1, MVP 8, v1.1 4)
- **Current State**: `README.md:7-10`에 명시된 architecture-only greenfield; `src/`, `src-tauri/`, `tests/`, manifests/config/workflows 없음
- **Default Decisions**: pnpm, Svelte+TypeScript+Vite, Rust 1.77.2+ (Task 1에서 생성 시점 stable patch를 `rust-toolchain.toml`에 고정), Vitest, Testing Library, WebdriverIO Tauri service, cargo test

---

## UX Design

### Before

```text
┌─────────────────────────────────────┐
│ 실행 앱 없음                        │
│ docs + 정적 Design Canvas 시안만 존재 │
└─────────────────────────────────────┘
```

### After — MVP

```text
              ┌──────── 별도 panel window ────────┐
 클릭 ───────>│ [Claude ★] [Codex]                │
              │ 5h / weekly / reset / source      │
┌ overlay ┐   │ refresh / primary / settings/quit │
│  5H arc │   └───────────────────────────────────┘
│  idle   │       provider별 독립 상태
│  WK arc │       주 provider만 overlay/bubble 반영
└─────────┘
 drag → 위치 저장/경계 clamp
 fullscreen → 창만 숨김; 수집/상태 계산 지속
```

### After — v1.1

```text
MVP state/events
  ├─ optional mood/sleep/dragging animation resolver
  ├─ opt-in native notification policy
  ├─ bounded provider history → panel graph
  └─ opt-in secondary-provider event notifications
```

### Interaction Changes

| Touchpoint | Before | After | Notes |
|---|---|---|---|
| Overlay | 정적 1a/1b/1c 시안 | **1a continuous split ring** + idle pet | 상반원 5h, 하반원 weekly; 1b는 계약 위반, 1c ticks는 비범위 |
| Click/drag | 시안만 존재 | `<4px` release는 panel toggle, `>=4px`는 drag | drag 중 bubble/interaction 중단 |
| Panel | 정적 OS별 mock | 별도 Tauri panel window, 두 provider 탭 | OS별 별도 마크업을 만들지 않고 semantic UI 하나를 플랫폼 스타일로 조정 |
| Stale | prototype은 panel 전체 opacity 감소 | ring만 dim + capture time/stale label | controls/text opacity 유지 |
| Fullscreen | 없음 | overlay/panel/bubble 숨김, polling 유지 | exclusive fullscreen 위 overlay 시도 안 함 |
| v1.1 notifications | 없음 | 기본 OFF, OS permission 후 critical/exhausted/auth만 | primary-only가 기본 |

---

## Mandatory Reading

| Priority | File | Lines | Why |
|---|---|---:|---|
| P0 | `docs/architecture.md` | 1-224 | 제품 경계, provider 수집, 보안, refresh/window/packaging/test 계약 |
| P0 | `docs/ui-contract.md` | 1-288 | UI authoritative 상태/전환/상수/MVP-v1.1/검증 계약 |
| P0 | `docs/UI-plan/CacheBite Pet UI.dc.html` | 23-450 | 시각 토큰, 1a ring, panel, badge gallery, demo logic |
| P1 | `docs/UI-plan/Pet.dc.html` | 14-26 | `idle`/`idle_{mood}` asset key derivation |
| P1 | `README.md` | 1-87 | 현재 상태, 목표, planned layout |
| P2 | `docs/UI-plan/assets/` | all | 4-frame PNG 원본 후보; production manifest에 직접 연결 금지 |
| P2 | `docs/UI-plan/image-slot.js` | 1-260 | Design Canvas 전용 generated helper임을 확인; production 코드로 복사 금지 |

## External Documentation

| Topic | Source | Key Takeaway / Gotcha |
|---|---|---|
| Scaffold | [Tauri Create Project](https://v2.tauri.app/start/create-project/) | 공식 Svelte/TS template과 Vite/Tauri dev flow 사용; 생성 결과 버전을 lockfile로 고정 |
| Project layout | [Tauri Project Structure](https://v2.tauri.app/start/project-structure/) | frontend root + `src-tauri` 구조, capabilities 포함 |
| Security | [Tauri Capabilities](https://v2.tauri.app/security/capabilities/) | window label별 최소 권한; capability 중복은 권한 합집합이므로 overlay/panel capability를 분리 |
| Tests | [Tauri Tests](https://v2.tauri.app/develop/tests/) | Rust mock runtime + WebDriver E2E 지원; production endpoint 없이 mock 사용 |
| WebDriver | [Tauri WebDriver](https://v2.tauri.app/develop/tests/webdriver/) | `@wdio/tauri-service`는 Windows/Linux/macOS 지원; renderer-only browser mode 병행 가능 |
| Autostart | [Tauri Autostart API](https://v2.tauri.app/reference/javascript/autostart/) | enable/disable/isEnabled를 설정 adapter 뒤에 격리 |
| Notifications | [Tauri Notification API](https://v2.tauri.app/reference/javascript/notification/) | permission state를 처리하고 v1.1 opt-in에서만 요청/발송 |
| CI/Release | [Tauri GitHub Pipeline](https://v2.tauri.app/distribute/pipelines/github/) | native runner + tauri-action; signing/notarization secret은 CI에만 둠 |

### Research Findings

```text
KEY_INSIGHT: Tauri 2 capability는 window/webview별 IPC 권한 경계다.
APPLIES_TO: overlay와 panel의 명령 allowlist, renderer 무권한 네트워크 원칙.
GOTCHA: 동일 window가 여러 capability에 속하면 권한이 합쳐진다.

KEY_INSIGHT: 빠른 renderer test는 mockIPC, 실제 desktop E2E는 WebdriverIO Tauri service로 분리 가능하다.
APPLIES_TO: Vitest component/integration + native smoke/E2E 계층.
GOTCHA: 각 test 후 Tauri mocks를 clear해 격리를 보장한다.

KEY_INSIGHT: notification/autostart official plugins는 Rust 1.77.2+를 요구한다.
APPLIES_TO: rust-toolchain.toml/MSRV와 dependency lock.
GOTCHA: Task 1에서 설치한 stable patch를 고정하고 lockfile update PR에서만 올린다.
```

---

## Unified Discovery Table

| Category | File:Lines | Pattern / Finding | Key Evidence |
|---|---|---|---|
| Current state | `README.md:7-10` | architecture-only | scaffold/packages 없음 |
| Layout | `docs/architecture.md:40-60` | monorepo UI+Rust+tests+native CI | `src/`, `src-tauri/`, `pets/`, `tests/` |
| Entry/data flow | `docs/architecture.md:64-80` | UI → state → normalized service/adapters/store | provider/OS wire format 격리 |
| Domain types | `docs/architecture.md:82-100` | normalized snapshot/window | optional windows, source/status |
| UI DTO | `docs/ui-contract.md:113-122` | UI-only failure metadata | `unavailable_reason`, `failure_class` |
| Errors | `docs/ui-contract.md:39-52,82-104` | auth/unavailable/network/provider/parse/internal 분리 | table-driven transition |
| Logging/security | `docs/architecture.md:150-160` | token/body/path/account redaction | renderer/cache/log에 credential 금지 |
| State | `docs/ui-contract.md:16-66` | system + stale + dual severity + mood | pure derivation |
| Refresh | `docs/architecture.md:164-175` | startup, 15m poll, debounce/backoff, 30m TTL | provider별 독립 |
| Window | `docs/architecture.md:176-196` | platform adapter + visible degradation | drag/clamp/fullscreen |
| Asset | `docs/ui-contract.md:192-209` | strict requested key → idle fallback | idle만 MVP 필수 |
| UI | `docs/UI-plan/CacheBite Pet UI.dc.html:89-112` | 1a literal split ring | contract와 일치 |
| Prototype logic | `docs/UI-plan/Pet.dc.html:14-24` | mood key mapping | demo behavior만 채택 |
| Tests | `docs/architecture.md:212-220`; `docs/ui-contract.md:279-288` | unit/integration/UI/native/package matrix | production endpoint 접촉 금지 |
| Existing implementation | repository-wide | **없음** | naming/error/log/test code pattern을 발명된 기존 패턴처럼 주장하지 말 것 |

---

## Strategic Architecture

### Approach

```text
OS/provider adapters (Rust)
  → CollectorOutcome (credential-free, typed)
  → normalized ProviderUsageSnapshot
  → provider-scoped refresh actor + versioned repository
  → allowlisted ProviderUiSnapshot(revision included) over Tauri IPC
  → TS provider stores (discard older revision)
  → pure PetUiState selectors + ephemeral interaction policies
  → overlay / separate panel / bubble
```

- Rust owns credential access, provider wire parsing, retry/backoff, snapshot freshness/expiry, persistence, platform adapters.
- TypeScript owns deterministic presentation derivation and ephemeral panel/drag/bubble state. Components contain no transition rules.
- Each provider has an independent async refresh actor, retry state, cache, revision. No global provider failure flag.
- Overlay and panel are separate Tauri windows. Overlay remains asset-sized and draggable; panel is interactive, anchored/clamped, and lifecycle/fullscreen synchronized.
- IPC payloads are explicit DTO allowlists, never `serde_json::Value` pass-through.

### Locked Contract Decisions

1. `docs/ui-contract.md` is authoritative when status meanings conflict with architecture prose.
2. Missing/invalid sign-in → `auth_required`; missing CLI/path → `unavailable(not_installed)`. `not_signed_in` is never rendered as unavailable.
3. Expired snapshot with no last failure → `error` with internal reason `expired_without_failure`; renderer에는 generic error만 전달.
4. Provider input percent must be finite; then clamp to `0..=100`. NaN/Infinity/type mismatch is parse failure.
5. `resets_at` timer emits one optimistic reset per `(provider, window, resets_at)`, displays `unknown` until refresh, and triggers a debounced refresh. A subsequent snapshot confirms value; duplicate timer/snapshot reset is suppressed.
6. Preserve provider transport source and add `is_cached`; do not overwrite source with ambiguous `cache` in UI DTO.
7. Platform capability degradation is a diagnostic/settings concern, not provider `PetUiState.system`.
8. 1a continuous split ring is the MVP visual. Prototype 1b/1c are not implemented.
9. Prototype cat=Claude/corgi=Codex is demo data, not core invariant. Settings/manifest selects a pet package independently of provider. **Release artifact에는 디자인을 번들하지 않는다**; repository에는 테스트 전용 generated geometric idle fixture만 두고, 실제 실행은 사용자가 공급한 manifest package를 app-data pet directory에서 불러온다.
10. MVP collector는 Claude OAuth usage endpoint와 Codex app-server RPC만 구현한다. Claude PTY, Codex direct backend/status fallback은 **v1.1 Task 10으로 확정**하며 MVP에서 feature flag나 미완성 adapter를 만들지 않는다.
11. Stale dims ring only; panel text/controls remain accessible and show capture time.
12. `>=80%` line/branch coverage applies to domain/state/policy modules; native OS adapter code additionally requires smoke tests because coverage instrumentation is not sufficient.

### Alternatives Considered

| Alternative | Decision |
|---|---|
| TS-only state/cache | Reject: backend expiry/persistence and UI clocks diverge |
| Rust-only UI reducer | Reject: ephemeral UI interaction becomes IPC-heavy and component testing harder |
| One expanding overlay+panel window | Reject: transparent drag/input/size and display clamp become coupled |
| Tauri store directly from renderer | Reject for core state: renderer should not own snapshot files; use Rust repository/migrations |
| Shared provider polling loop | Reject: violates provider isolation/backoff independence |
| State-specific art in MVP | Reject: contract requires only idle; strict fallback prepares v1.1 |

## Scope

### MVP WILL Build

- Scaffold, domain/state contracts, settings/snapshot persistence
- Claude OAuth and Codex app-server primary collectors
- Poll/debounce/backoff/TTL/reset orchestration and narrow IPC
- 1a ring, five system badges + loading, two-provider panel, bubbles
- always-on-top, drag/restore/clamp, fullscreen hide, start-at-login
- Windows MSI/NSIS, macOS DMG signing/notarization-ready, Linux AppImage; checksums
- Unit/integration/component/E2E/native/package validation with no production provider calls

### v1.1 WILL Build

- Optional mood/sleep/dragging assets with strict idle fallback
- Opt-in primary-provider OS notifications
- Versioned bounded per-provider history and graph
- Opt-in secondary-provider event notifications
- Guarded Claude PTY and Codex direct-backend/status collector fallbacks

## NOT Building

- CacheBite cloud/account/sync, browser-cookie scraping, Anthropic/OpenAI billing analytics
- Token-count-derived quota, credential refresh/write, configurable provider endpoints
- Desktop roaming, multiple simultaneous pets, exclusive-fullscreen overlay
- Remote renderer content or broad filesystem/shell/network permissions
- DEB before AppImage stability; auto-updater unless separately scoped
- Provider-to-animal hard binding or unlicensed prototype assets

---

## Patterns to Mirror

Greenfield이므로 아래는 실제 코드가 아니라 **문서의 authoritative contract snippet**이다. 구현 후 생기는 Rust/TS conventions는 Task 1에서 formatter/linter로 고정한다.

### DOMAIN_CONTRACT

```text
// SOURCE: docs/architecture.md:87-100
ProviderUsageSnapshot
  provider: claude | codex
  session: optional UsageWindow
  weekly: optional UsageWindow
  captured_at: ISO 8601 timestamp

UsageWindow
  used_percent: number from 0 through 100
  window_minutes: positive integer
  resets_at: optional ISO 8601 timestamp
```

### ERROR_HANDLING

```text
// SOURCE: docs/ui-contract.md:86-93
FETCH_FAIL(class)
class: network | provider | parse | internal
CREDS_MISSING
CLI_MISSING
SNAPSHOT_EXPIRED
```

### LOGGING_PATTERN

```text
// SOURCE: docs/architecture.md:152-160
tokens are never written to settings, caches, or logs
logs redact authorization headers, account identifiers, home paths, response bodies
responses have size limits, timeouts, schema validation, HTTPS-only endpoints
```

### REPOSITORY_PATTERN

```text
// SOURCE: docs/architecture.md:35,77 and 164-174
versioned JSON settings + latest provider snapshots
retain success up to 30 minutes
preserve independent Claude and Codex state
```

### UI_STATE_PATTERN

```text
// SOURCE: docs/ui-contract.md:23-33
PetUiState
  system: auth_required | unavailable | error | offline | loading | active
  stale: boolean
  session_sev / weekly_sev: ok | warn | critical | exhausted | unknown
  pet_mood = max(known severities), default ok
```

### ASSET_PATTERN

```js
// SOURCE: docs/UI-plan/Pet.dc.html:17-24
const m = ['ok','warn','critical','exhausted'].includes(this.props.mood)
  ? this.props.mood : 'ok';
const key = m === 'ok' ? 'idle' : 'idle_' + m;
```

### TEST_STRUCTURE

```text
// SOURCE: docs/ui-contract.md:283-288
severity boundaries; full transition table; freshness boundaries;
bubble dedupe/reset; GIF fallback; provider independence
```

### Naming Convention to Establish

- Rust: `snake_case` modules/functions/fields, `PascalCase` types/enums, `SCREAMING_SNAKE_CASE` constants; `Result<T, DomainError>` at boundaries.
- TypeScript/Svelte: `camelCase` values/functions, `PascalCase` types/components, kebab-free `.ts` domain filenames and PascalCase `.svelte` components; wire DTO field names remain `snake_case` exactly at IPC boundary and map once to typed app models.
- Never mutate shared state: reducers return new objects; Rust repository replaces immutable snapshots under scoped synchronization.

---

## Files to Change

The table is a complete target map; split files further if any exceeds 800 lines.

| File / Directory | Action | Justification |
|---|---|---|
| `package.json`, `pnpm-lock.yaml` | CREATE | scripts and pinned frontend dependencies |
| `svelte.config.js`, `vite.config.ts`, `tsconfig.json`, `vitest.config.ts`, `wdio.conf.ts` | CREATE | build/type/unit/E2E setup |
| `.editorconfig`, `.gitignore`, `eslint.config.js`, `.prettierrc` | CREATE | greenfield conventions |
| `src/main.ts`, `src/App.svelte`, `src/app.css` | CREATE | renderer entry and semantic tokens |
| `src/lib/contracts/{usage,ipc}.ts` | CREATE | explicit renderer-safe DTOs |
| `src/lib/domain/{severity,petUiState,events}.ts` | CREATE | pure derivation/reducer/events |
| `src/lib/stores/{providers,settings,interaction}.ts` | CREATE | immutable app/ephemeral state |
| `src/lib/api/tauri.ts` | CREATE | single typed IPC/event gateway |
| `src/lib/policies/{bubble,notification}.ts` | CREATE | dedupe/priority/delivery policy |
| `src/lib/assets/{manifest,resolver}.ts` | CREATE | manifest validation and strict fallback |
| `src/lib/actions/petPointer.ts` | CREATE | click/drag threshold logic |
| `src/lib/components/{PetOverlay,PetAnimation,SplitUsageRing,SystemBadge}.svelte` | CREATE | overlay surface |
| `src/lib/components/{UsagePanel,ProviderTabs,UsageGauge,SettingsPanel,SpeechBubble,HistoryGraph}.svelte` | CREATE | panel/MVP bubble/v1.1 graph |
| `src-tauri/Cargo.toml`, `Cargo.lock`, `build.rs`, `rust-toolchain.toml` | CREATE | Rust/Tauri build pinned |
| `src-tauri/tauri.conf.json` | CREATE | transparent overlay + separate hidden panel configuration |
| `src-tauri/capabilities/{overlay,panel}.json` | CREATE | least-privilege IPC/plugin capabilities |
| `src-tauri/src/{main,lib}.rs` | CREATE | application composition root |
| `src-tauri/src/domain/{mod,usage,error,outcome}.rs` | CREATE | normalized model/error taxonomy |
| `src-tauri/src/providers/{mod,claude,codex,credentials,normalize}.rs` | CREATE | collectors and broker |
| `src-tauri/src/services/{mod,usage,refresh}.rs` | CREATE | provider actors/orchestration |
| `src-tauri/src/store/{mod,settings,snapshots,history,migrations}.rs` | CREATE | versioned atomic JSON stores |
| `src-tauri/src/window/{mod,geometry,windows,macos,linux}.rs` | CREATE | platform adapters/capabilities |
| `src-tauri/src/ipc/{mod,commands,events,dto}.rs` | CREATE | allowlisted DTO commands/events |
| `src-tauri/src/security/{mod,redaction}.rs` | CREATE | safe structured logging |
| `tests/fixtures/pets/geometric-idle/*` | CREATE | non-product generated test fixture; release bundle 제외 |
| `tests/fixtures/{claude,codex,manifests}/*` | CREATE | malformed/variant/provider fixtures |
| `tests/fakes/{claude,codex}/*` | CREATE | fake executable/RPC fixtures |
| `tests/e2e/*.spec.ts` | CREATE | critical user flows |
| `.github/workflows/{ci,native-smoke,release}.yml` | CREATE | cross-platform checks/package |
| `README.md` | UPDATE | setup, commands, security, supported limitations |
| `docs/architecture.md`, `docs/ui-contract.md` | UPDATE | only when locked decisions materially amend source contracts |

Test files are colocated as `*.test.ts`/`*.test.svelte.ts` and Rust `#[cfg(test)]` modules; cross-layer tests live under `tests/`.

---

## Step-by-Step Tasks

### Task 1: Foundation and executable skeleton

- **PHASE**: Foundation / MVP prerequisite
- **ACTION**: Scaffold official Tauri 2 Svelte TypeScript app and pin toolchain/dependencies.
- **IMPLEMENT**: pnpm scripts `dev`, `build`, `check`, `lint`, `test`, `test:coverage`, `test:e2e`, `tauri`; Rust fmt/clippy/test; overlay and initially hidden panel labels; semantic CSS tokens; strict TS; Vitest/Testing Library/WDIO; CI caches.
- **MIRROR**: `docs/architecture.md:30-60` layout and technology.
- **IMPORTS**: official Tauri APIs only through `src/lib/api/tauri.ts`; no direct imports throughout components.
- **GOTCHA**: generated template versions must be committed with `pnpm-lock.yaml`/`Cargo.lock`; overlay and panel must not share a broad capability. Do not ingest generated Design Canvas JS.
- **VALIDATE**: empty app launches via `pnpm tauri dev`; `pnpm check`, `pnpm lint`, `pnpm test`, `cargo fmt --check`, `cargo clippy`, `cargo test` execute successfully.

### Task 2: Normalized domain, collection outcomes, and UI state engine (TDD RED→GREEN)

- **PHASE**: MVP
- **ACTION**: Define Rust normalized contracts and TS presentation contracts; implement deterministic derivation/event reducer.
- **IMPLEMENT**: `Provider`, `UsageWindow`, `ProviderUsageSnapshot`, `CollectionOutcome`, `FailureClass`, safe UI DTO, system precedence, finite/clamp severity, mood, freshness at exact 20/30-minute bounds, full transition table, one-shot reset events, monotonic `revision`, primary switch without refetch.
- **MIRROR**: domain/state/error snippets above and `docs/ui-contract.md:35-111,241-252`.
- **IMPORTS**: Rust `serde`, `serde_json`, `time` (`serde`, `parsing`, `formatting` features); TS contract imports only from `src/lib/contracts`.
- **GOTCHA**: `active` failure keeps snapshot until TTL; background refresh never flips to loading; exact `age == 20m` fresh and `age == 30m` stale, only `>30m` expired; cached transport source preserved.
- **VALIDATE**: table-driven tests cover every state×event cell, 69/70/89/90/99/100, invalid numeric payloads, 20/30 minutes, expired-without-failure, reset dedupe, out-of-order revision discard.

### Task 3: Versioned settings and per-provider snapshot repositories

- **PHASE**: MVP
- **ACTION**: Build Rust-owned, atomic, versioned JSON persistence.
- **IMPLEMENT**: settings schema includes schema version, primary provider, selected pet id, bubble toggle, start-at-login, logical position; snapshot file stores independent latest Claude/Codex snapshots, last outcome metadata, revision; temp-write+fsync/rename where supported; migration and corrupt quarantine/default recovery.
- **MIRROR**: `docs/architecture.md:35,77,164-175`.
- **IMPORTS**: filesystem access remains Rust-side app-data path APIs.
- **GOTCHA**: never serialize credentials, raw provider bodies, account id, absolute credential path; concurrent provider writes must merge by provider instead of last-writer clobber.
- **VALIDATE**: roundtrip/migration/corrupt file/concurrent merge/permission denied tests; serialized fixture assertion proves forbidden keys/secret marker absent.

### Task 4: Secure provider collectors and credential broker

- **PHASE**: MVP
- **ACTION**: Implement Claude OAuth and Codex app-server primary paths behind a shared collector trait.
- **IMPLEMENT**: read-only credential broker precedence; HTTPS allowlisted Claude request with exact headers; Codex hidden child app-server initialization and `account/rateLimits/read`; strict timeout/response size/schema; defensive known field variants; normalize to common windows; typed missing creds/CLI/failure outcomes; child cleanup on success/error/timeout/cancel.
- **MIRROR**: `docs/architecture.md:104-162`.
- **IMPORTS**: `reqwest` (`rustls-tls`, `json`, default features off), `tokio` (`process`, `time`, `io-util`, `sync`), `serde`/`serde_json`, `secrecy`, `thiserror`, `tracing`; exact compatible versions are resolved once by Cargo and committed in `Cargo.lock`.
- **GOTCHA**: no token crosses IPC; never log headers/body/home path/account id; fake executables and local mock HTTP only; direct internal endpoints can drift. Treat wire decoders as tolerant adapters: Claude accepts `five_hour`/`seven_day` objects containing utilization/percent and reset timestamp variants documented by fixtures; Codex request/response envelopes are pinned from the installed `codex app-server` protocol fixture and official `account/rateLimits/read` README before GREEN implementation. Unknown fields are ignored, missing required numeric fields are parse failures. If substantial Orca code is copied, include required MIT notice (`architecture.md:222-224`).
- **VALIDATE**: mock HTTP validates header presence without recording values; RPC handshake/order/id/timeout/malformed/oversize/cleanup tests; missing CLI/sign-in mapping; provider isolation; grep/serialization tests for secret marker.

### Task 5: Refresh actors, backoff, cache expiry, and narrow IPC

- **PHASE**: MVP
- **ACTION**: Compose one independent refresh actor per provider and expose renderer-safe commands/events.
- **IMPLEMENT**: startup immediate fetch, 15-minute poll, focus/resume/manual debounce, exponential backoff with jitter/cap constants, 30-minute TTL timer, reset-at refresh; commands `get_provider_states`, `refresh_provider`, `update_settings`, `show_panel`, `quit`; full DTO event with revision after state changes.
- **MIRROR**: `docs/architecture.md:164-175`; `docs/ui-contract.md:106-111,184-190`.
- **IMPORTS**: Tauri state/event APIs only in IPC module.
- **GOTCHA**: manual refresh does not set loading; provider locks are not held across await; stale timer and fetch completion race resolves by revision; capabilities allow only required commands per window.
- **VALIDATE**: paused-time tests for poll/debounce/backoff/TTL/reset; concurrent providers; older IPC revision ignored; generated capabilities audited for no shell/fs/http blanket access.

### Task 6: Asset package normalization and MVP overlay

- **PHASE**: MVP
- **ACTION**: Define/validate canonical pet manifest and implement 1a overlay.
- **IMPLEMENT**: manifest id/display/default size/animation/frame timing/states; require `idle`; load externally supplied packages from the app-data pet directory; keep only a generated geometric idle fixture under tests; frame animation loader; SVG upper/lower arcs; severity colors/unknown track/stale dim; active ring vs system badge; accessible status labels.
- **MIRROR**: `docs/architecture.md:198-203`, `docs/ui-contract.md:132-153`, prototype `:89-112`.
- **IMPORTS**: asset resolver and typed state selectors; components receive view models only.
- **GOTCHA**: `docs/UI-plan/assets`는 product input 예시일 뿐 release에 복사하지 않는다. Source folders use `dog` while prototype says `corgi`, and `exhusted` misspells `exhausted`; any user-supplied import is normalized to canonical manifest keys. Current source is four PNG frames, not GIF/WebP. MVP animation remains `idle` regardless mood.
- **VALIDATE**: invalid/missing idle/path traversal/unknown state manifests fail safely; visual tests for dual percentages, unknown, stale, six system states; system != active hides ring.

### Task 7: Window/platform controller and pointer interaction

- **PHASE**: MVP
- **ACTION**: Implement platform adapters, overlay drag, separate panel lifecycle, display clamp, fullscreen visibility.
- **IMPLEMENT**: trait methods from architecture; logical↔physical coordinates; nearest-display clamp; display-change recovery; `<4px` click vs `>=4px` drag; panel anchor/flipping/clamp; hide both windows over fullscreen while collectors continue; start-at-login adapter; explicit Linux capability diagnostics.
- **MIRROR**: `docs/architecture.md:176-196`; `docs/ui-contract.md:154-162`.
- **IMPORTS**: platform-specific crates/modules behind cfg; renderer pointer action calls narrow window commands.
- **GOTCHA**: negative monitor coordinates, scale changes, disconnected display, panel focus loss, Wayland limitations, dragging must suppress bubbles. Platform limitation must not mutate provider system state.
- **VALIDATE**: geometry property/table tests, pointer threshold tests, persisted recovery; autostart enable/disable/idempotency/restart-state/unsupported-capability tests; native smoke on Windows/macOS/Linux X11/Wayland; fullscreen test proves fetch actor still advances revision.

### Task 8: Provider panel, settings, and speech bubble policy

- **PHASE**: MVP
- **ACTION**: Build the interactive UI on top of pure selectors/events.
- **IMPLEMENT**: always-visible provider tabs, plan type, gauges/reset time/capture/source/status, loading-only skeleton, refresh disabled during debounce, set-primary/settings/quit, bubble event eligibility/priority/dedupe/8s dismissal/click-to-panel/no queue during drag/fullscreen.
- **MIRROR**: `docs/ui-contract.md:124-131,160-233`; prototype `:170-229`.
- **IMPORTS**: components consume provider/settings/interaction stores; policy functions consume typed transition events.
- **GOTCHA**: unavailable/auth provider must not block other tab; non-primary tab derives same state but never changes pet/bubble; reset clears dedupe; panel stale controls remain full opacity.
- **VALIDATE**: component tests for both tabs/system states/actions; fake timers for bubble replace/dismiss/dedupe/reset/recovery; bubble click opens panel; primary switch re-derives instantly without fetch.

### Task 9: MVP end-to-end, security, native smoke, and release gates

- **PHASE**: MVP release
- **ACTION**: Complete layered test matrix and native packages.
- **IMPLEMENT**: renderer mockIPC tests, local mock server/fake CLI integration, WDIO critical flows, OS smoke, install/launch/uninstall package tests; CI on PR; release on version tag; checksums; signing/notarization inputs from secrets only; dependency/license/audit steps.
- **MIRROR**: `docs/architecture.md:204-220`.
- **IMPORTS**: tauri-action and native runner prerequisites from official docs.
- **GOTCHA**: CI must never contact production provider URLs or expose test secret markers; macOS signing/notarization is a release credential gate; unsupported Wayland is visible degradation, not silent pass.
- **VALIDATE**: all validation commands below; 80%+ domain/state/policy line and branch coverage; artifact matrix produces MSI, NSIS, DMG, AppImage + checksums.

### Task 10: Optional animation states and deferred collector fallbacks

- **PHASE**: v1.1
- **ACTION**: Enable mood/sleep/dragging asset resolution and, if excluded at Task 4 gate, guarded CLI/direct collector fallbacks.
- **IMPLEMENT**: `idle_warn`, `idle_critical`, `idle_exhausted`, `sleep`, `dragging`; resolver is requested→idle only. Fallback collectors are separately feature-flagged, strict timeout, noninteractive, output-panel-only parsers.
- **MIRROR**: `docs/ui-contract.md:192-209,265-268`; `docs/architecture.md:123,137-148`.
- **IMPORTS**: existing resolver/collector traits only.
- **GOTCHA**: auth/error/loading always request idle; partial asset packages predictable; never persist conversation/status output; direct backend redirect remains allowlisted.
- **VALIDATE**: every declared/missing state, drag/sleep priority, malformed animation; fallback timeout/login prompt/output contamination/cleanup tests.

### Task 11: Opt-in native OS notifications

- **PHASE**: v1.1
- **ACTION**: Add notification adapter using the same transition eligibility/dedupe core as bubbles.
- **IMPLEMENT**: setting defaults OFF; request permission only on explicit enable; primary provider critical/exhausted/auth entry; reset re-arms dedupe; OS delivery adapter returns permission/capability result.
- **MIRROR**: `docs/ui-contract.md:235-239`.
- **IMPORTS**: official notification plugin isolated behind adapter.
- **GOTCHA**: do not duplicate policy logic or notify warn/reset/recovery; denial is a settings diagnostic, not provider error.
- **VALIDATE**: permission granted/denied/prompt, opt-out, dedupe/reset, primary switch, native notification smoke per OS.

### Task 12: Bounded per-provider history and panel graph

- **PHASE**: v1.1
- **ACTION**: Extend latest-only persistence with an explicit history contract, then implement graph.
- **IMPLEMENT**: adopt and first document these explicit v1.1 planning defaults: successful fresh samples only; append on materially new `captured_at`; provider/window independent; 15-minute nominal cadence; 30-day retention; max 3,000 samples/provider; UTC timestamps; reset produces discontinuity, no interpolation; versioned migration. These are bounded local-storage engineering defaults, not claims from the source contract. Then add an accessible graph with 5h/weekly toggles and empty/gap states.
- **MIRROR**: provider-calculated-only principle `docs/architecture.md:102`; v1.1 item `docs/ui-contract.md:269`.
- **IMPORTS**: reuse Svelte and native SVG paths; add no chart dependency for v1.1.
- **GOTCHA**: current architecture persists latest snapshot only, so this is a schema expansion; never estimate between samples; bound disk growth; clock rollback/out-of-order samples.
- **VALIDATE**: migration/retention/cap/dedupe/out-of-order/reset gap tests; graph empty/single/gap/dual-window visual and keyboard/accessibility tests.

### Task 13: Secondary-provider event notification option

- **PHASE**: v1.1
- **ACTION**: Allow explicit opt-in notifications for non-primary provider without changing overlay/pet semantics.
- **IMPLEMENT**: default OFF setting; per-provider dedupe namespace; notification text labels provider; bubble remains primary-only unless a separate future setting is approved.
- **MIRROR**: provider independence `docs/architecture.md:173`; v1.1 item `docs/ui-contract.md:270`.
- **IMPORTS**: reuse event stream and notification policy.
- **GOTCHA**: primary switch must not replay historical events; one provider reset only re-arms its own keys.
- **VALIDATE**: off/on, simultaneous severity events, primary switch, per-provider reset/dedupe tests.

---

## Dependency and Delivery Sequence

```text
Task 1 Foundation
  └─ Task 2 Domain/state
      ├─ Task 3 Persistence
      ├─ Task 4 Collectors
      └─ Task 6 Asset/overlay
          Task 3 + 4 ──> Task 5 Refresh/IPC
          Task 3 + 6 ──> Task 7 Window/interaction
          Task 5 + 7 ──> Task 8 Panel/bubbles
          Tasks 1–8 ───> Task 9 MVP release

Task 9
  ├─ Task 10 Animations/fallbacks
  ├─ Task 11 OS notifications ──> Task 13 secondary option
  └─ Task 12 History
```

Each task follows mandatory TDD: failing test first, minimal green implementation, refactor, affected coverage check, code review. Tasks 3–5 receive security review before merge; Task 9 receives full security/code review.

---

## Testing Strategy

### Unit / Integration Matrix

| Test | Input | Expected Output | Edge Case? |
|---|---|---|---|
| Severity | 69/70/89/90/99/100/>100 | exact enum after finite validation/clamp | Yes |
| Freshness | age 20m, 20m+ε, 30m, 30m+ε | fresh, stale, stale, expired | Yes |
| Transition | every system × event | contract table/locked defaults | Yes |
| Reset | timer + same confirming snapshot | one reset event, unknown until refresh | Yes |
| Revision | event 7 then 6 | 6 discarded | Yes |
| Provider isolation | Claude auth fail, Codex fresh | independent tab/state | Yes |
| Secret redaction | marker in token/header/path/body | absent from log/DTO/cache | Security |
| RPC | timeout/malformed/oversize/child exit | typed outcome + child cleanup | Yes |
| Persistence | old/corrupt/concurrent | migrate/recover/merge | Yes |
| Position | negative coords/DPI/removed display | nearest visible clamp | Yes |
| Bubble | duplicate, reset, drag/fullscreen | suppress/re-arm/drop without queue | Yes |
| Asset | missing mood/misspelled source dir | idle fallback/canonical package | Yes |
| History | out-of-order/clock rollback/>cap | sorted bounded samples/gaps | v1.1 |

### Required Test Layers

1. Rust/TS unit tests for pure domain, parsers, reducers, policies, geometry, migrations.
2. Integration tests using local mock HTTP and fake Claude/Codex executables.
3. Svelte component/integration tests using Tauri `mockIPC` and cleared mocks.
4. WDIO E2E in renderer-browser mode on every PR; embedded Tauri WDIO smoke on Windows/Linux/macOS native jobs using the locked `@wdio/tauri-service` version.
5. Native smoke on Windows, macOS, Linux X11 and one Wayland compositor.
6. Package install/launch/uninstall and checksum validation.

### Edge Cases Checklist

- [ ] Missing/invalid credentials vs missing CLI are distinct
- [ ] Optional session/weekly windows independently unknown
- [ ] NaN/Infinity/non-number/negative/>100 payloads
- [ ] Clock rollback, time zone/DST, expired cache without last failure
- [ ] Concurrent provider refresh/store writes and app shutdown
- [ ] Network timeout/TLS/redirect/oversize/malformed response
- [ ] Child process hangs, interactive login prompt, partial JSON-RPC output
- [ ] Monitor removed, negative origin, DPI change, panel near every edge
- [ ] Fullscreen/drag while bubble pending; no delayed queue
- [ ] Corrupt/old/read-only persistence
- [ ] Wayland unsupported capabilities shown separately
- [ ] Notification permission denial and settings rollback
- [ ] Asset provenance/license, missing idle, path traversal, source typo normalization

---

## Validation Commands

Task 1 must create these exact scripts; until Task 1, only `git diff --check` is available.

### Static Analysis

```bash
pnpm check
pnpm lint
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
```

EXPECT: zero type/lint/format/clippy errors.

### Unit and Coverage

```bash
pnpm test:coverage
cargo test --manifest-path src-tauri/Cargo.toml --all-features
```

EXPECT: all tests pass; domain/state/policy modules ≥80% line and branch coverage.

### Build

```bash
pnpm build
pnpm tauri build --debug
```

EXPECT: renderer and current-native debug bundle build successfully.

### E2E

```bash
pnpm test:e2e
```

EXPECT: critical flows pass against fakes; no production network.

### Security / Dependency

```bash
pnpm audit --audit-level high
cargo audit
git diff --check
git grep -nE '(api[_-]?key|authorization: bearer|refresh[_-]?token)' -- ':!tests/fixtures/**' ':!docs/**'
```

EXPECT: no high/critical unresolved advisory, whitespace errors, or hardcoded secrets. Task 9 installs a pinned `cargo-audit` release on CI before this command. Review grep matches manually; type/field names alone are not findings.

### Native Matrix

```text
GitHub Actions: ci.yml → check/lint/unit/build
native-smoke.yml → windows-latest, macos-latest, ubuntu X11 + Wayland job
release.yml → MSI, NSIS, signed/notarized DMG, AppImage, checksums
```

EXPECT: required OS jobs green; signing jobs run only with protected CI secrets.

### Manual Validation

- [ ] Fresh signed-in Claude and Codex show independent values/tabs.
- [ ] Primary switch immediately changes overlay without fetching.
- [ ] Click opens panel; 4px boundary reliably separates click/drag.
- [ ] Position survives restart and returns onscreen after monitor removal.
- [ ] Background refresh retains existing values; stale/expiry are visibly correct.
- [ ] Missing login, CLI, offline, provider/parse error, loading badges/messages are distinct.
- [ ] Fullscreen hides UI while mock collector revision continues.
- [ ] No credential/raw response appears in renderer devtools, cache, logs.
- [ ] v1.1 notifications are silent by default and permission-aware.
- [ ] v1.1 graph shows gaps/resets without fabricated interpolation.

---

## Acceptance Criteria

### MVP

- [ ] Claude OAuth and Codex app-server normalize 5h/weekly provider-calculated usage.
- [ ] Renderer receives only allowlisted credential-free DTOs.
- [ ] Provider state, retry, cache, panel remain independent.
- [ ] All six system states, stale decorator, dual severity, mood, reset events satisfy contract tests.
- [ ] 1a split ring, system badge, two-provider panel, bubbles match UI contract.
- [ ] `idle` package renders; missing optional state always falls back safely.
- [ ] Drag/restore/clamp, panel anchor, fullscreen hide, autostart work or visibly degrade per OS.
- [ ] MSI/NSIS, unsigned/ad-hoc-signing validation DMG, AppImage and checksums are produced on native runners.
- [ ] Production-signed/notarized DMG release gate runs only when protected signing credentials are configured; absence of credentials does not block code-complete MVP, but blocks public macOS release.
- [ ] No default test contacts production Anthropic/OpenAI.
- [ ] All validation commands pass and target modules meet 80% coverage.

### v1.1

- [ ] Optional animation keys and strict idle fallback work.
- [ ] Native notifications are opt-in, permission-aware, primary-only by default, deduped.
- [ ] History schema is documented, migrated, bounded, provider-independent, non-interpolating.
- [ ] Secondary-provider notification option does not alter overlay/pet primary semantics.
- [ ] Deferred fallback collectors, if enabled, pass timeout/noninteractive/redaction fixtures.

## Completion Checklist

- [ ] Every task executed RED→GREEN→REFACTOR with relevant tests
- [ ] Rust/TS naming and immutability conventions applied
- [ ] No file >800 lines; functions generally <50 lines and nesting ≤4
- [ ] Error classification and user messages match contract
- [ ] Structured logs redact forbidden data
- [ ] IPC/capability least privilege reviewed
- [ ] Store migrations and atomic writes verified
- [ ] Code review addresses all critical/high findings
- [ ] Security review completed for credentials, provider input, IPC, logs, CI secrets
- [ ] README/source contracts updated without duplicating temporary notes
- [ ] No unrelated prototype/generated files promoted into production

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---:|---:|---|
| Internal provider contracts drift | High | High | isolated parsers, fixtures/variants, typed failure, no UI coupling, fallback feature flags |
| Credential/log/DTO leak | Medium | Critical | secret wrapper, DTO allowlist, redaction/property tests, no renderer provider access |
| OS fullscreen/always-on-top/Wayland differences | High | High | adapter capability result, visible degradation, native smoke matrix |
| MVP scope is XL greenfield | High | High | dependency-gated tasks; release only after vertical slice Tasks 1–8 and Task 9 gates |
| Collector fallback scope ambiguous | Medium | Medium | explicit Task 4 product gate; primary paths are mandatory, fallback adapters independently testable |
| Prototype assets untracked, ~31MB, inconsistent names/provenance | High | Medium | curate/copy into canonical package, provenance/license gate, never bind source paths |
| History underspecified/latest-only schema | High | Medium | Task 12 documents retention/sampling/gaps before implementation and adds migration |
| Separate panel focus/z-order race | Medium | Medium | lifecycle state machine, revisioned visibility events, edge/native E2E |
| Signing/notarization credentials unavailable | Medium | High | unsigned/ad-hoc validation artifact proves packaging; public macOS release remains explicitly gated on protected secrets |
| Tauri/plugin API/version drift | Medium | Medium | official docs, pinned lockfiles/toolchain, dependabot/audit, implementation-time API verification |

## Notes

- `docs/UI-plan/` is currently untracked. Preserve user files; explicitly choose which prototype files/assets become tracked documentation versus curated production assets.
- Design Canvas `support.js` and `image-slot.js` are generated prototype infrastructure, not Svelte/Tauri implementation precedent.
- The UI prototype visually maps Claude→cat and Codex→corgi, but the architecture says pet packages are independent. Treat those as demo defaults only.
- Source assets contain `dog` vs `corgi` and `exhusted` vs `exhausted`; canonical destination names must follow manifest/UI contract.
- No PRD phase status is changed because all inputs are reference documents, not a PRD with pending phase metadata.

## Confidence Score

**8/10** for a single-pass staged implementation. The plan removes known contract ambiguity and captures the greenfield file/test map; remaining uncertainty is primarily native OS behavior, internal provider contract drift, signing credentials, and the explicit fallback product gate.
