# Plan: WSL Codex Discovery for NVM Installations

## Summary

Fix the Windows production collector so CacheBite discovers Codex installed in the default WSL distribution through NVM or another interactive-shell-only PATH setup. Preserve native-first selection, existing login-shell compatibility, fixed command security, bounded output, JSON-RPC behavior, and process-group cleanup.

## User Story

As a Windows CacheBite user with Codex installed in the default WSL distribution through NVM, I want CacheBite to discover and launch that CLI automatically, so I can see usage without modifying `~/.profile` or installing a second Windows copy of Codex.

## Problem -> Solution

The WSL collector probes and launches only with `bash -lc`; on the reproduced Ubuntu 22.04 setup that shell has only the system PATH, while `bash -ic` resolves `/home/<user>/.nvm/versions/node/<node-version>/bin/codex` -> Probe the existing login shell first, fall back to an interactive shell when needed, and launch Codex with the same fixed shell mode that passed discovery.

## Metadata

- **Complexity**: Medium
- **Source PRD**: N/A
- **PRD Phase**: N/A (standalone defect remediation)
- **Estimated Files**: 3

## UX Design

### Before

```text
Windows PATH misses Codex
        |
        v
WSL bash -lc misses NVM PATH
        |
        v
CliMissing -> Unknown gauges + "The Codex CLI is not installed"
```

### After

```text
Windows PATH misses Codex
        |
        v
WSL bash -lc probe ---- success ---> launch with bash -lc
        |
      missing
        v
WSL bash -ic probe ---- success ---> launch with bash -ic -> Codex usage
        |
      missing
        v
CliMissing
```

### Interaction Changes

| Touchpoint | Before | After | Notes |
|---|---|---|---|
| First Windows launch with WSL/NVM Codex | False “CLI is not installed” state | Usage loads through `cli_rpc` | No setup or profile edits |
| Existing WSL login-shell installation | Works through `bash -lc` | Continues using `bash -lc` | Login-first order prevents regression |
| Codex absent from both shell modes | “CLI is not installed” | Unchanged | Existing unavailable contract remains |
| Settings | No WSL distribution or path setting | Unchanged | Default WSL distribution remains the target |

## Mandatory Reading

| Priority | File | Lines | Why |
|---|---|---:|---|
| P0 | `src-tauri/src/collectors/wsl.rs` | 15-20 | Current fixed login-shell probe and launch scripts |
| P0 | `src-tauri/src/collectors/wsl.rs` | 108-198 | WSL Codex collection, probe mapping, launch, and diagnostics |
| P0 | `src-tauri/src/collectors/codex.rs` | 214-317 | PGID handshake, bounded startup-noise scan, RPC exchange, and cleanup ordering |
| P0 | `src-tauri/src/collectors/tests.rs` | 35-219 | Injected WSL process, exact-command tests, fake launcher, noise, and missing-CLI tests |
| P1 | `src-tauri/src/collectors/wsl.rs` | 329-381 | Five-second bounded probe process, hidden window, output cap, and child reaping |
| P1 | `src-tauri/src/lib.rs` | 135-167 | Native-first Windows collector composition and exact `CliMissing` fallback trigger |
| P1 | `src-tauri/src/refresh/actor.rs` | 289-340 | How `CliMissing` becomes persisted `NotInstalled` state |
| P1 | `src/lib/state/engine.ts` | 112-144, 216-234 | Why an unavailable state suppresses usage severities even with a cached snapshot |
| P1 | `src/lib/components/systemGuidance.ts` | 19-34 | User-visible false “not installed” message |
| P1 | `docs/superpowers/specs/2026-07-17-wsl-collector-bridge-design.md` | 5-37, 57-93, 103-128 | No-manual-configuration goal and WSL security/acceptance constraints |
| P2 | `.claude/PRPs/reports/2026-07-17-wsl-collector-bridge-report.md` | 35-37 | Historical login-shell assumption and missing real WSL smoke validation |

## External Documentation

| Topic | Source | Key Takeaway |
|---|---|---|
| External research | None required | The defect was reproduced on the target Windows/Ubuntu environment, and the repository already defines the shell, security, lifecycle, and status contracts needed for the fix. |

