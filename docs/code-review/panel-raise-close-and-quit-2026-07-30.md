# Code Review: 로컬 미커밋 변경분 (panel-raise-close-and-quit)

**리뷰 일자**: 2026-07-30
**브랜치**: `develop` (공유 브랜치, 미커밋 작업 트리)
**베이스**: `213f997` (Merge pull request #12 from chanhoan/chore/beta-prep)
**범위**: 수정 13개 파일 (+181 / −17) — 네이티브 3, 네이티브 설정 1, 렌더러 2, 테스트 4, 문서 3
**결정**: **REQUEST CHANGES** — CRITICAL 0, **HIGH 1**, MEDIUM 5, LOW 7

---

## Summary

issue #29의 세 가지 요구(더블클릭 무반응, 외부 클릭 시 패널이 뒤로 밀림, 명시적 닫기·실제 종료)를 구현한 변경. **진단과 구조 설계는 정확하다.** 특히:

- 패널을 노출하는 **세 경로**(조기 반환 → 전면화, grace 타이머, `resize_panel` 게이트 플립)를 `reveal_panel()` 한 곳으로 모았다(`ipc.rs:276-289`, `:309`, `:323`, `:423`). "더블클릭이 무반응"이라는 버그 클래스를 경로별 누락으로 재발시키지 않는 구조다.
- `is_minimized()` → `unminimize()`를 `show()` 앞에 둔 순서 판단이 옳다. 최소화된 창도 `is_visible() == true`를 보고하므로 이 순서가 아니면 포커스가 빈 곳으로 간다(`ipc.rs:277-281`).
- `set_focus()`를 best-effort로 남긴 판단(`ipc.rs:283-287`)이 `position_panel`의 기존 정책과 일치한다. 헤드리스 러너·Wayland 컴포지터의 포커스 거부를 에러로 승격시키지 않아 `native-smoke.yml` 픽스처 잡을 깨지 않는다.
- `resize_panel`에서 게이트가 무장된 경우에만 노출하므로, **탭 전환으로 높이가 바뀔 때 포커스를 훔치지 않는다**(`ipc.rs:422-424`). 새 `set_focus()`가 유발할 수 있었던 회귀를 이미 막고 있다.
- `show_panel`이 가시성 검사보다 `position_panel`을 **먼저** 호출하므로(`ipc.rs:303-311`), 이미 열린 패널을 다른 디스플레이로 옮긴 펫 옆으로 재앵커한다. 멀티스크린 요구가 전면화 경로에서도 성립한다.
- `✕`를 흐름 밖 절대 위치로 둔 접근은 `ResizeObserver` 측정 높이 불변을 실제로 보존하며, e2e 회귀 테스트로 고정했다(`renderer.spec.ts:214-247`, 실브라우저 6/6 통과).
- 프라이버시 계약과 무관하다 — 신규 IPC 커맨드 없음, `quit`은 기존 커맨드이며 `command_allowed`에서 여전히 `panel` 전용이다(`window/mod.rs:88-101`). 신규 로그·DTO 필드 없음.

막아야 할 지점은 하나다. **`✕`가 Codex 탭의 클릭 가능 영역 위에 겹쳐 있어, 탭 우측을 노린 클릭이 패널을 닫는다.** 구현 보고서는 이를 "사용자가 리플로 없는 순수 오버레이를 요구했으므로" 수용한 트레이드오프로 기록했지만, **그 전제가 성립하지 않는다** — 보존해야 하는 불변은 `resize_panel`이 나르는 *높이*이고(`ipc.rs:415`, 너비는 `PANEL_WIDTH_LOGICAL` 고정), 폭 방향 보정은 그 불변을 건드리지 않는다. 즉 제약을 지키면서 겹침을 없앨 수 있다.

또한 **계획서가 "필수"로 지정한 네이티브 수동 검증 8항목이 전부 미수행**이며, 그린인 `panel_reveal` 단위 테스트는 실제로 고친 동작(unminimize → show → set_focus)의 증거가 아니다. 이 점은 보고서 자신이 명시하고 있다.

---

## Findings

### CRITICAL

없음.

- 하드코딩된 자격 증명·토큰·API 키 없음
- 신규 `console.log`·디버그 출력 없음, `TODO`/`FIXME` 없음
- 렌더러 DTO·로그 표면 무변경 → 프라이버시 계약 유지
- 신규 IPC 커맨드 없음. `quit`의 인가는 `panel` 라벨 전용 유지(`overlay`는 호출 불가)
- CSP·asset 프로토콜 스코프 무변경
- 사용자 입력 파싱·경로 조작·외부 요청 경로 무변경
- 신규 함수 전부 50줄 미만, 변경 파일 전부 800줄 미만, 중첩 4단계 이하

---

### HIGH

#### H1. `✕`가 Codex 탭 위 14×18px을 덮어, 탭을 노린 클릭이 패널을 닫는다

**위치**: `src/lib/components/UsagePanel.svelte:29-34`, `:111-115` / `src/lib/components/ProviderTabs.svelte:34-40`

`.close-panel`은 `.usage-panel`(= 패널 콘텐츠 박스) 기준 `top: 0.375rem; right: 0.375rem`에 `1.5rem × 1.5rem`, `z-index: 2`로 놓인다. 탭 스트립은 `header { padding: var(--space-3) var(--space-4) 0 }` 안에서 `flex: 1` 두 버튼이 폭을 나눈다.

312px 패널(테두리 1px 양쪽 → 콘텐츠 310px) 기준 계산:

| 요소 | x 범위 | y 범위 |
|---|---|---|
| Codex 탭 | 155 → 294 | 12 → 약 54 |
| `✕` 버튼 | 280 → 304 | 6 → 30 |
| **겹침** | **280 → 294 (14px)** | **12 → 30 (18px)** |

겹침 영역에서는 `z-index: 2`인 `✕`가 포인터 이벤트를 받는다(`background: transparent`는 히트 테스트를 비우지 않는다). 결과:

1. Codex 탭 우측 상단을 클릭하면 **탭이 선택되지 않고 패널이 사라진다.**
2. 겹침은 Codex 탭 면적의 약 4%, 폭의 10%다. 탭 우측을 겨냥하는 사용자에게 반복 재현된다.

WCAG 2.2 SC 2.5.8 관점에서도 `✕`는 24×24px 최소치를 정확히 만족하지만, **인접 타깃과의 간격이 0이 아니라 음수(겹침)** 다.

구현 보고서도 동일한 수치(14×18px)를 스크린샷으로 확인해 기록했다. 즉 알려진 사실이며, 쟁점은 수용 근거다.

**보고서의 수용 근거가 성립하지 않는 이유**: `resize_panel`은 **높이만** 나르고(`ipc.rs:405`, `:415`) 너비는 `PANEL_WIDTH_LOGICAL = 312.0` 고정이다. 따라서 폭 방향 보정은 `ResizeObserver` 측정 높이 불변과 무관하다.

**수정안** (측정 높이 불변 유지):

```css
/* UsagePanel.svelte — 탭 스트립만 폭을 양보. 높이 무변경 */
header {
  padding: var(--space-3) calc(var(--space-4) + 1.5rem) 0 var(--space-4);
}
```

탭 하단 경계선이 우측에서 24px 짧아지는 시각 변화가 수반된다. 이를 원치 않으면 `.tabs`에 `margin-right: 1.5rem`을 주는 대안도 동일하게 높이를 보존한다.

**대안(수정하지 않을 경우)**: 겹침을 의도된 계약으로 `ui-contract.md`에 명시하고, e2e에 겹침 폭 상한을 고정하는 어서션을 추가해 향후 아이콘 크기·헤더 패딩 변경 시 조용히 악화되지 않게 해야 한다. 현재 신규 e2e 테스트(`renderer.spec.ts:214-247`)는 `closeInsideShell`만 검사하고 탭과의 겹침은 검사하지 않는다.

---

### MEDIUM

#### M1. 시작 실패·로딩 상태의 패널은 `✕`도 `Quit`도 없는데, 이제 항상 위에 뜬다

**위치**: `src/App.svelte:635-639` / `src-tauri/tauri.conf.json:43`

`startupState`가 `'loading'`/`'error'`인 분기는 `windowLabel === 'panel'` 검사보다 앞에 있어 `UsagePanel`이 마운트되지 않는다. 즉 `✕`와 푸터 `Quit` 둘 다 없다. 이 상태는 도달 가능하다 — `resize_panel`이 오지 않아도 150ms grace 타이머가 패널을 그대로 노출한다(`ipc.rs:315-326`).

이번 변경으로 패널은 `alwaysOnTop: true` + `decorations: false` + `transparent: true`가 되었다. 렌더러 시작이 실패하면 사용자에게는 **닫기 수단이 UI에 없는 항상-위 프레임리스 창**이 남는다. 복구는 작업표시줄 항목(패널에 `skipTaskbar`가 없어 존재) 우클릭 → 창 닫기뿐이며, `beta-testing.md`가 새로 약속한 "작업관리자가 필요할 일은 없다"와 긴장 관계에 있다.

**권장**: 시작 실패/로딩 분기에도 `✕`(또는 `Quit`)를 노출한다. 셸 레벨로 올리는 것이 가장 단순하다.

#### M2. Settings 화면에서는 `✕`가 사라져, 새로 쓴 계약과 어긋난다

**위치**: `src/App.svelte:641-655` / `docs/ui-contract.md:199-200`

`showSettings === true`면 `UsagePanel`이 언마운트되어 `✕`가 없다. 그런데 계약 문서는 "유일한 닫기 수단은 헤더 우측 상단의 `✕`"라고 무조건으로 단정한다. 실제로는 두 상태 중 하나에서만 성립한다.

구현 보고서는 이를 범위 밖으로 남겼다고 기록했다 — 판단은 존중한다. 다만 **계약 문서가 코드보다 강한 주장을 하는 상태**는 남겨두면 안 된다. `← Back` 경유가 필요하다는 예외를 문서에 적거나, `✕`를 셸 레벨로 올려 두 화면 모두에서 유지한다(M1과 같은 수정으로 동시 해소된다).

#### M3. "항상 위"를 계약으로 단정했지만, 플랫폼 계층은 이를 `unavailable`로 보고하고 아무도 표면화하지 않는다

**위치**: `docs/ui-contract.md:199` / `src-tauri/src/refresh/ipc.rs:209-216`

`get_platform_capabilities`는 `always_on_top`을 **무조건** `Unavailable { reason: "always-on-top support is unverified on this platform build" }`로 보고한다. `CLAUDE.md` 불변("미검증 capability는 unavailable을 보고한다")과는 일관되지만, 새 계약 문장은 "패널은 항상 위로 떠 있고"라고 조건 없이 단정한다.

게다가 렌더러는 `always_on_top`을 **어디에서도 소비하지 않는다** — `gateway.ts:61`의 타입 선언과 픽스처뿐이다. 패널은 `autostart`·`fullscreen_detection`의 `unavailable` 이유를 `role="status"`로 노출하지만(`App.svelte:677-682`), `always_on_top`만 표면이 없다.

**영향**: `alwaysOnTop`을 무시하는 컴포지터(다수의 Wayland 구성)에서 issue #29의 버그 #1이 **조용히 재발**한다. 사용자는 진단을 못 받고, 앱은 이를 실패로 보고하지도 않는다. 완화 요소는 이번에 고친 전면화 경로가 복구 수단이 된다는 점이다.

**권장**: 문서 문장을 "지원되는 플랫폼에서 항상 위"로 완화하고, `always_on_top`의 `unavailable` 이유도 나머지 두 capability와 같은 방식으로 패널에 노출한다.

#### M4. `✕` 기본 상태 색이 WCAG AA 텍스트 대비에 미달한다

**위치**: `src/lib/components/UsagePanel.svelte:125` / `src/lib/styles/tokens.css:10`, `:48`

`color: var(--color-text-faint)`, `font-size: 1rem`(16px, 확대 텍스트 아님) 조합의 대비:

| 테마 | 전경 | 배경 | 대비 | AA(4.5:1) |
|---|---|---|---|---|
| 라이트 | `#8a9099` | `rgb(250 250 252 / 92%)` | 약 3.1:1 | 미달 |
| 다크 | `#6b7280` | `rgb(28 31 36 / 92%)` | 약 3.4:1 | 미달 |

UI 컴포넌트 그래픽(1.4.11, 3:1) 기준으로는 간신히 통과하지만, `×`는 실제로 텍스트 노드로 렌더된다. 형제 `.ghost-action`이 쓰는 `--color-text-muted`는 라이트에서 약 6:1로 통과한다 — **이번에 추가된 컨트롤만 더 흐린 토큰을 골랐다.**

**권장**: `--color-text-faint` → `--color-text-muted`. hover/focus 상태는 이미 `--color-text`로 올라가므로 변경 불필요.

#### M5. `panel_reveal` 테스트는 항등 매핑을 검증하며, 실제로 고친 동작에는 자동 커버리지가 없다

**위치**: `src-tauri/src/window/mod.rs:106-129` / `src-tauri/src/window/tests.rs:345-350`

`panel_reveal(bool) -> PanelReveal`은 분기 로직이 없는 bool → enum 항등 매핑이고, 테스트는 그 매핑을 그대로 어서션한다. 정작 버그였던 동작 — unminimize가 show보다 앞서는가, 전면화 경로에서 게이트를 건드리지 않는가, `set_focus` 실패가 에러로 승격되지 않는가 — 는 `reveal_panel`(비공개, Tauri 창 의존)에 있어 검증되지 않는다.

Tauri 창을 모킹할 수 없으니 이 구조 자체는 실용적 타협으로 수용한다. 문제는 **커버리지 착시**다: 보고서가 기록한 8개 수동 검증 항목이 여전히 유일한 실제 게이트이고, 그중 하나도 수행되지 않았다.

**권장**: 커밋 전에 `pnpm tauri dev`로 최소 다음 3개를 확인한다 — (a) 다른 앱 클릭 후 펫 재더블클릭 시 전면 복귀, (b) 최소화된 패널 더블클릭 시 unminimize, (c) 푸터 `Quit` 후 프로세스 부재. 문서화된 8항목 중 이 세 개가 이번 diff의 핵심 주장에 직접 대응한다.

---

### LOW

1. **`pub` 가시성** — `panel_reveal`/`PanelReveal`(`window/mod.rs:106-129`)은 크레이트 내부(`refresh/ipc.rs`)에서만 쓰이지만 `pub`이라 라이브러리 공개 API가 넓어진다. 프로젝트 Rust 규칙("내부 공유는 `pub(crate)`")대로면 `pub(crate)`다. 다만 같은 모듈의 `command_allowed`·`NativeCommand`도 `pub`이므로 기존 관례와는 일관된다 — 일괄 정리 대상으로만 기록한다.
2. **조용한 no-op 기본값** — `onQuit = () => {}`, `onClose = () => {}`(`UsagePanel.svelte:18-19`). 배선을 빠뜨리면 `Quit` 버튼이 아무 신호 없이 죽는다. 기존 패턴과 동일하지만, 이제 두 컨트롤이 서로 다른 파괴적 동작을 가리키므로 실수 비용이 올라갔다.
3. **테스트 1개에 두 행동** — `UsagePanel.test.ts:130-155`가 `✕` → `onClose`와 `Quit` → `onQuit`를 한 테스트에서 검증한다. 실패 시 어느 배선이 깨졌는지 이름으로 드러나지 않는다. 두 개로 분리 권장.
4. **툴팁 없음** — `✕`는 `aria-label`만 있고 `title`이 없다. 스크린 리더 사용자는 안내를 받지만 마우스 사용자는 호버 힌트가 없다.
5. **e2e 어서션 취약성** — `expect(geometry.headerWidth).toBe(geometry.panelInnerWidth)`(`renderer.spec.ts:244`)는 `Math.round(fractional rect)`와 정수 `clientWidth`를 등호로 비교한다. DPR 1에서는 안전하지만 분수 배율 환경에서 1px 오차로 깨질 수 있다. `toBeCloseTo` 또는 ±1 허용 비교가 더 견고하다.
6. **AppIndicator 언급 정리 잔여** — `beta-testing.md`의 "트레이 아이콘을 제공하지 않는다"는 정확하다(소스 전체에 트레이 코드 없음, `tauri` 크레이트 features는 `protocol-asset`뿐). 다만 `ci.yml:35`·`native-smoke.yml:56,100`·`release.yml:50`과 `CLAUDE.md` 사전 요구 항목은 여전히 `libappindicator3-dev`를 설치·명시한다. 빌드타임 대 런타임 구분이라 모순은 아니지만, 빌드 의존이 실제로 필요한지 확인하는 후속 항목으로 남길 만하다.
7. **`✕` 타깃이 최소치 정확히 24×24px** — SC 2.5.8을 만족하되 여유가 없다. H1의 겹침 문제와 함께 다루면 자연히 해소된다.

---

## Validation Results

| Check | Result | Notes |
|---|---|---|
| svelte-check | Pass | `pnpm test:ci` 내 |
| eslint + prettier | Pass | `pnpm test:ci` 내 |
| Vitest + 커버리지 게이트(80%) | Pass | `src/lib/components` 98.14% stmt / 90.09% branch / 83.33% func |
| vite build | Pass | 156 modules, `index-*.js` 93.11 kB (gzip 31.59 kB) |
| `cargo clippy --all-features --all-targets -- -D warnings` | Pass | 경고 0 |
| `cargo test --all-features` | Pass | 108 passed, 0 failed |
| 렌더러 E2E (실브라우저, Chrome 150) | Pass | 6 passing — 신규 `layers the close control…` 포함 |
| 네이티브 수동 검증 (계획서 "필수" 8항목) | **미수행** | `pnpm tauri dev` 필요. M5 참조 |

E2E는 `pnpm dev --host 127.0.0.1` 선행 기동 후 실행했다(설정에 `onPrepare` 훅이 없어 서버를 직접 띄우지 않는다). 리뷰 후 개발 서버는 종료했다.

---

## Files Reviewed

| File | Action | 판정 |
|---|---|---|
| `src-tauri/src/refresh/ipc.rs` | Modified | 승인 — 단일 노출 경로 정리가 정확 |
| `src-tauri/src/window/mod.rs` | Modified | 승인 (LOW 1) |
| `src-tauri/src/window/tests.rs` | Modified | 조건부 — 커버리지 착시 주의 (M5) |
| `src-tauri/tauri.conf.json` | Modified | 조건부 — 미검증 capability 의존 (M3), 항상-위 부작용 (M1) |
| `src/lib/components/UsagePanel.svelte` | Modified | **수정 요청 (H1)**, 추가로 M4·LOW 2/4 |
| `src/App.svelte` | Modified | 조건부 — 시작 실패·Settings 상태에 닫기 수단 없음 (M1, M2) |
| `src/lib/components/UsagePanel.test.ts` | Modified | 승인 (LOW 3) |
| `src/App.test.ts` | Modified | 승인 — `✕`/`Quit` 분리 검증 적절 |
| `src/lib/api/gateway.test.ts` | Modified | 승인 — 커맨드명 오타 방지 어서션 |
| `tests/e2e/renderer.spec.ts` | Modified | 승인 (LOW 5), 단 탭 겹침 미검사 (H1) |
| `docs/ui-contract.md` | Modified | 조건부 — 코드보다 강한 단정 (M2, M3) |
| `docs/beta-testing.md` | Modified | 승인 (LOW 6), 단 M1과 긴장 관계 |
| `CLAUDE.md` | Modified | 승인 — 불변 문장이 실제 구현과 일치 |

---

## Next Steps

1. **H1 수정** — 헤더 우측 패딩(또는 `.tabs` 우측 마진) 24px로 겹침 제거. 측정 높이 불변은 영향받지 않는다. 수정하지 않기로 결정하면 계약 문서에 명시 + e2e 겹침 상한 어서션 추가.
2. **M1/M2 동시 해소** — `✕`를 패널 셸 레벨로 올려 시작 실패·Settings 화면에서도 유지.
3. **M3** — `ui-contract.md` 문장 완화 + `always_on_top` 진단을 패널에 노출.
4. **M4** — `--color-text-faint` → `--color-text-muted`.
5. **M5** — `pnpm tauri dev`로 수동 검증 8항목 수행 (최소 (a)(b)(c) 3개).
6. 그 후 `fix/` 브랜치로 분기해 커밋. 현재 변경은 공유 브랜치 `develop`의 작업 트리에 있다.
