# Plan: UI Design Pass — docs/UI-plan 시각 디자인 적용

## Summary

`docs/UI-plan/CacheBite Pet UI.dc.html` 시각 디자인 패스를 develop 브랜치 렌더러에 적용한다.
확정 결정: **1a 분할 링**, **OS별 적응형 패널**(macOS vibrancy / Win11 Mica / Linux Adwaita),
**라이트+다크 테마**, **cat/corgi 펫 패키지 생성 + 주 provider 연동**(cat=Claude, corgi=Codex).

## User Story

As a CacheBite 사용자, I want 디자인 패스의 시각 스타일(심각도 색상, 분할 링, OS 네이티브 느낌의 패널, 무드별 펫)이 실제 앱에 반영되기를, so that 상태를 한눈에 구분하고 데스크톱에 자연스럽게 녹아드는 펫을 볼 수 있다.

## Problem → Solution

현재: 다크 웜 톤 임시 토큰, 스타일 없는 시맨틱 HTML 패널, 이모지 배지, 지오메트릭 fixture 펫만 존재.
목표: 디자인 토큰(§severity tokens) + IBM Plex 타이포 + 1a 분할 링(5H/WK 라벨) + SVG 배지 + OS별 패널 스타일 + cat/corgi 무드 펫이 주 provider를 따라 전환.

## Metadata

- **Complexity**: Large
- **Source PRD**: N/A (standalone — 디자인 문서 `docs/UI-plan/CacheBite Pet UI.dc.html` + `docs/ui-contract.md` 기반)
- **PRD Phase**: N/A
- **Estimated Files**: ~21 (렌더러 14, Rust 3, 스크립트/에셋 3, 설정 1)

---

## 확정된 디자인 결정 (사용자 승인)

| 결정 | 선택 |
| --- | --- |
| 링 디자인 | **1a 연속 분할 링** — 상반원=5시간, 하반원=주간, 12시/6시에 `5H`/`WK` 마이크로 라벨 |
| 패널 스타일 | **OS별 적응형** — `data-platform` 속성 + CSS 변형 3종 |
| 테마 | **라이트 기본 + `prefers-color-scheme: dark` 다크 변형** (다크 값은 디자인 문서에 없음 → 본 계획 §디자인 토큰에서 정의) |
| 펫 에셋 | **포함** — cat/corgi 패키지 생성 + 주 provider에 따라 자동 전환 |

## 디자인 토큰 (디자인 문서에서 추출)

| 토큰 | 라이트 | 다크(본 계획에서 정의) | 근거 |
| --- | --- | --- | --- |
| `--sev-ok` | `#22c55e` | `#4ade80` | 디자인 doc 헤더 배지 |
| `--sev-warn` | `#f59e0b` | `#fbbf24` | 〃 |
| `--sev-critical` | `#f97316` | `#fb923c` | 〃 |
| `--sev-exhausted` | `#dc2626` | `#f87171` | 〃 |
| `--sev-unknown` | `#c3c8ce` | `#4b5563` | 〃 |
| `--color-text` | `#16191d` | `#f3f4f6` | doc body |
| `--color-text-muted` | `#5b6169` | `#9aa0a6` | doc 본문 |
| `--color-text-faint` | `#8a9099` / `#a1a7ad` | `#6b7280` | doc 캡션 |
| `--color-surface` | `#ffffff` | `#1c1f24` | 카드 배경 |
| `--color-surface-sunken` | `#eceef1` | `#16191d` | doc body bg |
| `--color-border` | `#e6e8eb` | `#2e333a` | 카드 테두리 |
| `--color-accent` | `#3b5bdb` | `#6d8dff` | 링크/스피너 |
| `--badge-lock` | `#6b7280` | 동일 | 상태 갤러리 |
| `--badge-slash` | `#8a9099` | 동일 | 〃 |
| `--badge-error` | `#dc2626` | 동일 | 〃 |
| `--badge-offline` | `#64748b` | 동일 | 〃 |
| `--badge-loading` | `#3b5bdb` | 동일 | 〃 |
| `--overlay-stale-dim` | `0.42` | `0.42` | 상태 갤러리 stale 카드 `opacity:.42` |
| 폰트 | IBM Plex Sans(400/500/600/700) + IBM Plex Mono(400/500/600) | 동일 | doc `<helmet>` |
| 링 두께 | viewBox 200 기준 13 → 현 viewBox 100 기준 **6.5** | 동일 | 1a SVG `stroke-width="13"` |

OS별 패널 변형 (디자인 doc §5):