## Relevant System Path

```text
src-tauri/src/lib.rs production_collectors
  -> native Codex collector
  -> FallbackCollector on CliMissing
  -> WslCodexCollector::collect
  -> bounded wsl.exe probe
  -> selected fixed shell-mode launch
  -> collect_app_server_child_with_pgid
  -> Codex initialize + account/rateLimits/read
  -> CollectionOutcome::Success or CliMissing
  -> refresh actor ProviderState
  -> Tauri provider-state event
  -> renderer gauges/guidance
```

## Patterns to Mirror

### NAMING_CONVENTION

// SOURCE: `src-tauri/src/collectors/wsl.rs:15-18`

```rust
pub const CLAUDE_CREDENTIAL_SCRIPT: &str = "...";
pub const CODEX_PROBE_SCRIPT: &str = "bash -lc 'command -v codex >/dev/null 2>&1'";
pub const CODEX_LAUNCH_SCRIPT: &str = "exec setsid --wait bash -lc '...; exec codex -s read-only -a untrusted app-server'";
pub const CODEX_CLEANUP_SCRIPT: &str = "...";
```

Keep repository-owned shell text in named constants. Introduce a small private `CodexShellMode` enum with methods returning fixed `&'static str` probe and launch scripts; do not build shell commands from runtime strings.

### ERROR_HANDLING

// SOURCE: `src-tauri/src/collectors/wsl.rs:151-164`

```rust
let probe = factory
    .process
    .run(&["--exec", "sh", "-c", CODEX_PROBE_SCRIPT])
    .await;
if !matches!(probe, Ok(ProcessOutput { status: 0, .. })) {
    return CollectionOutcome::CliMissing;
}
```

Retain the current public outcome: only after both fixed probes fail does WSL discovery return `CollectionOutcome::CliMissing`. Do not convert discovery failure into network/offline state.

### LOGGING_PATTERN

// SOURCE: `src-tauri/src/collectors/wsl.rs:155-163`

```rust
#[cfg(debug_assertions)]
eprintln!(
    "[CacheBite:codex] wsl probe status={:?}",
    probe.as_ref().map(|output| output.status)
);
#[cfg(debug_assertions)]
eprintln!("[CacheBite:codex] wsl probe failed -> CliMissing");
```

Log only the fixed mode label and status/error class. Never log the resolved Codex path, WSL home, distribution, startup output, account data, or JSON-RPC body.

### DATA_ACCESS_PATTERN

// SOURCE: N/A

No persistence schema, repository, IPC DTO, or renderer data-access change is needed. The collector must continue producing the existing `CollectionOutcome` variants.

### SERVICE_PATTERN

// SOURCE: `src-tauri/src/collectors/wsl.rs:135-174`

```rust
Box::pin(async move {
    match tokio::spawn(async move { collect_wsl_codex(factory, timeout).await }).await {
        Ok(outcome) => outcome,
        Err(_) => CollectorError::Internal.into_outcome(),
    }
})
```

Discovery and launch remain inside the existing spawned collection task. The selected shell mode is local immutable state for one collection attempt and is passed to the launch-script selector.

### TEST_STRUCTURE

// SOURCE: `src-tauri/src/collectors/tests.rs:141-183`

```rust
let process = Arc::new(RecordingWslProcess::default());
let collector = WslCodexCollector::new(
    WslCommandFactory::with_process_and_executable_for_test(
        process.clone(),
        PathBuf::from("Z:\\definitely-missing\\wsl.exe"),
    ),
);
assert_eq!(collector.collect().await, CollectionOutcome::CliMissing);
let calls = process.calls.lock().unwrap();
assert_eq!(calls[0], ["--exec", "sh", "-c", CODEX_PROBE_SCRIPT]);
```

Use injected process results to assert probe order and fake executables to assert the exact selected launch script and shared RPC behavior. Tests must not require live WSL, Codex credentials, or network access.

## Files to Change

