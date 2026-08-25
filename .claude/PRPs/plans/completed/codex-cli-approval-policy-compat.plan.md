# Plan: Codex CLI `--ask-for-approval` 호환성 복구 및 CLI 거부 가시화

## Summary

Codex CLI 0.149.0에서 `--ask-for-approval`의 `untrusted` 값이 제거되어(`on-request`, `never`만 잔존) CacheBite가 고정 인자로 넘기는 `codex -s read-only -a untrusted app-server`가 **즉시 exit 2**로 종료된다. stderr는 `Stdio::null()`로 버려지고 stdout은 JSON 한 줄도 내보내지 못한 채 닫히므로, 수집기는 `CollectorError::Protocol` → `FailureClass::Parse` → 렌더러 `system='error'`로 강등되어 **"Could not fetch usage. Retrying shortly."** 라는 무의미한 문구만 반복 표시한다. 인증 문제가 아니며, 인자만 고치면 실제 rate limit 데이터가 정상 반환된다.

이 계획은 (1) 인자 값을 `never`로 교정하고, (2) "CLI가 우리 호출 자체를 거부함"을 파싱 실패와 구분해 사용자에게 실행 가능한 문구로 노출하며, (3) 동일한 상류 변경을 사용자보다 먼저 감지할 자격증명 불필요 호환성 체크를 추가한다.

## User Story

**As a** CacheBite 사용자,
**I want** Codex CLI를 업데이트해도 사용량이 계속 수집되고, 만약 CLI 변경으로 깨졌다면 화면이 그 사실을 명확히 알려주기를,
**So that** 원인을 "로그인이 풀렸나?"로 오진하며 시간을 낭비하지 않는다.

## Problem → Solution

**현재**: Codex 열이 영구적으로 `error` 배지 + "Could not fetch usage. Retrying shortly." → 사용자는 auth 문제로 오인. 실제로는 CLI 인자 거부.

**목표**: `-a never`로 정상 수집 복구. 아울러 앞으로 CLI가 인자를 거부하면 `error` 배지 + "Codex CLI가 이 CacheBite 버전과 호환되지 않습니다..." 라는 진단 가능한 문구가 즉시 뜬다.

## Metadata

- **Complexity**: Medium (Phase 1은 Small, Phase 2가 cross-cutting)
- **Source PRD**: N/A (자유 형식 버그 리포트)
- **PRD Phase**: N/A
- **Estimated Files**: 16 (Phase 1: 6, Phase 2: 9, Phase 3: 2)

---

## 근본 원인 조사 기록 (Phase 1 — 완료)

이 절은 재조사를 막기 위한 증거 기록이다. 구현 중 다시 재현할 필요 없다.

### 재현 환경

- Codex: `codex-cli 0.149.0` (`@openai/codex@0.149.0`, npm global)
- 네이티브 바이너리: `…/node_modules/@openai/codex/node_modules/@openai/codex-win32-x64/vendor/x86_64-pc-windows-msvc/bin/codex.exe`
- 플랫폼: Windows 11 (10.0.26200)

### 증거 1 — 현재 인자는 즉시 거부된다

```
$ codex.exe -s read-only -a untrusted app-server
STDERR<< error: invalid value 'untrusted' for '--ask-for-approval <APPROVAL_POLICY>'
  [possible values: on-request, never]

For more information, try '--help'.
EXIT: 2
```

stdin으로 보낸 `initialize`는 이미 죽은 프로세스로 흘러들어가 무시된다.

### 증거 2 — 허용 값 축소 확인 (`codex --help`)

```
  -s, --sandbox <SANDBOX_MODE>
          [possible values: read-only, workspace-write, danger-full-access]   ← 변화 없음

  -a, --ask-for-approval <APPROVAL_POLICY>
          Possible values:
          - on-request: The model decides when to ask the user for approval
          - never:      Never ask for user approval …                          ← untrusted / on-failure 제거됨
```

### 증거 3 — `-a never`로 전체 핸드셰이크 성공 (네이티브 바이너리)

```
STDOUT<< {"id":1,"result":{"userAgent":"cachebite/0.149.0 (…)","codexHome":"…","platformFamily":"windows","platformOs":"windows"}}
STDOUT<< {"method":"remoteControl/status/changed","params":{…},"emittedAtMs":…}
STDOUT<< {"id":2,"result":{"rateLimits":{"limitId":"codex","primary":{"usedPercent":0,"windowDurationMins":10080,"resetsAt":1788142990},"secondary":null,"credits":{…},"planType":"team",…},"rateLimitsByLimitId":{…},"rateLimitResetCredits":{…}}}
```

### 증거 4 — `codex.cmd` npm shim 경로로도 동일하게 성공

`cmd.exe /d /s /c "codex.cmd -s read-only -a never app-server"`로도 위와 동일한 3줄이 반환됨. 즉 Windows에서 CacheBite가 실제로 resolve 하는 `.cmd` 경로에서도 수정이 유효하다.

### 파생 확인 사항 (수정 불필요 — 회귀 방지용 기록)

| 항목 | 확인 결과 |
| --- | --- |
| `initialize` 응답의 `jsonrpc` 필드 부재 | 여전히 부재. `validate_response`의 관용 로직(`codex.rs:571-593`) 그대로 유효 |
| `remoteControl/status/changed` 알림이 응답 사이에 끼어듦 | `read_matching_response`(`codex.rs:518-533`)가 `id:None && method:Some`을 건너뛰므로 이미 처리됨 |
| 응답 봉투 `{"rateLimits":{…}}` | `RateLimitEnvelope::Nested`(`codex.rs:618-635`)가 매칭. 변경 불필요 |
| `primary`에 주간(10080분) 창, `secondary:null` | `normalize`(`codex.rs:431-470`)가 슬롯 순서가 아닌 지속시간으로 분류. 변경 불필요 |
| `planType: "team"` | 렌더러는 불투명 문자열로 chip 표시(`ProviderColumn.svelte:50`). 화이트리스트 없음. 변경 불필요 |
| `clientInfo{name,version}` 요구 | 여전히 필요. `initialize_params()`(`codex.rs:475-482`) 유지 |

### 왜 조용히 실패했는가 (데이터 흐름 추적)

```
codex.exe exit 2 (stderr는 Stdio::null()로 폐기 — codex.rs:247)
  → stdout 파이프 즉시 닫힘
  → read_response()의 reader.read_u8() 가 Err          (codex.rs:542-545)
  → CollectorError::Protocol
  → into_outcome(): Protocol → Failed{ FailureClass::Parse }   (mod.rs:47-51)
  → engine.ts degraded: lastFailure!=='network' → 'error'      (engine.ts:200-202)
  → systemGuidance('error') → "Could not fetch usage. Retrying shortly."  (systemGuidance.ts:28-29)
```

