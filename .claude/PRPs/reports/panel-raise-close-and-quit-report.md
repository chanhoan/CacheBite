# Implementation Report: 클릭 패널 최상단 복귀 · 명시적 닫기(✕) · 실제 종료(Quit)

## Summary

GitHub issue #29와 사용자 직접 제공 요구사항 3건을 구현했다.

1. **버그 #2 (더블클릭 무반응)** — `show_panel`이 `is_visible()==true`일 때 조기 반환하던
   것을 `reveal_panel()`(unminimize → show → set_focus)로 교체했다. 패널을 노출하는 세 경로
   (조기 반환, grace 타이머, `resize_panel` 게이트 플립) 전부 이 헬퍼를 통과한다.
2. **버그 #1 (외부 클릭 시 뒤로 밀림)** — `panel` 창에 `alwaysOnTop: true`를 추가했다.
3. **요구 #3** — 패널 우측 상단에 절대 위치 `✕`(→ `hidePanel()`)를 추가하고, 푸터 `Quit`을
   새 `onQuit` prop을 통해 `gateway.quit()` → `app.exit(0)`에 연결했다. `✕`는 레이아웃
   흐름 밖에 있어 헤더·본문·푸터 치수와 `ResizeObserver` 측정 높이를 바꾸지 않는다.

## Assessment vs Reality

| Metric | Predicted (Plan) | Actual |
|---|---|---|
| Complexity | Medium | Medium — 정확 |
| Confidence | 8/10 | 근거 있었음. 실패한 것은 구현이 아니라 e2e 어서션 1건 |
| Files Changed | 13 | 13 — 정확 |
| Lines | (미예측) | +181 / -17 |

## Tasks Completed

| # | Task | Status | Notes |
|---|---|---|---|
| 1 | `panel_reveal` 순수 정책 함수 + 테스트 | 완료 | `window::tests` 14 → 15개 |
| 2 | `reveal_panel` 헬퍼 + `show_panel` 항상 최상단 | 완료 | 계획대로. exhaustive match 유지 |
| 3 | 패널 우측 상단 `✕` 절대 위치 오버레이 | 완료 | `.usage-panel`에 `position: relative`, `.close-panel` 신규 |
| 4 | 푸터 `Quit` → 실제 종료 배선 | 완료 | `onQuit` prop 추가, `onClose`는 `✕` 전용으로 유지 |
| 5 | 렌더러 테스트 갱신 (기존 3 + 신규 2) | 완료 | 버그 동작을 고정하던 테스트 2개 이름까지 갱신 |
| 6 | 패널 always-on-top | 완료 | `overlay` 창의 기존 문법 재사용 |
| 7 | e2e 레이아웃 회귀 테스트 | 완료 (교정) | 어서션 1건 수정 — 아래 Deviations 참조 |
| 8 | 문서 갱신 (3개 파일) | 완료 | `ui-contract.md` §5, `beta-testing.md`, `CLAUDE.md` invariant |

## Validation Results

| Level | Status | Notes |
|---|---|---|
| Static Analysis (svelte-check) | Pass | 0 errors, 0 warnings |
| Static Analysis (eslint + prettier) | Pass | 클린 |
| Static Analysis (cargo fmt) | Pass | `--check` 통과 |
| Static Analysis (cargo clippy) | Pass | `-D warnings` 무경고 |
| Unit Tests (renderer, 영향 범위) | Pass | 67 passed — `gateway` 17, `UsagePanel` 10, `App` 40 |
| Unit Tests (Rust 전체) | Pass | 108 passed, 0 failed |
| Full Suite (`pnpm test:ci`) | Pass | 커버리지 80% 게이트 통과 → vite build 완주 |
| Build (vite) | Pass | 156 modules, `index-*.js` 93.11 kB (gzip 31.59 kB) |
| E2E (renderer, 실브라우저) | Pass | 6 passing — 신규 1개 포함 |
| JSON config 검증 | Pass | `tauri.conf.json` 파싱 성공 |
| 시각 검증 (스크린샷) | Pass | `✕`가 우측 상단 레이어에 위치, 레이아웃 불변 확인 |

## Files Changed

