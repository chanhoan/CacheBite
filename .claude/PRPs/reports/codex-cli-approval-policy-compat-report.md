# Implementation Report: Codex CLI `--ask-for-approval` 호환성 복구

## Summary

Codex CLI 0.149.0이 `--ask-for-approval`에서 `untrusted`를 제거하면서 CacheBite의 고정 인자 `codex -s read-only -a untrusted app-server`가 exit 2로 즉시 거부되던 문제를 수정했다. 세 곳(네이티브 + WSL 로그인/인터랙티브 런처)의 인자 값을 `never`로 교정하고, 인자 목록을 `app_server_argv()` 단일 원본으로 모아 테스트로 잠갔다.

아울러 이 실패가 "잠시 후 재시도합니다"라는 회복 가능한 오류로 위장되던 원인을 제거했다: 프로토콜을 한 줄도 말하지 못하고 스스로 종료한 자식 프로세스를 `classify_launch_exit`가 `CliIncompatible`로 분류하고, 패널이 "The Codex CLI rejected this CacheBite build. Update CacheBite." 라는 실행 가능한 문구를 표시한다. 마지막으로 상류 CLI 변경을 사용자보다 먼저 감지할 자격증명 불필요 주간 CI 체크를 추가했다.

## Assessment vs Reality

| Metric | Predicted (Plan) | Actual |
| --- | --- | --- |
| Complexity | Medium | Medium — 예측대로 |
| Confidence | 9/10 | 타당했음. 근본 원인·수정안은 정확했고, 어긋난 것은 부수 사실 2건 (아래 Deviations) |
| Files Changed | 16 | **22** (19 수정 + 3 신규) |
| Rust tests | — | 154 → **157** |
| Renderer tests | — | 378 → **385** |

## Tasks Completed

| # | Task | Status | Notes |
| --- | --- | --- | --- |
| 1 | 네이티브 approval policy 교정 | 완료 | `app_server_argv()`로 추출 (Task 3과 통합) |
| 2 | WSL 런처 2종 교정 | 완료 | `-lc`/`-ic` 양쪽 모두 |
| 3 | 고정 인자 회귀 테스트 | 완료 | 편차 — 아래 D1 |
| 4 | `classify_launch_exit` 도입 | 완료 | dead `exit_127_is_cli_missing` 제거, 두 수집 경로 배선 |
| 5 | 분류 로직 테스트 | 완료 | 편차 — 아래 D2 |
| 6 | 렌더러에 `cli_incompatible` 전달 | 완료 | 편차 — 아래 D3 (플랜이 놓친 4번째 사이트 발견) |
| 7 | 문서 동기화 | 완료 | architecture.md / ui-contract.md / CLAUDE.md |
| 8 | 호환성 체크 + CI 잡 | 완료 | 편차 — 아래 D4, D5 (**플랜의 기술적 가정이 틀렸음**) |
| + | 스크립트 ↔ 코드 드리프트 방지 테스트 | 추가 | 플랜에 없던 항목 — 아래 D6 |

## Validation Results

| Level | Status | Notes |
| --- | --- | --- |
| Static Analysis (Rust) | 통과 | `cargo fmt --check` clean, `cargo clippy --all-features --all-targets -- -D warnings` 경고 0 |
| Static Analysis (TS) | 통과 | `svelte-check` 370 files / 0 errors / 0 warnings |
| Lint | 통과 | `eslint .` + `prettier --check .` clean (ProviderColumn.svelte 1회 `--write` 필요했음) |
| Unit Tests (Rust) | 통과 | 157 passed / 0 failed |
| Unit Tests (Renderer) | 통과 | 385 passed / 0 failed, 27 files |
| Coverage | 통과 | **96.35% stmts / 91.19% branches** (게이트 80%) |
| Build | 통과 | `vite build` 성공, 165 modules |
| Real-CLI 검증 | 통과 | `scripts/check-codex-cli-compat.sh` 양방향 확인 — 아래 참조 |
| GUI 수동 검증 | **미실행** | 헤드리스 세션이라 `pnpm tauri dev` 화면 확인 불가 — 아래 "미검증 항목" 참조 |

