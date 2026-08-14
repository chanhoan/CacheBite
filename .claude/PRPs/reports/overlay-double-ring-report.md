# Implementation Report: 오버레이 Double Ring (UI 전용)

## Summary

오버레이에 second provider용 작은 원(위성 링)을 추가했다. 큰 원은 종전대로 `primaryProvider`를 그리고 펫을 품으며, 작은 원은 `ringMode: 'double'`일 때만 나타나 `secondaryProvider(primary)`의 5H/WK 아크와 provider 로고를 보여준다. 동작 변경은 없다 — 패널, 알림, 업데이트, 핫키, 펫 무드 산출 경로를 한 줄도 건드리지 않았다.

## Assessment vs Reality

| Metric | Predicted (Plan) | Actual |
|---|---|---|
| Complexity | Medium | Medium — 예측대로 |
| Confidence | 9/10 | 실제로 큰 막힘 없음. 실패는 전부 기계적(픽스처 누락, JSDoc 캐스트) |
| Files Changed | 신규 3 · 수정 15 = 18 | 신규 2 · 수정 22 = 24 |

파일 수가 늘어난 이유: 계획이 예상하지 못한 테스트 픽스처 3곳(`App.test.ts`, `stores/settings.test.ts`, `state/domain.test.ts`)과 `tokens.css`가 추가로 필요했다. 반대로 신규 파일은 3개 → 2개로 줄었다(`SatelliteRing.test.ts`를 따로 만들지 않고 `PetOverlay.test.ts`의 `describe('double ring mode')` 블록에 넣는 편이 실제 조립 상태를 검증하기에 나았다).

## Tasks Completed

| # | Task | Status | Notes |
|---|---|---|---|
| 1 | second provider 헬퍼 | 완료 | `contracts/domain.ts`에 `secondaryProvider` 한 줄 |
| 2 | 네이티브 설정 스키마 v6 | 완료 | `RingMode` 열거형 + v5 마이그레이션 아암 |
| 3 | 마이그레이션 테스트 | 완료 | 편차 — 아래 참조 |
| 4 | 게이트웨이 계약 확장 | 완료 | `RingMode` 타입 + 양방향 매핑 |
| 5 | 설정 스토어/프레젠테이션 배선 | 완료 | |
| 6 | provider 로고 SVG | 완료 | 편차 — 아래 참조 |
| 7 | `SplitUsageRing`에 이름 prop | 완료 | 기본값 `'Provider'`로 기존 테스트 5건 무손상 |
| 8 | 뷰모델 확장 | 완료 | `SatelliteRingModel` + 필수 `satellite` |
| 9 | 작은 원 컴포넌트 | 완료 | `:global()` 스트로크 오버라이드가 1차에 적중 |
| 10 | `PetOverlay` 칩 + 포인터 표면 | 완료 | 기존 마크업 무변경, 추가만 |
| 11 | `App.svelte` 조립 | 완료 | 파생 1 + 필드 1 + 클램프 1 |
| 12 | 설정 UI | 완료 | Primary provider 바로 아래 `Ring` 셀렉트 |
| 13 | 문서 갱신 | 완료 | `ui-contract.md` §4.4 신설, `CLAUDE.md` 불변식 추가 |

## Validation Results

| Level | Status | Notes |
|---|---|---|
| svelte-check | 통과 | 0 errors / 0 warnings |
| ESLint | 통과 | |
| Prettier | 통과 | `src/**` 전부. 예외 1건은 아래 Issues 참조 |
| Vitest | 통과 | **304 tests / 24 files** (신규 케이스 13) |
| Coverage | 통과 | lines 96.08% · branches 90.58% · functions 90.96% (게이트 80%) |
| vite build | 통과 | 162 modules, 1.64s |
| cargo fmt | 통과 | |
| cargo clippy | 통과 | `--all-features --all-targets`, 경고 0 |
| cargo test | 통과 | **146 tests**, 0 실패 |
| E2E (renderer) | 통과 | 2 spec files, 14s |
| 시각 대조 | 산출 | 48px/128px 비교 HTML을 사용자에게 전달 |