| File | Action | Lines |
|---|---|---|
| `src-tauri/src/window/mod.rs` | UPDATED | +25 |
| `src-tauri/src/window/tests.rs` | UPDATED | +6 |
| `src-tauri/src/refresh/ipc.rs` | UPDATED | +31 / -5 |
| `src-tauri/tauri.conf.json` | UPDATED | +1 |
| `src/lib/components/UsagePanel.svelte` | UPDATED | +38 / -2 |
| `src/App.svelte` | UPDATED | +1 |
| `src/lib/components/UsagePanel.test.ts` | UPDATED | +13 / -4 |
| `src/App.test.ts` | UPDATED | +14 / -2 |
| `src/lib/api/gateway.test.ts` | UPDATED | +2 |
| `tests/e2e/renderer.spec.ts` | UPDATED | +36 |
| `docs/ui-contract.md` | UPDATED | +6 / -2 |
| `docs/beta-testing.md` | UPDATED | +7 / -2 |
| `CLAUDE.md` | UPDATED | +1 |

## Deviations from Plan

### 1. e2e 어서션 교정 — 셸 테두리 1px

**WHAT**: 계획의 Task 7은 `headerTopOffset = headerBox.top - panelBox.top`이 `0`이고
`headerWidth === panelWidth`라고 단정했다. 실제 실행에서 `headerTopOffset`이 `1`로 나와
실패했고, `clientTop`/`clientWidth` 기준으로 바꿨다:

```typescript
headerTopOffset: Math.round(headerBox.top - panelBox.top - panel.clientTop),
headerWidth: Math.round(headerBox.width),
panelInnerWidth: panel.clientWidth,
```

**WHY**: `main.panel`은 `border: 1px solid var(--color-border)`를 갖고
`getBoundingClientRect()`는 테두리 **바깥** 경계를 반환한다. 자식인 `header`는 그 1px
안쪽에서 시작하므로 offset은 1이고, 너비도 312가 아니라 310이다. 계획서의 GOTCHA에
*"getBoundingClientRect가 양쪽 모두 테두리를 포함하므로 같은 기준이다"*라고 적었으나
이는 **틀렸다** — 부모의 rect는 테두리를 포함하고 자식은 그 안쪽에서 시작한다.

**영향**: 구현 코드는 수정하지 않았다. 틀린 것은 어서션이며, 교정 후 `✕`가 공간을
차지하지 않는다는 원래 불변 조건을 더 정확하게 검증한다(`clientTop`/`clientWidth`가
테두리를 배제하므로 의도가 자명해짐).

### 2. E2E 실행에 dev 서버 선행 기동 필요 (계획 누락)

**WHAT**: 계획의 Task 7 VALIDATE는 `pnpm test:e2e:renderer`만 적었다. 그대로 실행하면
6개 스펙 **전부** 타임아웃한다.

**WHY**: `wdio.browser.conf.ts`는 `baseUrl: 'http://127.0.0.1:1420'`을 쓰지만 `onPrepare`
훅이 없어 Vite를 직접 띄우지 않는다. `ci.yml:40-41`이 `pnpm dev --host 127.0.0.1 &`로
먼저 기동한다. 동일하게 백그라운드 기동 후 재실행해 6/6 통과했다.

**후속 계획을 위한 메모**: e2e 검증 커맨드는 항상 `pnpm dev --host 127.0.0.1 &` 선행을
포함해야 한다.

## Issues Encountered

### 첫 E2E 실행의 오해 소지 있는 실패

dev 서버 없이 실행한 콜드런에서 기존 테스트("hydrates provider panel…",
"uses the unified 312px vibrancy panel shell")까지 실패해 회귀처럼 보였다. 서버를 띄운 뒤
재실행하니 **5 passing / 1 failing**으로, 실패한 것은 새로 추가한 테스트 하나뿐이었다.
기존 테스트 실패는 서버 부재로 인한 타임아웃 잔여물이었고 회귀가 아니었다.

교훈: e2e 실패를 회귀로 판단하기 전에 실행 전제(서버 기동)를 먼저 확인해야 한다.

### 해결되지 않은 것 — 없음

## Tests Written

