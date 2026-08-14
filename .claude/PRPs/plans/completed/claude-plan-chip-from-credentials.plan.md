# Plan: Claude 요금제 칩 — 자격증명 파일의 `subscriptionType` 사용

## Summary

클릭 패널의 Claude 컬럼에 요금제 칩(`pro` / `max`)을 표시한다. 값은 새 네트워크 요청이 아니라 **CacheBite가 이미 열고 있는 자격증명 파일**의 `claudeAiOauth.subscriptionType`에서 읽는다. 파서 한 곳(`parse_token_bytes`)만 넓히면 네이티브와 WSL 두 경로가 함께 해결되고, 렌더러는 전혀 손대지 않는다.

## User Story

CacheBite 사용자로서,
클릭 패널에서 **Codex처럼 Claude의 요금제도** 보고 싶다.
그래야 두 컬럼이 같은 정보를 같은 자리에 보여주고, 어느 쪽이 어떤 등급인지 앱을 떠나지 않고 알 수 있다.

## Problem → Solution

`collectors/claude.rs:175`가 `plan_type: None`을 상수로 넣어 Claude 컬럼에는 칩이 뜨지 않는다. Codex는 `codex.rs:462`에서 `limits.plan_type`을 RPC 응답에서 받아 채운다.

→ `broker.rs`의 자격증명 파서가 `subscriptionType`도 함께 반환하고, `ClaudeCollector`가 그것을 스냅샷의 `plan_type`으로 나른다.

## Metadata

