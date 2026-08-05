# Settings Update Affordance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move the available-update action into Settings and replace the main-panel banner with a small notification dot on the Settings button.

**Architecture:** `App.svelte` remains the owner of native update state. It derives the available version once, passes a boolean to `UsagePanel`, and passes the version plus install action to `SettingsPanel`; neither child invokes native APIs. Existing theme tokens provide the filled primary-button appearance in both light and dark modes.

**Tech Stack:** Svelte 5, TypeScript/JSDoc props, Vitest, Testing Library, WebdriverIO, Tauri 2.

## Global Constraints

- Show the red dot and the Settings update row only for update status `available`.
- Put `Update available — {version}` immediately below the Settings heading and above Appearance.
- Render no main-panel update banner, release notes, or `Later` action.
- Match normal Settings typography; reuse only the `Refresh now` filled shape and theme-token colors for the install button.
- Keep system, light, and dark themes working through semantic tokens, with no literal light/dark button colors.
- Preserve the existing Version and manual `Check for updates` rows.
- Preserve all unrelated user changes in the dirty worktree.

---

### Task 1: Settings notification dot

**Files:**
- Modify: `src/lib/components/UsagePanel.test.ts`
- Modify: `src/lib/components/UsagePanel.svelte`

**Interfaces:**
- Consumes: `updateAvailable?: boolean` from `App.svelte`.
- Produces: a decorative `.settings-update-dot` and an accessible Settings label that communicates availability.

- [ ] **Step 1: Write failing component tests**

Add tests that render `UsagePanel` with `updateAvailable: true`, expect a `data-testid="settings-update-dot"`, and query the control by `Settings, update available`. Render with the default prop and assert the dot is absent and the control remains queryable as `Settings`.

- [ ] **Step 2: Verify RED**

Run: `corepack pnpm vitest run src/lib/components/UsagePanel.test.ts`

Expected: FAIL because `updateAvailable` and the dot do not exist.

- [ ] **Step 3: Implement the minimal indicator**

Add `updateAvailable = false` to the component props. Wrap the visible label in a positioned span, conditionally render the decorative dot, and set the button `aria-label` to `Settings, update available` only when true. Style the dot with `var(--sev-exhausted)`, absolute positioning, and no pointer/focus behavior.

- [ ] **Step 4: Verify GREEN**

Run: `corepack pnpm vitest run src/lib/components/UsagePanel.test.ts`

Expected: all UsagePanel tests pass.

### Task 2: Available update row inside Settings

**Files:**
- Modify: `src/lib/components/SettingsPanel.test.ts`
- Modify: `src/lib/components/SettingsPanel.svelte`

**Interfaces:**
- Consumes: `availableUpdateVersion?: string | null`, `updateBusy?: boolean`, and `onInstallUpdate?: () => void`.
- Produces: the row `Update available — {version}` plus one `Install and restart` button.

- [ ] **Step 1: Write failing Settings tests**

Add tests proving the row appears before the `Appearance` label when `availableUpdateVersion` is set, is absent when null, invokes `onInstallUpdate` once per click, and disables the install button while `updateBusy` is true. Assert that `Later` and fixture release notes are absent.

- [ ] **Step 2: Verify RED**

Run: `corepack pnpm vitest run src/lib/components/SettingsPanel.test.ts`

Expected: FAIL because the new props and row do not exist.

- [ ] **Step 3: Implement the minimal Settings row**

Add the three props and conditionally render a `.update-available-row` directly after the heading. Use the same `.field` typography as other Settings rows. Add a compact `.install-update-action` whose border/background/text use `var(--color-text)`, `var(--color-text)`, and `var(--color-surface)`, with the same radius and hover/focus behavior as `Refresh now` but a Settings-sized height and font.

- [ ] **Step 4: Verify GREEN and theme token usage**

Run: `corepack pnpm vitest run src/lib/components/SettingsPanel.test.ts`

Expected: all SettingsPanel tests pass, and the implementation contains no literal black/white button colors.

