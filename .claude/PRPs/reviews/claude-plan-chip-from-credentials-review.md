# Code Review: Claude 요금제 칩 — 자격증명 파일의 `subscriptionType`

**Reviewed**: 2026-08-14
**Mode**: Local (uncommitted changes)
**Branch**: `feat/claude-plan-chip-from-credentials` → `develop`
**Decision**: APPROVE with comments

## Summary

자격증명 파서가 토큰과 함께 `claudeAiOauth.subscriptionType`을 반환하도록 넓히고, 그 값을 Claude 스냅샷의 `plan_type`으로 나른다. 변경은 좁고 목적에 정확히 맞으며, 비밀 취급 경계(토큰만 `SecretString`·`zeroize`)가 흐트러지지 않았다. **CRITICAL·HIGH 없음.** 지적 3건은 전부 LOW이며 병합을 막지 않는다.

특히 좋은 점 두 가지:

- `ClaudeCredential`에 `Debug`를 두지 않아 "자격증명 전체를 포맷 출력할 수 없다"가 **컴파일러로 강제**된다. 테스트에서 `unwrap_err()`가 컴파일 실패한 것이 그 증거이고, 우회(파생 추가) 대신 호출부를 고친 판단이 옳다.
- 파서 한 곳만 넓혀 네이티브·WSL 두 경로가 함께 해결됐고, `wsl_claude_carries_the_subscription_tier_alongside_the_secret`가 **파서 공유가 깨지는 것**을 잡는 회귀 테스트로 남는다.

## Findings

### CRITICAL

None.

### HIGH

None.

### MEDIUM

None.

### LOW

#### L1 — 요금제 문자열에 길이·공백 검사가 없고, 칩 CSS에도 상한이 없다

**위치**: `src-tauri/src/collectors/broker.rs:128-132` · `src/lib/components/ProviderColumn.svelte:102-109`

파서는 빈 문자열만 접는다(`.filter(|value| !value.is_empty())`). 따라서 `"   "`(공백만)은 통과해 렌더러에서 **내용 없는 알약 모양**으로 그려지고, 이론상 최대 ~64 KiB(파일 상한)까지의 문자열이 `.plan-chip`으로 흘러간다. `.plan-chip`에는 `max-width`·`overflow`·`text-overflow`·`white-space`가 없어 380px 고정 폭 패널에서 넘칠 수 있다.

**다만 이 diff가 새로 만든 문제는 아니다.** Codex 칩이 오늘 정확히 같은 노출을 갖는다 — `codex.rs`의 `limits.plan_type`도 RPC 응답에서 길이·공백 검사 없이 받는다(픽스처가 두 단어인 `'Fixture Pro'`를 쓴다). 이 변경은 Claude를 **기존에 수용된 동작과 대칭**으로 만들 뿐이다.

또한 보안 문제는 아니다. 값의 출처는 사용자 소유의 로컬 파일이며, 그 파일에 쓸 수 있는 사람은 이미 토큰을 갖고 있다.

**현실적 위험도**: 알려진 값(`pro`, `max`, `free`, `team`, `enterprise`, `max_20x`)은 전부 컬럼 폭 안에 들어간다. 조치는 선택이다.

**제안(선택)**: 지금 고친다면 provider 양쪽에 대칭으로 — 파서에서 `.filter(|value| !value.trim().is_empty())`, 그리고 `.plan-chip`에 `max-width` + `text-overflow: ellipsis`. 한쪽만 고치면 두 provider가 다시 비대칭이 된다.

#### L2 — `read_token`의 이름이 반환값과 어긋난다

**위치**: `src-tauri/src/collectors/broker.rs:97`

```rust
fn read_token(path: &std::path::Path) -> Result<Option<ClaudeCredential>, ()>
```

토큰이 아니라 자격증명을 돌려준다. private 함수이고 호출부가 한 곳(`broker.rs:75`)이라 `read_credential`로 바꾸는 비용은 2줄이다.

트레이트 이름 `ClaudeTokenSource`와 메서드 `claude_token`도 같은 어긋남을 갖지만, 이는 **계획이 명시적으로 범위 밖으로 둔 항목**이며(호출부가 늘어남) 그 판단은 타당하다. 이름을 바꾼다면 별도 커밋이 맞다.

#### L3 — 수정한 문단 바로 위의 `architecture.md` 서술이 코드와 맞지 않는다

**위치**: `docs/architecture.md:162-166`

```
The collector reads credentials through a credential broker:
1. The Claude Code keychain entry on macOS.
```

`broker.rs`에는 keychain 코드가 없다. `CredentialLocations::documented`는 파일 경로 두 개(`$CLAUDE_CONFIG_DIR/.credentials.json`, `~/.claude/.credentials.json`)만 만든다.

