# Code Review: Codex CLI `--ask-for-approval` 호환성 복구 + CLI 거부 가시화

**Reviewed**: 2026-08-24
**Branch**: `fix/codex-cli-approval-policy-compat`
**Base**: `19e2467 Merge pull request #87 from chanhoan/develop` (main)
**Scope**: `HEAD` 대비 로컬 변경(수정 19개 파일, +254 / −26) + 미추적 신규 4건 — `scripts/check-codex-cli-compat.sh`, `.github/workflows/codex-cli-compat.yml`, `.claude/PRPs/**` 2건
**Decision**: **APPROVE with comments** — CRITICAL 0건, HIGH 0건, MEDIUM 4건, LOW 5건. 전 검증 통과.
**Resolution**: MEDIUM 4건 + LOW 2건 반영 완료(하단 "반영 결과"). M1은 리뷰의 최초 권고를 검증 후 **철회하고 다른 수정**을 적용했다.

> **리뷰어 주의**: 이 리뷰는 동일 세션에서 해당 변경을 구현한 주체가 작성했다. 자기 코드 리뷰는 설계 의도를 공유한다는 점에서 맹점이 있으므로, 특히 M1(분류 규칙의 범위)과 M2(계약 잠금 누락)는 제3자 확인을 권한다.

## Summary

근본 원인 진단은 정확하고 실측으로 뒷받침된다. codex-cli 0.149.0이 `--ask-for-approval`에서 `untrusted`를 제거해 `codex -s read-only -a untrusted app-server`가 exit 2로 즉시 거부되고, stderr가 `Stdio::null()`로 버려지는 탓에 stdout EOF → `Protocol` → `FailureClass::Parse` → "Could not fetch usage. Retrying shortly."로 위장되던 경로가 문서화됐다. 이는 사용자가 이 문제를 "auth 문제"로 오진한 이유를 정확히 설명한다.

수정의 방향도 옳다. 세 곳에 흩어져 있던 인자 문자열을 `app_server_argv()` 단일 원본으로 모으고 테스트로 잠근 것(`native_codex_argv_is_fixed`)이 이번 변경의 가장 값진 부분이다 — **네이티브 argv는 이전까지 어떤 테스트도 잠그지 않았고, 그래서 상류 변경이 CI를 그대로 통과했다.** WSL 런처 두 종(`-lc` / `-ic`)을 한 테스트에서 함께 단언하도록 바꾼 것도 "두 런처가 같은 정책을 갖는다"는 실제 불변식을 표현한다.

`classify_launch_exit`의 도입은 이번 인시던트의 진짜 교훈 — 실패가 보이지 않았다는 것 — 을 겨냥한 올바른 조치다. 호출 순서(`terminate()` 이전에 `exit_code_with_grace()`)가 정확성의 핵심이고, 두 수집 경로 모두에서 그 순서가 지켜지며 주석으로 이유가 남아 있다. `exit_code_with_grace()`가 100ms 유예 후 `None`을 반환해 "프로토콜 중간에 깨졌지만 자식은 살아 있는" 정상 케이스를 원래 오류로 보존하는 설계도 오탐 방지에 맞다. 죽어 있던 `exit_127_is_cli_missing` 파라미터(호출부가 `false` 하나뿐이었다)를 제거하고 일반화한 것은 순수한 개선이다.

렌더러 쪽은 절제됐다. 새 `SystemState`나 배지 아이콘을 만들지 않고 기존 `error` 배지를 재사용하면서 문구만 특수화해, `SystemBadge`의 아이콘 분기·`BadgeState`·ui-contract §4.2 표가 연쇄 변경되는 것을 피했다. 구현 중 `providers.ts`의 인라인 리터럴 유니온 중복 정의를 발견해 공유 `FailureClass`로 교체한 것은 계획에 없던 부수적 정리다.

프라이버시 계약 위반 없음: 렌더러 DTO에 추가된 것은 열거형 문자열 하나(`cli_incompatible`)뿐이고, stderr는 여전히 앱에서 폐기된다. 종료 코드(정수)만 분류에 쓴다. 하드코딩된 비밀·디버그 잔재·`console.log` 없음.

차단 사유는 없다. 다만 **M1과 M2는 병합 전 처리를 권한다.** M1은 이번 변경이 새로 만든 경로에서 사용자에게 틀린 조치("CacheBite를 업데이트하세요")를 안내할 수 있고, M2는 이 PR이 고치려는 바로 그 "조용한 계약 드리프트"를 새 값에 대해 그대로 남겨둔다.