### Task 3: App integration and obsolete banner removal

**Files:**
- Modify: `src/App.test.ts`
- Modify: `src/App.svelte`
- Delete if unreferenced: `src/lib/components/UpdateNotice.svelte`
- Modify or delete if unreferenced: `src/lib/components/UpdateNotice.test.ts`

**Interfaces:**
- Consumes: `UpdateStateWire` and existing `gateway.installUpdate()`.
- Produces: `availableUpdateVersion`, `updateAvailable`, and Settings-owned install presentation.

- [ ] **Step 1: Write failing App tests**

Update the available-state tests to expect no main-panel status banner or `Later` button, expect the Settings control's available accessible name, open Settings, assert `Update available — 0.1.0-beta.5`, click `Install and restart`, and expect one `gateway.installUpdate()` call. Keep failed retry and downloading/installing state tests aligned with the lower Settings status line.

- [ ] **Step 2: Verify RED**

Run: `corepack pnpm vitest run src/App.test.ts`

Expected: FAIL because the old banner still renders and the new props are not wired.

- [ ] **Step 3: Implement App wiring**

Derive `availableUpdateVersion` only when `updateState?.status.status === 'available'`. Remove the `UpdateNotice` import/composition and the `Later` dismissal state/functions. Pass `updateAvailable={availableUpdateVersion !== null}` to `UsagePanel`; pass `availableUpdateVersion`, `updateBusy`, and `onInstallUpdate={() => void gateway.installUpdate().catch(() => {})}` to `SettingsPanel`. Retain update listening, manual checking, and status-line presentation.

- [ ] **Step 4: Remove only truly unused notice code**

Run `rg -n "UpdateNotice|dismissedUpdate|dismissUpdate|runUpdateAction" src tests` first. Delete the component/test only if no production reference remains; otherwise narrow it without disturbing unrelated tests.

- [ ] **Step 5: Verify GREEN**

Run: `corepack pnpm vitest run src/App.test.ts src/lib/components/UsagePanel.test.ts src/lib/components/SettingsPanel.test.ts src/lib/state/updatePresentation.test.ts`

Expected: all selected tests pass.

### Task 4: Native E2E contract and full verification

**Files:**
- Modify: `tests/e2e/native.spec.ts`

**Interfaces:**
- Consumes: fixture version `9.9.9` from `CACHEBITE_E2E_UPDATE=available`.
- Produces: an E2E assertion for the notification dot and Settings-owned install action.

- [ ] **Step 1: Update the native E2E assertion**

Replace the old `Install and Later` banner flow with: wait for native status `available`; find the Settings control by its available accessible label; verify the dot exists; click Settings; assert `Update available — 9.9.9`; assert `Install and restart` is enabled; assert `Later` is absent. Use supported WebdriverIO matchers such as `toHaveText` or text equality instead of `toHaveTextContaining`.

- [ ] **Step 2: Run static and unit verification**

Run `corepack pnpm check`, `corepack pnpm lint`, and `corepack pnpm test`.

Expected: every command exits 0.

- [ ] **Step 3: Run coverage and renderer build verification**

Run `corepack pnpm test:coverage` and `corepack pnpm build`.

Expected: every command exits 0 and all coverage metrics remain at least 80%.

- [ ] **Step 4: Run the available-update native fixture and capture the result**

Build with `corepack pnpm tauri build --debug --no-bundle --features webdriver`, then run `corepack pnpm test:e2e` with `CACHEBITE_E2E_FIXTURES=1`, `CACHEBITE_EXPECTED_COLLECTOR_MODE=fixture`, `CACHEBITE_NOTIFICATION_SMOKE=capability-only-no-prompt`, and `CACHEBITE_E2E_UPDATE=available`. Capture `artifacts/e2e/settings-update-available.png` after Settings opens.

Expected: native E2E exits 0 and the screenshot shows the themed compact row above Appearance with no main-panel banner or Later button.