**이 변경이 만든 문제가 아니다** — 이전부터 있던 문서/구현 불일치다. 다만 이번에 그 문단 바로 아래에 새 서술을 넣었으므로, 나중에 읽는 사람이 새 문장까지 같은 신뢰도로 의심하게 된다. 별도로 정리할 값어치가 있다.

## 확인했으나 문제 없었던 항목

| 검사 | 결과 |
|---|---|
| 하드코딩된 비밀·API 키·토큰 | 없음. 테스트 픽스처는 `"t"`, `"wsl-secret"`, `"env-value"` 등 합성 값 |
| CI 비밀/엔드포인트 가드 (`ci.yml:50-62`) | 로컬 재현 결과 **통과** |
| 프라이버시 계약 (렌더러 DTO에서 authorization·원본 본문·계정 식별자·자격증명 경로 배제) | 유지. `"pro"`/`"max"`는 등급이며 식별자가 아니고, Codex가 이미 같은 필드를 나른다 |
| 토큰 소거 경로 | `SecretString`(Drop 시 zeroize) + 빈 토큰 분기의 명시적 `zeroize()` + `read_token`의 `contents.zeroize()` 모두 유지 |
| `subscription_type`을 zeroize하지 않음 | 의도적. 근거 주석이 `broker.rs:183-184`에 있음 |
| 새 로그 유출 | 없음. `claude.rs`의 `#[cfg(debug_assertions)] eprintln!`은 HTTP 상태만 출력 |
| 영속 스키마 변경 | 없음. `plan_type`은 `ProviderUsageSnapshot`에 이미 있었고 `snapshots.rs:266`의 `validate_keys`에도 이미 등재. 마이그레이션 불필요 |
| 두 `ClaudeTokenSource` 구현체 정렬 | 완료. 감싸는 방식(ready future vs 진짜 async)은 각자 유지 |
| 함수 길이 / 파일 길이 / 중첩 깊이 | 위반 없음. `parse_token_bytes` 30줄, `broker.rs` 187줄 |
| 뮤테이션 패턴 | `.take()`는 파싱 후 소유권 이전이며 소거 규약의 일부다. 부적절한 뮤테이션 아님 |
| `console.log` / `TODO` / `FIXME` / 이모지 | 없음 |
| 렌더러 변경 | 0줄 — 수용 기준 충족 |

## 테스트 커버리지 평가

신규 6건 / 변경 7건. 계획의 엣지 케이스 9개 항목이 전부 코드로 고정됐다.

빠진 케이스 하나: **`claudeAiOauth`에 `subscriptionType`은 있지만 토큰은 최상위 레거시 필드에 있는 파일** — `{"claudeAiOauth":{"subscriptionType":"max"},"accessToken":"t"}`. 현재 구현은 토큰 `"t"`, 등급 `Some("max")`를 돌려준다(같은 파일·같은 계정이므로 합리적인 해석). 동작이 틀린 것은 아니고 단지 고정돼 있지 않다. `credential_parser_folds_a_missing_or_empty_tier_to_none`의 배열에 한 줄 추가하면 되지만, 필수는 아니다.

## Validation Results

| Check | Result |
|---|---|
| Format (`cargo fmt --check`) | Pass |
| Lint (`cargo clippy --all-features --all-targets -D warnings`) | Pass — 경고 0 |
| Tests (Rust, `--all-features`) | Pass — 153/153 |
| Tests (Renderer, `pnpm test:ci`) | Pass — 373/373, 27 files |
| Type check (`svelte-check`) | Pass — 370 files, 0 errors, 0 warnings |
| Build (`vite build`) | Pass |
| Coverage gate (80%) | Pass |
| CI secret/endpoint guard (로컬 재현) | Pass |

## Files Reviewed

| File | Change | Lines |
|---|---|---|
| `src-tauri/src/collectors/broker.rs` | Modified | +44 / -8 |
| `src-tauri/src/collectors/claude.rs` | Modified | +18 / -6 |
| `src-tauri/src/collectors/wsl.rs` | Modified | +4 / -5 |
| `src-tauri/src/collectors/tests.rs` | Modified | +98 / -10 |
| `docs/ui-contract.md` | Modified | +3 / -0 |
| `docs/architecture.md` | Modified | +2 / -0 |

## Decision Rationale

CRITICAL·HIGH 0건, 모든 검증 통과 → **APPROVE**. LOW 3건은 전부 선택 사항이며, 그중 L1·L3은 이 diff가 만든 문제가 아니라 이 diff가 **드러낸** 기존 상태다.

병합 전 남은 것은 코드가 아니라 실기기 확인이다: 계획의 Manual Validation(`corepack pnpm tauri dev`)이 `subscriptionType` 필드가 실제 자격증명 파일에 존재하는지에 대한 최종 확인 단계다. 이 리뷰는 그 확인을 대체하지 않는다.
