# Code Review: Pet Hide/Show Hotkey

**Reviewed**: 2026-07-31  
**Scope**: Uncommitted local changes (`HEAD` 대비 수정 20개 파일과 관련 미추적 계획/보고서)  
**Decision**: **REQUEST CHANGES**  
**Findings**: CRITICAL 0, HIGH 1, MEDIUM 2, LOW 0

## Summary

전역 단축키의 저장·마이그레이션·렌더러 매핑·전체 화면 숨김 상태 조정은 전반적으로 일관되게 구현되어 있다. 새 기본값과 명시적 비활성화(`null`)가 구분되고, 시작 시 등록 실패가 앱 시작을 막지 않으며, 프론트엔드와 Rust의 전체 자동 검증도 통과했다.

다만 단축키 교체/비활성화 시 OS 등록 해제 실패를 무시하므로 저장된 설정과 실제 활성 단축키가 달라질 수 있다. 이 경로는 사용자가 기능을 비활성화했다고 믿어도 기존 전역 단축키가 계속 동작하게 만들 수 있으므로 병합 전에 수정해야 한다.

## Findings

### CRITICAL

None.

### HIGH

**[HIGH]** `src-tauri/src/refresh/ipc.rs:281`

Issue: 단축키 교체와 비활성화 경로가 `manager.unregister(...)`의 오류를 모두 무시한다. 새 단축키 등록 후 기존 단축키 해제가 실패하면 두 단축키가 모두 활성 상태로 남을 수 있고, 비활성화 중 해제가 실패하면 설정 파일에는 `null`이 저장되고 성공 응답까지 반환되지만 기존 단축키는 계속 앱을 토글할 수 있다. 이는 저장된 상태, UI 표시, 실제 OS 상태 사이의 불일치이며 명시적인 비활성화 동작을 깨뜨린다.

Fix: 해제를 트랜잭션의 실패 가능한 단계로 처리한다. 교체 시 기존 단축키 해제가 실패하면 방금 등록한 새 단축키를 해제하고 이전 설정을 복원한 뒤 오류를 반환한다. 비활성화 시 기존 단축키 해제가 실패하면 이전 설정을 복원하고 성공 이벤트를 방출하지 않는다. 등록/해제 관리자를 추상화해 각 실패 분기의 보상 동작을 자동 테스트로 고정한다.

### MEDIUM

**[MEDIUM]** `src-tauri/src/refresh/ipc.rs:254`

Issue: 하나의 `update_settings` 요청이 `start_at_login`과 `hide_show_hotkey`를 함께 변경할 때, autostart 변경이 성공한 뒤 단축키 등록이 실패하면 저장 파일만 `previous`로 복원되고 이미 적용된 autostart 상태는 되돌리지 않는다. IPC는 전체 설정 객체를 받으므로 호출자가 여러 필드를 한 번에 변경할 수 있으며, 실패 응답 후 저장 상태와 OS autostart 상태가 달라질 수 있다.

Fix: 외부 부수효과를 적용하기 전에 변경 계획을 만들고, 뒤 단계 실패 시 앞서 적용한 autostart 변경까지 역순으로 보상한다. 보상 실패도 로그와 명확한 오류로 남긴다.

**[MEDIUM]** `src-tauri/src/refresh/ipc.rs:268`

Issue: 이번 기능의 가장 위험한 경계인 “등록 성공/실패, 기존 단축키 해제 성공/실패, 비활성화” 전환에는 자동 테스트가 없다. 현재 테스트는 문자열 파싱·저장소·순수 fullscreen 정책·렌더러 오류 문구를 검증하지만 실제 `update_settings` 상태 전환과 롤백 계약은 검증하지 않는다.

Fix: 전역 단축키와 autostart 작업을 주입 가능한 작은 어댑터로 분리하고, 새 단축키 충돌, 기존 단축키 해제 실패, 비활성화 실패, 다중 필드 변경 후 후속 단계 실패를 단위 테스트한다. 실제 OS 등록은 수동/E2E 체크로 유지한다.

## Validation Results

| Check | Result | Notes |
|---|---|---|
| `corepack pnpm test:ci` | Pass | svelte-check 0 errors/warnings, ESLint/Prettier pass, Vitest 245/245, Vite build pass |
| Frontend coverage | Pass | Statements 95.58%, branches 90.69%, functions 92.9%, lines 95.58% |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | Pass | 변경 없음 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings` | Pass | Clippy 경고 없음 |
| `cargo test --manifest-path src-tauri/Cargo.toml --all-features` | Pass | 115/115 |
| `git diff --check` | Pass | 공백 오류 없음 |
| `corepack pnpm audit --audit-level high` | Pass | 기존 audit 설정 기준 종료 코드 0; 전체 보고에는 low 1, moderate 7, ignored high 경로 3이 표시됨 |
| `cargo audit --file src-tauri/Cargo.lock` | Skipped | 로컬에 `cargo-audit` 명령이 설치되어 있지 않음 |
| Native global-hotkey manual test | Skipped | 실제 OS 단축키 등록·충돌·해제와 Windows fullscreen 조합은 이 리뷰에서 수동 실행하지 않음 |
| Renderer/native E2E | Skipped | 단위·정적·빌드 검증에 집중; 실제 OS 단축키 동작은 기존 renderer fixture로 검증할 수 없음 |

## Files Reviewed

### Source and configuration

- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock`
- `src-tauri/src/lib.rs`
- `src-tauri/src/refresh/ipc.rs`
- `src-tauri/src/store/settings.rs`
- `src-tauri/src/window/mod.rs`
- `src/App.svelte`
- `src/lib/api/fixtureGateway.ts`
- `src/lib/api/gateway.ts`
- `src/lib/components/SettingsPanel.svelte`
- `src/lib/state/presentation.ts`
- `src/lib/stores/settings.ts`

### Tests

- `src-tauri/src/store/tests.rs`
- `src-tauri/src/window/tests.rs`
- `src/App.test.ts`
- `src/lib/api/gateway.test.ts`
- `src/lib/components/SettingsPanel.test.ts`
- `src/lib/state/presentation.test.ts`
- `src/lib/stores/settings.test.ts`

### Documentation and implementation context

- `docs/beta-testing.md`
- `.claude/PRPs/plans/completed/pet-hide-show-hotkey.plan.md`
- `.claude/PRPs/plans/preset-pet-hide-show-hotkey.plan.md`
- `.claude/PRPs/reports/pet-hide-show-hotkey-report.md`

## Recommendation

기존 단축키 해제 오류와 복합 설정 롤백을 처리하고 해당 전환 테스트를 추가한 뒤 다시 검토한다. 그 전까지는 커밋/병합을 권장하지 않는다.
