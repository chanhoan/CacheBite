# Implementation Report: Default-Only Hide/Show Shortcut

## Summary

hide/show 단축키의 사용자 지정 경로를 전부 제거하고 고정 상수
`CommandOrControl+Shift+H` 하나만 남겼다. 단축키 상태는 더 이상 `settings.json`에
저장되지 않으며(스키마 v4 → v5), 앱은 매 실행마다 무조건 등록을 시도한다. 등록
실패는 설정 파일을 덮어쓰는 대신 `PlatformCapabilities.hide_show_hotkey` 진단으로
보고된다. Settings 화면은 편집 가능한 입력 대신 현재 OS 기준 실제 키를 읽기 전용으로
표시한다.

보고된 버그(`No shortcut active` 영구 고착)의 원인 — 일시적 등록 실패가
`clear_hotkey()`로 영구 비활성을 커밋하고, 파일이 이미 최신 스키마라 되돌릴 경로가
없었던 것 — 이 구조적으로 제거됐다. 저장할 상태가 없으면 손상될 상태도 없다.

## Assessment vs Reality

| Metric | Predicted (Plan) | Actual |
|---|---|---|
| Complexity | Large | Large |
| Confidence | 8/10 | 실현됨 — 계획 외 파일 변경 0건 |
| Files Changed | 18 | 18 (신규 0, 수정 18) |
| Net lines | — | +296 / −471 |

## Tasks Completed

| # | Task | Status | Notes |
|---|---|---|---|
| 1 | 네이티브 스키마 축소 테스트 선행 작성 | 완료 | 계획 외로 기존 마이그레이션 테스트 4건의 버전 단언도 갱신 필요 |
| 2 | 스키마 v5 + `hide_show_hotkey` 제거 | 완료 | `SettingsV4` 마이그레이션 구조체 추가 |
| 3 | `apply_hotkey_change` 계열 side effect 제거 | 완료 | `ipc.rs` 272줄 변동(대부분 삭제) |
| 4 | 상수 이전 + capability 진단 채널 | 완료 | 첫 적용 시 상수 중복 삽입 → 즉시 정정 |
| 5 | 시작 시 무조건 등록 + 진단 보고 | 완료 | 등록이 설정 로드 성공 여부와 무관해짐 |
| 6 | `get_platform_capabilities`에 진단 노출 | 완료 | |
| 7 | `window/tests.rs` 갱신 | 완료 | `linux_wayland` 인자 3개, Available/Unavailable 양쪽 |
| 8 | 렌더러 계약 정리 | 완료 | wire 양방향 필드 부재 단언 추가 |
| 9 | presentation/store 정리 + 라벨 헬퍼 | 완료 | `hideShowHotkeyLabel(os)` 신규 |
| 10 | SettingsPanel 읽기 전용 교체 | 완료 | 사용자 요청대로 도움말 2줄 분리 |
| 11 | SettingsPanel 테스트 재작성 | 완료 | textbox 부재 회귀 가드 포함 |
| 12 | App.svelte 배선 정리 | 완료 | `catch (error)` → `catch` 변경 필요(Deviations 참조) |
| 13 | App.test.ts 갱신 | 완료 | hotkey 실패 테스트 → capability 진단 테스트로 대체 |
| 14 | 베타 문서 갱신 | 완료 | |
| 15 | 전체 게이트 실행 및 diff 감사 | 완료 | 계획된 18개 파일 외 변경 없음 |

## Validation Results