## Files Changed

신규 2 · 수정 22, +499 / −19 라인.

| File | Action |
|---|---|
| `src/lib/components/ProviderLogo.svelte` | CREATED |
| `src/lib/components/SatelliteRing.svelte` | CREATED |
| `src/lib/components/PetOverlay.test.ts` | UPDATED (+165) |
| `src-tauri/src/store/tests.rs` | UPDATED (+55 / −8) |
| `src-tauri/src/store/settings.rs` | UPDATED (+48 / −2) |
| `src/App.svelte` | UPDATED (+39 / −5) |
| `src/lib/components/PetOverlay.svelte` | UPDATED (+38) |
| `docs/ui-contract.md` | UPDATED (+32) |
| `src/lib/components/SettingsPanel.test.ts` | UPDATED (+27) |
| `src/lib/components/SettingsPanel.svelte` | UPDATED (+19) |
| `src/lib/components/models.ts` | UPDATED (+17) |
| `src/lib/state/domain.test.ts` | UPDATED (+17 / −4) |
| `src/lib/api/gateway.ts` | UPDATED (+10) |
| `src/lib/contracts/domain.ts` | UPDATED (+9) |
| `src/lib/components/SplitUsageRing.svelte` | UPDATED (+9 / −3) |
| `src/lib/styles/tokens.css` | UPDATED (+6) |
| `src/lib/stores/settings.test.ts` | UPDATED (+6) |
| `src/lib/state/presentation.test.ts` | UPDATED (+6 / −3) |
| `src/lib/stores/settings.ts` | UPDATED (+4) |
| `src/lib/api/fixtureGateway.ts` | UPDATED (+3 / −1) |
| `src/App.test.ts` | UPDATED (+3 / −1) |
| `src/lib/state/presentation.ts` | UPDATED (+2) |
| `src-tauri/src/store/mod.rs` | UPDATED (+1 / −1) |
| `CLAUDE.md` | UPDATED (+1) |

## Deviations from Plan

1. **로고 추출 도구 (Task 6).** 계획은 Pillow 기반 Python 스크립트를 지정했으나 이 머신에 Pillow가 없었다(`py`는 있고 `PIL`은 없음). 저장소의 `scripts/build-pet-packages.py`가 이미 Pillow에 의존하므로 설치가 정당화될 수는 있었지만, 사용자 환경을 변경하지 않기 위해 Node 내장 `zlib`만 쓰는 최소 PNG 디코더를 scratchpad에 작성해 대체했다. 결과는 동등하고 저장소에 남는 산출물은 없다.

2. **Codex 글리프가 검정 획이 아니라 투명 knockout이었다 (Task 6).** 계획은 `stroke="#000"`으로 그리는 것을 전제했다. 실제 원본은 `>` 와 `_` 위치의 알파가 0이라 뒤가 비쳐 보이는 구조였다. `<mask>`로 구현해 원본 의도를 살렸고, 칩에서는 퍽 색이 글리프로 드러난다.

3. **Codex 블롭이 8 로브가 아니라 6 로브였다 (Task 6).** 계획은 "45° 간격 8개 원(반지름 46, 중심거리 52)"으로 추정했다. 실루엣을 10° 간격으로 실측하니 주기가 60°였고 반지름이 98↔92로 진동했다. 반지름 68·중심거리 30·15° 오프셋의 6원 합집합이 이 프로파일을 정확히 재현한다. 획 두께는 계획의 추정치 14가 실측(143px/2048)과 일치했다.

4. **`SatelliteRing.test.ts`를 별도로 만들지 않았다 (Task 9).** 칩 단독 렌더보다 `PetOverlay` 안에서의 조립 상태(두 링 공존, 배지 독립성, 포인터 표면 중복)가 실제 회귀 위험 지점이라, `PetOverlay.test.ts`에 `describe('double ring mode')` 블록으로 넣었다.