`exit_127_is_cli_missing` 플래그(`codex.rs:194`)가 존재하지만 **호출부가 `false` 하나뿐**(`codex.rs:187`)이라 종료 코드 분류 경로는 현재 완전한 dead code다. Phase 2는 이 배선을 되살려 일반화한다.

---

## UX Design

### Before

```
┌──────────────── CacheBite Panel ─────────────────┐
│  ★ Claude                    │  Codex            │
│  ┌───────┐                   │  ⚠ (error badge)  │
│  │  32%  │  5h               │                   │
│  └───────┘                   │  Could not fetch  │
│  ┌───────┐                   │  usage. Retrying  │
│  │  11%  │  Weekly           │  shortly.         │
│  └───────┘                   │                   │
│                              │  ← 영구히 이 상태  │
│                              │    (재시도해도 동일)│
└──────────────────────────────────────────────────┘
       사용자 해석: "Codex 로그인이 풀렸나?"  ← 오진
```

### After

**Phase 1 적용 후 (정상 경로 — 대다수 사용자)**

```
┌──────────────── CacheBite Panel ─────────────────┐
│  ★ Claude                    │  Codex   [team]   │
│  ┌───────┐                   │  ┌───────┐        │
│  │  32%  │  5h               │  │   —   │  5h    │
│  └───────┘                   │  └───────┘        │
│  ┌───────┐                   │  ┌───────┐        │
│  │  11%  │  Weekly           │  │   0%  │ Weekly │
│  └───────┘                   │  └───────┘        │
└──────────────────────────────────────────────────┘
```

**Phase 2 적용 후 (미래에 CLI가 또 인자를 거부할 때)**

```
┌──────────────── CacheBite Panel ─────────────────┐
│  ★ Claude                    │  Codex            │
│  ┌───────┐                   │  ⚠ (error badge)  │
│  │  32%  │  5h               │                   │
│  └───────┘                   │  The Codex CLI    │
│                              │  rejected this    │
│                              │  CacheBite build. │
│                              │  Update CacheBite.│
└──────────────────────────────────────────────────┘
       사용자 해석: "CacheBite를 업데이트해야겠군"  ← 정확
```

### Interaction Changes

| Touchpoint | Before | After | Notes |
| --- | --- | --- | --- |
| Codex 열 게이지 | 영구 공란 | 실제 사용률 | Phase 1 |
| Codex 배지 | ⚠ error (원인 불명) | 정상 시 배지 없음 | Phase 1 |
| CLI 인자 거부 시 문구 | "Could not fetch usage. Retrying shortly." | "The Codex CLI rejected this CacheBite build. Update CacheBite." | Phase 2 |
| CLI 미설치 시 문구 | "The Codex CLI is not installed" | 변경 없음 | 기존 `CliMissing` 경로 유지 |
| 배지 아이콘 세트 | 5종 | 변경 없음 | **새 `SystemState`를 만들지 않는 것이 설계 핵심** |

---

## Mandatory Reading

| Priority | File | Lines | Why |
| --- | --- | --- | --- |
| P0 | `src-tauri/src/collectors/codex.rs` | 171-265 | 수정 대상 인자 + 자식 프로세스 수명/종료 코드 배선 전부 |
| P0 | `src-tauri/src/collectors/wsl.rs` | 14-19, 130-140 | WSL 런처 스크립트 2종과 선택 로직 |
| P0 | `src-tauri/src/collectors/mod.rs` | 12-57 | `CollectorError` 정의 + `into_outcome` 매핑 |
| P0 | `src-tauri/src/domain.rs` | 29-36, 100-125 | `FailureClass` / `CollectionOutcome` / `ProviderUiSnapshot` |
| P1 | `src/lib/state/engine.ts` | 15-48, 185-215 | `SystemState` 유니온과 degraded 강등 규칙 |
| P1 | `src/lib/components/systemGuidance.ts` | 1-35 | 안내 문구 전체 (파일이 짧음 — 전부 읽을 것) |
| P1 | `src/lib/state/presentation.ts` | 40-75 | `toProviderPresentation` — 패널 모델 조립 지점 |
| P1 | `src/lib/components/panelModels.ts` | 1-22 | `PanelProviderModel` 인터페이스 |
| P2 | `src-tauri/src/collectors/tests.rs` | 73-90, 260-285, 1190-1270 | 고정 인자 assertion + RPC/파싱 테스트 패턴 |
| P2 | `src/lib/contracts/domain.ts` | 28-55 | `FailureClass` / `UnavailableReason` 유니온 |
| P2 | `docs/architecture.md` | 178-203 | Codex 수집 절 — 인자 문서화 위치 |
| P2 | `docs/ui-contract.md` | 148-160 | §4.2 시스템 배지 표 |

## External Documentation

| Topic | Source | Key Takeaway |
| --- | --- | --- |
| `--ask-for-approval` 허용 값 | 로컬 `codex --help` (0.149.0) | `on-request`, `never`만 유효. `untrusted`/`on-failure` 제거 |
| `--sandbox` 허용 값 | 로컬 `codex --help` (0.149.0) | `read-only` 유지 — **변경 금지** |
| `app-server` 하위 명령 옵션 | 로컬 `codex app-server --help` | `-s`/`-a`는 **루트** 옵션이며 하위 명령 앞에 와야 한다. 현재 인자 순서는 올바름 |
| npm 패키지 구조 | `@openai/codex@0.149.0/package.json` | `bin/codex.js`(Node ESM 런처)가 플랫폼별 vendor `codex.exe`를 `stdio:"inherit"`로 spawn |

```
KEY_INSIGHT: 0.149.0의 npm bin은 네이티브 바이너리가 아니라 Node 런처 스크립트다.
APPLIES_TO: 프로세스 종료/정리 (Task 4의 GOTCHA)
GOTCHA: Windows에서 `codex.cmd` → cmd.exe → node → codex.exe 3단 체인이 된다.
        `Child::kill()`은 최상위만 죽이므로 손자 프로세스가 남을 수 있다. 이는
        이번 버그와 무관한 선재 리스크이며 본 계획의 범위 밖(NOT Building 참조).
        단 Task 4의 종료 코드 판독은 이 체인을 그대로 통과함이 증거 4로 확인됨.
```

---

## Patterns to Mirror

### NAMING_CONVENTION — 고정 인자는 상수/단일 표현으로

```rust
// SOURCE: src-tauri/src/collectors/wsl.rs:14-19
pub const CLAUDE_CREDENTIAL_SCRIPT: &str = "if [ -n \"${CLAUDE_CONFIG_DIR:-}\" ] …";
pub const CODEX_PROBE_SCRIPT: &str = "bash -lc 'type -P codex >/dev/null 2>&1'";
pub const CODEX_LAUNCH_SCRIPT: &str = "exec setsid --wait bash -lc 'printf \"\\nCACHEBITE_PGID:%s\\n\" \"$$\"; exec codex -s read-only -a untrusted app-server'";
```