| Test File | Tests | Coverage |
|---|---|---|
| `src-tauri/src/window/tests.rs` | +1 (`a_visible_panel_is_raised_rather_than_left_behind_another_window`) | `panel_reveal` 두 분기 |
| `src/lib/components/UsagePanel.test.ts` | 2개 갱신 (`✕`/`Quit` 콜백 분리, `✕` 존재) | 컴포넌트 콜백 라우팅 |
| `src/App.test.ts` | 1개 갱신 + 1개 신규 (`exits CacheBite through the footer Quit button`) | 게이트웨이 배선 |
| `src/lib/api/gateway.test.ts` | +1 어서션 (`invoke('quit', {})`) | 커맨드 이름 오타 방지 |
| `tests/e2e/renderer.spec.ts` | +1 (`layers the close control over the header without reserving space`) | 레이아웃 흐름 제외 회귀 |

## 런타임 검증 — 수행 완료

네이티브 앱을 실제로 기동해 아래를 확인했다.

### 1. 앱 기동 (`alwaysOnTop` 추가가 창 생성을 깨지 않음)

`cargo test`/`clippy`는 라이브러리만 컴파일하므로 `tauri.conf.json` 변경은 런타임에서만
검증된다. 앱을 띄워 확인했다:

```
[CacheBite:native] setup:start v0.1.0
[CacheBite:native] setup:ready
[CacheBite:Claude] emit has_snapshot=true session=Some((83.0, 300)) weekly=Some((60.0, 10080)) ...
[CacheBite:Codex]  emit has_snapshot=true session=None weekly=Some((84.0, 10080)) ...
```

두 창이 생성되고 두 provider 모두 실데이터를 수집했다.

### 2. `WS_EX_TOPMOST` 직접 확인 — 요구 #1 적용 증명

`EnumWindows` + `GetWindowLong(GWL_EXSTYLE)`로 패널 창의 확장 스타일을 읽었다. 마침 이전
설치본이 함께 실행 중이어서 A/B 비교가 됐다:

| 프로세스 | 경로 | 패널 exStyle | `WS_EX_TOPMOST` (0x8) |
|---|---|---|---|
| 신규 dev 빌드 | `src-tauri/target/debug/cachebite.exe` | `0x00040118` | **True** |
| 기존 설치본 (변경 전 코드) | `%LOCALAPPDATA%\CacheBite\cachebite.exe` | `0x00040110` | **False** |

두 창의 exStyle 차이는 정확히 비트 `0x8` 하나다. `alwaysOnTop: true`가 실제로 창에
반영됨을 클릭 없이 확인한 것이다. `overlay` 창은 양쪽 모두 `0x00040118`(TOPMOST)로,
기존에 이미 always-on-top이었다는 사실과 일치한다.

## 수동 검증 — 합성 마우스 입력으로 수행 완료

당초 "GUI 제스처는 자동화 불가"라고 판단했으나 **틀렸다.** Win32 `mouse_event`로 실제 OS 입력을
합성하면 투명·무테두리 창도 정상적으로 받는다. 창에 메시지를 보내는 게 아니라 입력 큐에 넣기
때문이다. 아래는 그 방식으로 수행한 결과다.

하네스: PowerShell + P/Invoke. `SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2)`로 DPI 인식을
켜고(스케일 1.5 환경), 좌표는 전부 `GetWindowRect`/`GetClientRect`+`ClientToScreen`에서 계산하며
하드코딩하지 않았다. 클릭 **전에** `WindowFromPoint` + `GetAncestor(GA_ROOT)`로 대상 창을 확인하고
불일치 시 중단하도록 했다. 커서 위치는 저장·복원했다.

### (a) 전면화 복귀 — 이 diff의 핵심 주장. **통과**

```
round 1 - single click on pet     visible=True  min=False foreground=overlay
round 1 - double-click on pet     visible=True  min=False foreground=PANEL
round 2 - single click on pet     visible=True  min=False foreground=overlay
round 2 - double-click on pet     visible=True  min=False foreground=PANEL
```

패널이 **보이는 상태 + 최소화 아님**인데 포그라운드가 다른 창(오버레이)일 때, 펫 더블클릭으로
패널이 포그라운드로 복귀했다. 2회 재현. 이것이 정확히 `show_panel`이
`if panel.is_visible() { return Ok(()) }`로 빠져나가던 경로이며, 수정 전이라면 포그라운드가
오버레이에 그대로 머물렀을 것이다.

### (b) 최소화된 패널 복귀 — **통과**

