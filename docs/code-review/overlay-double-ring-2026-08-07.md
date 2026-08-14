# Code Review: Overlay Double Ring (satellite + orbiting primary mark)

**Reviewed**: 2026-08-07
**Branch**: `feat/overlay-double-ring`
**Scope**: `HEAD` 대비 로컬 변경사항(수정 22개 파일) + 미추적 신규 파일(`orbitPath.ts`, `orbitPath.test.ts`, `SatelliteRing.svelte`, `ProviderLogo.svelte`, `docs/UI-plan-2.0/`)
**Decision**: REQUEST CHANGES — HIGH 1건(시각적 결함, 수정 1줄)

## Summary

`ring_mode`(schema v6)를 표시 전용 설정으로 도입한 설계는 CLAUDE.md에 새로 명문화한 불변식과 정확히 일치한다. 큰 원은 항상 `primary_provider`에 묶여 있고, 작은 원은 `secondaryProvider(primary)`로 파생만 되며 어디에도 되쓰기(write-back)하지 않는다. 수집·새로고침·알림·펫 무드 경로에는 `ring_mode`가 전혀 닿지 않는다. Rust 마이그레이션(v5 → v6)은 `deny_unknown_fields` + `schema_version` 가드 조합을 기존 arm들과 동일한 패턴으로 유지했고, 왕복 저장 테스트까지 있다. 보안 결함, 자격증명 노출, 디버그 잔재는 없다.

차단 사유는 하나다: `PetOverlay`의 `.overlay :global(.ring)` 규칙이 `SatelliteRing`의 동일 특이도 규칙보다 나중에 배치되어, 작은 원의 지오메트리(`inset: 8%; width: 84%`)가 통째로 무시되고 있다. 빌드 산출물에서 확인된 사실이며 결과는 미미한 시각 차이지만, 코드가 선언한 동작과 실제 동작이 다르고 향후 해당 값을 조정해도 아무 반응이 없는 상태다.

## Findings

### CRITICAL

None.

### HIGH

**[HIGH]** `src/lib/components/PetOverlay.svelte:128-133` (+ `src/lib/components/SatelliteRing.svelte:48-53`)

Issue: 두 규칙의 특이도가 동일(클래스 3개)한데 번들 순서상 `.overlay`가 뒤에 온다. 따라서 위성 링에는 `.satellite ... .ring`이 아니라 `.overlay ... .ring`이 적용된다.

```
dist/assets/index-*.css
 6000: .satellite.svelte-4bbdo5 .ring { position:absolute; inset:8%;  width:84%;  height:84%  }
 6315: .overlay.svelte-1g9uxux  .ring { position:absolute; inset:0;   width:100%; height:100% }   ← 나중 = 승리
```

`.overlay :global(.ring)`는 자손 결합자라서 `section.overlay > div.satellite > svg.ring`도 함께 매칭한다. 위성 링은 퍽(puck) 안쪽 8% 여백 없이 가장자리에 딱 붙어 렌더되며, `SatelliteRing`의 지오메트리 선언과 그 위의 주석("Geometry only")은 사실상 죽은 코드다. 지금 화면이 의도치 않게 "괜찮아 보이는" 상태일 뿐, 누군가 `inset: 8%`를 조정해도 아무 변화가 없어 디버깅 비용이 발생한다. CSS 순서는 import 그래프에 우연히 의존하므로 향후 리팩터링에서 반대로 뒤집힐 수도 있다.

Fix: 큰 원은 `.overlay`의 직계 자식이고 위성 링은 한 단계 더 깊으므로, 자식 결합자로 좁히면 충돌 자체가 사라진다.

```css
/* PetOverlay.svelte */
.overlay > :global(.ring) {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
}
```

수정 후에는 위성 링이 실제로 8% 인셋으로 렌더되는지 육안 확인이 필요하다(현재 화면 대비 약 16% 작아진다). 그 결과가 원치 않는다면 반대로 `SatelliteRing`의 지오메트리 규칙을 삭제하고 "퍽에 꽉 차게 그린다"를 의도로 명시하는 쪽이 옳다. 어느 쪽이든 선언과 동작을 일치시켜야 한다.

### MEDIUM

**[MEDIUM]** `src/App.svelte:816-826`, `src/lib/components/PetOverlay.svelte:35`

Issue: 워커 경로는 `orbitPath(model.size, …)`로 **픽셀 절대 좌표**를 만들지만, 실제 `.overlay` 엘리먼트 폭은 말풍선이 뜨는 순간 CSS로 다시 잘린다.

```css
.overlay-stack.toast-visible :global(.overlay) { max-width: min(9.5rem, 100vw); }  /* 152px */
```