`UPPER_SNAKE_CASE` const, `&'static str`, 이스케이프된 셸 원문 그대로.

### ERROR_HANDLING — 타입화된 provider-scoped 오류, 원본 오류 전파 금지

```rust
// SOURCE: src-tauri/src/collectors/mod.rs:12-34
#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
pub enum CollectorError {
    #[error("credentials are missing")]
    CredentialsMissing,
    …
    #[error("provider CLI is unavailable")]
    CliMissing,
```

```rust
// SOURCE: src-tauri/src/collectors/mod.rs:36-57
impl CollectorError {
    pub fn into_outcome(self) -> CollectionOutcome {
        match self {
            Self::CredentialsMissing => CollectionOutcome::CredentialsMissing,
            Self::CliMissing => CollectionOutcome::CliMissing,
            Self::Network | Self::Timeout => CollectionOutcome::Failed { class: FailureClass::Network },
            …
```

새 variant는 `#[error("…")]` 문구를 반드시 붙이고 `into_outcome`의 match arm을 추가한다(비망라 match는 컴파일 에러로 잡힌다).

### LOGGING_PATTERN — 디버그 빌드에서만, 바이트 수만

```rust
// SOURCE: src-tauri/src/collectors/codex.rs:562-566
#[cfg(debug_assertions)]
eprintln!(
    "[CacheBite:codex] rpc line received ({} bytes)",
    bytes.len()
);
```

`[CacheBite:<module>]` 접두어. **내용은 절대 찍지 않고 길이만.** 릴리스 빌드에는 컴파일되지 않는다. 이 관례가 privacy contract를 지탱하므로 새 로그도 동일하게.

### PURE_POLICY_FUNCTION — 분류 로직은 순수 함수로 분리해 테스트

```rust
// SOURCE: src-tauri/src/collectors/codex.rs:341-347
pub(crate) fn classify_spawn_error(kind: std::io::ErrorKind) -> CollectorError {
    if kind == std::io::ErrorKind::NotFound {
        CollectorError::CliMissing
    } else {
        CollectorError::Internal
    }
}
```

Task 4의 `classify_launch_exit`는 이 형태를 그대로 따른다: 부작용 없음, `pub(crate)`, 입력은 원시 값.

### 프로세스 수명 관리 — RPC 결과 확정 → 종료 코드 판독 → terminate 순서

```rust
// SOURCE: src-tauri/src/collectors/codex.rs:196-212
let mut child = spawn_managed_app_server(command)?;
let mut stdin = child.stdin().ok_or(CollectorError::Internal)?;
let stdout = child.stdout().ok_or(CollectorError::Internal)?;
let result = RpcSession::new(timeout, MAX_RESPONSE_BYTES)
    .exchange(stdout, &mut stdin)
    .await;
drop(stdin);
let missing_cli = exit_127_is_cli_missing
    && result.is_err()
    && child.exit_code_with_grace().await == Some(127);
child.terminate().await;
if missing_cli {
    return Err(CollectorError::CliMissing);
}
let limits = result?;
normalize(limits, now)
```

**순서가 곧 정확성이다**: `terminate()` 전에 `exit_code_with_grace()`를 호출해야 "스스로 죽은 코드"와 "우리가 죽인 코드"가 구분된다.

### TEST_STRUCTURE (Rust) — 고정 인자 assertion

```rust
// SOURCE: src-tauri/src/collectors/tests.rs:73-89
#[test]
fn wsl_codex_arguments_are_fixed() {
    assert_eq!(
        CODEX_PROBE_SCRIPT,
        "bash -lc 'type -P codex >/dev/null 2>&1'"
    );
    assert!(CODEX_LAUNCH_SCRIPT.contains("exec codex -s read-only -a untrusted app-server"));
    assert!(CODEX_LAUNCH_SCRIPT.contains("setsid --wait bash -lc"));
    …
}
```

### TEST_STRUCTURE (Rust async RPC) — 인메모리 바이트 슬라이스로 exchange 구동

```rust
// SOURCE: src-tauri/src/collectors/tests.rs:1245-1258
#[tokio::test]
async fn rpc_accepts_codex_responses_that_omit_jsonrpc_version() {
    // Codex CLI 0.144.5 app-server sends valid id/result envelopes without
    // echoing the optional JSON-RPC version field.
    let replies = concat!(
        "{\"id\":1,\"result\":{\"userAgent\":\"cachebite/0.144.5\"}}\n",
        "{\"id\":2,\"result\":{\"primary\":{\"usedPercent\":6}}}\n"
    );
    let result = RpcSession::new(Duration::from_secs(1), 4096)
        .exchange(replies.as_bytes(), &mut Vec::new())
        .await
        .unwrap();

    assert_eq!(result.primary.unwrap().used_percent(), Some(6.0));
}
```

버전 특정 동작에는 **어느 codex 버전에서 관찰했는지 주석으로 남긴다.** 신규 테스트도 `0.149.0`을 명시할 것.

### TEST_STRUCTURE (TS) — AAA + 서술형 이름

```typescript
// SOURCE: src/lib/state/domain.test.ts (형태), 프로젝트 전반 관례
test('returns the CLI-incompatible guidance when the failure class says so', () => {
  // Arrange / Act / Assert
});
```

### RENDERER_COPY — 문구는 Record 상수로 provider별 분기

```typescript
// SOURCE: src/lib/components/systemGuidance.ts:4-11
const CLI_NAME: Record<Provider, string> = {
  claude: 'Claude',
  codex: 'Codex',
};
const SIGN_IN_COMMAND: Record<Provider, string> = {
  claude: 'claude login',
  codex: 'codex login',
};
```

---

## Files to Change

### Phase 1 — 인자 교정 (필수, 단독 배포 가능)

| File | Action | Justification |
| --- | --- | --- |
| `src-tauri/src/collectors/codex.rs` | UPDATE | L178 `-a untrusted` → `-a never` + 사유 주석 |
| `src-tauri/src/collectors/wsl.rs` | UPDATE | L16, L18 런처 스크립트 2종 동일 교정 |
| `src-tauri/src/collectors/tests.rs` | UPDATE | L79, L275 고정 인자 assertion 갱신 + 네이티브 argv assertion 신규 |
| `docs/architecture.md` | UPDATE | L182 문서화된 명령줄 갱신 |
| `CLAUDE.md` | UPDATE | L58 collectors 설명의 명령줄 갱신 |
| `.claude/PRPs/plans/codex-cli-approval-policy-compat.plan.md` | CREATE | 본 문서 |

### Phase 2 — CLI 거부 가시화 (권장)