5. **테스트 픽스처 4곳이 계획에 없었다.** 계획은 `PetOverlay.test.ts`/`presentation.test.ts`/`SettingsPanel.test.ts`만 예상했으나 `App.test.ts`, `stores/settings.test.ts`도 필수 필드 누락으로 깨졌다. `state/domain.test.ts`에는 `secondaryProvider` 테스트를 추가했다.

6. **`tokens.css`가 수정 목록에 없었다.** 계획 본문은 `--logo-claude` 토큰을 언급했지만 Files to Change 표에서 빠져 있었다. 3개 토큰(`--logo-claude`, `--logo-codex-from`, `--logo-codex-to`)을 `:root`에만 넣었다 — 브랜드 색이므로 다크 변형을 두지 않았다.

## Issues Encountered

1. **Rust 문자열 어서션 누락.** 계획은 `assert_eq!(loaded.schema_version, 5)` 7건만 짚었는데, `legacy_settings_are_migrated_and_rewritten`에 JSON 텍스트를 검사하는 `assert!(rewritten.contains("\"schema_version\": 5"))`가 하나 더 있었다. 첫 테스트 실행에서 잡아 수정했다.

2. **`.ts` 파일에서 JSDoc `@type {const}` 캐스트가 동작하지 않음.** 처음 작성한 테스트 헬퍼가 `/** @type {const} */ ('active')` 형태였는데 TypeScript 파일이라 무시되어 `string`으로 추론되고 11개 타입 오류가 났다. 헬퍼 시그니처를 `(overrides: Partial<SatelliteRingModel>): SatelliteRingModel` 형태의 TS 주석으로 바꿔 해결했다.

3. **`querySelectorAll` 인덱싱이 `strict` 하에서 `possibly undefined`.** 스프레드로 배열화한 뒤 `.map()` 결과를 한 번에 비교하도록 바꿨다.

4. **미해결 (범위 밖):** `corepack pnpm lint`가 `.superpowers/sdd/task-4-report.md` 하나를 포맷 위반으로 잡는다. 이 파일은 git에 추적되지 않는 로컬 스킬 산출물이며 이번 변경과 무관하다. CI에는 존재하지 않으므로 파이프라인에 영향이 없다. 로컬에서 `pnpm lint`를 깨끗하게 돌리려면 `.prettierignore`에 `.superpowers`를 추가하면 되지만, 이번 작업 범위 밖이라 손대지 않았다.

## Tests Written

| Test File | Tests | Coverage |
|---|---|---|
| `src/lib/components/PetOverlay.test.ts` | +7 (`double ring mode`) | 칩 렌더/미렌더, provider별 접근명, 칩 위 제스처(더블클릭·우클릭), 접근성 트리 배제, 링별 stale 독립, 양방향 배지 독립성, Claude/Codex 로고 분기 |
| `src/lib/components/SettingsPanel.test.ts` | +2 | Ring 셀렉트가 primary를 건드리지 않음, Appearance와 분리 유지 |
| `src/lib/state/domain.test.ts` | +2 | `secondaryProvider` 매핑과 involution 성질 |
| `src-tauri/src/store/tests.rs` | +2 | v5→v6 마이그레이션(필드 보존), `ring_mode` 왕복 |
| `src/lib/stores/settings.test.ts` | 보강 | 기본값이 Rust `RingMode::Single`과 일치, 다른 setter가 건드리지 않음 |

## Next Steps

- [ ] 전달된 `double-ring-check.html`로 로고 시각 대조 확인
- [ ] `pnpm tauri dev`로 실기 확인 — 특히 패널에서 Ring 변경 시 오버레이 즉시 반영, 칩 위 드래그
- [ ] **배포 전 상표 확인** — Claude Code·Codex 마크는 타사 상표. 교체가 필요하면 `ProviderLogo.svelte` 한 파일만 바꾸면 된다
- [ ] `/code-review`
- [ ] `/prp-pr`