| File | Action | Justification |
|---|---|---|
| `src-tauri/src/collectors/wsl.rs` | UPDATE | Add login-first/interactive-fallback discovery and launch with the successful fixed shell mode |
| `src-tauri/src/collectors/tests.rs` | UPDATE | Add regression coverage for NVM-style interactive PATH, probe order, selected launch mode, missing CLI, noise bounds, and cleanup preservation |
| `docs/superpowers/specs/2026-07-17-wsl-collector-bridge-design.md` | UPDATE | Correct the WSL Codex discovery/launch contract to document dual fixed shell modes and no profile-edit requirement |

## NOT Building

- No Windows-native Codex installation, PATH modification, WSL profile modification, or automatic NVM installation.
- No user-configurable executable path, shell command, WSL distribution picker, or arbitrary shell interpolation.
- No scan across every installed WSL distribution; continue using the Windows-configured default distribution.
- No changes to Claude credential discovery, Codex JSON-RPC messages, normalized usage models, persistence, IPC, renderer state, or UI wording.
- No direct parsing or execution of a path returned by `command -v`; the probe remains status-only so untrusted stdout never becomes command text.
- No relaxation of timeout, output-size, process-group, hidden-window, or child-reaping protections.

## Step-by-Step Tasks

### Task 1: Add failing shell-discovery regression tests

- **ACTION**: Extend the Rust collector suite before changing production constants or logic.
- **IMPLEMENT**: Add a sequenced fake `WslProcess` that records calls and returns login failure followed by interactive success. Add tests proving: login success performs one probe and selects the login launch; login failure then interactive success performs exactly two probes and selects the interactive launch; both failures return `CliMissing` without starting the launcher. Add a `#[cfg(unix)]` controlled-HOME test with a temporary `.bashrc` and executable `codex` fixture to reproduce that the interactive fixed probe succeeds when the login fixed probe cannot see the NVM-style bin directory.
- **MIRROR**: `TEST_STRUCTURE`, plus `FakeWslProcess` and `RecordingWslProcess` at `src-tauri/src/collectors/tests.rs:35-70,118-153`.
- **IMPORTS**: Add `VecDeque` only if the sequenced fake uses it; reuse `Arc`, `Mutex`, `PathBuf`, `fs`, and Unix `PermissionsExt` already present.
- **GOTCHA**: Keep the controlled test child-only: set its temporary `HOME` and minimal PATH on `std::process::Command`; never mutate global process environment because Rust tests run concurrently. The test should assert exit status, not shell warning text.
- **VALIDATE**: Run `cargo test --manifest-path src-tauri/Cargo.toml --all-features wsl_codex`; the new NVM/interactive cases must fail against the current login-only implementation for the expected reason.

### Task 2: Implement login-first, interactive-fallback selection

- **ACTION**: Replace the single shell-script pair with a private fixed-mode selector and update `collect_wsl_codex`.
- **IMPLEMENT**: Define private `CodexShellMode::{Login, Interactive}`. Each mode returns a fixed probe (`bash -lc ...` or `bash -ic ...`) and matching fixed launch wrapper (`setsid --wait bash -lc ...` or `setsid --wait bash -ic ...`). Probe `Login` first; probe `Interactive` only when login does not return status 0. On success, launch with that exact mode. If neither succeeds, return `CliMissing`. Keep outer arguments fixed as `wsl.exe --exec sh -c <repository constant>`, emit the PGID marker before `exec codex`, and retain existing cleanup callback and timeout.
- **MIRROR**: `NAMING_CONVENTION`, `ERROR_HANDLING`, `LOGGING_PATTERN`, and `SERVICE_PATTERN`.
- **IMPORTS**: No production dependency additions; use existing `ProcessOutput`, `WslCommandFactory`, and `CollectorError` types.
- **GOTCHA**: Do not replace login mode outright; that would regress users whose PATH exists only in `.profile`. Do not use `bash -lic` as a single compromise because startup-file behavior differs and the reproduced evidence specifically distinguishes `-lc` from `-ic`. Interactive startup may emit stdout, so preserve the 64 KiB probe cap and the bounded pre-marker scanner at `codex.rs:267-317`; stderr remains null. Never expose shell output in logs.
- **VALIDATE**: Targeted tests pass, including exact fixed arguments, login precedence, interactive fallback, startup-noise handling, timeout, cancellation, and descendant reaping.