| 플랫폼 | 배경 | radius | 폰트 오버라이드 | 액센트 |
| --- | --- | --- | --- | --- |
| macOS | `rgba(250,250,252,.92)` + `backdrop-filter: blur(20px)` | 14px | (기본 IBM Plex) | `#16191d` 버튼 |
| Windows | `#f3f3f3` 불투명 | 8px | `'Segoe UI', system-ui` | `#0067c0` |
| Linux | `#fafafb` 불투명 | 12px | `'Cantarell','Ubuntu', system-ui` | `#3584e4` |

---

## UX Design

### Before

```
┌──────────────────────────┐   ┌────────────────────────┐
│ 오버레이: 다크 웜 링       │   │ 패널: 무스타일 HTML     │
│  ~73c991/f4c95d 색        │   │  <progress> 기본 룩     │
│  이모지 배지 🔒⊘⚠☁◌      │   │  버튼 브라우저 기본      │
│  지오메트릭 fixture 펫     │   │  다크 웜 배경           │
└──────────────────────────┘   └────────────────────────┘
```

### After

```
┌──────────────────────────┐   ┌────────────────────────────┐
│      ╭── 5H ──╮          │   │ [Claude ★] [Codex]  ← 탭   │
│    ╱▓▓▓▓▓▓▓▓▓▓╲ #f59e0b │   │ Claude            [Pro]    │
│   │  🐱 cat/corgi │      │   │ 5-hour window        68%   │
│   │  (mood 프레임) │      │   │ ▓▓▓▓▓▓▓░░░ (sev 색 바)     │
│    ╲▓▓▓▓▓▓▓░░░╱ #f97316 │   │ resets in 1h 12m · warn    │
│      ╰── WK ──╯          │   │ weekly window        31%   │
│  SVG 배지(자물쇠/구름/…)   │   │ ▓▓▓░░░░░░░                 │
│  stale → opacity .42     │   │ ● Fresh · oauth_api        │
└──────────────────────────┘   │ [Refresh now][Set primary] │
  주 provider 바뀌면            │  Settings          Quit*   │
  cat ↔ corgi 자동 전환        └────────────────────────────┘
                                * Quit은 이번 범위 밖(IPC 없음)
                                패널 룩 = OS별(mac/Win/Linux)
```

### Interaction Changes

| Touchpoint | Before | After | Notes |
| --- | --- | --- | --- |
| 오버레이 링 | 웜 톤, 라벨 없음, dim 0.45 | sev 토큰 색, `5H`/`WK` 라벨, dim 0.42 | 구조(경로/dasharray)는 유지 |
| 시스템 배지 | 이모지 5종 | 디자인 SVG 아이콘 5종 + 상태별 배경색, 흰 테두리 | role="status"/aria-label 유지 |
| 펫 | selectedPetId 고정 | 주 provider → cat/corgi 자동, mood 프레임 재생 | 폴백 체인은 기존 resolver 그대로 |
| 패널 탭 | 평범한 버튼 2개 | 밑줄 강조 탭 + 주 provider ★ 표시 | `primary` prop 신규 |
| 게이지 | `<progress>` 기본 | sev 색 채움 바 + mono % + resets 줄 | `role="progressbar"` div로 교체 |
| 패널 푸터 | 버튼 2개 나열 | Refresh now / Set as primary 2단 배치 | Quit IPC는 NOT Building |
| 테마 | 다크 고정 | 라이트 기본 + 다크 미디어쿼리 | 오버레이 배경은 항상 투명 |

---

## Mandatory Reading

| Priority | File | Lines | Why |
| --- | --- | --- | --- |
| P0 | `docs/UI-plan/CacheBite Pet UI.dc.html` | all | 시각 스펙 원본 (색·치수·SVG 아이콘 path·패널 3종) |
| P0 | `docs/ui-contract.md` | §4, §6 | 링/배지/GIF 계약 — 디자인이 이 계약을 픽셀화한 것 |
| P0 | `src/lib/styles/tokens.css` | all | 교체 대상 토큰 |
| P0 | `src/lib/components/SplitUsageRing.svelte` | all | 1a 링 기존 구현 (구조 유지, 스타일만 교체) |
| P0 | `src/App.svelte` | 250–273, 349–392 | petPackage 로딩·overlayModel 파생 — 펫 전환 로직 삽입 지점 |
| P1 | `src/lib/components/SystemBadge.svelte` | all | 이모지→SVG 교체 대상 |
| P1 | `src/lib/components/UsagePanel.svelte`, `UsageGauge.svelte`, `ProviderTabs.svelte` | all | 패널 재스타일 대상 |
| P1 | `src/lib/assets/manifest.ts` | 38–41, 82–141 | 매니페스트 검증 규칙 (`SAFE_ASSET_PATH`, `PACKAGE_ID`) |
| P1 | `src/lib/assets/resolver.ts` | 30–43 | `requestedAnimationKey` — mood→키 매핑 이미 존재 |
| P1 | `src-tauri/src/store/pets.rs` | all | Rust 쪽 패키지 로딩·id 검증·asset URL 규칙 |
| P1 | `src-tauri/src/refresh/ipc.rs` | 148–175 | `get_pet_package`/`get_platform_capabilities` 명령 |
| P2 | `src/lib/components/PetOverlay.test.ts` | all | 컴포넌트 테스트 패턴 |
| P2 | `tests/fixtures/pets/geometric-idle/manifest.json` | all | 매니페스트 실물 예시 |
| P2 | `src/lib/api/gateway.ts` | 46–115 | wire 타입·snake_case↔camelCase 변환 패턴 |