| File | Action | Justification |
| --- | --- | --- |
| `src-tauri/src/collectors/mod.rs` | UPDATE | `CollectorError::CliIncompatible` + `into_outcome` arm |
| `src-tauri/src/domain.rs` | UPDATE | `FailureClass::CliIncompatible` |
| `src-tauri/src/collectors/codex.rs` | UPDATE | `classify_launch_exit` 신설, 두 수집 경로에 배선, dead `exit_127_is_cli_missing` 제거 |
| `src-tauri/src/collectors/tests.rs` | UPDATE | 분류 함수 단위 테스트 + 종료 코드 통합 테스트 |
| `src/lib/contracts/domain.ts` | UPDATE | `FailureClass` 유니온에 `'cli_incompatible'` 추가 |
| `src/lib/components/panelModels.ts` | UPDATE | `PanelProviderModel.failureClass` 필드 추가 |
| `src/lib/state/presentation.ts` | UPDATE | `toProviderPresentation`에서 `state.lastFailure` 전달 |
| `src/lib/components/systemGuidance.ts` | UPDATE | 3번째 인자 `failureClass`로 `error` 문구 특수화 |
| `src/lib/components/ProviderColumn.svelte` | UPDATE | `systemGuidance` 호출에 `model.failureClass` 전달 |
| `docs/ui-contract.md` | UPDATE | §4.2 표에 `error` 상태의 두 번째 문구 변형 명시 |

### Phase 3 — 상류 변경 조기 감지 (선택)

| File | Action | Justification |
| --- | --- | --- |
| `scripts/check-codex-cli-compat.sh` | CREATE | 자격증명 불필요 인자 수용성 체크 |
| `.github/workflows/codex-cli-compat.yml` | CREATE | 주간 스케줄 잡 — 상류 CLI 변경 시 이슈로 조기 경보 |

## NOT Building

- **CLI 인자 자동 폴백/재시도** (`-a never` 실패 시 `-a` 없이 재시도 등). 상류 변경을 조용히 덮어 문제를 다시 비가시화한다. Phase 2의 명시적 오류 표면이 올바른 해법이다.
- **Codex CLI 버전 파싱/버전 게이트** (`codex --version` 후 분기). 스폰 1회 추가 + 버전-동작 매핑 유지보수 부담. 종료 코드 판독이 더 싸고 정확하다.
- **stderr 캡처 후 UI/로그 노출.** privacy contract(렌더러 DTO·로그에서 자격증명 경로 제외)를 위협한다. 종료 코드(정수)만으로 충분하다.
- **Windows 3단 프로세스 체인(cmd → node → codex.exe) 고아 프로세스 정리.** 실재하는 선재 리스크지만 이번 버그와 무관. 별도 이슈로 분리할 것.
- **`-a` 옵션 완전 제거.** `-s read-only`만 남기면 approval policy가 사용자 `config.toml`에서 상속된다. 명시적 `never`가 더 강한 보증이다.
- **`-s read-only` 변경.** 0.149.0에서 여전히 유효하며 sandbox 방어선의 핵심이다.
- **`planType: "team"` 대응 코드.** 렌더러가 불투명 문자열로 통과시키므로 변경 불필요.
- **새 `SystemState` 또는 새 배지 아이콘 추가.** `error` 배지를 재사용하고 문구만 특수화한다.

---

## Step-by-Step Tasks

### Task 1: 네이티브 수집 경로의 approval policy 값 교정

- **ACTION**: `src-tauri/src/collectors/codex.rs:178`의 `"untrusted"`를 `"never"`로 바꾸고, 값 선택 근거를 주석으로 남긴다.
- **IMPLEMENT**:
  ```rust
  pub async fn collect_app_server(
      executable: &Path,
      now: OffsetDateTime,
  ) -> Result<ProviderUsageSnapshot, CollectorError> {
      validate_executable(executable)?;
      let mut command = Command::new(executable);
      // `never` is the most restrictive approval policy codex-cli still accepts:
      // 0.149.0 dropped `untrusted` and `on-failure`, leaving only `on-request`
      // and `never`. This collector never opens a conversation, so the policy is
      // inert in practice — but `never` is the one value that cannot escalate out
      // of `-s read-only` by prompting a human who is not there. Do not relax it
      // to `on-request`, and do not drop `-a`: without it the policy is inherited
      // from the user's config.toml.
      command
          .args(["-s", "read-only", "-a", "never"])
          .arg("app-server");
      collect_app_server_child(command, now).await
  }
  ```
- **MIRROR**: ERROR_HANDLING / 기존 함수 구조 그대로 유지 (시그니처·반환형 무변경)
- **IMPORTS**: 없음
- **GOTCHA**: `-s read-only`는 **그대로 둘 것.** 0.149.0에서 여전히 유효한 값이며 sandbox 방어선이다. 또한 `-s`/`-a`는 루트 옵션이므로 반드시 `app-server` 하위 명령 **앞**에 와야 한다 — 현재 `.args(...)` 후 `.arg("app-server")` 순서가 이미 올바르므로 순서를 바꾸지 말 것.
- **VALIDATE**: `cargo test --manifest-path src-tauri/Cargo.toml --all-features` — Task 3 갱신 전까지는 `wsl_codex_arguments_are_fixed`가 실패하는 것이 정상.

### Task 2: WSL 런처 스크립트 2종 교정

- **ACTION**: `src-tauri/src/collectors/wsl.rs:16`과 `:18`의 `-a untrusted`를 `-a never`로 바꾼다.
- **IMPLEMENT**:
  ```rust
  // See collectors::codex::collect_app_server for why the policy is `never`.
  pub const CODEX_LAUNCH_SCRIPT: &str = "exec setsid --wait bash -lc 'printf \"\\nCACHEBITE_PGID:%s\\n\" \"$$\"; exec codex -s read-only -a never app-server'";
  const CODEX_INTERACTIVE_PROBE_SCRIPT: &str = "bash -ic 'type -P codex >/dev/null 2>&1'";
  const CODEX_INTERACTIVE_LAUNCH_SCRIPT: &str = "exec setsid --wait bash -ic 'printf \"\\nCACHEBITE_PGID:%s\\n\" \"$$\"; exec codex -s read-only -a never app-server'";
  ```
- **MIRROR**: NAMING_CONVENTION (기존 const 형태 유지)
- **IMPORTS**: 없음
- **GOTCHA**: **두 상수 모두** 고쳐야 한다. `Login`(`-lc`)과 `Interactive`(`-ic`)는 `wsl.rs:137-138`에서 분기 선택되며, 하나만 고치면 NVM 등 인터랙티브 PATH 환경의 사용자에게 버그가 그대로 남는다. `printf`의 `\\n` 이스케이프 수준을 건드리지 말 것 — PGID 핸드셰이크가 깨진다.
- **VALIDATE**: `cargo test --manifest-path src-tauri/Cargo.toml collectors::tests::wsl_codex_arguments_are_fixed` (Task 3 이후 통과)