### Task 3: Align the design contract and run acceptance validation

- **ACTION**: Update the WSL bridge design and validate the complete collector boundary.
- **IMPLEMENT**: Amend the Codex data-flow section to state that CacheBite probes the default WSL distribution with fixed login then interactive Bash commands, launches with the successful mode, accepts bounded startup noise before the PGID marker, and requires no user profile edits. Preserve the native-first, fixed-command, no-user-input, default-distribution, output-bound, and process-cleanup constraints.
- **MIRROR**: Existing concise architecture language at `docs/superpowers/specs/2026-07-17-wsl-collector-bridge-design.md:22-37,57-93`.
- **IMPORTS**: N/A.
- **GOTCHA**: Treat `.claude/PRPs/reports/2026-07-17-wsl-collector-bridge-report.md` as historical evidence; do not rewrite its old validation result. Document the corrected behavior in the current design and this plan.
- **VALIDATE**: Run formatting, Clippy, all Rust tests, renderer CI checks, and a Windows debug-MSI smoke test using the reproduced NVM-only setup. Confirm `bash -lc 'command -v codex'` may remain empty while CacheBite still reaches active `cli_rpc` usage.

## Testing Strategy

### Unit Tests

| Test | Input | Expected Output | Edge Case? |
|---|---|---|---|
| Login precedence | Login probe status 0 | One probe; login launch script selected | No |
| Interactive fallback | Login status nonzero, interactive status 0 | Two ordered probes; interactive launch selected; RPC succeeds | Yes |
| NVM-style startup | Temporary HOME where `.bashrc` alone adds fake Codex bin | Interactive probe succeeds while login probe does not | Yes |
| Missing in both modes | Two nonzero probe statuses | `CollectionOutcome::CliMissing`; launcher not started | Yes |
| Probe infrastructure errors | Bounded process errors/nonzero results | No panic or output leak; final public mapping remains `CliMissing` | Yes |
| Interactive startup noise | Lines/partial line before PGID marker | Marker found within bounds; RPC succeeds | Yes |
| Excess startup noise | More than the existing 64 KiB scan budget | Parse/protocol failure; no unbounded allocation | Yes |
| Timeout/cancellation | Hanging fake app-server in selected mode | WSL child and Linux process group are killed and reaped | Yes |

### Edge Cases Checklist

- [ ] Empty probe stdout with status 0 is accepted; discovery is status-based.
- [ ] Nonzero login probe falls back exactly once to interactive mode.
- [ ] Both probes missing/failing produce `CliMissing`, not offline.
- [ ] Interactive `.bashrc` output is bounded and ignored before the PGID marker.
- [ ] Oversized or malformed PGID remains rejected.
- [ ] Paths and shell output never appear in diagnostics or renderer state.
- [ ] Concurrent refresh/cancellation does not orphan WSL or Codex descendants.
- [ ] Permission denied or unavailable `wsl.exe` preserves the existing missing-provider behavior.
- [ ] Existing login-shell-only installations retain precedence and behavior.

## Validation Commands