## External Documentation

| Topic | Source | Key Takeaway |
| --- | --- | --- |
| @fontsource/ibm-plex-sans, -mono | fontsource.org | `import '@fontsource/ibm-plex-sans/400.css'` 형태로 main.ts에서 임포트 — 자체 호스팅이므로 CSP/오프라인 안전. Google Fonts CDN 링크는 사용 금지 |
| Tauri v2 bundle resources | tauri.app/v2 config | `bundle.resources: ["resources/pets/**/*"]` → 런타임 `app.path().resource_dir()` 로 접근 |
| `std::env::consts::OS` | Rust std | `"macos" | "windows" | "linux"` 문자열 — OS 판별에 사용 |

---

## Patterns to Mirror

### NAMING_CONVENTION

```
// SOURCE: src/lib/components/ — 컴포넌트 PascalCase.svelte, 테스트 동일 이름 .test.ts
// SOURCE: src/lib/styles/tokens.css:3-11 — CSS 변수 --color-*, --space-*, --radius-*, --shadow-*
--color-surface: #171512;
--color-status-ok: #73c991;
// 새 토큰도 --sev-*, --color-*, --badge-* 접두어 유지
```

### COMPONENT_PROPS (Svelte 5 runes + JSDoc)

```svelte
<!-- SOURCE: src/lib/components/SplitUsageRing.svelte:1-13 -->
<script>
  /** @type {{ session: import('./models').RingWindowModel; weekly: import('./models').RingWindowModel; stale: boolean }} */
  let { session, weekly, stale } = $props();
  const percent = (window) => ...;
</script>
```

`.svelte`는 JSDoc 타입 주석 + plain `<script>`, `App.svelte`만 `<script lang="ts">`. `$derived`/`$state` 룬 사용.

### SEVERITY_STYLING (data-attribute 셀렉터)

```svelte
<!-- SOURCE: src/lib/components/SplitUsageRing.svelte:26-31, 62-73 -->
<path class="usage" data-severity={session.severity} ... />
<style>
  .usage[data-severity='ok'] { stroke: var(--color-status-ok); }
</style>
```

패널 게이지·탭도 동일하게 `data-severity`/`data-platform` 속성 + CSS 셀렉터로 분기. JS로 색상 계산하지 않는다.

### ERROR_HANDLING (조용한 폴백 + role="status")

```ts
// SOURCE: src/App.svelte:261-273
try {
  petPackage = { manifest: validatePetManifest(loadedPetPackage.manifest), ... };
  petPackageError = false;
} catch {
  petPackageError = true;
}
// 표시는 {#if petPackageError}<p role="status">Pet package unavailable</p>
```

### SETTINGS_MUTATION (직렬화 큐 + 스토어 재동기화)

```ts
// SOURCE: src/App.svelte:394-441 changeSettings
// 설정 변경은 반드시 changeSettings 큐를 통해서만. appSettings 직접 변이 금지.
void changeSettings({ ...$settingsStore, primaryProvider: provider });
```

### RUST_IPC_COMMAND

```rust
// SOURCE: src-tauri/src/refresh/ipc.rs:161-175
#[tauri::command]
pub fn get_platform_capabilities(window: tauri::WebviewWindow) -> Result<PlatformCapabilities, IpcError> {
    authorize(&window, NativeCommand::GetPlatformCapabilities)?;
    Ok(PlatformCapabilities { ... })
}
```

### TEST_STRUCTURE (Testing Library + vitest)

```ts
// SOURCE: src/lib/components/PetOverlay.test.ts:12-31
describe('PetOverlay', () => {
  afterEach(cleanup);
  it('shows two accessible usage arcs for active usage', () => {
    render(PetOverlay, { props: { model: { ... } } });
    expect(screen.getByLabelText('5-hour usage: 74%')).toBeTruthy();
  });
});
```