`model.size`는 그대로이므로 `offset-path`의 반지름과 중심이 축소된 링과 어긋난다. 위성(`right/bottom/width`가 %)은 함께 줄어드는 반면 워커만 옛 크기의 궤도를 돌게 되어, 말풍선이 떠 있는 동안 마크가 링에서 떨어져 공중을 도는 상태가 된다.

현재 번들 펫 3종(`corgi`/`momo`/`tabby`)은 모두 `defaultSize` 128px이고 `overlaySize = min(240 / 1.366, 128) = 128 < 152`라 이 경로는 **아직 도달 불가**다. 다만 CLAUDE.md가 "펫 추가는 폴더 추가일 뿐"이라고 명시한 이상, 153px 이상을 선언한 서드파티 패키지가 들어오면 곧바로 재현된다.

Fix: `.overlay`의 실제 렌더 폭을 단일 출처로 만든다. 가장 단순한 방법은 토스트 가시성을 `overlaySize` 계산에 반영해 `model.size` 자체를 152로 낮추는 것이고(그러면 CSS `max-width` 클램프는 제거), 아니면 `offset-path`를 픽셀 대신 `%` 기반 좌표계로 바꿔 크기 변화에 자동으로 따라가게 한다.

**[MEDIUM]** `src/App.svelte:548-608` / `src/App.test.ts`

Issue: 이번 기능의 배선(wiring)에 대한 조합 루트 테스트가 없다. `App.test.ts`는 픽스처에 `ringMode: 'single'` 필드를 추가했을 뿐이고, `double`로 설정했을 때 위성이 실제로 그려지는지, `panelProviders[satelliteProvider]`가 올바른 provider의 사용량을 실어 나르는지, `orbit.provider`가 primary를 따르는지 검증하지 않는다. `PetOverlay.test.ts`는 컴포넌트에 모델을 직접 주입해서 테스트하므로 이 구간을 덮지 못한다.

즉 `overlayModel`에서 `satellite:` 또는 `orbit:` 필드가 통째로 빠지거나 primary/secondary가 뒤바뀌어도 CI는 전부 통과한다. 렌더러 E2E의 `limits overlay hit testing to the circular surface`도 픽스처가 `single`이라 위성 히트 영역을 전혀 밟지 않는다.

Fix: `App.test.ts`에 `getSettings`가 `ringMode: 'double'`을 반환하는 케이스를 추가해 (1) `provider-logo-*` testid가 secondary 로고를 렌더하는지, (2) 워커의 `aria-label`이 primary를 가리키는지, (3) `secondaryNotificationsEnabled` 등 알림 경로가 `ringMode`에 반응하지 **않는지**(불변식의 핵심)를 확인한다.

**[MEDIUM]** `docs/ui-contract.md:167-200`

Issue: §4.4로 링 모드는 잘 문서화됐지만, 이번 변경의 절반인 **궤도 워커**가 계약 문서에 전혀 없다(`walker`/`orbit`/`궤도` 검색 결과 0건). 문서화되지 않은 항목: 워커의 지오메트리와 `OVERLAY_BOUNDS_FACTOR` 클램프(오버레이 크기 상한이 240 → 175.7px로 바뀐 근거), `role="img"` + `"Primary provider: …"` 접근명, `prefers-reduced-motion` 처리, 회전 방향이 실행마다 무작위라는 사실, 그리고 새 토큰 `--logo-claude` / `--logo-codex-*` / `--satellite-puck`의 테마 불변 규칙. CLAUDE.md의 새 불변식 항목도 링 모드만 다루고 워커는 언급하지 않는다.

`docs/ui-contract.md`는 저장소가 "표시 계약의 source of truth"로 선언한 문서이므로, 오버레이에 상시 존재하는 신규 요소가 빠지면 문서의 권위가 깨진다.

Fix: §4.4에 워커 소절을 추가하고, 특히 "오버레이 크기 상한은 이제 펫이 아니라 워커의 도달 범위가 결정한다"는 점을 명시한다.

**[MEDIUM]** `src/App.svelte:553-556`

Issue: `Math.random()`이 조합 루트에서 직접 호출된다. 그 결과 (1) 방향 로직은 단위 테스트로 고정할 수 없고, (2) 향후 오버레이 스크린샷/비주얼 회귀 테스트는 구조적으로 flaky해지며, (3) 워커를 렌더하지 않는 panel 창에서도 매번 실행된다. 프로젝트가 정책을 순수 리듀서로 분리해 테스트해 온 규약(`interaction/`)과도 어긋난다.

Fix: 방향 선택을 주입 가능한 형태로 빼거나(예: `orbitDirection(seed: number)` 순수 함수 + 호출부에서만 난수 공급), 최소한 `windowLabel === 'overlay'`일 때만 계산한다.

### LOW

**[LOW]** `src/lib/components/PetOverlay.svelte:181-183`