### Task 3: 고정 인자 회귀 테스트 갱신 및 신규 argv 잠금

- **ACTION**: `tests.rs:79`, `tests.rs:275`의 기대 문자열을 갱신하고, 네이티브 경로의 argv를 명시적으로 잠그는 테스트를 추가한다.
- **IMPLEMENT**:
  1. `tests.rs:79` → `assert!(CODEX_LAUNCH_SCRIPT.contains("exec codex -s read-only -a never app-server"));`
  2. 같은 테스트에 인터랙티브 변형 커버리지를 추가한다(현재 누락 — 이번 버그가 반쪽만 잡힐 수 있었던 이유):
     ```rust
     // codex-cli 0.149.0 dropped `untrusted` from --ask-for-approval; both the
     // login-shell and interactive-shell launchers must carry the same policy.
     assert!(CODEX_LAUNCH_SCRIPT.contains("-a never"));
     assert!(!CODEX_LAUNCH_SCRIPT.contains("untrusted"));
     ```
  3. `tests.rs:275`의 기대 문자열에서 `-a untrusted` → `-a never`.
  4. 신규 테스트 (네이티브 argv는 현재 어떤 테스트도 잠그지 않는다):
     ```rust
     #[test]
     fn native_codex_argv_is_fixed() {
         // codex-cli 0.149.0 accepts only `on-request` and `never` for
         // --ask-for-approval; `-s`/`-a` are root options and must precede the
         // `app-server` subcommand.
         assert_eq!(
             app_server_argv(),
             ["-s", "read-only", "-a", "never", "app-server"]
         );
     }
     ```
     이를 위해 `codex.rs`에 순수 함수를 추출한다:
     ```rust
     /// The fixed argument list handed to the Codex CLI. Extracted so a test can
     /// lock it: the values are a CLI-surface contract, not an implementation
     /// detail (codex-cli 0.149.0 removed `untrusted` and broke collection).
     pub(crate) fn app_server_argv() -> [&'static str; 5] {
         ["-s", "read-only", "-a", "never", "app-server"]
     }
     ```
     그리고 `collect_app_server`는 `command.args(app_server_argv());`로 단순화한다.
- **MIRROR**: TEST_STRUCTURE (Rust) — 버전 명시 주석 관례
- **IMPORTS**: `tests.rs` 상단 `use` 목록에 `app_server_argv` 추가 (`tests.rs:8` 인근의 기존 `use super::codex::{…}` 블록에 합류)
- **GOTCHA**: `app_server_argv()`를 도입하면 Task 1의 주석은 이 함수 위로 옮긴다(중복 금지). `pub(crate)` 가시성을 넘기지 말 것 — 렌더러/외부에 노출할 이유가 없다.
- **VALIDATE**: `cargo test --manifest-path src-tauri/Cargo.toml --all-features` 전부 통과

### Task 4: `classify_launch_exit` 도입 — CLI 거부를 파싱 실패와 분리 *(Phase 2)*

- **ACTION**: 종료 코드 기반 분류 순수 함수를 신설하고, 두 수집 경로에 배선한다. 죽어 있던 `exit_127_is_cli_missing` 파라미터를 제거한다.
- **IMPLEMENT**:
  ```rust
  // in mod.rs — CollectorError
  #[error("provider CLI rejected the invocation")]
  CliIncompatible,
  ```
  ```rust
  // in mod.rs — into_outcome
  Self::CliIncompatible => CollectionOutcome::Failed {
      class: FailureClass::CliIncompatible,
  },
  ```
  ```rust
  // in domain.rs — FailureClass
  CliIncompatible,   // serde(rename_all = "snake_case") → "cli_incompatible"
  ```
  ```rust
  // in codex.rs
  /// A CLI that exited on its own without ever speaking the protocol rejected the
  /// invocation itself. 127 is a shell's "command not found"; any other nonzero
  /// code means this Codex build no longer accepts our fixed argument list (clap
  /// reports a usage error as 2 — that is how codex-cli 0.149.0 dropping
  /// `--ask-for-approval untrusted` surfaced). `None` means the child was still
  /// running, so the failure was genuinely mid-protocol.
  pub(crate) fn classify_launch_exit(code: Option<i32>) -> Option<CollectorError> {
      match code {
          Some(127) => Some(CollectorError::CliMissing),
          Some(code) if code != 0 => Some(CollectorError::CliIncompatible),
          _ => None,
      }
  }
  ```
  `collect_app_server_child_with_options`를 다음으로 교체(파라미터 4개 → 3개):
  ```rust
  pub(crate) async fn collect_app_server_child_with_options(
      command: Command,
      now: OffsetDateTime,
      timeout: Duration,
  ) -> Result<ProviderUsageSnapshot, CollectorError> {
      let mut child = spawn_managed_app_server(command)?;
      let mut stdin = child.stdin().ok_or(CollectorError::Internal)?;
      let stdout = child.stdout().ok_or(CollectorError::Internal)?;
      let result = RpcSession::new(timeout, MAX_RESPONSE_BYTES)
          .exchange(stdout, &mut stdin)
          .await;
      drop(stdin);
      let launch_failure = if result.is_err() {
          classify_launch_exit(child.exit_code_with_grace().await)
      } else {
          None
      };
      child.terminate().await;
      if let Some(error) = launch_failure {
          return Err(error);
      }
      normalize(result?, now)
  }
  ```
  `collect_app_server_child_with_pgid`에도 동일 블록을 추가한다(`cleanup(pgid).await` 호출 **전에** `exit_code_with_grace()`를 읽을 것).
- **MIRROR**: PURE_POLICY_FUNCTION (`classify_spawn_error`), 그리고 "RPC 결과 확정 → 종료 코드 판독 → terminate" 순서 패턴
- **IMPORTS**: 없음 (모두 기존 모듈 내)
- **GOTCHA**:
  1. **호출 순서가 정확성의 전부다.** `child.terminate()` **이전에** `exit_code_with_grace()`를 호출해야 한다. 순서를 뒤집으면 우리가 kill한 코드를 CLI 거부로 오분류한다.
  2. `exit_code_with_grace()`는 100ms grace 후 `None`을 반환한다 — 프로토콜 중간에 깨졌지만 자식이 살아 있는 정상 케이스가 여기로 떨어져 `None` → 기존 오류가 그대로 보존된다. 이것이 오탐을 막는 핵심이므로 grace 값을 늘리지 말 것.
  3. `result.is_err()`로 넓게 잡되 `Ok`일 때는 절대 분류하지 않는다. 성공 후 자식이 비정상 종료해도 스냅샷은 유효하다.
  4. **미로그인 상태는 이 경로로 오지 않는다** — 증거 3에서 확인했듯 `initialize`가 성공하고 `account/rateLimits/read`가 오류 객체를 반환하므로 `validate_response`(`codex.rs:571-593`)가 `CredentialsMissing`으로 먼저 잡는다. 즉 종료 코드 분류가 인증 오류를 가로챌 위험은 없다.
  5. `result`를 소비하지 않도록 `is_err()`로 검사한 뒤 마지막에 `result?`를 쓴다 (위 예시대로). `match result { … }`로 이동시키면 뒤에서 재사용할 수 없다.