접근성 쿼리(`getByLabelText`, `getByRole`) 우선, 스타일 검증은 `data-*` 속성으로. `it.each`로 상태 매트릭스.

### MANIFEST_FIXTURE

```json
// SOURCE: tests/fixtures/pets/geometric-idle/manifest.json
{ "id": "geometric-idle", "displayName": "Geometric Idle",
  "defaultSize": { "width": 128, "height": 128 },
  "animations": { "idle": { "type": "frames", "frames": ["frames/idle-01.svg", "frames/idle-02.svg"], "frameDurationMs": 240 } } }
```

---

## Files to Change

| File | Action | Justification |
| --- | --- | --- |
| `package.json` | UPDATE | `@fontsource/ibm-plex-sans`, `@fontsource/ibm-plex-mono` 의존성 추가 |
| `src/main.ts` | UPDATE | fontsource CSS 임포트 |
| `src/lib/styles/tokens.css` | UPDATE | 라이트/다크 토큰 전면 교체 (§디자인 토큰 표) |
| `src/lib/styles/global.css` | UPDATE | 라이트 기본 + 다크 미디어쿼리, 패널 `data-platform` 훅 |
| `src/lib/components/SplitUsageRing.svelte` | UPDATE | sev 토큰, 5H/WK 라벨, stroke 6.5, dim 0.42 |
| `src/lib/components/SystemBadge.svelte` | UPDATE | 이모지→디자인 SVG 5종 + 상태별 배경색 |
| `src/lib/components/PetOverlay.svelte` | UPDATE | 배지 위치/크기 미세 조정 (디자인: bottom-right, 흰 테두리 2.5px) |
| `src/lib/components/UsagePanel.svelte` | UPDATE | 헤더/본문/푸터 3단 구조 + OS별 스타일 + freshness 줄 |
| `src/lib/components/UsageGauge.svelte` | UPDATE | `<progress>`→`role="progressbar"` div 바, sev 색, mono % |
| `src/lib/components/ProviderTabs.svelte` | UPDATE | 밑줄 탭 + `primary` prop(★) |
| `src/lib/components/SettingsPanel.svelte` | UPDATE | 체크박스/셀렉트 재스타일 (구조 유지) |
| `src/lib/components/SpeechBubble.svelte` | UPDATE | 말풍선 룩 (흰 배경, 그림자, 꼬리) |
| `src/lib/components/HistoryGraph.svelte` | UPDATE | 토큰 색 적용만 (신규 디자인 없음 — 최소 변경) |
| `src/App.svelte` | UPDATE | (1) 주 provider→펫 id 동기화 + 패키지 재로딩 (2) `data-platform` 클래스 |
| `src/lib/api/gateway.ts` | UPDATE | `PlatformCapabilities.os: 'macos'|'windows'|'linux'` 추가 |
| `src/lib/api/fixtureGateway.ts` | UPDATE | fixture에 `os` 필드 반영 |
| `src-tauri/src/refresh/ipc.rs` | UPDATE | `get_platform_capabilities`에 `os` 필드 (`std::env::consts::OS`) |
| `src-tauri/src/lib.rs` | UPDATE | setup에서 번들 리소스 pets → `app_data/pets` 첫 실행 복사 |
| `src-tauri/tauri.conf.json` | UPDATE | `bundle.resources`에 `resources/pets/**/*` |
| `scripts/build-pet-packages.py` | CREATE | docs/UI-plan/assets → `src-tauri/resources/pets/{cat,corgi}` 변환 |
| `src-tauri/resources/pets/{cat,corgi}/` | CREATE(생성물) | manifest.json + `frames/{state}_{01..04}.png` |
| 각 `*.test.ts` (PetOverlay, UsagePanel, SettingsPanel, gateway 등) | UPDATE | 변경된 마크업/타입 반영 + 신규 케이스 |

## NOT Building

- **Quit 버튼 IPC** — gateway에 종료 명령 없음. 푸터에 자리만 잡지 않고 생략 (후속 작업)
- 링 옵션 1b/1c, 임계값 눈금
- `sleep`/`dragging` 전용 에셋 (이미지 없음 — 폴백 체인이 idle 처리)
- OS 알림/말풍선 로직 변경 (스타일만)
- 히스토리 그래프 신규 디자인 (디자인 doc에 없음)
- 흰 배경 자동 제거(알파 키잉)의 완성도 보장 — 스크립트는 단순 임계값 키잉, 결과는 수동 확인 (Risks 참조)
- 테마 수동 토글 설정 (prefers-color-scheme만 따름)

---

## Step-by-Step Tasks

### Task 1: 폰트 의존성 + 임포트

