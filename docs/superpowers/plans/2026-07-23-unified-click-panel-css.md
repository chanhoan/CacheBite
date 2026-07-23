# Unified Click Panel CSS Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every desktop platform use the UI-plan macOS vibrancy popover layout and CSS, with a 312px panel and no obsolete OS-specific style branches.

**Architecture:** Keep the existing shared Svelte panel markup and make its outer shell styling platform-independent. Lock the native panel window width to the same 312px contract, enable native transparency, and observe the rendered shell so the native height follows its content across usage and settings views.

**Tech Stack:** Svelte 5, CSS, Tauri 2 JSON configuration, Vitest, WebdriverIO.

## Global Constraints

- Use the macOS vibrancy popover layout on every OS.
- Panel width is exactly `312px`.
- Header padding is `12px 16px 0`.
- Body padding is `16px`.
- Footer padding is `12px 16px 14px`.
- The native panel window is transparent so the shared vibrancy shell can composite correctly.
- The panel has no viewport minimum height; its native height follows the rendered content.
- Remove unused `data-platform` CSS selectors and OS-specific panel overrides.
- Preserve unrelated uncommitted changes.

---

### Task 1: Lock the panel layout contract with failing tests

**Files:**
- Modify: `src/securityConfig.test.ts`
- Modify: `tests/e2e/renderer.spec.ts`

**Interfaces:**
- Consumes: `src-tauri/tauri.conf.json` panel window configuration and the rendered `main.panel`.
- Produces: Regression assertions for the 312px native window width and platform-independent computed panel styling.

- [ ] **Step 1: Add the native-width assertion**

Extend the panel window type in `src/securityConfig.test.ts` with `width?: number` and assert:

```ts
expect(panel).toMatchObject({
  width: 312,
  transparent: true,
  decorations: false,
  resizable: false,
});
```

- [ ] **Step 2: Add the rendered-layout assertion**

In `tests/e2e/renderer.spec.ts`, load the panel fixture and read computed values from `main.panel`:

```ts
const layout = await browser.execute(() => {
  const panel = document.querySelector<HTMLElement>('main.panel');
  if (!panel) throw new Error('panel shell missing');
  const styles = getComputedStyle(panel);
  return {
    width: panel.getBoundingClientRect().width,
    borderRadius: styles.borderRadius,
    backdropFilter: styles.backdropFilter,
  };
});

expect(layout).toMatchObject({
  width: 312,
  borderRadius: '14px',
  backdropFilter: 'blur(20px)',
});
```

- [ ] **Step 3: Run the focused tests and verify RED**

Run:

```bash
pnpm test src/securityConfig.test.ts
pnpm test:e2e:renderer
```

Expected: the native width assertion reports `360`, and the fixture (which reports Linux) does not satisfy the unified vibrancy shell contract.

### Task 2: Unify the panel CSS and native width

**Files:**
- Modify: `src/App.svelte`
- Modify: `src/lib/api/gateway.ts`
- Modify: `src/lib/api/fixtureGateway.ts`
- Modify: `src/lib/styles/global.css`
- Modify: `src/lib/components/UsagePanel.svelte`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/capabilities/panel.json`

**Interfaces:**
- Consumes: the existing `main.panel` class and shared `UsagePanel` markup.
- Produces: one platform-independent vibrancy shell with a 312px native/CSS width.

- [ ] **Step 1: Replace platform branches with one shell rule**

Move the macOS vibrancy declarations into `main.panel`, set `width: min(19.5rem, 100%)`, override inherited panel padding with `padding: 0`, and delete the macOS, Windows, and Linux `data-platform` selectors.

- [ ] **Step 2: Preserve the unified dark appearance**

Replace the OS-specific dark selectors with one rule:

```css
@media (prefers-color-scheme: dark) {
  main.panel {
    background: rgb(28 31 36 / 92%);
  }
}
```

- [ ] **Step 3: Match the footer inset**

Change the footer padding in `src/lib/components/UsagePanel.svelte` from `var(--space-3) var(--space-4) var(--space-4)` to `var(--space-3) var(--space-4) 0.875rem`.

- [ ] **Step 4: Match the native panel width**

Change the panel window width in `src-tauri/tauri.conf.json` from `360` to `312` and set `transparent` to `true`.

- [ ] **Step 5: Fit the native height to rendered content**

Remove the panel viewport minimum height and observe the panel shell with
`ResizeObserver`. Send the measured height through the typed gateway to a
panel-only `resize_panel` native command, which applies the 312px logical width,
updates the height, and re-anchors the panel beside the overlay. Keep direct
native window resize permission out of the renderer capability.

- [ ] **Step 6: Run focused tests and verify GREEN**

Run:

```bash
pnpm test src/securityConfig.test.ts src/lib/components/UsagePanel.test.ts
pnpm test:e2e:renderer
```

Expected: all focused tests pass.

### Task 3: Verify quality and dead-code removal

**Files:**
- Review: `src/lib/styles/global.css`
- Review: `src/lib/components/UsagePanel.svelte`
- Review: `src-tauri/tauri.conf.json`

**Interfaces:**
- Consumes: the completed unified style.
- Produces: verified renderer and native configuration with no OS-specific panel CSS.

- [ ] **Step 1: Confirm obsolete selectors are gone**

Run:

```bash
rg -n "main\\.panel\\[data-platform|font-family: 'Segoe UI'|font-family: 'Cantarell'" src/lib/styles/global.css
```

Expected: no matches.

- [ ] **Step 2: Run renderer verification**

Run:

```bash
pnpm check
pnpm lint
pnpm test:coverage
pnpm build
```

Expected: all commands exit successfully and coverage gates remain at or above 80%.

- [ ] **Step 3: Review the final diff**

Run:

```bash
git diff -- src/lib/styles/global.css src/lib/components/UsagePanel.svelte src-tauri/tauri.conf.json src/securityConfig.test.ts tests/e2e/renderer.spec.ts
```

Expected: only the unified panel contract, its tests, and pre-existing user changes appear.
