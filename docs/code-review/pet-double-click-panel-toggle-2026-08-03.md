# Code Review: Pet Double-Click Panel Toggle

**Reviewed**: 2026-08-03  
**Scope**: `HEAD` 대비 로컬 변경사항 및 관련 미추적 계획/구현 보고서  
**Decision**: APPROVE WITH COMMENTS

## Findings

### CRITICAL

None.

### HIGH

None.

### MEDIUM

**[MEDIUM]** `tests/e2e/native.spec.ts:136`

Issue: 테스트 이름과 주석은 패널을 시작 상태로 되돌린다고 설명하지만, 토글을 세 번 수행하면 최종 상태는 시작 상태의 반대가 된다. 현재 검증은 반환값이 `A -> B -> A`로 교대하는지만 확인하며, `beforeEach`도 창 전환만 하고 가시성을 초기화하지 않는다. 따라서 이 테스트는 반대 상태를 다음 테스트에 넘기고, 파일 내 테스트가 실행 순서에 의존하게 된다.

Fix: 각 테스트 시작 전에 panel 창에서 `hide_panel`을 호출해 숨김 상태로 초기화하거나, 해당 테스트가 짝수 번 토글한 뒤 원래 상태로 복원하도록 변경한다. 정리 로직은 테스트 실패 시에도 실행되도록 `afterEach` 또는 `try/finally`에 두는 편이 안전하다.

**[MEDIUM]** `tests/e2e/native.spec.ts:150`

Issue: 새 네이티브 E2E는 `toggle_panel`의 반환 문자열만 비교하고 실제 네이티브 panel 창이 표시되거나 숨겨졌는지는 확인하지 않는다. `begin_reveal`, 150ms 레이아웃 게이트, `reveal_panel`, 또는 `conceal_panel`이 회귀해도 명령이 `shown`/`hidden`을 반환하기만 하면 테스트가 통과한다. `section`의 DOM 존재 확인도 숨겨진 Tauri 창의 DOM이 유지되므로 OS 창 가시성을 증명하지 못한다.

Fix: 각 토글 뒤 Tauri window API의 `isVisible()` 결과를 기다려 반환값과 실제 창 가시성이 일치하는지 검증한다. 최소한 show 경로는 grace timeout 이후 `true`, hide 경로는 `false`가 되는지 확인해야 한다.

### LOW

None.

## Summary

네이티브 가시성과 pending reveal을 기준으로 토글을 결정하고, 닫기·핫키·전체화면 경로에서 레이아웃 게이트를 해제하는 구현은 일관적이다. IPC 권한도 overlay 전용 토글과 panel 전용 숨김으로 분리되어 있으며 렌더러는 가시성 사본을 보관하지 않는다. 프로덕션 코드에서 차단할 결함은 발견하지 못했지만, 새 네이티브 E2E가 상태를 복원하지 않고 실제 창 가시성을 검증하지 않아 테스트 신뢰성 보완을 권장한다.

## Validation Results

| Check | Result |
|---|---|
| `git diff --check HEAD` | Pass |
| `corepack pnpm test:ci` | Pass — 23 files, 247 tests; build pass |
| Coverage | Pass — statements 95.79%, branches 90.40%, functions 93.61%, lines 95.79% |
| `corepack pnpm test:e2e:renderer` | Pass — 2 specs, 8 tests |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | Pass |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings` | Pass |
| `cargo test --manifest-path src-tauri/Cargo.toml --all-features` | Pass — 116 tests |
| `corepack pnpm test:e2e` | Not run — WebDriver 전용 Tauri 빌드와 fixture 환경이 필요한 별도 native-smoke 절차 |

Rust 테스트 중 MSVC linker가 import library 생성 메시지를 warning으로 출력했지만 테스트 실패나 Clippy 경고는 없었다.

## Files Reviewed

### Source / Config

- `src-tauri/src/lib.rs`
- `src-tauri/src/refresh/ipc.rs`
- `src-tauri/src/window/mod.rs`
- `src/App.svelte`
- `src/lib/api/fixtureGateway.ts`
- `src/lib/api/gateway.ts`
- `src/lib/components/PetOverlay.svelte`

### Tests

- `src-tauri/src/window/tests.rs`
- `src/App.test.ts`
- `src/lib/api/gateway.test.ts`
- `src/lib/components/PetOverlay.test.ts`
- `tests/e2e/native.spec.ts`

### Documentation / Project Context

- `CLAUDE.md`
- `docs/beta-testing.md`
- `docs/ui-contract.md`
- `.claude/PRPs/plans/completed/pet-double-click-panel-toggle.plan.md`
- `.claude/PRPs/reports/pet-double-click-panel-toggle-report.md`

## Recommendation

병합을 차단할 CRITICAL/HIGH 항목은 없다. 위 두 MEDIUM 항목을 반영해 native E2E를 상태 독립적으로 만들고 실제 창 가시성을 검증한 뒤 병합하는 것을 권장한다.