| Level | Status | Notes |
|---|---|---|
| Rust fmt | 통과 | `cargo fmt --check` |
| Rust clippy | 통과 | `--all-features -- -D warnings`, 경고 0 |
| Rust tests | 통과 | 115 passed / 0 failed |
| svelte-check | 통과 | 0 errors, 0 warnings |
| eslint + prettier | 통과 | `test:ci` 내 포함 |
| Renderer tests | 통과 | 246 tests, 커버리지 80% 게이트 통과 |
| Vite build | 통과 | 156 modules, 94.11 kB (gzip 31.96 kB) |
| Renderer E2E | 7 passed / 0 failed (웜 캐시) | 콜드 캐시 1회차는 실패 — 아래 분석 |
| `git diff --check` | 통과 | 공백 오류 없음 |
| `cargo audit` | 미실행 | 로컬 미설치(CI `audit:ci` 전용). `Cargo.toml`/`Cargo.lock` 무변경이라 권고 노출면 불변 |

### Renderer E2E 분석

두 번의 헛발질이 있었다. 첫 실행은 Vite dev 서버를 띄우지 않아 7건 전부
`net::ERR_CONNECTION_REFUSED` → Timeout이었다(`wdio.browser.conf.ts`는
`baseUrl: http://127.0.0.1:1420`만 지정하고 서버를 직접 띄우지 않는다). 서버를 띄운
뒤에도 같은 오류가 났는데, Vite 기본 바인딩이 `[::1]:1420`(IPv6)이라 IPv4
`127.0.0.1`로 붙는 Chrome이 거부당한 것이었다. `--host 127.0.0.1`로 재기동하니
3 passed / 4 failed로 실제 결과가 나왔다.

남은 실패는 **이번 변경의 회귀가 아니라 콜드 Vite 캐시에서만 나는 하네스 플레이키**다.
격리된 worktree 실험으로 측정했다.

| 실행 | 트리 | Vite 캐시 | 소요 | 결과 |
|---|---|---|---|---|
| 1 | HEAD worktree | 웜 | 1.9s | 7 passed |
| 2 | HEAD worktree | 웜 | 2.1s | 7 passed |
| 3 | **HEAD worktree + 이번 변경의 렌더러 소스 6개 전부** | 웜 | — | **7 passed** |
| 4 | 작업 트리 | 콜드(`.vite` 삭제 직후) | 32s | 2 passed / 5 failed |
| 5 | 작업 트리 | 웜(동일 서버 재실행) | 2.1s | **7 passed** |

- 실행 3이 결정적이다. `App.svelte`·`gateway.ts`·`fixtureGateway.ts`·`presentation.ts`·
  `stores/settings.ts`·`SettingsPanel.svelte`를 HEAD 위에 그대로 얹어도 전부 통과한다.
  코드가 원인이면 여기서 재현돼야 한다.
- 실행 4→5는 코드를 한 줄도 바꾸지 않고 캐시 상태만 다르다. 콜드일 때 32초가 걸리며
  앞쪽 오버레이 테스트가 mocha 타임아웃으로 죽고, 그 여파로 `browser.setViewport`
  복원이 건너뛰어져 뒤따르는 패널 기하 단언이 240px 뷰포트에서 측정된다
  (`outerWidth` 312 기대 → 225 수신).

로컬 실행 절차: `pnpm dev --host 127.0.0.1`을 먼저 띄우고(Vite 기본 바인딩은 IPv6
전용이라 wdio의 IPv4 `baseUrl`과 어긋난다), 캐시가 콜드면 1회 워밍업 후 판정한다.
`wdio.browser.conf.ts`에 서버 기동·준비 대기가 없다는 점은 이 변경과 별개의 하네스
개선 과제다.

## Files Changed