### 실제 CLI 검증 (codex-cli 0.149.0, Windows 11)

```
$ bash scripts/check-codex-cli-compat.sh
Codex CLI argument surface OK: codex-cli 0.149.0                     # exit 0

$ sed 's/-a never app-server/-a untrusted app-server/' … | bash      # 회귀 시뮬레이션
Codex CLI rejected CacheBite's fixed arguments (installed: codex-cli 0.149.0).
--- CLI stderr ---
error: invalid value 'untrusted' for '--ask-for-approval <APPROVAL_POLICY>'
  [possible values: on-request, never]
------------------                                                   # exit 1
```

Windows 통합 테스트 `a_cli_that_rejects_our_arguments_reports_cli_incompatible`가 실제 `.cmd` 자식 프로세스를 띄워 `Failed { class: CliIncompatible }`까지 도달함을 확인 — 버그가 보고된 바로 그 플랫폼에서 배선이 검증됐다.

## Files Changed

### 신규 (3)

| File | Action | Lines |
| --- | --- | --- |
| `scripts/check-codex-cli-compat.sh` | CREATED | +45 |
| `.github/workflows/codex-cli-compat.yml` | CREATED | +33 |
| `.claude/PRPs/plans/codex-cli-approval-policy-compat.plan.md` | CREATED | (계획 문서, 이후 `completed/`로 이동) |

### 수정 (19) — `+254 / -26`

| File | Action | Lines |
| --- | --- | --- |
| `src-tauri/src/collectors/codex.rs` | UPDATED | +59 / -7 |
| `src-tauri/src/collectors/tests.rs` | UPDATED | +112 / -4 |
| `src-tauri/src/collectors/mod.rs` | UPDATED | +8 |
| `src-tauri/src/collectors/wsl.rs` | UPDATED | +4 / -3 |
| `src-tauri/src/domain.rs` | UPDATED | +2 |
| `src/lib/contracts/domain.ts` | UPDATED | +6 / -1 |
| `src/lib/stores/providers.ts` | UPDATED | +1 / -1 |
| `src/lib/components/panelModels.ts` | UPDATED | +3 / -1 |
| `src/lib/state/presentation.ts` | UPDATED | +1 |
| `src/lib/components/systemGuidance.ts` | UPDATED | +8 / -2 |
| `src/lib/components/ProviderColumn.svelte` | UPDATED | +3 / -1 |
| `src/lib/state/presentation.test.ts` | UPDATED | +17 |
| `src/lib/components/systemGuidance.test.ts` | UPDATED | +21 |
| `src/lib/components/{ProviderColumn,UsagePanel,panelModels}.test.ts` | UPDATED | 각 +1 |
| `docs/architecture.md` | UPDATED | +8 / -1 |
| `docs/ui-contract.md` | UPDATED | +2 / -1 |
| `CLAUDE.md` | UPDATED | +1 / -1 |

## Deviations from Plan

### D1 — 플랜의 커버리지 공백 주장이 부정확했음 (Task 3)

**WHAT**: 플랜은 "인터랙티브 WSL 런처는 현재 테스트 커버리지가 없다"고 단정했다.

**WHY**: 사실이 아니었다. `tests.rs`의 WSL 통합 테스트가 fake `wsl.exe`에 전달된 argv 파일로 인터랙티브 런처 문자열을 이미 검증하고 있었다. 다만 `wsl_codex_arguments_are_fixed` 단위 테스트에서는 빠져 있었으므로, `CODEX_INTERACTIVE_LAUNCH_SCRIPT`를 `pub(crate)`로 올리고 두 런처를 **한 테스트에서** 함께 단언하도록 했다 — "두 런처가 같은 정책을 갖는다"는 것이 이번 버그의 실제 교훈이기 때문.