- **ACTION**: `pnpm add @fontsource/ibm-plex-sans @fontsource/ibm-plex-mono`
- **IMPLEMENT**: `src/main.ts` 상단에
  `import '@fontsource/ibm-plex-sans/400.css'` (+500/600/700), `import '@fontsource/ibm-plex-mono/400.css'` (+500/600)
- **MIRROR**: main.ts 기존 `import './lib/styles/tokens.css'` 순서 앞에 배치
- **GOTCHA**: Google Fonts CDN `<link>` 금지 (Tauri CSP·오프라인). fontsource는 vite가 번들
- **VALIDATE**: `pnpm build` 후 dist에 woff2 포함 확인

### Task 2: tokens.css 교체 (라이트+다크)

- **ACTION**: §디자인 토큰 표 그대로 `:root`(라이트) + `@media (prefers-color-scheme: dark)` 블록 작성
- **IMPLEMENT**: 기존 `--color-status-*` 이름을 삭제하고 **새 이름 `--sev-*`로 통일** — 사용처는 SplitUsageRing/전역뿐이므로 전수 교체 가능. `font-family: 'IBM Plex Sans', ...; --font-mono: 'IBM Plex Mono', ...` 추가. `color-scheme: light dark`
- **MIRROR**: 기존 tokens.css의 `--접두어-역할` 네이밍
- **GOTCHA**: 오버레이 창은 `body { background: transparent }` 유지 필수 — surface 색을 body에 깔면 데스크톱 위 사각형 발생. 다크 토큰 값은 디자인 doc에 없어 본 계획 표가 유일한 출처
- **VALIDATE**: `pnpm check && pnpm test` — SplitUsageRing 테스트가 새 변수로 통과

### Task 3: SplitUsageRing 재스타일

- **ACTION**: sev 토큰 색, stroke-width 6.5, `--overlay-stale-dim: 0.42`, 12시·6시 `5H`/`WK` 라벨
- **IMPLEMENT**: 기존 path/dasharray 구조 그대로. 라벨은 SVG `<text>` 요소(mono 폰트, `--color-text-faint`, letter-spacing) — viewBox 스케일 안전. `aria-hidden="true"` (기존 aria-label이 이미 접근성 담당)
- **MIRROR**: SEVERITY_STYLING 패턴 (`data-severity` 셀렉터 유지)
- **GOTCHA**: 기존 테스트가 `stroke-dasharray="0 100"`·`data-stale` 검증 — 구조 바꾸면 깨짐. 라벨 추가로 `getByLabelText` 쿼리 영향 없는지 확인
- **VALIDATE**: `pnpm test -- SplitUsageRing PetOverlay`

### Task 4: SystemBadge SVG 교체

- **ACTION**: 이모지 5종 → 디자인 doc 상태 갤러리의 inline SVG 5종 (lock/slash/warn-triangle/cloud-off/spinner — doc 라인 358, 368, 378, 388, 398에서 복사)
- **IMPLEMENT**: `badges` 맵을 `{ color, label }`로 확장하고 마크업은 `{#if system === '...'}` 정적 `<svg>` 분기 (`{@html}` 금지 — XSS 정책). 배경 `--badge-*` 토큰, `border: 2.5px solid var(--color-surface)`, 그림자 유지
- **MIRROR**: 기존 `role="status"` + `aria-label` + loading spin 애니메이션/`prefers-reduced-motion` 처리 그대로
- **GOTCHA**: 기존 테스트가 aria-label 문자열 5종을 고정 검증 — 문자열 변경 금지
- **VALIDATE**: `pnpm test -- SystemBadge PetOverlay` (PetOverlay.test의 `it.each` 배지 케이스 통과)

### Task 5: 펫 에셋 빌드 스크립트

- **ACTION**: `scripts/build-pet-packages.py` 작성 후 실행
- **IMPLEMENT**:
  1. 매핑: `docs/UI-plan/assets/pet/cat→cat`, `pet/dog→corgi`; 상태 디렉터리 `ok→idle`, `warn→idle_warn`, `critical→idle_critical`, `exhusted→idle_exhausted`(오타 주의)
  2. 각 디렉터리 파일을 이름순 정렬 → `frames/{state}_{01..04}.png`로 리네임 복사
  3. PIL로 512×512 다운스케일(LANCZOS) + RGBA 변환 + 흰 배경 임계값 키잉(예: RGB 각 채널 ≥ 247 → alpha 0) — 원본 1254×1254 RGB
  4. `manifest.json` 생성: `id: "cat"|"corgi"`, `displayName: "Cat"|"Corgi"`, `defaultSize: {width:128,height:128}`, animations 4종(`frameDurationMs: 240`), `states: { idle:"idle", idle_warn:"idle_warn", idle_critical:"idle_critical", idle_exhausted:"idle_exhausted" }`
  5. 출력: `src-tauri/resources/pets/{cat,corgi}/`