```
4. after minimizing panel          visible=True  min=True   foreground=other
5. after double-click on minimized visible=True  min=False  foreground=PANEL
```

최소화된 창도 `is_visible() == true`를 보고하므로 수정 전에는 역시 조기 반환 경로였다.
`reveal_panel`의 `is_minimized()` → `unminimize()` 순서가 실제로 동작함을 확인했다.

### (c) 푸터 `Quit` — **통과**

푸터 기하(패딩 12/16/14, 2열 그리드 gap 8, `.ghost-action` 30px)에서 버튼 중심을 산출해
`css=(228, 352.3)` → 물리 `(-395, 705)`. 클릭 전 화면 캡처로 좌표가 `Quit` 위에 있음을 육안
확인한 뒤 클릭했다(오차 시 "Set as primary"를 눌러 **사용자 설정이 영구 변경**될 수 있어 눈으로
먼저 확인했다).

```
RESULT: process GONE - Quit exited CacheBite
any cachebite left: False
```

`cargo run`도 exit code 0으로 종료 — `app.exit(0)` 경로 확인.

### (d) `✕` — **통과**

`client=468x572, scale=1.5`(468/312 = 1.5로 클라이언트 매핑 정확성 확인). 산출한 `✕` 중심
`(-296, 204)`가 패널 창 위임을 확인 후 클릭:

```
after clicking the X    visible=False  min=False  foreground=overlay
process still alive: True
```

패널만 숨고 프로세스는 유지됐다.

### 부수적으로 확인된 것 — 멀티스크린 앵커

펫이 보조 디스플레이(음수 좌표)에 있었다: 오버레이 rect `(-240,287)-(120,647)`. 패널은
`(-737,177)`에 앵커됐다 — 펫의 **왼쪽**이다. `anchor_panel`이 우측 오버플로를 감지해 좌측으로
플립한 결과이며, 음수 좌표 디스플레이에서 정상 동작함을 보여준다.

## 여전히 미수행

- [ ] 작업표시줄 항목 클릭 복귀 (기존 경로, 이번 diff가 건드리지 않음)
- [ ] 라이트 테마 `✕` hover/focus 육안 확인 — 대비는 다크 6.43:1 실측, 라이트 6.0:1 산출로 둘 다 AA 통과
- [ ] Linux Wayland always-on-top 거부 시 provider 실패 미보고 — Linux 호스트 필요

## 별건 발견 — 투명 패널 스크린샷이 배경을 함께 노출한다

`Quit` 좌표 확인용으로 패널 영역을 캡처했더니 **패널 뒤 화면 내용(이메일 주소들)이 배경 투과로
함께 찍혔다.** 패널은 `transparent: true`이고 셸 배경이 `rgb(28 31 36 / 92%)`이므로 8%가 비친다.

이것은 `docs/beta-testing.md`의 두 문장과 긴장 관계다:

- "Screenshots of the pet or panel are welcome."
- "Never include … Account identifiers or email addresses belonging to your provider accounts."

패널 스크린샷은 구조적으로 뒤 화면을 함께 담는다. 베타 리포터가 선의로 패널 캡처를 첨부하면
무관한 창의 계정 식별자가 함께 올라갈 수 있다. `CLAUDE.md`의 프라이버시 계약이 렌더러 DTO·로그를
겨냥하는 것과 별개로, **리포트 경로에 남아 있는 노출 통로**다. 후속 이슈 후보:
스크린샷 요청 문구에 "패널 뒤에 민감한 창이 없는 상태에서 캡처하라"는 안내를 추가하거나,
불투명도를 100%로 올리는 옵션을 검토.

(해당 캡처 파일은 이 세션에서 삭제했고 사용자에게 전달하지 않았다.)

## 계획서 Manual Validation — 원래 목록 (위에서 대체됨)

아래는 OS 레벨 마우스 제스처가 필요해 자동 검증이 불가능하다. `panel_reveal` 단위 테스트가
그린이라는 것이 배선 동작의 증거가 아니라는 점(`stabilization-and-quality-pass-report.md`에
기록된 실패 모드)을 다시 강조한다.