### D2 — 크로스플랫폼 fixture 헬퍼가 존재하지 않았음 (Task 5)

**WHAT**: 플랜은 "기존 fixture 헬퍼를 따르라"고 했으나, 기존 fake-CLI fixture는 전부 `#[cfg(unix)]` 셸 스크립트다.

**WHY**: 개발/버그 보고 환경이 Windows이므로 unix 전용 테스트는 로컬에서 아예 실행되지 않는다. `#[cfg(unix)]` 셸 스크립트 버전과 `#[cfg(windows)]` `.cmd` 버전을 같은 이름으로 둘 다 작성해 두 플랫폼 모두에서 종료 코드 배선이 검증되게 했다. 실제로 Windows 쪽이 로컬에서 실행되어 통과했다.

### D3 — 플랜이 예측하지 못한 4번째 타입 에러 사이트 (Task 6)

**WHAT**: 플랜은 `FailureClass` 확장으로 픽스처 3곳이 깨질 것이라 예측했다. `svelte-check`는 **4곳**을 보고했다.

**WHY**: `src/lib/stores/providers.ts`에 공유 타입을 쓰지 않고 인라인 리터럴 유니온 `'network' | 'provider' | 'parse' | 'internal'`이 **중복 정의**되어 있었다. 이것이 `App.svelte`의 대입을 막았다. 인라인 유니온을 이미 import 되어 있던 `FailureClass`로 교체 — 결과적으로 이번 작업이 기존 타입 중복도 하나 제거했다.

### D4 — **플랜의 기술적 가정이 틀렸음: `--help`는 값 검증을 수행하지 않는다** (Task 8)

**WHAT**: 플랜은 GOTCHA로 "`--help`를 하위 명령까지 붙이면 루트 옵션 값 검증이 실제로 일어난다"고 명시했고, 스크립트도 그렇게 설계했다.

**WHY**: 구현 중 음성 테스트(일부러 `untrusted`로 되돌려 실행)가 **통과해버려** 발견했다. 실측:

```
codex -s read-only -a untrusted app-server --help     → exit 0   ← 검증 안 함
codex -s read-only -a untrusted app-server </dev/null → exit 2   ← 실제 거부
codex -s read-only -a never     app-server </dev/null → exit 0
```

clap은 `--help`에서 단락 실행하여 값 검증을 건너뛴다. 플랜대로 만들었다면 **이 가드는 아무것도 잡지 못하는 무용지물**이었을 것이다.

**수정**: 스크립트가 `--help` 대신 stdin을 EOF로 닫은 **실제 호출**을 실행한다. app-server는 요청을 한 건도 받지 못하고 exit 0으로 종료하므로 여전히 자격증명이 필요 없다. 또한 **exit 2(clap usage error)일 때만 실패**시키고, 그 외 비정상 종료는 "inconclusive"로 보고만 하여 인자 표면과 무관한 런타임 문제가 오탐을 내지 않도록 했다.

### D5 — CI 잡은 이슈 자동 생성 대신 잡 실패로 신호 (Task 8)

**WHAT**: 플랜은 "실패 시 이슈를 연다"였으나, 실패하면 잡이 실패하는 것으로 끝낸다.

**WHY**: 스케줄 잡 실패는 이미 저장소 소유자에게 알림이 간다. 이슈 자동 생성은 `issues: write` 권한이 필요하고, 고쳐질 때까지 **매주 중복 이슈**를 쌓는다. 동일한 신호를 더 적은 권한과 소음으로 얻는 쪽을 택했다.

### D6 — 플랜에 없던 추가: 가드 ↔ 코드 드리프트 방지 테스트

**WHAT**: `the_compatibility_script_checks_the_argv_the_collector_actually_sends` 테스트를 추가했다.