- **VALIDATE**:
  ```bash
  cargo test --manifest-path src-tauri/Cargo.toml collectors::tests::classify_launch_exit
  cargo clippy --manifest-path src-tauri/Cargo.toml --all-features -- -D warnings
  ```

### Task 5: 분류 로직 테스트 *(Phase 2)*

- **ACTION**: 순수 함수 단위 테스트 + 즉시 종료하는 가짜 CLI 통합 테스트를 추가한다.
- **IMPLEMENT**:
  ```rust
  #[test]
  fn classify_launch_exit_separates_a_missing_cli_from_a_rejected_invocation() {
      // Arrange / Act / Assert
      assert_eq!(classify_launch_exit(Some(127)), Some(CollectorError::CliMissing));
      // clap reports a usage error as 2 — codex-cli 0.149.0 dropped
      // `--ask-for-approval untrusted` and exited this way.
      assert_eq!(classify_launch_exit(Some(2)), Some(CollectorError::CliIncompatible));
      assert_eq!(classify_launch_exit(Some(1)), Some(CollectorError::CliIncompatible));
      assert_eq!(classify_launch_exit(Some(0)), None);
      // Still running: the failure was genuinely mid-protocol.
      assert_eq!(classify_launch_exit(None), None);
  }

  #[tokio::test]
  async fn a_cli_that_rejects_our_arguments_reports_cli_incompatible() {
      // Arrange: a fake CLI that writes a usage error to stderr and exits 2
      // without ever producing a protocol line — exactly codex-cli 0.149.0's
      // behaviour for `-a untrusted`.
      …
      // Assert
      assert!(matches!(outcome, CollectionOutcome::Failed { class: FailureClass::CliIncompatible }));
  }
  ```
- **MIRROR**: TEST_STRUCTURE (Rust) + `tests.rs:1370-1400`의 가짜 app-server fixture 패턴을 재사용
- **IMPORTS**: `tests.rs` 상단에 `classify_launch_exit`, `FailureClass` 추가
- **GOTCHA**: 통합 테스트의 가짜 CLI는 **stdout에 아무것도 쓰지 않고** 종료해야 한다. 한 줄이라도 쓰면 `Protocol`이 아닌 다른 경로로 빠질 수 있다. 크로스플랫폼을 위해 셸 스크립트 대신 기존 fixture 헬퍼를 따를 것.
- **VALIDATE**: `cargo test --manifest-path src-tauri/Cargo.toml --all-features`

### Task 6: 렌더러 계약에 `cli_incompatible` 전달 *(Phase 2)*

- **ACTION**: TS `FailureClass` 유니온 확장 → 패널 모델에 `failureClass` 노출 → `systemGuidance` 문구 특수화 → 컴포넌트 배선.
- **IMPLEMENT**:
  1. `src/lib/contracts/domain.ts:31`:
     ```typescript
     export type FailureClass =
       | 'network'
       | 'provider'
       | 'parse'
       | 'internal'
       | 'cli_incompatible';
     ```
  2. `src/lib/components/panelModels.ts` — `PanelProviderModel`에 추가:
     ```typescript
     readonly failureClass: FailureClass | null;
     ```
     (`import type { FailureClass, Provider } from '../contracts/domain';`로 import 확장)
  3. `src/lib/state/presentation.ts` — `toProviderPresentation` 반환 객체에 추가:
     ```typescript
     failureClass: state.lastFailure,
     ```
  4. `src/lib/components/systemGuidance.ts`:
     ```typescript
     export function systemGuidance(
       system: SystemState,
       provider: Provider,
       failureClass: FailureClass | null = null,
     ): string | null {
       switch (system) {
         case 'auth_required':
           return `Sign in to the ${CLI_NAME[provider]} CLI: ${SIGN_IN_COMMAND[provider]}`;
         case 'unavailable':
           return `The ${CLI_NAME[provider]} CLI is not installed`;
         case 'error':
           // A CLI that rejected our invocation will never recover on retry, so
           // the generic "retrying shortly" line would be a lie.
           return failureClass === 'cli_incompatible'
             ? `The ${CLI_NAME[provider]} CLI rejected this CacheBite build. Update CacheBite.`
             : 'Could not fetch usage. Retrying shortly.';
         case 'offline':
           return 'Cannot reach the network';
         default:
           return null;
       }
     }
     ```
  5. `src/lib/components/ProviderColumn.svelte:19`:
     ```javascript
     const guidance = $derived(
       systemGuidance(model.system, model.provider, model.failureClass),
     );
     ```
- **MIRROR**: RENDERER_COPY (`CLI_NAME` Record 재사용), 기존 `systemGuidance` switch 구조
- **IMPORTS**: `panelModels.ts`와 `systemGuidance.ts`에 `FailureClass` 타입 import 추가
- **GOTCHA**:
  1. `failureClass`에 **기본값 `null`을 주어야** 기존 테스트/호출부가 깨지지 않는다.
  2. **`SystemState`에 새 값을 추가하지 말 것.** 새 상태를 만들면 `SystemBadge.svelte:6-18`의 `badges` Record와 아이콘 SVG 분기, `models.ts`의 `BadgeState`, ui-contract §4.2 표가 전부 따라와야 한다. `error` 배지 재사용이 의도된 설계다.
  3. `PanelProviderModel`에 필드를 추가하면 `panelModels.test.ts:18`, `ProviderColumn.test.ts:15`, `UsagePanel.test.ts:18`의 픽스처 객체가 타입 에러를 낸다 — 세 곳 모두 `failureClass: null` 추가 필요.
  4. `fixtureGateway.ts`는 wire DTO(`failure_class`)를 다루므로 이미 `FailureClass | null`을 통과시킨다. 신규 필드 불필요.
- **VALIDATE**:
  ```bash
  pnpm check        # svelte-check: 위 3번의 픽스처 누락을 여기서 잡는다
  pnpm test
  ```

### Task 7: 문서 동기화

