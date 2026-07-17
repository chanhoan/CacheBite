# Code Review: UI Design Pass (docs/UI-plan 시각 디자인 적용)

**Reviewed**: 2026-07-17
**Mode**: Local (uncommitted working-tree changes)
**Branch**: develop
**Scope**: 렌더러 시각 디자인, 펫 패키지 생성/설치, 플랫폼 감지
**Decision**: **APPROVE with comments** — CRITICAL/HIGH 없음. MEDIUM 2건 권고.

## Summary

`.claude/PRPs/plans/ui-design-pass.plan.md` 계획을 충실히 구현. 디자인 토큰(라이트+다크),
1a 분할 링(5H/WK 라벨), OS별 적응형 패널, SVG 시스템 배지, cat/corgi 펫 패키지 생성·번들·주 provider
연동까지 계획대로 반영됐다. 타입 체크·단위 테스트(130 통과)·Rust fmt 모두 통과. 접근성 계약(aria-label,
role 구조)도 유지. 남은 것은 **패널의 캡처 시각 표기 손실**과 **주간 게이지 "resets in" 문구**
두 가지 카피/완성도 이슈.

## Findings

### CRITICAL
None.

### HIGH
None.

### MEDIUM

**M1 — 패널에서 캡처 시각(capturedAt) 표기가 사라짐 (완성도 / 계약 위반)**
`src/lib/components/UsagePanel.svelte`
- 변경 전: `<small>{current.capturedAt} · {current.source}</small>` — 캡처 시각 노출.
- 변경 후: `<small>● {Fresh|Stale} · {source}{· cached?}</small>` — **캡처 시각이 완전히 제거**됨.
- `docs/ui-contract.md` §5는 본문에 "캡처 시각 · source 라벨"을 요구하고, 디자인 문서(`CacheBite Pet UI.dc.html:215,449`)도 "Fresh · captured 2 min ago" / "Stale · captured 24 min ago"로 **캡처 시각을 함께** 표기한다.
- `PanelProvider.capturedAt` 필드는 여전히 `panelModel`(App.svelte:342)에서 계산되지만 뷰에서 미사용 — 죽은 데이터.
- **영향**: stale 판단만 노출되고 "언제 캡처됐는지"는 사용자가 알 수 없음. ui-contract §5의 stale 시 "캡처 시각 노출" 취지가 깨짐.
- **권고**: freshness 줄에 상대 캡처 시각을 덧붙이거나(예: `● Stale · captured 24m ago · {source}`), 최소한 `capturedAt`을 `<time datetime>`으로 재노출. 미사용이면 타입에서도 제거해 계약과 코드를 일치시킬 것.

**M2 — 주간 게이지 "resets in {resetsAt}" 문구가 절대 시각에 부적합 (카피/UX)**
`src/lib/components/UsageGauge.svelte`: `<time>resets in {usage.resetsAt}</time>`
- 5시간 창은 상대값("1h 12m")이라 "resets in 1h 12m"이 맞지만, 주간 창은 절대 시각이다. 디자인 문서는 이를 구분한다: session은 "resets in ..."(라인 203), weekly는 "resets ..."(라인 211).
- 현재 구현은 **두 창 모두 "resets in"**을 하드코딩 → 주간은 "resets in Mon 09:00"처럼 어색. 실제로 `UsageGauge.test.ts:30`이 `resetsAt: '2026-07-20T09:00:00Z'`를 넣어 **"resets in 2026-07-20T09:00:00Z"**(원시 ISO + 잘못된 전치사)가 렌더된다.
- **영향**: 주간 리셋 안내가 어색하고, resetsAt 원시 ISO가 그대로 노출(디자인은 "Mon 09:00" 휴먼화 의도).
- **권고**: label 또는 창 종류에 따라 접두사 분기("resets in" vs "resets"), 그리고 절대 시각은 최소한 로케일 포맷/상대 시간으로 휴먼화. (원시 ISO 노출은 이전에도 있었으나 "resets in" 접두사가 주간에서 오독을 키움.)

### LOW

**L1 — `pnpm lint` 레드: `.tmp-msi-build/` 빌드 산출물이 eslint 대상에 포함**
`pnpm lint` 실패(354 errors)의 전량이 `.tmp-msi-build/**`(Tauri MSI 빌드 임시물)와 벤더 `docs/UI-plan/support.js`·`image-slot.js`에서 발생. **`src/` 소스 에러는 0**. 이번 diff가 만든 문제는 아니나 lint 게이트가 막힘.
- **권고**: eslint `ignores`(및 `.gitignore`)에 `.tmp-msi-build/`, `docs/UI-plan/*.js` 추가. `package.json`의 `test:ci`가 lint를 포함하므로 CI 그린을 위해 필요.