**WHY**: `scripts/check-codex-cli-compat.sh`가 argv를 문자열로 하드코딩하므로, 누군가 `app_server_argv()`만 바꾸면 가드가 조용히 엉뚱한 인자를 검사하게 된다. 이번 버그의 근본 패턴(단일 원본 없이 흩어진 인자)이 가드 자체에서 재발하는 것을 막는다.

## Issues Encountered

| Issue | Resolution |
| --- | --- |
| `pnpm`이 PATH에 없음 | `corepack pnpm ...`으로 실행 (저장소가 corepack 전제) |
| `--help` 음성 테스트가 통과해버림 | D4 — 플랜 가정 오류를 실측으로 반증하고 스크립트를 실제 호출 기반으로 재작성 |
| `presentation.test.ts` 첫 작성분이 실패 | `applyProviderUpdate`에 스냅샷 형태를 넘겨 `system`이 `active`로 남았다. 강등 경로는 `{kind:'fetch_failed'}` 상태 업데이트 형태라야 함 — 구현이 아니라 테스트가 틀렸던 케이스라 테스트를 수정 |
| `rustfmt`/`prettier` 재포맷 필요 | 각각 `cargo fmt`, `prettier --write` 1회 적용 후 재검증 |

## Tests Written

| Test File | Tests | Coverage |
| --- | --- | --- |
| `src-tauri/src/collectors/tests.rs` | `native_codex_argv_is_fixed` | 네이티브 argv 고정 (기존에 아무 테스트도 잠그지 않던 지점) |
| `src-tauri/src/collectors/tests.rs` | `wsl_codex_arguments_are_fixed` (확장) | 두 WSL 런처의 정책 일치 + `untrusted` 부재 |
| `src-tauri/src/collectors/tests.rs` | `classify_launch_exit_separates_a_missing_cli_from_a_rejected_invocation` | 127 / 2 / 1 / 0 / None 5케이스 — 오탐 방지용 `None` 포함 |
| `src-tauri/src/collectors/tests.rs` | `a_cli_that_rejects_our_arguments_reports_cli_incompatible` ×2 (unix, windows) | spawn → 종료 코드 판독 → `Failed{CliIncompatible}` 전 구간 |
| `src-tauri/src/collectors/tests.rs` | `the_compatibility_script_checks_the_argv_the_collector_actually_sends` | 가드 스크립트 ↔ `app_server_argv()` 드리프트 |
| `src/lib/components/systemGuidance.test.ts` | +5 | `cli_incompatible` 전용 문구 (provider별) + 회복 가능 클래스 3종의 기존 문구 유지 |
| `src/lib/state/presentation.test.ts` | +1 | `lastFailure` → `PanelProviderModel.failureClass` 전달 |

## 미검증 항목 (정직한 한계)

- **GUI 실행 검증 없음.** 헤드리스 세션이라 `pnpm tauri dev`로 패널을 눈으로 확인하지 못했다. 프로토콜 수준에서는 실제 CLI가 정상 데이터를 반환함을 확인했고(`account/rateLimits/read` → `planType: "team"`, 주간 창), Windows 통합 테스트가 거부 경로를 커버한다. 그러나 **화면에 게이지가 실제로 그려지는 것은 확인하지 않았다.**
- **E2E 미실행.** `pnpm test:e2e:renderer`는 실행하지 않았다 — 변경이 fixture 게이트웨이 형태를 바꾸지 않으므로 영향이 없다고 판단했다. CI가 검증한다.
- **WSL 경로 실기기 검증 없음.** WSL 런처 수정은 문자열 단언과 fake `wsl.exe` 통합 테스트로만 검증됐다.

## Next Steps

- [ ] `pnpm tauri dev`로 Codex 열에 사용량이 표시되는지 육안 확인 (사용자 환경 필요)
- [ ] `/code-review`로 변경 검토
- [ ] `/prp-pr`로 PR 생성 — 저장소 흐름상 `fix/codex-cli-approval-policy-compat` → `develop` → `main`
