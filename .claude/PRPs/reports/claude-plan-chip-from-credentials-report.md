# Implementation Report: Claude 요금제 칩 — 자격증명 파일의 `subscriptionType` 사용

## Summary

클릭 패널 Claude 컬럼에 요금제 칩(`Pro` / `Max`)을 띄우기 위해, CacheBite가 이미 토큰을 읽으려고 열고 있는 자격증명 파일에서 `claudeAiOauth.subscriptionType`을 함께 꺼내 스냅샷의 `plan_type`으로 나른다. 파서(`parse_token_bytes`) 한 곳만 넓혔고 네이티브·WSL 두 경로가 같은 파서를 공유하므로 함께 해결됐다. **렌더러는 0줄 변경** — 표시 파이프라인은 Codex용으로 이미 완성돼 있었다.

새 네트워크 요청은 없다.

## Assessment vs Reality

| Metric | Predicted (Plan) | Actual |
|---|---|---|
| Complexity | Medium | Medium — 예측대로 |
| Confidence | 조사 완료(재조사 불필요) | 정확. 엔드포인트 재조사 없이 완료 |
| Files Changed | 6 (Rust 4 / 문서 2) | 6 (Rust 4 / 문서 2) |
| 신규 테스트 | 8 | 6 (그룹화, 아래 이탈 #1 참조) |
| 변경 테스트 | 4 | 7 (broker 3곳 추가 발견, 이탈 #2) |

## Tasks Completed

| # | Task | Status | Notes |
|---|---|---|---|
| 1 | `ClaudeCredential` 타입 도입 + 파서 확장 (TDD) | 완료 | 테스트 선작성 → RED(18 에러) 확인 후 구현 |
| 2 | `ClaudeTokenSource` 트레이트 확장 | 완료 | `CredentialBroker`(ready future) / `WslCredentialSource`(진짜 async) 두 구현체 모두 정렬 |
| 3 | 스냅샷에 요금제 싣기 | 완료 | 구조 분해로 부분 move 회피 |
| 4 | WSL 경로 회귀 테스트 | 완료 | 파서 공유가 깨지면 실패하는 테스트 고정 |
| 5 | 계약 문서화 | 완료 | `ui-contract.md` §5 + `architecture.md` Claude collection |

## Validation Results

| Level | Status | Notes |
|---|---|---|
| Static Analysis (`cargo fmt --check`) | 통과 | 초기 2건 차이 → `cargo fmt` 적용 후 clean |
| Static Analysis (`cargo clippy --all-features --all-targets -D warnings`) | 통과 | 경고 0 |
| Unit Tests (Rust) | 통과 | 153 passed / 0 failed |
| Unit Tests (Renderer) | 통과 | 373 passed / 27 files — **변경 없음이 정상** |
| Static Analysis (svelte-check) | 통과 | 370 files, 0 errors, 0 warnings |
| Build (`vite build`) | 통과 | 165 modules, 1.35s |
| Coverage Gate (80%) | 통과 | `test:ci`가 강제 |
| Edge Cases | 통과 | 9개 항목 전부 테스트로 고정 (아래) |
| Manual Validation (`tauri dev`) | **미실행** | 실기기 GUI + 실제 Claude 로그인이 필요 — 사용자 확인 필요 |

### Edge Case 체크리스트 → 고정한 테스트

| 케이스 | 테스트 |
|---|---|
| 요금제 + 토큰 둘 다 있음 | `credential_parser_reads_the_tier_beside_the_token` |
| 요금제 없음 → 토큰만 | `credential_parser_folds_a_missing_or_empty_tier_to_none` |
| 빈 문자열 요금제 → `None` | 〃 |
| 레거시 최상위 `accessToken` → 요금제 `None` | 〃 |
| 빈 토큰 + 요금제 있음 → `Ok(None)` | `credential_parser_rejects_an_empty_token_even_when_a_tier_is_present` |
| 환경변수 토큰 → 요금제 `None` | `broker_reports_no_tier_for_an_environment_token` |
| WSL 경로 → 요금제 실림 | `wsl_claude_carries_the_subscription_tier_alongside_the_secret` |
| 파일 크기 상한 초과 → `Err(())` | 기존 `broker_maps_missing_and_rejects_oversize_credentials`, `wsl_claude_maps_absence_and_rejects_invalid_or_oversized_credentials` (회귀 없음) |
| 토큰이 `Debug`·로그에 새지 않음 | `broker_uses_environment_before_file_and_never_writes` + `ClaudeCredential`에 `Debug` 자체를 두지 않음 (컴파일러가 강제) |

## Files Changed

| File | Action | Lines |
|---|---|---|
| `src-tauri/src/collectors/broker.rs` | UPDATED | +44 / -8 |
| `src-tauri/src/collectors/claude.rs` | UPDATED | +18 / -6 |
| `src-tauri/src/collectors/wsl.rs` | UPDATED | +4 / -5 |
| `src-tauri/src/collectors/tests.rs` | UPDATED | +98 / -10 |
| `docs/ui-contract.md` | UPDATED | +3 / -0 |
| `docs/architecture.md` | UPDATED | +2 / -0 |

`src/` 아래 변경 0건 — 계획의 "렌더러 0줄" 수용 기준 충족.

## Deviations from Plan

1. **파서 테스트를 5개 별도 `#[test]`가 아니라 3개 그룹 테스트로 작성.**
   - WHY: `collectors/tests.rs`의 기존 컨벤션은 관련 단언을 행동 단위로 묶는다(`wsl_claude_maps_absence_and_rejects_invalid_or_oversized_credentials`는 단언 5개). 계획이 "Patterns to Mirror"로 지목한 `tests.rs:588-597` 역시 그 형태다. 계획 표의 **5개 케이스는 전부 커버**했고 이름만 행동 단위로 묶었다.

2. **`CredentialBroker::claude_token` 인헌트 메서드를 직접 호출하는 broker 테스트 3곳을 추가 정렬.**
   - WHAT: `tests.rs`의 `broker_uses_environment_before_file_and_never_writes`, `broker_uses_first_available_read_only_location`, `broker_skips_corrupt_candidate_but_reports_all_corrupt`의 `.expose_secret()` → `.token.expose_secret()`.
   - WHY: 계획의 테스트 전략 표에 이 3곳이 빠져 있었다. inherent `pub fn claude_token`도 반환 타입이 바뀌므로 필수다.

3. **`wsl_claude_diagnostics_do_not_expose_output`의 `.unwrap_err()` → `.err().expect(...)`.**
   - WHY: `Result::<T, E>::unwrap_err`는 `T: Debug`를 요구하는데, 계획의 Completion Checklist가 `ClaudeCredential`에 `Debug`를 **파생하지 말 것**을 명시한다. 파생 대신 호출부를 바꿔 원칙을 지켰다 — 결과적으로 "자격증명은 Debug 출력 자체가 불가능"이 컴파일러로 강제된다.

4. **`wsl.rs`의 `use secrecy::SecretString;` 제거.**
   - WHY: 반환 타입이 `ClaudeCredential`로 바뀌면서 미사용이 됐고, `clippy -D warnings`가 이를 에러로 잡는다.

5. **계획에 없던 `broker_reports_no_tier_for_an_environment_token` 테스트 추가.**
   - WHY: 계획의 Edge Case 체크리스트에 "환경변수 토큰 → 요금제 `None`"이 있는데 이를 고정할 테스트가 지정돼 있지 않았다. 자격증명 파일에 `subscriptionType`이 있어도 환경변수 경로가 그것을 **읽지 않는다**는 것이 실제 계약이므로, 픽스처 파일에 `"max"`를 넣고 결과가 `None`인지 확인한다.

## Issues Encountered

| 이슈 | 해결 |
|---|---|
| `ClaudeCredential`에 `Debug`가 없어 `unwrap_err()` 컴파일 실패 (E0277) | 이탈 #3 — 호출부를 `.err().expect(...)`로 변경. `Debug` 파생 금지 원칙 유지 |
| `wsl.rs`의 `SecretString` import 미사용 → clippy 에러 | 이탈 #4 — import 제거 |
| `cargo fmt --check` 2건 차이 | `cargo fmt` 적용, 재검증 clean |

계획이 예고한 위험 중 "트레이트 확장이 한쪽 구현체를 놓침"은 발생하지 않았다 — 계획이 두 위치를 명시해 왕복이 없었다.

## Tests Written

| Test File | Tests | Coverage |
|---|---|---|
| `src-tauri/src/collectors/tests.rs` | 신규 6 / 변경 7 | 자격증명 파서(요금제+토큰, 부재·빈 문자열·레거시 폴백, 빈 토큰 거부), 환경변수 경로의 요금제 부재, WSL 경로의 요금제 전달, `parse_usage`의 요금제 전달 |

신규 테스트 목록:

- `credential_parser_reads_the_tier_beside_the_token`
- `credential_parser_folds_a_missing_or_empty_tier_to_none`
- `credential_parser_rejects_an_empty_token_even_when_a_tier_is_present`
- `broker_reports_no_tier_for_an_environment_token`
- `wsl_claude_carries_the_subscription_tier_alongside_the_secret`
- `claude_parser_carries_the_credential_tier_into_the_snapshot`

모든 픽스처는 인라인 바이트/문자 리터럴이며 실제 자격증명을 쓰지 않는다.

## Completion Checklist (계획 기준)

- [x] `subscription_type`이 `SecretString`이 **아니고** `zeroize` 대상도 아니다
- [x] `OAuthCredentials`에 왜 이 필드만 더 여는지, `rateLimitTier`는 왜 안 여는지 주석이 있다
- [x] 두 `ClaudeTokenSource` 구현체가 모두 정렬됐다
- [x] 캐시 한계와 환경변수 한계가 계약에 적혀 있다 (`ui-contract.md` §5, `architecture.md`)
- [x] 자격증명 파일을 **쓰지 않는다** — `broker_uses_environment_before_file_and_never_writes`가 파일 바이트 불변을 단언
- [x] 테스트 픽스처에 실제 자격증명이 없다
- [ ] **Manual Validation 미실행** — 실기기에서 칩이 실제로 뜨는지는 확인하지 못했다

## 남은 확인 (사용자 필요)

계획의 Manual Validation은 GUI 실행과 실제 Claude 로그인이 필요해 이 세션에서 수행하지 않았다. 계획 자체가 "실기기에서 칩이 실제로 뜨는지는 Manual Validation에서 확인해야 한다 — 그 단계가 필드 존재의 최종 확인"이라고 적어 둔 항목이다.

```bash
corepack pnpm tauri dev
```

- [ ] 펫 더블클릭 → 패널의 Claude 컬럼 헤딩 오른쪽에 요금제 칩이 보인다
- [ ] 칩 텍스트가 `text-transform: capitalize`로 렌더된다 (`max` → `Max`)
- [ ] Codex 칩과 같은 자리·같은 스타일이다
- [ ] 두 컬럼의 행 정렬이 그대로다
- [ ] `CLAUDE_CODE_OAUTH_TOKEN`을 설정하고 재기동 → Claude 칩이 사라지고 레이아웃은 정상

## Next Steps

- [ ] Manual Validation (위)
- [ ] `/code-review`로 변경 리뷰
- [ ] `/prp-pr`로 PR 생성 (이슈 #85 닫기)