---

## Findings

### CRITICAL

없음.

### HIGH

없음.

### MEDIUM

#### M1 — `classify_launch_exit`의 분류 범위가 주석이 약속한 것보다 넓고, 셸 가드와 규칙이 어긋난다

**위치**: `src-tauri/src/collectors/codex.rs:371-386`, `src-tauri/src/collectors/codex.rs:216-222`, `scripts/check-codex-cli-compat.sh:29`

Doc 주석은 이렇게 선언한다:

> Classifies a child that exited on its own **before the protocol ever started**.

그러나 코드가 실제로 확인하는 조건은 `result.is_err()` 하나뿐이다:

```rust
let launch_failure = if result.is_err() {
    classify_launch_exit(child.exit_code_with_grace().await)
} else {
    None
};
```

`RpcSession::exchange`는 `initialize` 응답을 받고 `initialized` 알림을 보낸 뒤 `account/rateLimits/read`를 읽는다. **CLI가 `initialize`까지 정상 응답한 뒤 두 번째 요청 처리 중 크래시하면**(예: Rust 패닉 exit 101, 또는 자체 오류 exit 1) 두 번째 read가 EOF를 만나 `Protocol`이 되고, 자식은 이미 종료했으므로 `Some(101)` → `CliIncompatible`로 분류된다. 사용자에게는 "The Codex CLI rejected this CacheBite build. Update CacheBite."가 뜬다 — **상류 크래시에 대해 우리 앱을 업데이트하라는 틀린 조치 안내**다.

같은 현상에 대해 이번 PR이 함께 도입한 셸 가드는 **다른 규칙**을 쓴다:

```bash
# scripts/check-codex-cli-compat.sh
if [ "$status" -eq 2 ]; then   # 오직 exit 2만 인자 거부로 판정
...
if [ "$status" -ne 0 ]; then   # 그 외 비정상 종료는 "inconclusive"
```

셸 가드는 "인자 표면과 무관한 런타임 문제가 오탐을 내지 않도록" 명시적으로 좁혔는데, Rust 분류기는 좁히지 않았다.

> **정정 (검증 후)**: 이 리뷰는 처음에 "두 가드가 어긋나므로 Rust를 `Some(2) | Some(127)`로 좁혀라"를 권했다. **그 권고는 틀렸다.**
>
> 첫째, 두 가드의 실행 맥락이 다르므로 임계값이 달라도 모순이 아니다. 셸 가드는 네트워크·샌드박스·설정 부재 등 인자와 무관한 실패 모드가 많은 CI에서 돌고, Rust 분류기는 방금 우리 argv로 스폰한 통제된 상황에서 돈다.
>
> 둘째, 좁히기는 오분류를 **다른 방향으로 옮길 뿐**이다. 미래의 CLI가 exit 2가 아닌 코드로 인자를 거부하면 원래 버그(조용한 `Parse` 실패)가 그대로 재현된다. 이번 사건은 그 조용함이 문제였다.
>
> 셋째, "비싸다"고 기각했던 대안이 실제로는 싸다. 진짜 구분 기준은 종료 코드가 아니라 **"CLI가 프로토콜을 한 마디라도 했는가"** 이고, `RpcSession`에 `AtomicBool` 하나면 그 신호를 얻는다.

**적용된 수정** — `RpcSession`이 첫 응답을 읽는 순간 플래그를 세우고, 호출부가 그 플래그로 재분류를 가드한다. 분류 규칙 자체(비-0은 전부 `CliIncompatible`)는 넓게 유지해 "다른 종료 코드로 거부하는 CLI"도 계속 잡는다.

```rust
// RpcSession::exchange — 첫 응답 직후, validate 전에 세운다.
// 오류 응답도 "CLI가 말한 것"이므로 launch 실패가 아니다.
self.saw_response.store(true, Ordering::Relaxed);
```

```rust
// 두 수집 경로의 호출부
let launch_failure = if result.is_err() && !session.saw_response() {
    classify_launch_exit(child.exit_code_with_grace().await)
} else {
    None
};
```

`classify_launch_exit`의 doc 주석도 "callers must establish that precondition themselves by checking `RpcSession::saw_response`"로 고쳐, 주석이 약속하는 전제를 실제로 누가 보장하는지 명시했다.

---