`offset-path` / `offset-anchor` / `offset-rotate`는 Chromium·WebKit 최신 버전에서만 완전 동작한다(WebKit은 Safari 16 계열 이후). 미지원 엔진에서는 애니메이션만 빠지는 게 아니라 워커가 `top:0; left:0` 즉 오버레이 **좌상단 모서리에 박제**된다 — 원 밖에 로고가 붙어 있는 명백한 깨짐이다. 프로젝트는 Linux X11/Wayland를 native-smoke에서 정식 지원 대상으로 두고 있으므로, 구형 WebKitGTK에서의 폴백을 정해 두는 편이 안전하다.

Fix: `@supports not (offset-path: path("M 0 0")) { .walker { display: none } }` 한 블록이면 "안 보이는" 쪽으로 안전하게 퇴화한다.

**[LOW]** `src/lib/components/orbitPath.ts:104-109`

두 원이 교차하지 않는 상수 조합에서는 `Math.acos(|x| > 1)`이 `NaN`을 반환하고, 그 `NaN`이 그대로 `A NaN NaN …` 경로 문자열로 흘러나간다(브라우저는 경로를 조용히 무시). 현재 상수(중심거리 48.51, 반지름 52.25 / 27)로는 도달 불가이며 테스트도 이를 검증하지만, 상수가 `export`되어 조정 대상이라는 점을 감안하면 방어가 한 줄이다.

Fix: `const clamp = (v) => Math.min(1, Math.max(-1, v))`를 두 `acos` 인자에 적용하거나, 비교차 시 `hasSatellite=false` 경로로 폴백한다.

**[LOW]** `src/App.svelte:588`, `src/App.svelte:606`

`provider === 'claude' ? 'Claude' : 'Codex'` 매핑이 같은 파일 안에서 두 번 반복된다. provider 표시명은 `contracts/domain.ts`에 상수 맵으로 두고 두 곳이 공유하는 편이 낫다(향후 provider 추가 시 삼항 연산자가 조용히 틀린 이름을 내놓는 것을 막는다).

**[LOW]** `src/lib/stores/settings.ts:2`

`RingMode`를 `../api/gateway`(와이어 계약 모듈)에서 import한다. 같은 파일이 `Provider`는 `../contracts/domain`에서 가져오고 있어 계층이 섞였다. `RingMode`는 순수 도메인 유니온이므로 `contracts/domain.ts`에 두고 `gateway.ts`가 재수출하는 편이 기존 레이어링과 일치한다.

**[LOW]** `src/lib/components/ProviderLogo.svelte:17`, `docs/UI-plan-2.0/`

`docs/UI-plan-2.0/`(원본 PNG 2종 + HTML 시안)이 아직 미추적 상태다. `ProviderLogo`의 주석이 `docs/UI-plan-2.0/Claudecode_Logo.png`를 출처로 인용하고 있으므로, 커밋에 포함하지 않으면 주석이 존재하지 않는 경로를 가리키게 된다. 아울러 Anthropic·OpenAI의 상표를 벡터로 재현해 배포물에 포함하는 건이므로, CI의 license inventory와 별개로 상표 사용 범위를 한 번 확인해 두는 것을 권한다.

## Validation Results

| Check | Result |
|---|---|
| `svelte-check --tsconfig ./tsconfig.json` | **Pass** — 0 errors, 0 warnings |
| `eslint .` | **Pass** — exit 0 |
| `prettier --check .` | **Fail (무관)** — `.superpowers/sdd/task-4-report.md` 1건. `.superpowers/sdd/.gitignore`로 git 추적 제외 상태이나 `.prettierignore`에는 없다. 이번 변경과 무관한 로컬 잔재이며 CI 클린 체크아웃에는 존재하지 않는다 |
| `vitest run --coverage` | **Pass** — 25 files, 312 tests |
| Coverage | **Pass** — statements 96.25%, branches 90.83%, functions 91.13%, lines 96.25% (게이트 80%) |
| `vite build` | **Pass** — 2.23s, JS 104.11 kB / gzip 35.26 kB |
| `wdio run ./wdio.browser.conf.ts` | **Pass** — 2 specs, 8 tests |
| `cargo test --all-features` | **Pass** — 146 tests |
| `cargo clippy --all-features --all-targets -- -D warnings` | **Pass** |
| `cargo fmt -- --check` | **Pass** |
| `pnpm test:e2e` (native) | **Not run** — webdriver 피처 디버그 빌드가 선행되어야 하는 별도 절차 |

## 확인한 불변식