- [ ] 펫 더블클릭 → 패널 표시 → 다른 앱 클릭 → 펫 재더블클릭 시 **최상단 복귀 + 포커스**
      (요구 #2 — 이번 변경의 핵심, 자동 검증 불가)
- [ ] 작업표시줄 "CacheBite Usage" 항목 클릭 복귀 경로 회귀 없음
- [ ] 최소화된 패널에 더블클릭 → `unminimize` 후 전면
- [ ] `✕` 클릭 → 패널만 숨고 펫·프로세스 유지
- [ ] 푸터 `Quit` → 프로세스 완전 종료
- [ ] 멀티 스크린: 펫을 보조 디스플레이로 옮긴 뒤 더블클릭 → 해당 디스플레이 앵커 + 전면
- [ ] 라이트 테마 `✕` hover/focus 시각 구분 (다크 테마는 스크린샷 확인됨)
- [ ] Linux Wayland: always-on-top 거부 시에도 provider 실패 미보고 (Linux 호스트 필요)

## 별건 발견 — `pnpm tauri dev`가 Windows에서 깨진다 (기존 결함)

`pnpm tauri dev`를 실행하면 exit 1로 실패한다. **이 변경과 무관한 기존 문제다.**

```
Error: EBUSY: resource busy or locked,
  watch 'C:\playground\CacheBite\src-tauri\target\debug\deps\cachebite.exe'
```

`vite.config.ts`에 `server.watch.ignored`가 없어 Vite가 `src-tauri/target/`까지 감시한다.
cargo가 exe를 링크하는 동안 Vite 워처가 그 파일을 감시 목록에 추가하려다 Windows 파일 락에
걸리고, 크래시가 `beforeDevCommand`를 죽여 `tauri dev` 전체가 내려간다. Tauri 스캐폴드가
기본 제공하는 설정이 이 프로젝트에는 빠져 있다.

이번 세션의 우회: 링크를 먼저 완료(`cargo build`) → Vite 기동 → `cargo run`. 경합이
사라져 정상 동작했다.

제안 패치 (이번 범위 밖 — 적용하지 않았다):

```typescript
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ['**/src-tauri/**'] },
  },
```

`CLAUDE.md`가 `pnpm tauri dev`를 표준 개발 커맨드로 문서화하고 있으므로 별도 이슈로 등록할
가치가 있다.

## 알려진 트레이드오프 (계획서 기록대로 유지)

- `✕`가 Codex 탭 우측 상단 약 **14×18px**을 덮는다. 스크린샷으로 확인했다. Codex 탭은
  140px 중 126px이 클릭 가능하게 남는다. 사용자가 리플로 없는 순수 오버레이를 명시
  요구했으므로 헤더 `padding-right` 보정은 넣지 않았다.
- Settings 화면(`showSettings === true`)에서는 `UsagePanel`이 언마운트되어 `✕`가 사라진다.
  기존 `← Back` 경로로 복귀 가능. 범위 밖으로 남겼다.

## 커밋 상태

**커밋하지 않았다.** 사용자가 커밋을 요청하지 않았고, 현재 브랜치는 공유 브랜치인
`develop`이다. 변경은 전부 작업 트리에만 있다. `/prp-implement` 흐름은 `main`에 있을 때만
브랜치 생성을 지시하므로 브랜치도 새로 만들지 않았고, 원격 `pull`도 하지 않았다
(진행 중 충돌 위험 회피).

커밋 전 권장: `feat/` 또는 `fix/` 브랜치로 분기.

## 코드 리뷰 대응 (`docs/code-review/panel-raise-close-and-quit-2026-07-30.md`)

리뷰 결정은 REQUEST CHANGES (CRITICAL 0, HIGH 1, MEDIUM 5, LOW 7).

### H1 — `✕`가 Codex 탭 우측 상단을 가로챈다: 확정, **겹침 유지 + 계약화**로 처리

브라우저 히트 테스트로 리뷰어의 주장을 확정했다. 산술뿐 아니라 실제 이벤트 타깃까지 일치한다:

| 측정 | 결과 |
|---|---|
| 겹침 | 정확히 14 × 18px (Codex 탭 면적의 4.2%, 폭의 10.1%) |
| `elementFromPoint` 겹침 중심 | `close-panel` / `Close usage panel` — `✕`가 이김 |
| Codex 탭 우측 상단 2px 안쪽 | 역시 `✕` |

**폭 양보 없이는 겹침을 없앨 수 없다.** 탭 스트립 위 여유는 12.7px, 헤더 우측 패딩은 16px인데
WCAG SC 2.5.8 최소 타깃은 24×24px이다. 24px 컨트롤은 어느 틈에도 들어가지 않는다. 즉 선택지는
"탭이 24px를 양보한다" 또는 "겹침을 수용한다" 둘뿐이다.

사용자가 **겹침 유지 + 계약화**를 선택했다. 조치:

- `docs/ui-contract.md` §5에 겹침을 **의도된 계약**으로 명시했다 — 수치(14×18px, 탭 폭 약 10%),
  겹침 영역의 포인터 이벤트가 `✕`로 간다는 사실, 탭 폭 89% 이상이 클릭 가능해야 한다는 하한.
- e2e에 겹침 상한 어서션을 추가했다 (`renderer.spec.ts`,
  `keeps the close control overlap over the second tab within contract`): 폭 ≤14px, 높이 ≤18px,
  탭 폭 대비 ≤10.5%, 그리고 탭 중앙·좌측 끝의 히트 타깃이 여전히 `Codex`임을 검증한다.
  아이콘을 키우거나 헤더 패딩을 줄이면 이 테스트가 실패한다 — 조용히 악화되지 않는다.

리뷰어가 지적한 "보고서의 수용 근거가 성립하지 않는다"는 **맞다.** `resize_panel`은 높이만 나르고
너비는 `PANEL_WIDTH_LOGICAL` 고정이므로, 폭 보정은 측정 높이 불변과 무관하다. 이번 수용 근거는
높이 불변이 아니라 **"`✕`가 어떤 컴포넌트도 밀어내지 않는다"는 사용자의 시각적 요구**이며,
계약 문서에 그대로 적었다.

### M2 — Settings 화면 계약 불일치: 문서 정정

`← Back`이 존재하므로 기능적 결함이 아니다. 실제 문제는 내가 쓴 계약 문장이 "유일한 닫기 수단은
`✕`"라고 무조건 단정한 것이었다. `docs/ui-contract.md`에 `✕`가 사용량 화면에만 있다는 예외를
명시했다.

### M1 — 시작 실패 화면: 계약화, 코드 변경 없음

리뷰어 지적대로 도달 가능한 상태다. 정확한 메커니즘: `show_panel`의 150ms grace 타이머
(`ipc.rs:315-326`)가 렌더러 상태와 무관하게 창을 노출하므로, 패널이 **안 나오는 게 아니라
나오는데** 내용이 시작 실패 텍스트뿐이고 `UsagePanel`이 마운트되지 않아 `✕`·`Quit`이 없다.

다만 심각도는 MEDIUM보다 낮다:

- 도달 조건이 좁다 — 더블클릭을 받을 **오버레이는 정상인데 패널 렌더러만** 실패/멈춰야 한다.
- 복구 경로가 존재한다 — 패널 창에 `skipTaskbar`가 없어 작업표시줄 항목이 있고(Win32 열거에서
  `'CacheBite Usage'` 창 확인), 우클릭 → 창 닫기로 종료된다. **작업관리자는 필요하지 않으므로**
  `beta-testing.md`의 새 약속은 깨지지 않는다.

`docs/ui-contract.md`에 시작 실패 화면에서는 작업표시줄 항목이 유일한 닫기 경로이며, 그것이
패널 창에 `skipTaskbar`를 설정하지 않는 이유 중 하나라고 명시했다.

### M4 — `✕` 대비 미달: 수정

`--color-text-faint` → `--color-text-muted`. 브라우저에서 실측 검증했다:

| | 이전 | 이후 |
|---|---|---|
| 색 | `#6b7280` (다크) | `#9aa0a6` (다크) |
| 셸 배경 대비 | 약 3.4:1 | **6.43:1** |
| WCAG AA 본문(4.5:1) | 미달 | 통과 |

형제 `.ghost-action`이 이미 쓰는 토큰이라 일관성도 회복됐다.

### M3 — 절반 수용, 절반 반론

**수용(문서)**: 계약 문장을 "지원되는 플랫폼에서 항상 위"로 완화하고, always-on-top을 거부하는
컴포지터에서는 펫 더블클릭 전면화가 복구 경로임을 명시했다.

**반론(진단 노출)**: `always_on_top` 진단을 패널에 노출하라는 권고는 **지금 적용하면 안 된다.**
`ipc.rs:210`은 `fullscreen_detection`과 달리 `cfg!` 분기 없이 **모든 플랫폼에서** `Unavailable`을
반환한다. 그런데 Windows에서 `GetWindowLong(GWL_EXSTYLE)`로 `WS_EX_TOPMOST=True`를 직접
확인했다 — 실제로는 동작한다. 지금 노출하면 정상 동작하는 Windows·macOS 사용자 전원에게 거짓
경고가 뜬다. 올바른 순서는 capability 보고를 플랫폼별로 정확히 만든 뒤 노출하는 것이고, 그것은
이 diff의 범위가 아니다. **후속 이슈 후보로 남긴다.**

### M5 — 커버리지 착시: 지적 수용, 부분 검증 완료

`panel_reveal`이 항등 매핑이고 실제 고친 동작(unminimize → show → set_focus)에 자동 커버리지가
없다는 지적은 정확하다. 이후 런타임 검증으로 always-on-top 적용과 앱 기동을 확인했다
(위 "런타임 검증" 절). 리뷰어가 최소 요구한 3개 중 **(a) 전면화 복귀, (c) `Quit` 후 프로세스
부재는 여전히 미검증**이다 — OS 레벨 마우스 제스처가 필요하다.

### LOW 처리

| # | 항목 | 조치 |
|---|---|---|
| 1 | `pub` → `pub(crate)` | **하지 않음.** 리뷰어도 인정했듯 같은 모듈의 `command_allowed`·`NativeCommand`가 `pub`이다. 신규 심볼만 바꾸면 모듈 내부가 불일치해진다. 일괄 정리 항목 |
| 2 | 조용한 no-op 기본값 | 조치 없음 — 기존 패턴 |
| 3 | 테스트 1개에 두 행동 | 조치 없음 — `✕`/`Quit` 대비를 한 테스트에서 보는 것이 배선 교차 회귀(둘이 뒤바뀌는 경우)를 잡는다. `App.test.ts`에는 이미 분리된 두 테스트가 있다 |
| 4 | 툴팁 없음 | **수정** — `title="Close usage panel"` 추가 |
| 5 | e2e 어서션 취약성 | **수정** — `toBe` → ±1px 허용 비교, 분수 DPR 근거를 주석에 기록 |
| 6 | AppIndicator 빌드 의존 | 조치 없음 — `tray-icon` 크레이트가 의존 그래프에 있어 Linux 빌드타임에 실제로 필요할 수 있다. 확인 없이 CI 시스템 의존을 제거하면 빌드가 깨진다. 후속 확인 항목 |
| 7 | 타깃 최소치 24×24px | H1 결정(겹침 유지)에 따라 크기 유지 |

### 리뷰 대응 후 검증

| Check | Result |
|---|---|
| svelte-check | 0 errors / 0 warnings |
| eslint + prettier | 클린 |
| `pnpm test:ci` (커버리지 게이트 + 빌드) | 23 files / **242 tests** passed, 전체 95.53% stmt / 90.7% branch |
| 렌더러 E2E | **7 passing** (신규 겹침 상한 테스트 포함) |
| `✕` 대비 실측 | 6.43:1 (AA 통과) |

### 리뷰 대응으로 변경된 파일

| File | 변경 |
|---|---|
| `src/lib/components/UsagePanel.svelte` | M4 색 토큰, LOW 4 `title` |
| `docs/ui-contract.md` | H1 계약화, M2 예외, M1 복구 경로, M3 문장 완화 |
| `tests/e2e/renderer.spec.ts` | H1 겹침 상한 테스트 신규, LOW 5 허용 오차 |

## Next Steps

- [ ] `pnpm tauri dev`로 위 Manual Validation 체크리스트 수행 (필수)
- [ ] `/code-review`로 변경 검토
- [ ] 기능 브랜치 분기 후 커밋
- [ ] issue #29 종료, 그 다음 #30 (전역 hide/show shortcut) 착수
- [ ] 후속 이슈 등록 검토: 시스템 트레이 아이콘 (`tray-icon` feature — Linux CI에 이미
      `libappindicator3-dev` 설치됨, `Cargo.lock`에 해소되어 있어 진입 장벽 낮음)