- **MIRROR**: MANIFEST_FIXTURE 구조. 경로는 `manifest.ts:40` `SAFE_ASSET_PATH`(ASCII만, `..` 금지)와 `pets.rs:101` id 규칙(소문자 케밥) 준수
- **GOTCHA**: 원본 파일명 한글·공백·괄호 — 원본은 건드리지 말고 복사만. `states` 키는 `PET_STATES`에 있는 키만 허용
- **VALIDATE**: 생성된 manifest를 `validatePetManifest`에 통과시키는 단위 테스트(Task 10) + `python3 scripts/build-pet-packages.py && ls src-tauri/resources/pets/cat/frames | wc -l` = 16

### Task 6: Rust — 번들 리소스 + 첫 실행 설치 + os 필드

- **ACTION**: 3개 파일 수정
- **IMPLEMENT**:
  1. `tauri.conf.json` bundle에 `"resources": ["resources/pets/**/*"]`
  2. `lib.rs` setup: `PetPackageRepository::new(...)` 직전에 `resource_dir()/resources/pets`의 `cat`,`corgi`를 `app_data/pets/`로 **존재하지 않을 때만** 복사(재귀 copy, 실패는 로그 후 무시 — 펫 없음은 renderer가 `petPackageError`로 처리)
  3. `ipc.rs` `PlatformCapabilities` 구조체에 `pub os: &'static str` 추가, `std::env::consts::OS` 반환 (`"macos"|"windows"|"linux"`, 그 외는 `"linux"` 폴백)
- **MIRROR**: RUST_IPC_COMMAND 패턴, `pets.rs`의 io::Error 스타일
- **GOTCHA**: 복사는 고정 id 2개만 — traversal 없음. dev 모드(`tauri dev`)에서 resource_dir는 `src-tauri/` 기준으로 해석됨 — dev에서도 동작 확인
- **VALIDATE**: `cargo test --manifest-path src-tauri/Cargo.toml && cargo fmt --check --manifest-path src-tauri/Cargo.toml`

### Task 7: gateway 타입 + fixture 갱신

- **ACTION**: `PlatformCapabilities`에 `readonly os: 'macos' | 'windows' | 'linux'` 추가
- **IMPLEMENT**: `gateway.ts:53` 인터페이스 수정. `fixtureGateway.ts`의 capabilities 응답에 `os: 'linux'` 추가. wire 변환 불필요(단일 필드)
- **MIRROR**: gateway.ts 기존 `CapabilityDiagnostic` 선언부 스타일
- **GOTCHA**: `gateway.test.ts`·`tauri.test.ts`에 capabilities mock 있으면 필드 추가 필요 — 컴파일 에러가 위치를 알려줌
- **VALIDATE**: `pnpm check`

### Task 8: App.svelte — 펫 자동 전환 + 플랫폼 클래스

- **ACTION**: 주 provider → 펫 id 동기화, `data-platform` 부여
- **IMPLEMENT**:
  1. 상수 `const PROVIDER_PET: Record<Provider, string> = { claude: 'cat', codex: 'corgi' }`
  2. `loadPetPackage()` 헬퍼 추출(기존 App.svelte:261–273 try/catch + `gateway.getPetPackage()` 호출부 재사용)
  3. start()에서: `settings.selectedPetId !== PROVIDER_PET[settings.primaryProvider]`이면 `gateway.updateSettings({...settings, selectedPetId: 원하는 id})` 후 로딩 — 실패 시 기존 selectedPetId로 폴백 로딩
  4. `changeSettings`에서 primaryProvider 변경 감지 시 merged에 `selectedPetId: PROVIDER_PET[next.primaryProvider]` 포함, 저장 성공 후 오버레이 창이면 `loadPetPackage()` 재호출
  5. `<main>`에 `data-platform={platformCapabilities?.os ?? 'linux'}` — capabilities는 현재 panel 창만 로딩하므로 **overlay 창에서도 `getPlatformCapabilities()`를 호출하도록 start() 수정**
- **MIRROR**: SETTINGS_MUTATION 패턴 — appSettings 직접 변이 금지, 큐 직렬화 유지
- **GOTCHA**: 오버레이/패널 두 창이 각각 App 인스턴스 — 설정 저장은 패널에서, 펫 표시는 오버레이에서. 오버레이는 설정 변경 이벤트를 수신하지 않음 → **v1 한계로 문서화**: 오버레이 펫 전환은 다음 앱 시작 시 반영. 설정 이벤트 브릿지 신설은 범위 밖(과설계 금지)
- **VALIDATE**: `pnpm test -- App nativeWorkflow` + `pnpm check`