| File | Action | Lines |
|---|---|---|
| `src-tauri/src/refresh/ipc.rs` | UPDATED | 272 |
| `src-tauri/src/store/tests.rs` | UPDATED | 101 |
| `src-tauri/src/store/settings.rs` | UPDATED | 65 |
| `src-tauri/src/lib.rs` | UPDATED | 36 |
| `src-tauri/src/window/mod.rs` | UPDATED | 24 |
| `src-tauri/src/window/tests.rs` | UPDATED | 10 |
| `src/lib/components/SettingsPanel.test.ts` | UPDATED | 90 |
| `src/lib/components/SettingsPanel.svelte` | UPDATED | 45 |
| `src/App.test.ts` | UPDATED | 30 |
| `docs/beta-testing.md` | UPDATED | 21 |
| `src/App.svelte` | UPDATED | 20 |
| `src/lib/state/presentation.test.ts` | UPDATED | 16 |
| `src/lib/state/presentation.ts` | UPDATED | 14 |
| `src/lib/api/gateway.test.ts` | UPDATED | 10 |
| `src/lib/api/gateway.ts` | UPDATED | 6 |
| `src/lib/api/fixtureGateway.ts` | UPDATED | 4 |
| `src/lib/stores/settings.ts` | UPDATED | 2 |
| `src/lib/stores/settings.test.ts` | UPDATED | 1 |

## Deviations from Plan

1. **도움말 문구를 두 줄로 분리** — WHAT: `Hides and shows the pet.<br />Usage keeps
   updating while hidden.` WHY: 사용자가 한 줄이면 길어서 자동 줄바꿈될 것을 지적하고
   명시적 줄바꿈을 요청.
2. **기존 마이그레이션 테스트 4건의 버전 단언 갱신** — WHAT:
   `assert_eq!(loaded.schema_version, 4)` → `5`, 재작성 JSON 검사도 `"schema_version": 5`.
   WHY: 스키마가 5로 오르면 legacy/v1/v2 마이그레이션도 모두 5에 도달한다. 계획이 이
   테스트들을 열거하지 않았으나 변경이 강제된다.
3. **`App.svelte`의 `catch (error)` → `catch`** — WHAT: 바인딩 제거. WHY: hotkey 분기를
   지우면서 `error`가 미사용이 되어 eslint가 실패한다.
4. **`SettingsV4.hide_show_hotkey`에 `#[allow(dead_code)]`** — WHAT: 읽지 않는 필드에
   속성 부여. WHY: `deny_unknown_fields` 때문에 선언은 필수지만 값은 버려지므로
   clippy `-D warnings`가 dead_code로 막는다.
5. **SettingsPanel 도움말 단언에 정규식 사용** — WHAT: `queryByText('…')` →
   `queryByText(/…/)`. WHY: `<br>`이 문단을 두 텍스트 노드로 쪼개 완전 일치 매처가
   `null`을 반환한다.
6. **`an_unchanged_autostart_value_is_never_reapplied` 테스트 추가** — WHAT: 계획에 없던
   테스트. WHY: 삭제된 hotkey 테스트들이 간접적으로 지키던
   `previous.start_at_login != settings.start_at_login` 가드를 명시적으로 고정.

## Issues Encountered

1. **동일 편집이 중복 적용됨.** 병렬 편집 중 일부가 정책 게이트에 막혀 재시도하는
   과정에서 `DEFAULT_HIDE_SHOW_HOTKEY` 상수가 `window/mod.rs`에 두 번 삽입되고,
   `PlatformCapabilities`의 필드 추가는 누락됐다. `cargo test`가 E0428(중복 정의) +
   E0560(없는 필드)로 즉시 잡아냈고 정정했다. 단계마다 검증을 돌린 것이 유효했다.
2. **E2E 기준선 비교 중 작업 트리를 stash함.** 실패가 기존부터 있던 것인지 확인하려고
   `git stash`를 실행해 18개 파일 변경이 일시적으로 사라졌다. 사용자 중단 직후
   `git stash pop`으로 전량 복원(+296/−471 동일 확인). 이런 비교는 stash 대신 별도
   worktree에서 해야 한다. 결과적으로 stash 없이 diff 분석만으로 판정이 가능했다.
3. **E2E 실행 절차 누락.** `test:e2e:renderer`는 dev 서버를 띄우지 않는다. 추가로
   Vite 기본 바인딩이 IPv6 전용이라 wdio의 IPv4 `baseUrl`과 어긋난다. 로컬 실행 시
   `pnpm dev --host 127.0.0.1`을 먼저 띄워야 한다.