- **Complexity**: Medium
- **Source**: GitHub 이슈 [#85](https://github.com/chanhoan/CacheBite/issues/85) + [정정 코멘트](https://github.com/chanhoan/CacheBite/issues/85#issuecomment-5289492600)
- **PRD Phase**: standalone
- **Estimated Files**: 6 (Rust 4 / 문서 2)

---

## 조사로 확정된 사실 (재조사 불필요)

이 계획은 다음을 **실제로 확인한 뒤** 작성했다. 구현 중 다시 알아볼 필요가 없다.

| 사실 | 확인 방법 | 결과 |
|---|---|---|
| usage 엔드포인트에 요금제 정보가 있는가 | 로그인된 계정으로 `GET /api/oauth/usage` 호출, **키 경로와 타입만** 출력 | **없음.** 최상위 18키 중 일치 0건 |
| `limits[]` 원소에 있는가 | 같은 호출 | 없음 — `kind, group, percent, severity, resets_at, scope, is_active` |
| 자격증명 파일에 있는가 | [claude-code#43639](https://github.com/anthropics/claude-code/issues/43639) — 제목이 *"CLI caches subscriptionType and rateLimitTier"* | **있음.** `claudeAiOauth.subscriptionType` |

usage 응답 최상위 키 (참고, 값 미포함):
```
five_hour, seven_day, seven_day_oauth_apps, seven_day_opus, seven_day_sonnet,
seven_day_cowork, seven_day_omelette, tangelo, iguana_necktie,
omelette_promotional, nimbus_quill, cinder_cove, amber_ladder, extra_usage,
limits, spend, member_dashboard_available
```

> **엔드포인트를 다시 떠보지 말 것.** 위 결과로 충분하고, 자격증명이 붙은 호출이다.

---

## UX Design

### Before

```text
┌──────────────────┬─────────────────────┐
│ Claude ★         │ Codex        Plus   │  ← Codex만 칩
│ 5-hour      82%  │ 5-hour        41%   │
└──────────────────┴─────────────────────┘
```

### After

```text
┌──────────────────┬─────────────────────┐
│ Claude ★    Max  │ Codex        Plus   │  ← 양쪽 다 칩
│ 5-hour      82%  │ 5-hour        41%   │
└──────────────────┴─────────────────────┘
```

### Interaction Changes

| Touchpoint | Before | After | Notes |
|---|---|---|---|
| Claude 컬럼 헤딩 | 이름 + ★ 만 | 이름 + ★ + 요금제 칩 | `planType`이 있을 때만. 렌더러 코드 변경 **없음** |
| Codex 컬럼 | 변화 없음 | 변화 없음 | |
| `CLAUDE_CODE_OAUTH_TOKEN` 사용자 | 칩 없음 | **여전히 칩 없음** | 파일을 안 거치므로 값이 없다. 의도된 동작 |

### UX Edge Cases

- **요금제 업그레이드 직후**: 값이 자격증명 파일에 캐시되어 있어 재로그인 전까지 갱신되지 않는다. 사용자에게는 "옛 등급이 남아 있는" 것으로 보인다 → 계약에 명시할 것
- **파일에 필드가 없는 구버전 CLI**: `Option`이므로 `None` → 칩 생략. 기존과 동일
- **칩이 없어도 레이아웃은 정상**: `{#if model.planType}`이므로 빈 자리가 생기지 않는다 (`ProviderColumn.svelte`)

---

## Mandatory Reading

| Priority | File | Lines | Why |
|---|---|---|---|
| P0 | `src-tauri/src/collectors/broker.rs` | 1-160 | 변경의 중심. 트레이트·파서·소거 규약 전부 여기 |
| P0 | `src-tauri/src/collectors/claude.rs` | 1-50, 107-183 | `Collector` 구현과 `parse_usage` 호출 지점 |
| P0 | `src-tauri/src/collectors/wsl.rs` | 274-300, 345-350 | **두 번째 `ClaudeTokenSource` 구현체.** 같은 파서를 공유 |
| P1 | `src-tauri/src/collectors/codex.rs` | 455-470 | 미러 대상 — `plan_type: limits.plan_type` |
| P1 | `src-tauri/src/domain.rs` | 85-100 | `ProviderUsageSnapshot.plan_type: Option<String>` |
| P2 | `src-tauri/src/collectors/tests.rs` | 1040-1090 | `parse_usage` 테스트 3곳 (시그니처 변경 영향) |
| P2 | `src-tauri/src/collectors/tests.rs` | 587-615 | `wsl_claude_parses_secret_from_bounded_output` 등 WSL 자격증명 테스트 |
| P2 | `docs/ui-contract.md` | §5 | 칩 계약을 적을 곳 |

## External Documentation

| Topic | Source | Key Takeaway |
|---|---|---|
| 자격증명 파일 스키마 | [claude-code#43639](https://github.com/anthropics/claude-code/issues/43639) | `claudeAiOauth`에 `subscriptionType`·`rateLimitTier`. **CLI가 이 값을 캐시**하므로 요금제 변경이 즉시 반영되지 않음 |
| 파일 위치 | [Claude Code Docs — Authentication](https://code.claude.com/docs/en/authentication) | Windows `%USERPROFILE%\.claude\.credentials.json`, Linux/macOS `~/.claude/.credentials.json`. `CredentialLocations::documented`가 이미 두 위치를 다룬다 |

**추가 리서치 불필요** — 나머지는 전부 내부 패턴이다.

---

## Patterns to Mirror

### CREDENTIAL_PARSE — 최소 파싱 + 소거
```rust
// SOURCE: src-tauri/src/collectors/broker.rs:103-145
pub(crate) fn parse_token_bytes(contents: &[u8]) -> Result<Option<SecretString>, ()> {
    if contents.len() as u64 > MAX_CREDENTIAL_BYTES {
        return Err(());
    }
    let mut wire: ClaudeCredentials = serde_json::from_slice(contents).map_err(|_| ())?;
    let token = wire
        .claude_ai_oauth
        .as_mut()
        .and_then(|oauth| oauth.access_token.take())
        .or_else(|| wire.oauth_access_token.take())
        .or_else(|| wire.access_token.take());
    match token {
        Some(mut value) if value.is_empty() => {
            value.zeroize();
            Ok(None)
        }
        Some(value) => Ok(Some(SecretString::from(value))),
        None => Ok(None),
    }
}

#[derive(Deserialize)]
struct OAuthCredentials {
    #[serde(rename = "accessToken")]
    access_token: Option<String>,
}

impl Drop for OAuthCredentials {
    fn drop(&mut self) {
        self.access_token.zeroize();
    }
}
```
> **세 가지를 그대로 지킬 것**: (1) `serde(rename)`로 camelCase 매핑, (2) `Option<String>` + `.take()`, (3) `Drop`에서 **토큰만** `zeroize`.

### TOKEN_SOURCE_TRAIT — 두 구현체, 감싸는 방식이 다름
```rust
// SOURCE: src-tauri/src/collectors/broker.rs:9-13
pub trait ClaudeTokenSource: Send + Sync {
    fn claude_token(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<SecretString, CollectorError>> + Send + '_>>;
}

// SOURCE: broker.rs:72-78 — 동기 함수를 ready future로 감쌈
impl ClaudeTokenSource for CredentialBroker {
    fn claude_token(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<SecretString, CollectorError>> + Send + '_>> {
        Box::pin(std::future::ready(self.claude_token()))
    }
}

// SOURCE: wsl.rs:345-350 — 진짜 async
impl ClaudeTokenSource for WslCredentialSource {
    fn claude_token(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<SecretString, CollectorError>> + Send + '_>> {
        Box::pin(WslCredentialSource::claude_token(self))
    }
}
```

### SNAPSHOT_CONSTRUCTION — 미러 대상
```rust
// SOURCE: src-tauri/src/collectors/codex.rs:460-469
Ok(ProviderUsageSnapshot {
    provider: Provider::Codex,
    plan_type: limits.plan_type,
    session,
    weekly,
    captured_at: now,
    source: Source::CliRpc,
    is_cached: false,
    revision: 0,
})
```

### COLLECT_FLOW
```rust
// SOURCE: src-tauri/src/collectors/claude.rs:30-45
fn collect(
    &self,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = CollectionOutcome> + Send + '_>> {
    Box::pin(async move {
        let result = match self.token_source.claude_token().await {
            Ok(token) => {
                fetch_usage_with_client(&self.client, token, OffsetDateTime::now_utc()).await
            }
            Err(error) => Err(error),
        };
        match result {
            Ok(snapshot) => CollectionOutcome::Success { snapshot },
            Err(error) => error.into_outcome(),
        }
    })
}
```

### TEST_STRUCTURE — 자격증명 파서 테스트
```rust
// SOURCE: src-tauri/src/collectors/tests.rs:588-597
#[tokio::test]
async fn wsl_claude_parses_secret_from_bounded_output() {
    let source = fake_wsl(Ok(ProcessOutput {
        status: 0,
        stdout: br#"{"claudeAiOauth":{"accessToken":"wsl-secret"}}"#.to_vec(),
    }));
    assert_eq!(
        source.claude_token().await.unwrap().expose_secret(),
        "wsl-secret"
    );
}
```
> 자격증명 픽스처는 **인라인 바이트 리터럴**이며 실제 자격증명을 쓰지 않는다.

---

## Files to Change

| File | Action | Justification |
|---|---|---|
| `src-tauri/src/collectors/broker.rs` | UPDATE | `ClaudeCredential` 구조체 도입, 트레이트·파서 반환 타입 확장, `OAuthCredentials`에 필드 추가 |
| `src-tauri/src/collectors/claude.rs` | UPDATE | 새 반환 타입 수용, `parse_usage`에 요금제 전달, `plan_type: None` 제거 |
| `src-tauri/src/collectors/wsl.rs` | UPDATE | 두 번째 `ClaudeTokenSource` 구현체 시그니처 정렬 |
| `src-tauri/src/collectors/tests.rs` | UPDATE | 기존 테스트 시그니처 정렬 + 신규 8건 |
| `docs/ui-contract.md` | UPDATE | §5에 칩 계약과 두 한계 명시 |
| `docs/architecture.md` | UPDATE | 자격증명 파서 서술이 있을 경우만 갱신 |

**렌더러는 변경 없음** — `plan_type` → wire → `PanelProviderModel.planType` → 칩 파이프라인이 Codex용으로 이미 완성돼 있다.

## NOT Building

- **`rateLimitTier` 읽기.** 같은 객체에 있지만 표시할 자리가 없다. 최소 파싱 원칙을 지킨다
- **요금제 캐시 무효화 / 재로그인 유도.** CLI가 캐시하는 값이고, CacheBite는 자격증명 파일을 **쓰지 않는다**(불변식)
- **환경변수 경로용 대체 소스.** `CLAUDE_CODE_OAUTH_TOKEN` 사용자는 칩 없이 둔다
- **요금제 문자열 정규화·표시명 매핑.** 칩은 `text-transform: capitalize`로 이미 처리한다(`ProviderColumn.svelte`). `"max"` → `Max`
- **렌더러 변경 일체.** `ProviderColumn.svelte`·`panelModels.ts`·게이트웨이 DTO 모두 그대로
- **usage 엔드포인트 재조사.** 위 표에서 확정됐다

---

## Step-by-Step Tasks

### Task 1: `ClaudeCredential` 타입 도입 + 파서 확장 (TDD)

- **ACTION**: `broker.rs`에 토큰과 요금제를 함께 나르는 타입을 만들고 `parse_token_bytes`가 그것을 반환하게 한다. 테스트를 **먼저** 작성한다.
- **IMPLEMENT**:
  ```rust
  /// A Claude credential as the collectors need it: the bearer token, plus the
  /// subscription tier that sits beside it in the same file.
  ///
  /// The tier is a plain `String`, not a `SecretString`: `"pro"` / `"max"` is a
  /// plan grade, not an authorization value or an account identifier, and the
  /// panel is meant to display it. Only the token is zeroized.
  pub struct ClaudeCredential {
      pub token: SecretString,
      pub subscription_type: Option<String>,
  }

  pub(crate) fn parse_token_bytes(contents: &[u8]) -> Result<Option<ClaudeCredential>, ()> {
      if contents.len() as u64 > MAX_CREDENTIAL_BYTES {
          return Err(());
      }
      let mut wire: ClaudeCredentials = serde_json::from_slice(contents).map_err(|_| ())?;
      // Read the tier before the token is taken: the tier lives in the same
      // nested object, and taking the token first is a refactor away from
      // leaving this reading an emptied struct.
      let subscription_type = wire
          .claude_ai_oauth
          .as_mut()
          .and_then(|oauth| oauth.subscription_type.take())
          .filter(|value| !value.is_empty());
      let token = wire
          .claude_ai_oauth
          .as_mut()
          .and_then(|oauth| oauth.access_token.take())
          .or_else(|| wire.oauth_access_token.take())
          .or_else(|| wire.access_token.take());
      match token {
          Some(mut value) if value.is_empty() => {
              value.zeroize();
              Ok(None)
          }
          Some(value) => Ok(Some(ClaudeCredential {
              token: SecretString::from(value),
              subscription_type,
          })),
          None => Ok(None),
      }
  }
  ```
  `OAuthCredentials`에 필드 추가:
  ```rust
  #[derive(Deserialize)]
  struct OAuthCredentials {
      #[serde(rename = "accessToken")]
      access_token: Option<String>,
      // The one field beyond the token this parser opens. Claude Code's own
      // `/status` reads the tier from here; the usage endpoint does not carry
      // it (checked — 18 top-level keys, none a tier). `rateLimitTier` sits
      // beside it and is deliberately left unparsed: nothing displays it.
      #[serde(rename = "subscriptionType")]
      subscription_type: Option<String>,
  }

  impl Drop for OAuthCredentials {
      fn drop(&mut self) {
          // Only the token. The tier is not a secret, and zeroizing it would
          // tell the next reader that it is one.
          self.access_token.zeroize();
      }
  }
  ```
  `read_token` 반환 타입도 `Result<Option<ClaudeCredential>, ()>`로 바꾼다 (본문은 `parse_token_bytes` 호출만 바뀜).
- **MIRROR**: `broker.rs:103-145` (CREDENTIAL_PARSE) — `serde(rename)`, `.take()`, `Drop` 소거 세 가지를 그대로.
- **IMPORTS**: 추가 없음. `SecretString`·`Zeroize`·`Deserialize` 모두 이미 있다.
- **GOTCHA**:
  - **`subscription_type`을 토큰보다 먼저 꺼낼 것** (위 주석 참조)
  - **`subscription_type`을 `zeroize`하지 말 것.** 비밀이 아니며, 소거하면 후속 독자에게 잘못된 신호를 준다
  - 빈 문자열은 `None`으로 접는다(`filter`). 토큰의 빈 값 처리와 같은 규약
  - `ClaudeCredential`에 `Debug`를 **파생하지 말 것** — `SecretString`이 막아 주지만 파생 자체를 두지 않는 편이 명확하다
  - `contents.zeroize()`(`read_token:99`)는 그대로 유지 — 원본 바이트 소거는 요금제 추가와 무관하다
- **VALIDATE**: `cargo test --manifest-path src-tauri/Cargo.toml --all-features collectors::tests`

**신규 테스트 5건** (`collectors/tests.rs`):

| Test | Input | Expected |
|---|---|---|
| 요금제와 토큰을 함께 읽는다 | `{"claudeAiOauth":{"accessToken":"t","subscriptionType":"max"}}` | token `"t"`, tier `Some("max")` |
| 요금제가 없어도 토큰은 읽힌다 | `{"claudeAiOauth":{"accessToken":"t"}}` | token `"t"`, tier `None` |
| 빈 요금제는 `None`으로 접힌다 | `{"claudeAiOauth":{"accessToken":"t","subscriptionType":""}}` | tier `None` |
| 레거시 최상위 토큰은 요금제가 없다 | `{"accessToken":"t"}` | token `"t"`, tier `None` |
| 빈 토큰은 여전히 `None` (회귀) | `{"claudeAiOauth":{"accessToken":"","subscriptionType":"max"}}` | `Ok(None)` — 요금제가 있어도 토큰이 없으면 자격증명이 아니다 |

---

### Task 2: `ClaudeTokenSource` 트레이트 확장

- **ACTION**: 트레이트의 `Output`을 `Result<ClaudeCredential, CollectorError>`로 바꾸고 **두 구현체를 모두** 정렬한다.
- **IMPLEMENT**:
  ```rust
  pub trait ClaudeTokenSource: Send + Sync {
      fn claude_token(
          &self,
      ) -> Pin<Box<dyn Future<Output = Result<ClaudeCredential, CollectorError>> + Send + '_>>;
  }
  ```
  `CredentialBroker::claude_token`(동기, broker.rs:52)의 환경변수 분기:
  ```rust
  if let Some(value) = &self.environment_token {
      // `CLAUDE_CODE_OAUTH_TOKEN` bypasses the credential file, which is the
      // only place the tier is recorded — so there is nothing to report and
      // the panel simply shows no chip.
      return Ok(ClaudeCredential {
          token: SecretString::from(value.expose_secret().to_owned()),
          subscription_type: None,
      });
  }
  ```
  파일 순회 분기는 `read_token`이 이미 `ClaudeCredential`을 돌려주므로 `Ok(Some(credential)) => return Ok(credential)`로 바뀌기만 한다.
- **MIRROR**: `broker.rs:72-78`(ready future)와 `wsl.rs:345-350`(진짜 async) — **두 구현체의 감싸는 방식이 다르다.** 각자 형태를 유지하고 타입만 바꾼다.
- **IMPORTS**: `wsl.rs:2`의 `broker::{parse_token_bytes, ClaudeTokenSource, MAX_CREDENTIAL_BYTES}`에 `ClaudeCredential` 추가.
- **GOTCHA**:
  - **구현체가 둘이다.** `CredentialBroker`(broker.rs:72)와 `WslCredentialSource`(wsl.rs:345). 하나만 고치면 컴파일이 깨진다 — 컴파일러가 잡아 주지만 미리 알고 가면 왕복이 없다
  - `WslCredentialSource::claude_token`(wsl.rs:274)은 `parse_token_bytes`를 재사용하므로(wsl.rs:286) **요금제를 자동으로 얻는다.** 반환 타입만 맞추면 된다
  - 트레이트 이름은 `ClaudeTokenSource` 그대로 둔다 — 이름 변경은 범위 밖이고 호출부가 늘어난다
- **VALIDATE**: `cargo build --manifest-path src-tauri/Cargo.toml --all-features`

---

### Task 3: 스냅샷에 요금제 싣기

- **ACTION**: `claude.rs`가 자격증명의 요금제를 `parse_usage`로 넘겨 스냅샷에 넣는다.
- **IMPLEMENT**:
  - `collect()`(claude.rs:34):
    ```rust
    let result = match self.token_source.claude_token().await {
        Ok(credential) => {
            fetch_usage_with_client(&self.client, credential, OffsetDateTime::now_utc()).await
        }
        Err(error) => Err(error),
    };
    ```
  - `fetch_usage_with_client`의 두 번째 인자를 `credential: ClaudeCredential`로 바꾸고, `ClaudeRequestSpec::new(credential.token)`으로 요청을 만든 뒤 마지막 줄(claude.rs:148)을:
    ```rust
    parse_usage(&body, now, credential.subscription_type)
    ```
    > `credential.token`이 먼저 move 되므로 `subscription_type`은 그 전에 지역 변수로 빼두거나, 구조 분해(`let ClaudeCredential { token, subscription_type } = credential;`)로 나눈다. **구조 분해를 권장** — 부분 move 경고를 피한다.
  - `parse_usage`(claude.rs:160) 시그니처와 스냅샷:
    ```rust
    pub fn parse_usage(
        body: &[u8],
        now: OffsetDateTime,
        plan_type: Option<String>,
    ) -> Result<ProviderUsageSnapshot, CollectorError> {
        ...
        Ok(ProviderUsageSnapshot {
            provider: Provider::Claude,
            // From the credential file, not this response: the usage endpoint
            // carries no plan field. Claude Code's own `/status` reads the tier
            // from the same file this collector already opens for the token.
            plan_type,
            session,
            weekly,
            captured_at: now,
            source: Source::OauthApi,
            is_cached: false,
            revision: 0,
        })
    }
    ```
- **MIRROR**: `codex.rs:460-469` (SNAPSHOT_CONSTRUCTION) — 필드 순서와 형태를 맞춘다.
- **IMPORTS**: `claude.rs:1`의 `use super::{broker::ClaudeTokenSource, ...}`를 `broker::{ClaudeCredential, ClaudeTokenSource}`로.
- **GOTCHA**:
  - `parse_usage`는 `pub`이고 **테스트 3곳**(`tests.rs:1043, 1074, 1085`)이 호출한다. 전부 세 번째 인자가 필요하다
  - 요금제를 `Option<String>`으로 **소유권째 넘긴다**. `&str`로 빌리면 `credential`의 수명을 호출까지 붙들어야 해서 지저분해진다
  - **`plan_type`을 응답에서 찾으려 하지 말 것.** `UsageWire`에 필드를 추가해도 항상 `None`이다
- **VALIDATE**: `cargo test --manifest-path src-tauri/Cargo.toml --all-features collectors`

**신규/변경 테스트 2+3건**:

| Test | Input | Expected |
|---|---|---|
| 요금제가 스냅샷에 실린다 | `parse_usage(valid_body, now, Some("max".into()))` | `snapshot.plan_type.as_deref() == Some("max")` |
| 요금제가 없으면 `None` | `parse_usage(valid_body, now, None)` | `snapshot.plan_type.is_none()` |
| 기존 3개 호출부 | 세 번째 인자 `None` 추가 | 기존 단언 그대로 통과 |

---

### Task 4: WSL 경로 회귀 테스트

- **ACTION**: WSL 자격증명 경로도 요금제를 나르는지 고정한다.
- **IMPLEMENT**: `tests.rs`의 `wsl_claude_parses_secret_from_bounded_output`(588행) 옆에 추가:
  ```rust
  #[tokio::test]
  async fn wsl_claude_carries_the_subscription_tier_alongside_the_secret() {
      let source = fake_wsl(Ok(ProcessOutput {
          status: 0,
          stdout: br#"{"claudeAiOauth":{"accessToken":"wsl-secret","subscriptionType":"pro"}}"#
              .to_vec(),
      }));
      let credential = source.claude_token().await.unwrap();
      assert_eq!(credential.token.expose_secret(), "wsl-secret");
      assert_eq!(credential.subscription_type.as_deref(), Some("pro"));
  }
  ```
- **MIRROR**: `tests.rs:588-597` (TEST_STRUCTURE) — 인라인 바이트 리터럴 픽스처.
- **IMPORTS**: 이미 있는 것들(`ExposeSecret` 포함).
- **GOTCHA**: 이 테스트가 **파서 공유가 깨지는 것**을 잡는다. 누군가 WSL 경로에 별도 파서를 만들면 여기서 실패한다. 기존 588행 테스트도 `.claude_token().await.unwrap()`이 이제 `ClaudeCredential`을 돌려주므로 `.token.expose_secret()`으로 고쳐야 한다.
- **VALIDATE**: `cargo test --manifest-path src-tauri/Cargo.toml --all-features wsl_claude`

---

### Task 5: 계약 문서화

- **ACTION**: `docs/ui-contract.md` §5의 규칙 목록에 칩 계약과 두 한계를 명시한다.
- **IMPLEMENT**: §5 규칙 목록에 추가:
  > - **요금제 칩은 provider가 등급을 보고할 때만 뜬다.** Codex는 JSON-RPC 응답에서, Claude는 자격증명 파일의 `subscriptionType`에서 온다 — Claude 사용량 엔드포인트에는 등급 필드가 없다. 값이 없으면 칩을 렌더하지 않으며 레이아웃은 그대로다.
  > - **Claude 등급은 CLI가 캐시한 값이다.** 요금제를 올려도 재로그인 전까지 갱신되지 않는다. CacheBite는 자격증명 파일을 **쓰지 않으므로** 이를 고칠 수 없고, 고쳐서도 안 된다.
  > - **`CLAUDE_CODE_OAUTH_TOKEN`으로 로그인하면 Claude 칩이 없다.** 그 경로는 자격증명 파일을 거치지 않아 등급이 기록될 자리가 없다.
- **MIRROR**: §5의 기존 서술 톤(한국어 규칙 목록 + 근거 문장).
- **IMPORTS**: 해당 없음.
- **GOTCHA**: `docs/architecture.md`에 자격증명 파서가 무엇을 읽는지 서술한 대목이 있으면 함께 갱신한다. `grep -n "credential" docs/architecture.md`로 먼저 확인하고, 없으면 건너뛴다.
- **VALIDATE**: `corepack pnpm lint` (prettier가 마크다운도 검사)

---

## Testing Strategy

### Unit Tests

| 파일 | 신규 | 변경 |
|---|---|---|
| `collectors/tests.rs` — 자격증명 파서 | 5 | — |
| `collectors/tests.rs` — `parse_usage` | 2 | 3 (세 번째 인자) |
| `collectors/tests.rs` — WSL | 1 | 1 (`.token.expose_secret()`) |

### Edge Cases Checklist

- [ ] 요금제 + 토큰 둘 다 있음 → 둘 다 읽힘
- [ ] 요금제 없음 → 토큰만, 칩 생략
- [ ] 빈 문자열 요금제 → `None`
- [ ] 빈 토큰 + 요금제 있음 → `Ok(None)` (자격증명 아님)
- [ ] 레거시 최상위 `accessToken` → 요금제 `None`
- [ ] 환경변수 토큰 → 요금제 `None`
- [ ] WSL 경로 → 요금제 실림
- [ ] 파일 크기 상한 초과 → 기존대로 `Err(())` (요금제 추가가 상한 검사를 우회하지 않음)
- [ ] 토큰이 `Debug`·로그에 새지 않음 — `SecretString` 유지 확인

`Concurrent access` / `Network failure`는 이 변경(파일 파싱)의 범위 밖이며 기존 경로가 그대로 처리한다.

---

## Validation Commands

### Static Analysis
```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-features -- -D warnings
```
EXPECT: 오류 0건

### Unit Tests
```bash
cargo test --manifest-path src-tauri/Cargo.toml --all-features collectors
```
EXPECT: 전건 통과

### Full Suite
```bash
cargo test --manifest-path src-tauri/Cargo.toml --all-features
corepack pnpm test:ci
```
EXPECT: 회귀 없음. **렌더러 테스트는 변경이 없어야 정상** — 이 계획이 렌더러를 안 건드렸다는 증거다

### Manual Validation
```bash
corepack pnpm tauri dev
```
- [ ] 펫 더블클릭 → 패널의 Claude 컬럼 헤딩 오른쪽에 요금제 칩이 보인다
- [ ] 칩 텍스트가 `text-transform: capitalize`로 렌더된다 (`max` → `Max`)
- [ ] Codex 칩과 같은 자리·같은 스타일이다
- [ ] 두 컬럼의 행 정렬이 그대로다 (칩이 헤딩 행 안에 있으므로 높이 불변)
- [ ] `CLAUDE_CODE_OAUTH_TOKEN`을 설정하고 재기동 → Claude 칩이 사라지고 **레이아웃은 정상**

---

## Acceptance Criteria

- [ ] Claude 컬럼에 요금제 칩이 표시된다
- [ ] 값이 없으면 칩이 생략되고 레이아웃이 깨지지 않는다
- [ ] WSL 경로도 요금제를 나른다
- [ ] **렌더러 코드 변경 0줄**
- [ ] **추가 네트워크 요청 0건**
- [ ] 토큰은 여전히 `SecretString`이며 `zeroize`된다
- [ ] 모든 검증 명령 통과

## Completion Checklist

- [ ] `subscription_type`이 `SecretString`이 **아니고** `zeroize` 대상도 아니다
- [ ] `OAuthCredentials`에 왜 이 필드만 더 여는지, `rateLimitTier`는 왜 안 여는지 주석이 있다
- [ ] 두 `ClaudeTokenSource` 구현체가 모두 정렬됐다
- [ ] 캐시 한계와 환경변수 한계가 계약에 적혀 있다
- [ ] 자격증명 파일을 **쓰지 않는다** (불변식)
- [ ] 테스트 픽스처에 실제 자격증명이 없다

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| 요금제가 캐시되어 업그레이드가 반영 안 됨 | **높음** (상류 동작) | 중간 | 고칠 수 없다 — 계약에 명시. 리포트가 오면 재로그인 안내 |
| 트레이트 확장이 한쪽 구현체를 놓침 | 중간 | 낮음 | 컴파일러가 잡는다. 계획에 두 위치를 명시 |
| `subscriptionType` 값이 예상 밖 문자열 | 낮음 | 낮음 | 칩은 문자열을 그대로 렌더한다. `"team"` 같은 값도 그냥 표시되면 된다 |
| 최소 파싱 원칙 약화로 읽히는 것 | 중간 | 낮음 | 필드 하나만, 주석으로 근거. `rateLimitTier`는 의도적으로 제외 |
| 자격증명 파일 스키마 변경 | 낮음 | 낮음 | `Option`이라 사라지면 칩만 없어지고 토큰 경로는 무사 |

## Notes

- **이 계획의 성격**: 데이터 소스 연결이지 새 기능이 아니다. 표시 파이프라인은 Codex용으로 완성돼 있고 Claude 쪽 입력만 비어 있었다.

- **왜 트레이트를 넓히는가 (대안 기각 근거)**:
  - *두 번째 트레이트 메서드*(`claude_subscription_type()`) — 파일을 두 번 읽게 되고, 두 읽기 사이에 파일이 바뀌면 토큰과 등급이 어긋난다
  - *`parse_usage`에서 응답을 뒤진다* — 응답에 없음이 확인됨
  - *별도 엔드포인트 호출* — 요청이 늘고, 어느 엔드포인트가 주는지 모르며, 계정 식별자 노출 위험이 생긴다

  하나의 파일 읽기에서 두 값을 함께 꺼내는 것이 유일하게 원자적이다.

- **프라이버시 판단 근거**: 렌더러에서 배제되는 것은 authorization 값 / 원본 응답 본문 / 계정 식별자 / 자격증명 경로 네 가지다. `"pro"`·`"max"`는 등급이지 식별자가 아니고, Codex가 이미 같은 필드를 나른다. 다만 `OAuthCredentials`의 필드가 하나뿐이었던 것은 **의도적 최소화**로 보이므로, 늘리는 만큼 주석으로 근거를 남긴다.

- **조사 방법 기록**: usage 엔드포인트 확인 시 **키 경로와 타입만** 출력했고 값·토큰·계정 식별자는 찍지 않았다. 자격증명 파일 직접 확인은 권한 분류기에 차단되어 수행하지 않았으며, 필드 존재는 상류 이슈([#43639](https://github.com/anthropics/claude-code/issues/43639))로 확인했다. **구현 시 Task 1의 테스트가 파서 동작을 코드로 고정하므로 파일을 다시 열 필요는 없다.** 다만 실기기에서 칩이 실제로 뜨는지는 Manual Validation에서 확인해야 한다 — 그 단계가 필드 존재의 최종 확인이다.