**L2 — macOS 패널 주 버튼 색이 디자인과 불일치**
`src/lib/styles/global.css` + `UsagePanel.svelte` `.primary-action`은 `--color-accent`(#3b5bdb) 사용. Windows/Linux는 `data-platform`에서 accent를 재정의하지만 **macOS는 재정의 없음** → 디자인 문서 §5 macOS 주 버튼(`#16191d`, 다크)과 다르게 파란 accent로 렌더. 시각적 소규모 편차.

**L3 — SystemBadge 아이콘 색 `#fff` 하드코딩**
`SystemBadge.svelte` `.badge { color: #fff }`. 배경이 항상 유채색 배지라 실용상 문제없으나, 계획의 "컴포넌트 내 hex 하드코딩 없음" 원칙과는 어긋남. 토큰화(`--badge-icon`) 권장(선택).

**L4 — 오버레이 펫 라이브 전환 없음 (문서화된 v1 한계)**
주 provider 변경 시 오버레이 창은 설정 이벤트를 수신하지 않아 펫(cat↔corgi)이 재시작 후 반영. 계획에 이미 v1 한계로 명시됨 — 수용 가능. 후속 이벤트 브릿지 대상.

## Validation Results

| Check | Result |
|---|---|
| Type check (`pnpm check`) | ✅ Pass (0 errors, 0 warnings) |
| Lint (`pnpm lint`) | ⚠️ Fail — 전량 `.tmp-msi-build/`·벤더 JS, `src/` 0건 (L1) |
| Unit tests (`pnpm test`) | ✅ Pass (19 files, 130 tests) |
| Rust fmt (`cargo fmt --check`) | ✅ Pass |
| Rust tests (`cargo test`) | ⏭️ Skipped (비용) — 신규 로직에 단위 테스트 2건 동반, fmt 통과 |
| Build (`pnpm build`) | ⏭️ Skipped (비용) |

## 잘한 점 (근거)

- **첫 실행 설치 안전성**: `install_bundled_pet_packages`가 기존 패키지 존재 시 스킵(덮어쓰기 금지) + 실패는 조용히 로그, `petPackageError` UI로 폴백. 덮어쓰기 방지 단위 테스트 포함(`lib.rs`).
- **플랫폼 정규화**: `platform_os`가 미지원 OS를 `linux`로 폴백 + `window/tests.rs` 케이스. overlay 창도 `GetPlatformCapabilities` 인가됨(`window/mod.rs:79`) → `data-platform` 정상 동작.
- **접근성 유지**: 배지 aria-label 5종 문자열 불변, 링 라벨 `aria-hidden`, 게이지 `role="progressbar"`+valuenow, 탭 ★은 `aria-label`에 "(primary)" 반영.
- **스타일 패턴 일관**: 색상은 전부 `data-severity`/`data-platform` + CSS 셀렉터로 분기(JS 색 계산 없음), 토큰 단일 원본.
- **원본 에셋 무변경**: 빌드 스크립트가 `docs/UI-plan/assets` 읽기만, `src-tauri/resources/pets`로 출력. 매니페스트 `validatePetManifest` 통과 테스트 동반.

## Files Reviewed (source only; 바이너리 PNG/아이콘 제외)

Rust (Modified): `src-tauri/src/lib.rs`, `src/refresh/ipc.rs`, `src/window/mod.rs`, `src/window/tests.rs`, `tauri.conf.json`
Renderer (Modified): `src/App.svelte`, `src/main.ts`, `src/lib/api/{gateway,fixtureGateway}.ts`, `src/lib/assets/manifest.ts`, `src/lib/styles/{tokens,global}.css`, `src/lib/components/{SplitUsageRing,SystemBadge,PetOverlay,ProviderTabs,UsageGauge,UsagePanel,SettingsPanel,SpeechBubble,HistoryGraph}.svelte`
Added: `scripts/build-pet-packages.py`, `src-tauri/resources/pets/{cat,corgi}/**`, `src/lib/components/{ProviderTabs,UsageGauge}.test.ts`
Tests (Modified): `src/App.test.ts`, `src/nativeWorkflow.test.ts`, `src/lib/api/gateway.test.ts`, `src/lib/assets/manifest.test.ts`, `src/lib/components/{PetOverlay,UsagePanel}.test.ts`
Docs (Modified): `README.md`, `docs/ui-contract.md`

## Next Steps

1. M1: freshness 줄에 캡처 시각 복원 또는 `capturedAt` 필드 제거(계약/코드 일치).
2. M2: 창별 "resets in"/"resets" 접두사 분기 + 절대 시각 휴먼화.
3. L1: eslint ignore에 `.tmp-msi-build/`·벤더 JS 추가 → `pnpm lint`/`test:ci` 그린.
4. 커밋 전 `cargo test --manifest-path src-tauri/Cargo.toml`, `pnpm build` 실측 권장.