### Task 9: 패널 컴포넌트 재스타일 (UsagePanel/UsageGauge/ProviderTabs/SettingsPanel/SpeechBubble/HistoryGraph/global.css)

- **ACTION**: 디자인 doc §5 macOS 스펙을 기본으로 3단 구조 + `data-platform` CSS 변형
- **IMPLEMENT**:
  - `global.css`: `main.panel`을 `[data-platform]`별 배경/radius/그림자/폰트로 분기 (§OS별 패널 변형 표). macOS만 `backdrop-filter: blur(20px)` + 반투명
  - `ProviderTabs`: `primary` prop 추가, 선택 탭 `border-bottom: 2px solid var(--color-text)`, ★은 `<span aria-hidden>` + `aria-label`에 "(primary)" 포함
  - `UsageGauge`: `<progress>` → `<div role="progressbar" aria-valuenow={percent} aria-valuemin="0" aria-valuemax="100">` + 내부 채움 div `data-severity` 색. 우측 mono %(`--sev-*` 색), 아래 `resets in …` mono 캡션. stale이면 바만 opacity 0.42
  - `UsagePanel`: 헤더(탭)/본문(plan 칩, 게이지 2, freshness 줄 `● Fresh|Stale · source`)/푸터(Refresh now=채움 버튼, Set as primary=외곽 버튼) 구조. fresh 도트=`--sev-ok`, stale=`--color-text-faint`
  - `SettingsPanel`/`SpeechBubble`/`HistoryGraph`: 토큰 기반 재스타일만
- **MIRROR**: COMPONENT_PROPS + SEVERITY_STYLING. 기존 aria 구조(`role="tablist"`, `aria-selected`) 절대 유지
- **GOTCHA**: `UsagePanel.test.ts`가 `usage-skeleton` testid·버튼 라벨(`Refresh now`, `Make {other} primary`)을 검증. 디자인은 "현재 탭을 primary로"이므로 `onPrimary(selected)`로 의미 변경 + 라벨 `Set as primary` + 테스트 갱신. e2e(`tests/e2e/*.spec.ts`)의 셀렉터도 `grep -rn "Make " tests/`로 선행 확인
- **VALIDATE**: `pnpm test -- UsagePanel ProviderTabs SettingsPanel SpeechBubble HistoryGraph`

### Task 10: 테스트 신규/갱신

- **ACTION**: 변경 전 실패하는 테스트 먼저(RED) — TDD
- **IMPLEMENT**:
  - `manifest.test.ts`: 생성된 `src-tauri/resources/pets/{cat,corgi}/manifest.json` 검증 케이스 — `validatePetManifest` 통과 + states 4키
  - `ProviderTabs`: primary ★ 표시/미표시
  - `UsageGauge`: progressbar aria-valuenow, sev data-attr, 150% 클램프
  - `UsagePanel`: Set as primary가 `onPrimary(selected)` 호출, freshness 줄
  - `App.test.ts`: primaryProvider=codex 설정 시 selectedPetId 'corgi' 동기화 호출
  - `gateway.test.ts`: os 필드 passthrough
- **MIRROR**: TEST_STRUCTURE
- **VALIDATE**: `pnpm test:coverage` — 80%+ 유지 (현 기준선 하회 금지)

### Task 11: 문서 갱신

- **ACTION**: `docs/ui-contract.md` §4.1에 확정 색상 토큰 값 각주 추가(시맨틱→실값 결정 기록), README에 펫 패키지 빌드 스크립트 사용법 1줄
- **VALIDATE**: 링크·표 렌더 확인

---

## Testing Strategy

### Unit Tests

| Test | Input | Expected | Edge? |
| --- | --- | --- | --- |
| ring stale dim | stale=true | `data-stale="true"`, dim 0.42 | |
| ring unknown | usedPercent=null | dasharray `0 100`, `--sev-unknown` 트랙 | ✓ |
| badge matrix | 5 system states | 기존 aria-label 5종 + SVG 렌더 | |
| gauge progressbar | 68% warn | `aria-valuenow=68`, `data-severity=warn` | |
| gauge clamp | 150% | valuenow 100 | ✓ |
| tabs primary | primary='claude' | Claude 탭에 ★ | |
| panel set-primary | 클릭 | `onPrimary(selected)` | |
| pet sync | primary=codex, selectedPetId='cat' | updateSettings에 'corgi' | |
| manifest cat/corgi | 생성 파일 | validatePetManifest 통과 | |
| capabilities os | wire {os:'windows'} | passthrough | |

### Edge Cases Checklist

