# WSL Collector Bridge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the native Windows CacheBite app automatically collect from Claude and Codex installed and authenticated in the default WSL distribution when native Windows sources are missing.

**Architecture:** Add a Windows-only, fixed-command WSL process boundary and compose it behind the existing collectors with native-first fallback. Reuse the current Claude credential parser/HTTP collector and Codex JSON-RPC session so WSL changes transport and credential discovery without changing provider contracts.

**Tech Stack:** Rust, Tokio process I/O, Tauri 2, Windows `wsl.exe`, existing CacheBite collector/domain abstractions.

## Global Constraints

- Use only the Windows-configured default WSL distribution; do not add settings UI.
- Native collectors retain precedence and fallback occurs only for `CredentialsMissing` or `CliMissing`.
- No user-controlled shell text, token persistence, credential IPC, sensitive logging, visible console, unbounded output, or unreaped child process.
- Missing WSL maps to provider missing/auth required, not offline.
- Tests use injected/fake process boundaries and do not require WSL or live credentials.

---

### Task 1: Native-first fallback collector

**Files:**
- Create: `src-tauri/src/collectors/fallback.rs`
- Modify: `src-tauri/src/collectors/mod.rs`
- Test: `src-tauri/src/collectors/tests.rs`

**Interfaces:**
- Produces: `FallbackCollector::new(provider, primary, fallback, trigger: FallbackTrigger)` implementing `Collector`.
- `FallbackTrigger` is `CredentialsMissing` or `CliMissing` and matches exactly one `CollectionOutcome` variant.

- [ ] **Step 1: Write failing tests** proving native success is returned unchanged, the configured missing outcome invokes fallback, and all failure classes do not invoke fallback. Use counting fake collectors and assert call counts.
- [ ] **Step 2: Run** `cargo test collectors::tests::fallback --manifest-path src-tauri/Cargo.toml` and confirm missing imports/types fail.
- [ ] **Step 3: Implement** an immutable `FallbackCollector` that awaits the primary once and invokes fallback only when `FallbackTrigger::matches(&outcome)` is true.
- [ ] **Step 4: Re-run the targeted tests** and require all pass.

### Task 2: Bounded WSL process and Claude credential source

**Files:**
- Create: `src-tauri/src/collectors/wsl.rs`
- Modify: `src-tauri/src/collectors/mod.rs`
- Modify: `src-tauri/src/collectors/broker.rs`
- Modify: `src-tauri/src/collectors/claude.rs`
- Test: `src-tauri/src/collectors/tests.rs`

**Interfaces:**
- Produces: `WslCommandFactory::from_system_directory() -> Result<Self, CollectorError>`.
- Produces: `WslCredentialSource::new(factory)` and async `claude_token() -> Result<SecretString, CollectorError>`.
- Produces: a token-source abstraction used by `ClaudeCollector` so native and WSL sources share the existing HTTP request path.

- [ ] **Step 1: Write failing tests** for the exact fixed Claude script, missing/invalid/oversized credentials, bounded output, and secret parsing without path/token diagnostics.
- [ ] **Step 2: Run** `cargo test collectors::tests::wsl_claude --manifest-path src-tauri/Cargo.toml` and confirm the new interfaces are absent.
- [ ] **Step 3: Implement** a fixed `sh -c` script that checks `${CLAUDE_CONFIG_DIR}/.credentials.json` then `${HOME}/.claude/.credentials.json`, with no interpolated arguments. Resolve `%SystemRoot%\System32\wsl.exe`, apply `CREATE_NO_WINDOW`, cap output at 64 KiB, use a five-second timeout, and map command/file absence to `CredentialsMissing`.
- [ ] **Step 4: Extract/reuse** the existing credential JSON parser for byte input and introduce an async token-source interface so `ClaudeCollector` can use either native or WSL credentials without exposing the token.
- [ ] **Step 5: Run targeted tests** and require all pass.

### Task 3: WSL Codex app-server transport

**Files:**
- Modify: `src-tauri/src/collectors/wsl.rs`
- Modify: `src-tauri/src/collectors/codex.rs`
- Test: `src-tauri/src/collectors/tests.rs`

**Interfaces:**
- Produces: `WslCodexCollector::new(factory) -> Self` implementing `Collector`.
- Extracts: `collect_app_server_child(command: Command, now: OffsetDateTime)` so native and WSL transports share `RpcSession` and normalization.

- [ ] **Step 1: Write failing tests** for exact fixed WSL Codex arguments, missing Codex mapping, successful fake JSON-RPC, timeout, and child reaping.
- [ ] **Step 2: Run** `cargo test collectors::tests::wsl_codex --manifest-path src-tauri/Cargo.toml` and confirm failure due to missing transport.
- [ ] **Step 3: Refactor** native Codex launch into a command builder plus shared child-session function while keeping existing native tests green.
- [ ] **Step 4: Implement** WSL launch with fixed arguments `--exec codex -s read-only -a untrusted app-server`, hidden window flags, piped stdio, kill-on-drop, timeout, explicit kill, and wait. Map spawn/tool absence to `CliMissing`; retain protocol/network classifications.
- [ ] **Step 5: Run all collector tests** and require pass with no orphan fake process.

### Task 4: Windows startup wiring and acceptance validation

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/refresh/ipc.rs`
- Test: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/collectors/tests.rs`
- Create: `.claude/PRPs/reports/2026-07-17-wsl-collector-bridge-report.md`

**Interfaces:**
- Consumes: `FallbackCollector`, `WslCredentialSource`, and `WslCodexCollector`.
- Produces: Windows production collectors with native-first fallback; fixture and non-Windows construction remains unchanged.

- [ ] **Step 1: Write failing construction tests** asserting Windows production mode selects native-first Claude and Codex composites while fixture mode remains isolated.
- [ ] **Step 2: Run targeted Rust tests** and verify the construction assertion fails.
- [ ] **Step 3: Wire collectors** under `#[cfg(windows)]`; when `WslCommandFactory` cannot resolve, retain the existing missing-provider collector. Do not change renderer DTOs or add settings.
- [ ] **Step 4: Run formatting and static validation:** `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `corepack pnpm check`, and `git diff --check`.
- [ ] **Step 5: Run tests:** Windows `cargo test --manifest-path E:\dev\CacheBite\.tmp-msi-build\src-tauri\Cargo.toml`, plus `corepack pnpm test -- --run`.
- [ ] **Step 6: Build** release and diagnostic Debug MSI on Windows and record SHA-256 hashes.
- [ ] **Step 7: Perform security and correctness review** for command injection, executable resolution, output limits, secret exposure, failure mapping, and process cleanup; resolve all Critical/High findings.
- [ ] **Step 8: Write the implementation report** with completed tasks, deviations, validation results, changed files, and manual WSL smoke-test status.

## Acceptance Criteria

- Windows CacheBite automatically falls back to the default WSL distribution for WSL-only Claude and Codex.
- Native configured collectors always win.
- Claude credentials never leave native memory except for the bounded WSL stdout transfer into the parent process and are never persisted/logged/exposed to IPC.
- Codex uses the existing app-server protocol over the WSL child transport.
- WSL/tool absence shows auth required/not installed; only real network failures show offline.
- CSP, native providers, fixture mode, frontend tests, Rust tests, checks, and MSI builds pass.