#### M2 — 새 `FailureClass::CliIncompatible`의 wire 문자열이 어떤 테스트로도 잠겨 있지 않다

**위치**: `src-tauri/src/domain.rs:30-38`, `src-tauri/src/domain_test.rs:74-83`, `src/lib/contracts/domain.ts:31-36`

`domain_test.rs`에는 wire 포맷을 고정하는 테스트가 이미 존재한다:

```rust
#[test]
fn collection_outcomes_are_typed_and_credential_free() {
    let outcome = CollectionOutcome::Failed { class: FailureClass::Parse };
    assert_eq!(
        serde_json::to_string(&outcome).unwrap(),
        r#"{"kind":"failed","class":"parse"}"#
    );
}
```

새 variant는 여기에 추가되지 않았다. Rust는 `#[serde(rename_all = "snake_case")]`로 `"cli_incompatible"`을 내보내고 TS 유니온도 `'cli_incompatible'`을 갖지만, **이 일치를 검증하는 것이 아무것도 없다.**

구체적 실패 시나리오: 누군가 Rust variant를 `CliRejected`로 개명한다 → serde 문자열이 `"cli_rejected"`로 바뀐다 → Rust 컴파일 통과, Rust 테스트 통과(`FailureClass::CliIncompatible`을 문자열로 비교하는 곳이 없음), TS는 `'cli_incompatible'`을 그대로 갖고 있어 컴파일 통과, `systemGuidance`의 `failureClass === 'cli_incompatible'`이 영원히 거짓이 되어 **패널이 조용히 일반 문구로 돌아간다.** 어떤 테스트도 빨간불이 되지 않는다.

이것은 이 PR이 고치려는 문제와 정확히 같은 유형 — 조용한 계약 드리프트 — 이며, 이번에 새로 만든 계약 지점에 그대로 남아 있다.

**권장 수정**: 기존 테스트를 확장한다.

```rust
    // The renderer switches its guidance copy on this exact string
    // (systemGuidance.ts). Renaming the variant without updating the TS union
    // would silently fall back to the generic "retrying shortly" line.
    assert_eq!(
        serde_json::to_string(&CollectionOutcome::Failed {
            class: FailureClass::CliIncompatible
        })
        .unwrap(),
        r#"{"kind":"failed","class":"cli_incompatible"}"#
    );
```

참고: `into_outcome` 매핑 자체는 통합 테스트 `a_cli_that_rejects_our_arguments_reports_cli_incompatible`이 end-to-end로 덮고 있으므로 공백이 아니다. 누락된 것은 **직렬화 문자열**뿐이다. (`errors_map_to_typed_outcomes_without_provider_cross_talk`는 망라적 목록이 아닌 spot-check라 새 variant를 강제하지 않는다.)

---

#### M3 — 가드 스크립트가 예측 가능한 이름의 임시 파일을 공용 디렉터리에 쓴다 (CWE-377), 정리도 하지 않는다

**위치**: `scripts/check-codex-cli-compat.sh:26, 32, 40`

```bash
codex ... 2>"${TMPDIR:-/tmp}/codex-compat-err.txt" || status=$?
```

`TMPDIR`이 없는 리눅스에서 경로가 `/tmp/codex-compat-err.txt`로 고정된다. 다중 사용자 개발 장비에서 다른 사용자가 그 경로에 심볼릭 링크를 미리 심어두면 `2>` 리다이렉트가 링크를 따라가 **해당 사용자가 쓰기 권한을 가진 임의 파일을 절단**한다. CI 러너는 단일 사용자라 실질 위험이 낮지만, 이 스크립트는 개발자가 로컬에서 직접 실행하도록 설계됐다(주석과 리포트 모두 그렇게 안내한다). 또한 성공 경로에서도 파일이 남는다.

**권장 수정**:

```bash
err="$(mktemp)"
trap 'rm -f "$err"' EXIT

status=0
codex -s read-only -a never app-server </dev/null >/dev/null 2>"$err" || status=$?
```

이후 `head -20 "$err"`로 참조한다. 저장소의 보안 규칙(임시 파일·경로 취급)에 부합하고 수정 비용이 사실상 0이다.

---

#### M4 — 가드 스크립트가 무한정 멈출 수 있다 (timeout 없음)

**위치**: `scripts/check-codex-cli-compat.sh:26`

스크립트는 "app-server가 stdin EOF에서 스스로 exit 0으로 종료한다"는 **상류 동작 가정**에 의존한다. 그런데 이 스크립트의 존재 이유 자체가 "상류 동작은 바뀐다"는 것이다. 미래의 codex가 EOF에서 종료하지 않고 대기하면:

- **CI**: `timeout-minutes: 10`이 잡아 잡 실패 → 신호는 오지만 "인자가 거부됐다"가 아니라 "타임아웃"이라는 모호한 형태로 온다.
- **로컬**: 개발자 터미널이 **무기한 멈춘다.** 가드가 스스로 장애 지점이 된다.

**권장 수정**: `timeout`으로 감싸고 124를 별도 처리한다.

```bash
status=0
timeout 30 codex -s read-only -a never app-server </dev/null >/dev/null 2>"$err" || status=$?

if [ "$status" -eq 124 ]; then
  echo "Inconclusive: codex did not exit on stdin EOF within 30s (installed: ${version})." >&2
  echo "The check assumes app-server terminates at EOF; that assumption may have changed." >&2
  exit 0
fi
```

`timeout`은 coreutils라 CI 우분투 러너와 대부분의 개발 환경에 존재한다. macOS 기본 환경에는 없으므로(`gtimeout`) `command -v timeout`으로 감싸 없으면 그대로 실행하는 폴백을 두면 안전하다.

---

### LOW

#### L1 — 드리프트 방지 테스트가 파일 전체 `contains`라 주석만으로도 만족된다

**위치**: `src-tauri/src/collectors/tests.rs` (`the_compatibility_script_checks_the_argv_the_collector_actually_sends`)

```rust
assert!(script.contains(&format!("codex {}", app_server_argv().join(" "))));
```

스크립트 파일 전체를 대상으로 하므로, 실제 실행 라인이 드리프트해도 **설명 주석에 올바른 argv가 남아 있으면 테스트가 통과한다.** 의도(가드와 코드의 일치)는 옳고 방향도 좋지만 단언이 느슨하다.

실행 라인까지 포함해 매칭하면 강해진다:

```rust
assert!(script.contains(&format!("codex {} </dev/null", app_server_argv().join(" "))));
```

#### L2 — CI가 `@openai/codex@latest`를 고정 없이 설치한다 — 의도적이므로 그 사실을 명시할 것

**위치**: `.github/workflows/codex-cli-compat.yml:29`

버전을 고정하지 않는 것이 **이 잡의 목적 그 자체**다(상류 최신을 봐야 변경을 감지한다). 저장소는 모든 GitHub Action을 full commit SHA로 고정하는 강한 관례를 갖고 있어, 미래의 기여자나 Dependabot 정리 작업이 "일관성"을 이유로 이 줄을 고정해 **가드를 무력화**할 위험이 있다. 잡은 `permissions: contents: read`에 시크릿이 없어 실행 위험은 최소이므로, 고정하지 말라는 이유를 그 줄 바로 위에 남기는 것으로 충분하다.

```yaml
      # Deliberately unpinned: this job exists to see what the LATEST Codex CLI
      # accepts. Pinning it would make the guard permanently blind.
      - name: Install the latest Codex CLI
        run: npm install -g @openai/codex@latest
```

#### L3 — `systemGuidance`의 세 번째 인자가 `= null` 기본값이라 호출부가 조용히 빠뜨릴 수 있다

**위치**: `src/lib/components/systemGuidance.ts:22`

기본값은 기존 테스트 호출부를 깨지 않기 위한 선택으로 합리적이다. 다만 새 호출부가 인자를 잊으면 타입 에러 없이 일반 문구로 떨어진다. 프로덕션 호출부가 현재 `ProviderColumn.svelte` 하나뿐이라 실제 위험은 낮다. 필수 인자로 바꾸고 테스트에서 명시적으로 `null`을 넘기는 편이 더 안전하지만, 비용 대비 이득은 판단에 맡긴다.

#### L4 — `CliIncompatible`은 WSL 폴백을 유발하지 않는다 (설계 판단 필요)

**위치**: `src-tauri/src/lib.rs:252-256`, `src-tauri/src/collectors/fallback.rs:11-19`

폴백 트리거는 `FallbackTrigger::CliMissing`뿐이다. 네이티브 Codex가 우리 인자를 거부하고(`CliIncompatible`) WSL에는 정상 동작하는 Codex가 있는 상황에서, 이제도 WSL을 시도하지 않고 하드 에러로 끝난다.