- **ACTION**: 명령줄이 문서화된 3곳과 UI 계약 표를 갱신한다.
- **IMPLEMENT**:
  1. `docs/architecture.md:182`:
     ```
     1. Start `codex -s read-only -a never app-server` as a hidden child process.
     ```
     같은 절 끝에 한 줄 추가:
     ```
     The approval policy is `never` because codex-cli 0.149.0 removed `untrusted`;
     `-s read-only` remains the sandbox boundary. A CLI that rejects this fixed
     argument list exits nonzero without speaking the protocol and is reported as
     `cli_incompatible`, not as a parse failure.
     ```
  2. `CLAUDE.md:58` — `codex -s read-only -a untrusted app-server` → `codex -s read-only -a never app-server`
  3. `docs/ui-contract.md` §4.2 표의 `error` 행을 두 변형으로 분리:
     ```
     | `error` (일반) | 경고 삼각형 | "사용량을 가져오지 못했습니다. 잠시 후 재시도합니다" |
     | `error` (`cli_incompatible`) | 경고 삼각형 | "Codex CLI가 이 CacheBite 빌드를 거부했습니다. CacheBite를 업데이트하세요" |
     ```
- **MIRROR**: 기존 문서의 언어 관례(ui-contract는 한국어, architecture.md는 영어)
- **IMPORTS**: N/A
- **GOTCHA**: `docs/superpowers/plans/*`와 `.claude/PRPs/plans/*`의 과거 계획 문서에도 `untrusted` 문자열이 남아 있으나 **역사 기록이므로 건드리지 말 것**(`wsl-codex-nvm-discovery.plan.md:110`, `2026-07-17-wsl-collector-bridge.md:71`). 또한 `in-app-updates-*.plan.md:1170`과 `src/nativeWorkflow.test.ts:305`의 `untrusted comment:`는 minisign 서명 포맷으로 **완전히 무관**하다.
- **VALIDATE**: `grep -rn "a untrusted" src-tauri/src src docs/architecture.md CLAUDE.md` → 0건

### Task 8: 자격증명 불필요 호환성 체크 *(Phase 3, 선택)*

- **ACTION**: 인자 수용성만 검사하는 스크립트와 주간 CI 잡을 추가한다.
- **IMPLEMENT**:
  ```bash
  #!/usr/bin/env bash
  # scripts/check-codex-cli-compat.sh
  # Verifies that the installed Codex CLI still accepts CacheBite's fixed argument
  # list. `--help` makes this credential-free: clap validates the root options
  # before printing, so a removed enum value fails here exactly as it would at
  # collection time. This is what would have caught codex-cli 0.149.0 dropping
  # `--ask-for-approval untrusted` before users did.
  set -euo pipefail
  if ! codex -s read-only -a never app-server --help >/dev/null 2>&1; then
    echo "Codex CLI rejected CacheBite's fixed arguments." >&2
    codex -s read-only -a never app-server --help 2>&1 | head -20 >&2
    echo "Installed: $(codex --version 2>&1 || echo unknown)" >&2
    exit 1
  fi
  echo "Codex CLI argument surface OK: $(codex --version)"
  ```
  워크플로는 `schedule: cron` + `workflow_dispatch`로 `npm i -g @openai/codex@latest` 후 위 스크립트를 실행하고, 실패 시 이슈를 연다.
- **MIRROR**: 기존 `.github/workflows/*.yml`의 액션 SHA 전체 고정 관례(CLAUDE.md CI 절)를 반드시 따를 것
- **IMPORTS**: N/A
- **GOTCHA**:
  1. `--help`는 **하위 명령까지 붙여야** 루트 옵션 검증이 실제로 일어난다. `codex --help`만으로는 `-a` 값 검증이 트리거되지 않는다.
  2. 이 잡은 **자격증명이 없어도 통과**해야 한다 — 로그인 상태를 요구하는 명령을 넣지 말 것. `native-smoke.yml`의 credential-free 원칙과 동일하다.
  3. 새 액션은 반드시 full commit SHA로 고정 (Dependabot이 추적).
- **VALIDATE**: 로컬에서 `bash scripts/check-codex-cli-compat.sh` → 0 종료. 의도적으로 `never`를 `untrusted`로 바꿔 실행 → 1 종료 확인.

---

## Testing Strategy

### Unit Tests

| Test | Input | Expected Output | Edge Case? |
| --- | --- | --- | --- |
| `native_codex_argv_is_fixed` | — | `["-s","read-only","-a","never","app-server"]` | No |
| `wsl_codex_arguments_are_fixed` | `CODEX_LAUNCH_SCRIPT` | `-a never` 포함, `untrusted` 미포함 | No |
| `wsl_codex_arguments_are_fixed` (인터랙티브) | `CODEX_INTERACTIVE_LAUNCH_SCRIPT` | `-a never` 포함 | Yes — 기존 커버리지 공백 |
| `classify_launch_exit_…` | `Some(127)` | `Some(CliMissing)` | No |
| `classify_launch_exit_…` | `Some(2)` | `Some(CliIncompatible)` | Yes — clap usage error |
| `classify_launch_exit_…` | `Some(0)` | `None` | Yes — 정상 종료 |
| `classify_launch_exit_…` | `None` | `None` | Yes — 아직 실행 중 (오탐 방지) |
| `a_cli_that_rejects_our_arguments_…` | 즉시 exit 2 하는 가짜 CLI | `Failed{CliIncompatible}` | Yes |
| `systemGuidance` | `('error','codex','cli_incompatible')` | "…rejected this CacheBite build…" | Yes |
| `systemGuidance` | `('error','codex','parse')` | "Could not fetch usage. Retrying shortly." | No |
| `systemGuidance` | `('error','codex')` (3번째 인자 생략) | 기존 문구 (기본값 동작) | Yes |
| 기존 `rpc_*` 테스트 | 변경 없음 | 전부 통과 | 회귀 감시 |

### Edge Cases Checklist

- [ ] CLI 미설치 → 여전히 `CliMissing` ("not installed" 문구), `CliIncompatible`로 오분류되지 않음
- [ ] 미로그인 → 여전히 `CredentialsMissing` (`auth_required` 배지). 종료 코드 분류가 가로채지 않음 — 증거 3 참조
- [ ] 프로토콜 중간 절단 (자식은 생존) → `exit_code_with_grace()`가 `None` → 기존 `Protocol`/`Parse` 보존
- [ ] 타임아웃 → `Timeout` → `Network` 클래스 유지 (분류 진입 안 함)
- [ ] 수집 성공 직후 자식 비정상 종료 → 스냅샷 유지 (`Ok` 분기는 분류하지 않음)
- [ ] WSL 경로: `-lc`(로그인 셸)와 `-ic`(인터랙티브 셸) **양쪽** 모두 교정
- [ ] `PanelProviderModel` 필드 추가로 인한 테스트 픽스처 3곳 갱신
- [ ] 응답 크기 초과 → `ResponseTooLarge` 유지