4. **`pnpm`이 PATH에 없음.** `corepack pnpm …` 형태로 실행.
5. **`cargo audit` 로컬 미설치.** 의존성 매니페스트 무변경이라 결과가 바뀔 수 없고
   CI `audit:ci`가 커버한다.

## Tests Written

| Test File | Tests | Coverage |
|---|---|---|
| `src-tauri/src/store/tests.rs` | 신규 2 / 재작성 1 | v4→v5 마이그레이션(값 보존), 사용자 지정 값 quarantine 미발생, v3→v5 |
| `src-tauri/src/refresh/ipc.rs` | 신규 1 / 재작성 2 | autostart 미변경 시 재적용 없음, 실패 시 저장 롤백, 롤백 실패 보고 |
| `src-tauri/src/window/tests.rs` | 단언 2 추가 | capability Available/Unavailable 양쪽 |
| `src/lib/state/presentation.test.ts` | 신규 1 | macOS/Windows/Linux 라벨 |
| `src/lib/components/SettingsPanel.test.ts` | 신규 2 | 고정 키 표시 + textbox 부재 가드, 충돌 안내 표시/미표시 |
| `src/lib/api/gateway.test.ts` | 단언 2 추가 | wire 양방향에서 hotkey 필드 부재 |
| `src/App.test.ts` | 재작성 1 | 충돌 진단 시 안내 표시 + `updateSettings` 미호출 |

## Behavior Verification

코드·테스트 수준에서 확인된 사항:

- 기존 `%APPDATA%\dev.cachebite.app\settings.json`(현재 `hide_show_hotkey: null`)은
  v5로 마이그레이션되며 `logical_position`·알림·pet·`start_at_login`이 보존된다
  (`version_four_settings_drop_the_persisted_hotkey`가 정확히 이 형태를 검증).
- 등록이 `if let Ok(settings)` 블록 밖에 있어 설정 로드 실패와 무관하다.
- 등록 실패 경로에 파일 쓰기가 존재하지 않는다.

실기 확인이 남은 항목: 실제 앱 실행 후 `Ctrl+Shift+H` 동작, 충돌 앱이 있는 상황,
macOS `Cmd+Shift+H`. 계획서의 Manual Validation 체크리스트 참조.

## Post-Review Changes (2026-08-03 리뷰 반영)

`docs/code-review/default-only-hide-show-hotkey-2026-08-03.md`의 MEDIUM 2건, LOW 1건 처리:

1. **`register_default_hotkey` 매핑 테스트 부재** → `window::hide_show_hotkey_capability`
   순수 함수로 추출하고 성공/실패 두 분기를 단위 테스트
   (`hide_show_hotkey_capability_reports_both_registration_outcomes`). 호출부는 분기 없는
   단일 표현식으로 남겨 매핑 반전이 그 자리에서 발생할 수 없게 했다. 네이티브 테스트
   115 → 116.
2. **E2E 기준선 미확인** → 위 표의 격리 worktree 실험으로 확정하고 보고서 갱신.
3. **`docs/beta-testing.md` 문구 모순** → "fixed and always on" → "fixed and claimed on
   every launch"로 바꾸고 충돌 시 Settings가 알린다는 예외를 같은 문장에 넣었다.

## Next Steps

- [ ] 실기 수동 검증 (특히 기존 `settings.json` 마이그레이션, 충돌 시나리오)
- [ ] (별도 과제) `wdio.browser.conf.ts`에 dev 서버 기동·준비 대기 추가로 콜드 캐시
      플레이키 제거
- [ ] `/code-review`로 변경 리뷰
- [ ] `/prp-commit` → `/prp-pr`
- [ ] (별도 이슈 권장) single-instance 플러그인 도입 — 로그인 자동 실행과 수동 실행이
      겹칠 때 두 번째 인스턴스가 등록에 실패하는 상황 자체를 없앤다
