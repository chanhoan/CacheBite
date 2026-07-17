# Native CI Parity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore PR checks and make macOS native E2E cover the same fixture and production contracts as the other desktop platforms.

**Architecture:** Use the optional embedded WebDriver server in every native smoke binary and select the embedded WDIO provider unconditionally. Keep OS-specific display launchers, add macOS to production composition, and pin the Rust advisory scanner to the CVSS 4.0-compatible `0.22.2` release against `src-tauri/Cargo.lock`.

**Tech Stack:** GitHub Actions, WebdriverIO 9, Tauri 2, Vitest, Rust/cargo-audit

## Global Constraints

- Production release builds must not enable the embedded WebDriver feature.
- Every native smoke build must enable `--features webdriver`.
- Credential-free production composition must run on Ubuntu and macOS.
- Do not weaken or bypass security advisory checks.

---

### Task 1: Lock native workflow parity with tests

**Files:**
- Modify: `src/nativeWorkflow.test.ts`
- Test: `src/nativeWorkflow.test.ts`

**Interfaces:**
- Consumes: `.github/workflows/native-smoke.yml`, `wdio.conf.ts`, `.github/workflows/ci.yml`
- Produces: source-level workflow contract assertions

- [ ] **Step 1: Write failing contract tests**

Assert that `wdio.conf.ts` selects only `embedded` without external-driver installation, all three native build sites contain `--features webdriver`, production composition has an `os: [ubuntu-latest, macos-latest]` matrix with OS-specific data roots, release builds omit `webdriver`, and the advisory gate uses `cargo-audit 0.22.2` on `src-tauri/Cargo.lock`.

- [ ] **Step 2: Verify RED**

Run: `corepack pnpm test -- src/nativeWorkflow.test.ts`

Expected: failures for the current Darwin-only provider, missing Linux/Windows feature flags, missing macOS production matrix, and old cargo-audit pin.

### Task 2: Unify native smoke transport and production coverage

**Files:**
- Modify: `wdio.conf.ts`
- Modify: `.github/workflows/native-smoke.yml`

**Interfaces:**
- Produces: embedded WebDriver configuration for all native smoke jobs

- [ ] **Step 1: Select embedded provider unconditionally**

Set `driverProvider` to the literal `embedded`, remove `autoInstallTauriDriver`, and retain the absolute application path and existing timeout settings.

- [ ] **Step 2: Build every native smoke binary with the WebDriver feature**

Pass `--features webdriver` in Windows/macOS fixture, Linux display fixture, and production composition builds.

- [ ] **Step 3: Add the macOS production matrix entry**

Use `strategy.fail-fast: false` with `os: [ubuntu-latest, macos-latest]`. Install display packages and use Xvfb only on Ubuntu; execute `pnpm test:e2e` directly on macOS. Set both `HOME` and `XDG_DATA_HOME` for the Linux E2E step, and `HOME` for the macOS E2E step, so credential fallback paths and application data are isolated without affecting tool installation.

- [ ] **Step 4: Verify GREEN**

Run: `corepack pnpm test -- src/nativeWorkflow.test.ts`

Expected: all native workflow contract tests pass.

### Task 3: Restore Rust advisory compatibility

**Files:**
- Modify: `.github/workflows/ci.yml`
- Test: `src/nativeWorkflow.test.ts`

**Interfaces:**
- Produces: a RustSec gate that parses current CVSS 4.0 records

- [ ] **Step 1: Update the cargo-audit installation**

Replace the incompatible pin with `cargo install cargo-audit --version 0.22.2 --locked`, then run `cargo audit --file src-tauri/Cargo.lock` so the reproducible scanner reads the existing Rust lockfile.

- [ ] **Step 2: Re-run the focused contract test**

Run: `corepack pnpm test -- src/nativeWorkflow.test.ts`

Expected: pass.

### Task 4: Full verification and publication

**Files:**
- Review all changed files

**Interfaces:**
- Produces: one reviewed commit on `feat/cachebite-runtime-hardening`

- [ ] **Step 1: Run frontend verification**

Run: `corepack pnpm check && corepack pnpm lint && corepack pnpm test:ci`

Expected: zero check/lint errors and all tests pass with at least 80% coverage.

- [ ] **Step 2: Run Rust verification**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check && cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings && cargo test --manifest-path src-tauri/Cargo.toml --all-features`

Expected: all commands pass.

- [ ] **Step 3: Review security and scope**

Run: `git diff --check`, inspect `git diff`, and verify no credentials, production WebDriver enablement, or unrelated files are included.

- [ ] **Step 4: Commit and push**

Stage the intended files, commit with `fix: restore native CI parity`, and push `feat/cachebite-runtime-hardening` to `origin` with upstream tracking.