---

## Validation Commands

### Static Analysis

```bash
pnpm check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-features -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

EXPECT: 타입 에러 0, clippy 경고 0, 포맷 차이 0

### Unit Tests

```bash
cargo test --manifest-path src-tauri/Cargo.toml collectors::tests
pnpm vitest run src/lib/components/systemGuidance.test.ts
pnpm vitest run src/lib/state/presentation.test.ts
```

EXPECT: 전부 통과

### Full Test Suite

```bash
cargo test --manifest-path src-tauri/Cargo.toml --all-features
pnpm test:ci
```

EXPECT: 회귀 없음, 커버리지 게이트(branches/functions/lines/statements 각 80%) 통과

### 실제 CLI 검증 (이 계획의 핵심 — 반드시 수행)

```bash
codex --version
codex -s read-only -a never app-server --help >/dev/null && echo "args accepted"
```

EXPECT: `args accepted` 출력. 실패하면 상류 CLI가 또 바뀐 것이므로 Task 1의 값을 재조사할 것.

### App Validation

```bash
pnpm tauri dev
```

EXPECT: Codex 열에 실제 사용률 표시. 배지 없음. `planType` chip 표시(계정에 따라 `team`/`pro` 등).

### Manual Validation

- [ ] `pnpm tauri dev` 실행 → 패널 열기 → Codex 열이 `error` 배지 없이 사용률을 보여준다
- [ ] Codex 열의 5시간/주간 게이지가 `codex -s read-only -a never app-server`의 `account/rateLimits/read` 응답과 일치한다
- [ ] `CACHEBITE_CODEX_PATH`를 존재하지 않는 경로로 설정 → "The Codex CLI is not installed" (변경 없음 확인)
- [ ] *(Phase 2)* `codex.rs`의 `"never"`를 일시적으로 `"untrusted"`로 되돌려 실행 → "The Codex CLI rejected this CacheBite build. Update CacheBite." 표시 확인 → **원복**
- [ ] *(Phase 2)* 위 상태에서 Codex 열 배지가 여전히 경고 삼각형(신규 아이콘 아님)인지 확인
- [ ] Claude 열은 전 과정에서 영향 없음 (provider별 독립 refresh actor 불변식)
- [ ] 수집 후 orphan `codex.exe` / `node.exe`가 남지 않는지 작업 관리자 확인

---

## Acceptance Criteria

- [ ] `grep -rn "a untrusted" src-tauri/src src docs/architecture.md CLAUDE.md` → 0건
- [ ] 네이티브 경로와 WSL 경로(`-lc`, `-ic`) 3곳 모두 `-a never`
- [ ] 실제 Codex CLI 0.149.0에서 사용량이 패널에 표시된다
- [ ] *(Phase 2)* CLI 인자 거부가 `cli_incompatible`로 분류되고 전용 문구가 뜬다
- [ ] *(Phase 2)* 새 `SystemState`/배지 아이콘이 추가되지 않았다
- [ ] 모든 validation 명령 통과, 커버리지 80% 유지
- [ ] 타입 에러 0, lint 에러 0

## Completion Checklist

- [ ] 코드가 발견된 패턴을 따른다 (`classify_spawn_error` 형태의 순수 분류 함수)
- [ ] 오류 처리가 `CollectorError` → `into_outcome` 관례를 따른다
- [ ] 로그가 `#[cfg(debug_assertions)]` + 길이만 기록 관례를 따른다
- [ ] 테스트가 버전 명시 주석 관례(`codex-cli 0.149.0 …`)를 따른다
- [ ] 하드코딩된 값이 `app_server_argv()` 단일 원본으로 모였다
- [ ] `docs/architecture.md`, `docs/ui-contract.md`, `CLAUDE.md` 동기화 완료
- [ ] privacy contract 유지: stderr 미노출, 종료 코드(정수)만 사용
- [ ] 불필요한 범위 확장 없음 (NOT Building 준수)

## Risks

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| 상류가 `never`마저 제거/개명 | Low | High | Phase 2의 `cli_incompatible` 표면 + Phase 3 주간 체크로 조기 감지 |
| 종료 코드 오분류 (우리가 kill한 자식을 CLI 거부로 판정) | Medium | Medium | `terminate()` **이전** 판독 + 100ms grace 후 `None` 반환 (Task 4 GOTCHA 1·2). 전용 단위 테스트 |
| WSL 인터랙티브 스크립트 누락 | Medium | Medium | Task 3에서 인터랙티브 변형 assertion 신규 추가 (현재 커버리지 공백) |
| `PanelProviderModel` 필드 추가로 픽스처 3곳 타입 에러 | High | Low | `pnpm check`가 즉시 검출. Task 6 GOTCHA 3에 파일 명시 |
| Windows 3단 프로세스 체인의 고아 프로세스 | Medium | Medium | 이번 범위 밖(NOT Building). 수동 검증 체크리스트에 관찰 항목만 포함, 별도 이슈로 분리 |
| 구버전 Codex CLI(≤0.14x) 사용자에게 `never` 미지원 | Very Low | Medium | `never`는 0.14x에도 존재하는 값이므로 하위 호환. 만약 문제가 되면 `cli_incompatible` 문구가 즉시 알린다 |
| 커버리지 게이트 하락 (신규 TS 분기) | Low | Low | Task 6의 `systemGuidance` 3케이스 테스트로 분기 전부 커버 |

## Notes

- **이것은 인증 문제가 아니다.** 사용자 보고("codex auth가 안 되는 것 같다")는 UI 문구가 원인을 감춘 결과다. Phase 2가 정확히 이 오진을 겨냥한다 — 즉 Phase 2는 "있으면 좋은 것"이 아니라 이번 인시던트가 실제로 증명한 결함의 수정이다.
- **Phase 1만 단독 배포 가능하다.** 급하면 Task 1~3 + Task 7만 먼저 머지해도 사용자 문제는 즉시 해소된다. Phase 2/3는 후속 PR로 분리 가능.
- `exit_127_is_cli_missing` 파라미터는 현재 호출부가 `false` 하나뿐이라 **완전한 dead code**다. Task 4가 이를 제거하고 일반화하므로 순 코드량 증가는 크지 않다.
- 0.149.0 `initialize` 응답의 `userAgent`가 `cachebite/0.149.0 … (cachebite; 0.1.0)`로 나온다 — 우리가 보낸 `clientInfo`가 정상 반영된다는 증거이며, `clientInfo` 요구사항이 여전히 유효함을 확인해 준다.
- `remoteControl/status/changed` 알림은 0.149.0에서 새로 관찰되었으나 기존 `read_matching_response`의 알림 skip 로직이 이미 처리한다. 다만 알림 종류가 늘어날 경우 64회 루프 예산에 유의.
