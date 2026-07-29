# Code Review Remediation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix all verified findings in the 2026-07-17 local changes review with regression coverage and honest native validation.

**Architecture:** Keep state transitions in the existing pure renderer engine, process trust in native collector discovery, and durability in the refresh persistence boundary. Extend the current fixture/native test split rather than introducing a second application composition root.

**Tech Stack:** Svelte 5, TypeScript, Vitest, Tauri 2, Rust 1.97.1, Tokio, WebdriverIO, GitHub Actions.

## Global Constraints

- Tests precede production edits and must be observed failing for the intended reason.
- Credentials, provider bodies, account identifiers, and filesystem paths never enter renderer DTOs or logs.
- Native fixture tests never contact provider endpoints or locally installed provider CLIs.
- Settings and state updates remain immutable.
- Do not guess GitHub Action SHAs; verify each pin from authoritative action repositories.

---

### Task 1: Reset and startup failure state

**Files:**
- Modify: `src/App.svelte`
- Modify: `src/lib/stores/providers.ts`
- Test: `src/App.test.ts`
- Test: `src/lib/state/domain.test.ts`

**Interfaces:**
- Produces: `providersStore.markResetPending(provider, revision)` returning transition events.
- Produces: explicit renderer startup error and retry action.

- [ ] Add failing tests proving retained pre-reset usage becomes unknown and an IPC rejection renders retry UI instead of permanent loading.
- [ ] Run `corepack pnpm vitest run src/App.test.ts src/lib/state/domain.test.ts` and confirm both new assertions fail.
- [ ] Implement the reset transition and a caught/retryable startup routine with cleanup-safe listener registration.
- [ ] Re-run the focused tests and confirm they pass.

### Task 2: Trusted Codex executable discovery and collector-mode truth

**Files:**
- Modify: `src-tauri/src/collectors/codex.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/refresh/ipc.rs`
- Test: `src-tauri/src/collectors/tests.rs`
- Test: `src-tauri/src/window/tests.rs`

**Interfaces:**
- Produces: `resolve_codex_executable(path: &OsStr) -> Result<PathBuf, CollectorError>`.
- Produces: allowlisted `CollectorModeDto { claude, codex }` for native test assertions.

- [ ] Add failing Rust tests rejecting bare/relative executable paths and distinguishing fixture from production collector mode.
- [ ] Run the isolated Rust harness or `cargo test --manifest-path src-tauri/Cargo.toml` and confirm expected failures.
- [ ] Implement startup PATH resolution, canonical absolute validation, and the narrow diagnostic command.
- [ ] Re-run focused Rust tests and Clippy with `-D warnings`.

### Task 3: Settings serialization and persistence diagnostics

**Files:**
- Modify: `src/App.svelte`
- Modify: `src-tauri/src/refresh/actor.rs`
- Test: `src/App.test.ts`
- Test: `src-tauri/src/refresh/tests.rs`

**Interfaces:**
- Produces: one settings promise chain that applies responses in request order.
- Produces: path-free persistence diagnostics and retryable pending writes.

- [ ] Add failing tests with reversed settings response completion and injected snapshot/history write failures.
- [ ] Verify the tests fail because stale responses overwrite settings and write errors disappear.
- [ ] Serialize settings updates; retain failed persistence work for retry and emit sanitized diagnostics.
- [ ] Re-run focused frontend and Rust tests.

### Task 4: Coverage and window-position regression tests

**Files:**
- Modify: `vite.config.ts`
- Modify: `src/lib/api/gateway.test.ts`
- Modify only if tests require a seam: `src/lib/api/gateway.ts`

**Interfaces:**
- Verifies: current `AppGateway.listenPositionMoved` contract.

- [ ] Add fake-timer tests for repeated movement, per-event scale factors, one debounced save, and cleanup flush.
- [ ] Run `corepack pnpm vitest run src/lib/api/gateway.test.ts` and confirm missing coverage fails.
- [ ] Add the minimal event/window seam needed by tests and expand coverage include to application source with narrow documented exclusions.
- [ ] Run `corepack pnpm test:ci` and keep all thresholds at or above 80%.

### Task 5: Honest native IPC E2E

**Files:**
- Modify: `tests/e2e/native.spec.ts`
- Modify: `.github/workflows/native-smoke.yml`
- Modify: `wdio.conf.ts`

**Interfaces:**
- Consumes: collector-mode diagnostic from Task 2.
- Verifies: hydration plus representative settings/history IPC and per-window authorization.

- [ ] Add native E2E assertions that fail under the current fixture-only indistinguishable mode.
- [ ] Split fixture and production-composition workflow invocations while preserving the no-network fixture suite.
- [ ] Run renderer E2E locally where Chrome permissions allow and validate workflow commands statically otherwise.

### Task 6: Immutable GitHub Action pins and final review

**Files:**
- Modify: `.github/workflows/ci.yml`
- Modify: `.github/workflows/native-smoke.yml`
- Modify: `.github/workflows/release.yml`
- Modify: `README.md`

**Interfaces:**
- Produces: SHA-pinned workflow dependencies with version comments.

- [ ] Resolve each existing action tag to a reviewed commit using its authoritative GitHub repository.
- [ ] Replace mutable refs, preserving human-readable version comments and Dependabot configuration.
- [ ] Scan workflows to ensure every `uses:` reference is a full SHA.
- [ ] Run frontend CI, Rust checks available on this host, secret scans, `git diff --check`, and focused code/security review.

## Completion Criteria

- All ten review findings are fixed or explicitly narrowed with evidence.
- Frontend tests, coverage, lint, type checks, and build pass.
- Rust tests/Clippy pass in a native-capable environment; local prerequisite blocks are reported exactly.
- Native fixture and production-composition tests make their selected backend mode observable.
- No mutable GitHub Action refs remain.