- [ ] usedPercent null/음수/150 클램프
- [ ] 펫 패키지 부재(리소스 복사 실패) → `petPackageError` 경로 회귀 없음
- [ ] 다크 모드에서 배지 테두리 색 (`--color-surface` 사용, 흰색 하드코딩 금지)
- [ ] `prefers-reduced-motion` 스피너 정지 유지
- [ ] platformCapabilities null(overlay fetch 실패) → 'linux' 폴백 클래스
- [ ] 이미 pets 디렉터리 존재 시 Rust 복사 스킵(덮어쓰기 금지)

## Validation Commands

### Static Analysis
```bash
pnpm check && pnpm lint
```
EXPECT: 0 errors

### Unit Tests
```bash
pnpm test
```
EXPECT: all pass

### Coverage
```bash
pnpm test:coverage
```
EXPECT: ≥80%, 기존 기준선 유지

### Rust
```bash
cargo fmt --check --manifest-path src-tauri/Cargo.toml && cargo test --manifest-path src-tauri/Cargo.toml
```
EXPECT: pass

### Build
```bash
pnpm build
```
EXPECT: 성공, woff2 번들 포함

### E2E (renderer fixture)
```bash
pnpm test:e2e:renderer
```
EXPECT: 기존 시나리오 회귀 없음 (셀렉터 변경분 반영 후)

### Manual Validation
- [ ] `pnpm tauri dev` — 오버레이: 링 색/라벨/stale dim, cat 펫 프레임 애니메이션
- [ ] 패널: OS별 스타일(현 OS), 탭 ★, 게이지 sev 색, freshness 줄
- [ ] 설정에서 primary를 Codex로 변경 → 재시작 후 corgi 표시
- [ ] OS 다크 모드 전환 → 패널/배지 다크 토큰 적용
- [ ] 펫 이미지 흰 배경 키잉 품질 육안 확인 (라이트/다크 배경 모두)

## Acceptance Criteria

- [ ] 4개 확정 결정(1a/OS적응형/라이트+다크/펫 패키지) 전부 반영
- [ ] 모든 validation 명령 통과
- [ ] 기존 접근성 계약(aria-label 문자열, role 구조) 불변
- [ ] ui-contract §4/§6 준수 (stale dim, 폴백 체인, 배지 시맨틱)

## Completion Checklist

- [ ] 토큰만으로 색 제어 — 컴포넌트 내 hex 하드코딩 없음 (SVG 배지 포함)
- [ ] `data-severity`/`data-platform` 속성 패턴 일관
- [ ] 테스트 접근성 쿼리 우선
- [ ] 원본 docs/UI-plan 에셋 무변경 (복사만)
- [ ] console.log 없음
- [ ] 커밋 분리: (1) 토큰/폰트 (2) 오버레이 (3) 패널 (4) 에셋+Rust (5) 테스트/문서

## Risks

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| 펫 PNG 흰 배경(알파 없음) 키잉 품질 저하 | High | Medium | 임계값 키잉 후 육안 확인; 불량 시 키잉 생략 + 원형 마스크(CSS `border-radius:50%` + `overflow:hidden`)로 대체 |
| 오버레이 창이 설정 변경 이벤트 미수신 → 펫 전환 지연 | High | Low | v1 한계로 문서화 (재시작 시 반영). 이벤트 브릿지는 후속 |
| `backdrop-filter` 플랫폼별 미지원(Linux WebKitGTK) | Medium | Low | macOS 변형에만 사용; Win/Linux는 불투명 배경 |
| 버튼 라벨 변경으로 e2e 셀렉터 파손 | Medium | Medium | Task 9에서 `grep -rn "Make " tests/` 선행 확인 |
| 512px×16프레임×2펫 번들 용량 증가 | Low | Low | PNG optimize=True; 필요시 384px로 축소 |
| Rust 리소스 복사 dev/prod 경로 차이 | Medium | Medium | dev에서 `resource_dir` 실측 후 조정; 실패는 조용히 폴백(petPackageError UI 존재) |

## Notes

- 디자인 doc의 데모 수치(68%/31% 등)는 예시일 뿐 — 코드에 하드코딩 금지
- `PET_STATES`(manifest.ts:1)에 레거시 키('ok','warn'…)가 남아 있으나 states 맵은 `idle_*` 키만 사용 — 정리는 범위 밖
- 다크 토큰 값은 이 계획이 결정 원본. 구현 중 변경 시 이 파일과 ui-contract 각주를 함께 갱신할 것
- 링 옵션·패널 스타일·테마·에셋 범위는 2026-07-17 사용자 선택으로 확정 (1a / OS별 적응형 / 라이트+다크 / 펫 패키지 포함)