**회귀는 아니다** — 종전에도 이 경우는 `Protocol`이었고 마찬가지로 폴백을 유발하지 않았다. 오히려 이번 변경으로 **네이티브 exit 127 → `CliMissing` → WSL 폴백**이 새로 열린 것은 개선이다. 다만 "네이티브 CLI가 우리 호출을 거부함"은 폴백이 존재하는 이유에 정확히 부합하는 상황이므로, `FallbackTrigger`에 `CliIncompatible`을 추가할지는 후속 논의 가치가 있다. 이번 범위에서 처리할 필요는 없다.

#### L5 — 가드 스크립트가 inconclusive 경로에서 codex stderr를 그대로 출력한다

**위치**: `scripts/check-codex-cli-compat.sh:39-41`

인자 파싱과 무관한 종료에서 stderr 20줄을 그대로 찍는다. codex는 오류 메시지에 `codexHome` 같은 로컬 경로를 포함할 수 있다. CI에서는 러너 경로라 무해하고, 로컬에서는 사용자 자신의 경로다. 저장소의 프라이버시 계약은 렌더러 DTO와 앱 로그를 대상으로 하므로 이 개발용 스크립트는 그 범위 밖이다. 다만 **출력을 이슈에 붙여넣기 전에 한 번 확인**할 필요가 있다는 점은 알아둘 만하다.

---

## Validation Results

| Check | Result |
|---|---|
| Type check (`svelte-check`) | **Pass** — 370 files, 0 errors, 0 warnings |
| Lint (`eslint` + `prettier --check`) | **Pass** |
| Rust format (`cargo fmt --check`) | **Pass** |
| Rust lint (`cargo clippy --all-features --all-targets -- -D warnings`) | **Pass** — 경고 0 |
| Tests — Rust | **Pass** — 161 passed / 0 failed (154 → 161, 반영 후) |
| Tests — Renderer | **Pass** — 385 passed / 0 failed, 27 files (378 → 385) |
| Coverage | **Pass** — 96.35% stmts / 91.19% branches (게이트 80%) |
| Build (`vite build`) | **Pass** — 165 modules |
| E2E | **Skipped** — fixture 게이트웨이 형태 무변경. CI에 위임 |

### 실제 CLI 검증 (codex-cli 0.149.0, Windows 11)

```
$ bash scripts/check-codex-cli-compat.sh
Codex CLI argument surface OK: codex-cli 0.149.0                    # exit 0

$ # -a never → -a untrusted 로 되돌린 사본
Codex CLI rejected CacheBite's fixed arguments (installed: codex-cli 0.149.0).
error: invalid value 'untrusted' for '--ask-for-approval <APPROVAL_POLICY>'
  [possible values: on-request, never]                              # exit 1
```

Windows 통합 테스트 `a_cli_that_rejects_our_arguments_reports_cli_incompatible`이 실제 `.cmd` 자식 프로세스로 `Failed { class: CliIncompatible }`까지 도달함을 확인 — 버그가 보고된 플랫폼에서 배선이 검증됐다.

### 검증되지 않은 것

- **GUI 미실행.** `pnpm tauri dev`로 패널에 게이지가 실제로 그려지는 것은 확인하지 않았다. 프로토콜 수준(실제 CLI가 `account/rateLimits/read`에 정상 응답)과 거부 경로(Windows 통합 테스트)는 확인됐다.
- **WSL 경로 실기기 미검증.** 문자열 단언과 fake `wsl.exe` 통합 테스트로만 확인.

---

## Files Reviewed

### Added (4)

| File | Notes |
|---|---|
| `scripts/check-codex-cli-compat.sh` | M3, M4, L5 |
| `.github/workflows/codex-cli-compat.yml` | L2 |
| `.claude/PRPs/plans/completed/codex-cli-approval-policy-compat.plan.md` | 계획 산출물 — 리뷰 대상 아님 |
| `.claude/PRPs/reports/codex-cli-approval-policy-compat-report.md` | 구현 리포트 — 리뷰 대상 아님 |

### Modified (19)

