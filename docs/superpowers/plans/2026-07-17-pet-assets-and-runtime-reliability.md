# Pet Assets and Runtime Reliability Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Normalize all Pet asset paths and make Pet URL/package failures recoverable.

**Architecture:** Rename source and bundled assets to one ASCII snake-case contract, updating all references mechanically. Validate the two platform-specific Tauri asset origins at the resolver boundary and catch resolution failures before Svelte rendering can abort startup.

**Tech Stack:** Svelte 5, TypeScript, Vitest, Rust, Tauri 2, WiX MSI

## Global Constraints

- Runtime Pet IDs are exactly `cat` and `corgi`.
- Runtime states are exactly `idle`, `warn`, `critical`, and `exhausted`.
- Image filenames match `^(cat|corgi)_(idle|warn|critical|exhausted)_(01|02|03|04)\.png$`.
- Only `asset://localhost` and `http://asset.localhost` are accepted package origins.
- Existing user-authored work outside Pet paths is preserved.

---

### Task 1: Asset URL contract

**Files:**
- Modify: `src/lib/assets/resolver.ts`
- Test: `src/lib/assets/manifest.test.ts`

**Interfaces:**
- Consumes: `resolveIdleAnimation(manifest, packageRoot)` and `resolvePetAnimation(manifest, packageRoot, requested)`
- Produces: validation for the two exact Tauri origins

- [ ] Add failing cases for Windows `http://asset.localhost` and hostile HTTP/asset origins.
- [ ] Run `corepack pnpm exec vitest run src/lib/assets/manifest.test.ts` and confirm the Windows case fails.
- [ ] Accept the two exact origins while retaining credential, query, hash, and traversal checks.
- [ ] Re-run the targeted test and confirm it passes.

### Task 2: Recoverable Pet rendering

**Files:**
- Modify: `src/App.svelte`
- Test: `src/App.test.ts`

**Interfaces:**
- Consumes: validated `petPackage` and `resolvePetAnimation`
- Produces: `resolvedAnimation` as `ResolvedAnimation | null` plus `Pet package unavailable` on failure

- [ ] Add a failing composition test whose gateway returns a non-Tauri package root.
- [ ] Confirm the test observes a thrown render/startup failure.
- [ ] Resolve animation in a guarded helper that sets the Pet diagnostic and returns `null`.
- [ ] Confirm the application reaches ready and renders the diagnostic.

### Task 3: Repository-wide Pet naming

**Files:**
- Rename: `src-tauri/resources/pets/{cat,corgi}/frames/*.png`
- Modify: `src-tauri/resources/pets/{cat,corgi}/manifest.json`
- Rename: `docs/UI-plan/assets/pet/**/*`
- Modify: references under `docs/UI-plan/`

**Interfaces:**
- Produces: `{pet}_{state}_{frame}.png` for every Pet PNG

- [ ] Rename runtime frames and update both manifests.
- [ ] Rename documentation `dog` to `corgi`, `exhusted` to `exhausted`, and every image to the naming contract.
- [ ] Replace all HTML/JavaScript references with the new paths.
- [ ] Scan the repository for stale original names and invalid Pet filenames; expect no matches.

### Task 4: Verification and packages

**Files:**
- Output: `artifacts/msi/CacheBite_0.1.0_x64_en-US.msi`
- Output: `artifacts/msi/CacheBite_0.1.0_x64_debug.msi`

**Interfaces:**
- Consumes: all preceding tasks
- Produces: tested Release and diagnostic installers

- [ ] Run the complete frontend test, check, lint, and production build commands.
- [ ] Run Windows Rust tests with one test thread.
- [ ] Build Release and Debug MSIs in the isolated Windows build directory.
- [ ] Copy installers to `artifacts/msi`, calculate SHA-256 hashes, and inspect the final diff.