| 불변식 | 결과 |
|---|---|
| `ring_mode`가 수집·새로고침·알림·펫 무드에 닿지 않음 | Pass — 렌더러 `App.svelte`에서만 소비되며 `primaryUi`/`notificationPolicy` 경로와 분리 |
| 큰 원은 항상 `primary_provider` | Pass — 연결 상태 기반 스왑/강등 코드 없음 |
| 위성 provider는 파생만, 되쓰기 없음 | Pass — `secondaryProvider()`는 순수 함수이며 `updateSettings` 페이로드에 미포함 |
| 미연결 위성도 링을 유지하고 배지 표시 | Pass — `PetOverlay.test.ts` "badges a failing satellite without blanking the big ring" |
| 렌더러 명령 권한 per-window | Pass — 신규 IPC 명령 없음, 기존 `update_settings`(panel 전용) 재사용 |
| 프라이버시 계약(자격증명/식별자 미노출) | Pass — 신규 DTO는 `ring_mode` 열거형 하나뿐 |
| 설정 스키마 마이그레이션 | Pass — v5 arm은 현재 스키마 arm 뒤·v1/v2/v4 arm 앞에 위치. v5 파일이 `SettingsV4`로도 파싱되지만 `schema_version` 가드가 분리하며, v6 파일은 `deny_unknown_fields`로 v5 arm에 걸리지 않음 |
| 렌더러/Rust 기본값 일치 | Pass — `RingMode::Single` ↔ `defaultSettings.ringMode = 'single'`, 테스트로 고정 |

## Files Reviewed

### Source (Modified)

- `src-tauri/src/store/settings.rs` — `RingMode` 추가, `SETTINGS_SCHEMA_VERSION` 6, `SettingsV5` 마이그레이션 arm
- `src-tauri/src/store/mod.rs` — `RingMode` 재수출
- `src/lib/api/gateway.ts` — `RingMode` 타입, `SettingsWire.ring_mode` 양방향 매핑
- `src/lib/api/fixtureGateway.ts` — 픽스처 스키마 v6
- `src/lib/contracts/domain.ts` — `secondaryProvider()`
- `src/lib/state/presentation.ts` — `ringMode`를 스토어 상태로 통과
- `src/lib/stores/settings.ts` — `SettingsState.ringMode` + 기본값
- `src/lib/components/models.ts` — `SatelliteRingModel`, `OrbitModel`, `OrbitDirection`
- `src/lib/components/PetOverlay.svelte` — 위성 렌더/히트 영역, 궤도 워커
- `src/lib/components/SplitUsageRing.svelte` — `name` prop으로 접근명 분기
- `src/lib/components/SettingsPanel.svelte` — Ring 선택 컨트롤
- `src/lib/styles/tokens.css` — `--logo-*`, `--satellite-puck`
- `src/App.svelte` — `satelliteProvider`/`orbitDirection`/`overlaySize` 파생 및 모델 조립

### Source (Added)

- `src/lib/components/orbitPath.ts` — 궤도 지오메트리 단일 출처
- `src/lib/components/SatelliteRing.svelte`
- `src/lib/components/ProviderLogo.svelte`

### Tests

- `src/lib/components/orbitPath.test.ts` (신규, 5 tests) — 단일/이중 모드 아웃라인, 크기 독립성, 이음매 연속성, 경계 여유
- `src/lib/components/PetOverlay.test.ts` — 이중 링 8건 + 워커 3건 추가
- `src/lib/components/SettingsPanel.test.ts`, `src/lib/state/domain.test.ts`, `src/lib/state/presentation.test.ts`, `src/lib/stores/settings.test.ts`, `src/App.test.ts`
- `src-tauri/src/store/tests.rs` — v5 마이그레이션 + `ring_mode` 왕복 저장

### Docs

- `CLAUDE.md` — "Ring mode is display-only" 불변식
- `docs/ui-contract.md` — §4.4 링 모드

### Untracked (미커밋)

- `docs/UI-plan-2.0/` — 원본 아트 2종 + HTML 시안
- `.claude/PRPs/plans/completed/overlay-double-ring.plan.md`, `.claude/PRPs/reports/overlay-double-ring-report.md`

## 잘한 점

- `orbitPath.ts`가 지오메트리 상수의 단일 출처가 되고, 두 컴포넌트가 CSS custom property로만 읽는 구조 — 숫자 드리프트가 구조적으로 불가능하다.
- `orbitPath.test.ts`가 "모든 앵커가 두 원 중 하나 위에 있고 다른 원 **안쪽으로는 들어가지 않는다**"를 검증한다. 좌표 스냅샷이 아니라 합집합 아웃라인이라는 성질 자체를 테스트한 좋은 예다.
- 접근명 비대칭(큰 원은 generic, 작은 원만 provider명)에 근거를 남겼고, 위성 히트 영역을 `aria-hidden`으로 둔 이유까지 주석과 테스트로 고정했다.
- `ProviderLogo`의 인스턴스 카운터로 SVG id 충돌을 선제 차단했다.
- Rust 마이그레이션에서 v5가 `SettingsV4`로도 파싱된다는 함정을 주석으로 남겼다.