| File | Notes |
|---|---|
| `src-tauri/src/collectors/codex.rs` | 핵심 변경. `app_server_argv()` 추출, `classify_launch_exit` 신설, dead 파라미터 제거 — **M1** |
| `src-tauri/src/collectors/tests.rs` | 신규 테스트 5종 — **L1** |
| `src-tauri/src/collectors/mod.rs` | `CliIncompatible` variant + `into_outcome` arm |
| `src-tauri/src/collectors/wsl.rs` | 런처 2종 교정, 인터랙티브 상수 `pub(crate)` 승격 |
| `src-tauri/src/domain.rs` | `FailureClass::CliIncompatible` — **M2** |
| `src/lib/contracts/domain.ts` | TS 유니온 확장 — **M2** |
| `src/lib/stores/providers.ts` | 인라인 리터럴 유니온 중복 제거 (부수적 개선) |
| `src/lib/components/panelModels.ts` | `failureClass` 필드 추가 |
| `src/lib/state/presentation.ts` | `state.lastFailure` 전달 |
| `src/lib/components/systemGuidance.ts` | 문구 특수화 — **L3** |
| `src/lib/components/ProviderColumn.svelte` | 인자 전달 |
| `src/lib/state/presentation.test.ts` | +1 test |
| `src/lib/components/systemGuidance.test.ts` | +5 tests |
| `src/lib/components/{ProviderColumn,UsagePanel,panelModels}.test.ts` | 픽스처 필드 추가 |
| `docs/architecture.md` | Codex 수집 절 갱신 |
| `docs/ui-contract.md` | §4.2 배지 표에 `cli_incompatible` 행 추가 |
| `CLAUDE.md` | collectors 설명의 명령줄 갱신 |

---

## 반영 결과 (2026-08-24, 같은 브랜치)

| # | 조치 | 결과 |
|---|---|---|
| **M1** | 리뷰의 최초 권고(exit 2로 좁히기)를 **철회**. `RpcSession::saw_response` 신호를 도입해 "CLI가 한 마디라도 했는가"로 재분류를 가드 | 반영 — 근거는 M1 본문의 정정 블록 |
| **M2** | `domain_test.rs`에 `{"kind":"failed","class":"cli_incompatible"}` 단언 추가 | 반영 |
| **M3** | `mktemp "${TMPDIR:-/tmp}/codex-compat.XXXXXX"` + `trap ... EXIT` (BSD/GNU 양쪽에서 동작하는 템플릿 형태) | 반영 — 실행 후 잔여 파일 없음 확인 |
| **M4** | `timeout 30` 래핑 + exit 124 전용 처리. `timeout` 부재 환경(stock macOS)에서는 `command -v` 가드로 무래핑 실행 | 반영 |
| **L1** | 드리프트 테스트 단언을 `codex <argv> </dev/null`까지 포함하도록 강화 | 반영 |
| **L2** | 워크플로에 "의도적으로 미고정" 사유 주석 추가 | 반영 |
| **L3** | `systemGuidance`의 기본값 인자 | **미반영** — 프로덕션 호출부가 1곳뿐이고, 필수 인자화는 기존 테스트 6곳을 건드리면서 얻는 게 미미하다 (YAGNI) |
| **L4** | `CliIncompatible`의 WSL 폴백 유발 | **미반영** — 회귀가 아니고 버그 수정 범위를 넘는 동작 변경이다. 별도 이슈로 분리 권장 |
| **L5** | inconclusive 경로의 stderr 출력 | **미반영(문서화로 대체)** — 스크립트에 "prints codex's own stderr, which can name local paths" 주석 추가 |

### M1 반영으로 추가된 테스트

| Test | 무엇을 잠그나 |
|---|---|
| `rpc_reports_that_a_cli_which_answered_before_dying_did_speak` | 응답 후 스트림 종료 → `saw_response() == true` |
| `rpc_reports_that_a_cli_which_never_answered_stayed_silent` | 출력 0바이트 → `saw_response() == false` |
| `rpc_counts_an_error_response_as_the_cli_speaking` | 오류 envelope도 "CLI가 말한 것"으로 센다 |
| `a_cli_that_crashes_after_answering_keeps_its_own_failure` (windows) | `initialize` 응답 후 exit 3 → `Failed{Parse}` 유지, `CliIncompatible` 아님 |

프로세스를 스폰하는 두 통합 테스트(`a_cli_that_rejects_our_arguments_reports_cli_incompatible`, `a_cli_that_crashes_after_answering_keeps_its_own_failure`)는 플레이크 확인을 위해 5회 연속 실행해 전부 통과했다.

### 반영 후 검증

`cargo test --lib` **161 passed / 0 failed** · `cargo clippy --all-features --lib --tests -- -D warnings` 경고 0 · `cargo fmt --check` clean · `pnpm lint` clean (렌더러 코드는 이번 반영에서 변경 없음).