### Static Analysis

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
pnpm check
git diff --check
```

EXPECT: Zero format, Clippy, type, Svelte, or whitespace errors.

### Unit Tests

```bash
cargo test --manifest-path src-tauri/Cargo.toml --all-features wsl_codex
```

EXPECT: All targeted WSL Codex tests pass, including the new login/interactive selection cases.

### Full Test Suite

```bash
cargo test --manifest-path src-tauri/Cargo.toml --all-features
pnpm test:ci
```

EXPECT: All native and renderer tests pass with existing coverage gates.

### Build

Run from Windows PowerShell because MSI is a Windows bundle:

```powershell
Set-Location C:\playground\CacheBite
npx.cmd --yes pnpm@10.15.1 tauri build --debug --bundles msi
```

EXPECT: Debug MSI succeeds and is emitted under `src-tauri\target\debug\bundle\msi\`.

### Manual Validation

- [ ] On the default `Ubuntu-22.04` WSL distribution, keep Codex installed only under NVM; do not add its bin directory to `~/.profile`.
- [ ] From PowerShell, confirm the reproduced precondition: `wsl.exe -- bash -lc 'command -v codex'` prints nothing.
- [ ] From PowerShell, confirm `wsl.exe -- bash -ic 'command -v codex; codex --version'` prints the NVM Codex path and version.
- [ ] Install/run the debug Windows build, fully quit any prior CacheBite process, and reopen it.
- [ ] Open the Codex panel and verify both usage windows load through `cli_rpc` rather than showing “The Codex CLI is not installed.”
- [ ] Trigger Refresh now and verify a second successful sample without a visible console window.
- [ ] Quit CacheBite and confirm no orphaned `codex app-server` process remains in WSL.
- [ ] Temporarily test a WSL environment with no Codex in either shell mode and confirm the original not-installed state still appears.

## Acceptance Criteria

- [ ] CacheBite discovers Codex when it is available only through the default WSL distribution's interactive NVM PATH.
- [ ] Users do not need to edit `.profile`, hardcode a versioned NVM directory, or install Codex on Windows.
- [ ] Login-shell discovery remains first and unchanged for existing users.
- [ ] Launch uses the same fixed shell mode that passed its probe.
- [ ] Codex absent from both modes still maps to `CliMissing`/`not_installed`.
- [ ] Native Windows Codex still wins and WSL is invoked only after native `CliMissing`.
- [ ] No user-controlled shell text, resolved path execution, credential/path logging, or new IPC data is introduced.
- [ ] Probe output, startup noise, timeout, hidden-window behavior, PGID cleanup, cancellation, and child reaping remain bounded and tested.
- [ ] Targeted tests, full Rust tests, renderer CI checks, and Windows debug MSI build pass.
- [ ] Manual NVM-only Windows/WSL smoke validation passes.

## Completion Checklist

- [ ] Code follows the fixed-script and injected-process patterns discovered above.
- [ ] Error handling preserves existing `CollectionOutcome` semantics.
- [ ] Debug logging contains only shell mode and stable status/error class.
- [ ] Tests are written before implementation and reproduce the actual `-lc` versus `-ic` failure.
- [ ] No hardcoded NVM version or user home path is added.
- [ ] WSL bridge design documentation is updated.
- [ ] No unrelated renderer, persistence, Claude, or distribution-selection work is included.
- [ ] Existing unrelated dirty-worktree changes are preserved.
- [ ] The implementation can proceed from this plan without further codebase search.

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Interactive `.bashrc` emits output before Codex RPC | Medium | High | Existing bounded handshake scanner ignores pre-marker lines; add interactive-noise regression tests and retain stderr suppression |
| Replacing login mode regresses `.profile` installations | Medium | High | Probe login first and launch with its mode; interactive is fallback only |
| User startup script hangs | Low/Medium | Medium | Preserve five-second bounded probe and ten-second collection timeout; test timeout and reaping |
| Shell output or resolved path leaks private data | Low | High | Keep probe status-only, zeroize captured output on drop, and log only mode/status |
| Probe succeeds but launch mode differs | Medium without explicit design | High | Represent mode as an enum and select both scripts from the same immutable value; assert exact launch args |
| Fix weakens fixed-command injection boundary | Low | High | Keep all scripts repository constants and accept no settings, path, stdout, or user interpolation |
| Unix-only test passes but Windows WSL differs | Medium | Medium | Require the documented Windows NVM-only MSI smoke test before completion |

## Notes

- Reproduction evidence from the target machine:
  - `wsl.exe -- bash -lc 'echo "$PATH"; command -v codex'` returned only system directories and no Codex.
  - `wsl.exe -- bash -ic 'command -v codex; codex --version'` returned `/home/<user>/.nvm/versions/node/<node-version>/bin/codex` and the installed Codex CLI version.
- The existing implementation report explicitly recorded that a real interactive WSL smoke test had not been performed. This defect closes that validation gap.
- The target source/test files were clean at planning time, but the repository contains unrelated user changes; implementation must avoid touching or reverting them.
