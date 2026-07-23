# Panel Readability, Primary Action, and Reset Countdown Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep freshness copy on one line without collector source names, make `Set as primary` perform the primary-provider change, and render weekly resets as day-aware relative countdowns.

**Architecture:** Preserve the existing Svelte component boundaries. Extend the pure time formatter for day-aware countdowns, have both gauges consume the same relative format, keep provider tab selection local, and route only the explicit primary button through the existing serialized settings update.

**Tech Stack:** Svelte 5, TypeScript, Vitest, Testing Library, WebdriverIO.

## Global Constraints

- Freshness renders only `Fresh|Stale · captured <age>`.
- Freshness never renders collector source or cached markers and stays on one visual line.
- Both gauges render `resets in <relative countdown>`.
- Provider tabs browse providers without changing persisted primary settings.
- `Set as primary` changes primary through the existing `changeSettings` queue.
- Preserve unrelated worktree changes and do not create a commit.

---

### Task 1: Day-aware relative reset formatting

**Files:**

- Modify: `src/lib/format/time.test.ts`
- Modify: `src/lib/format/time.ts`
- Modify: `src/lib/components/UsageGauge.test.ts`
- Modify: `src/lib/components/UsageGauge.svelte`
- Modify: `src/lib/components/UsagePanel.svelte`

**Interfaces:**

- Consumes: `relativeFromNow(isoTimestamp: string, nowMs: number): string | null`
- Produces: day-aware strings such as `6d 22h 10m` and one relative reset rendering path for both gauges.

- [ ] **Step 1: Write failing formatter and gauge tests**

Add formatter cases:

```ts
[24 * 60 * 60_000, '1d 0h 0m'],
[(6 * 24 * 60 + 22 * 60 + 10) * 60_000, '6d 22h 10m'],
```

Change the weekly gauge test to pass `nowMs` and expect:

```ts
expect(screen.getByText('resets in 3d 21h 0m')).toBeTruthy();
```

- [ ] **Step 2: Run RED tests**

Run:

```bash
corepack pnpm test src/lib/format/time.test.ts src/lib/components/UsageGauge.test.ts --run
```

Expected: day cases and weekly relative copy fail.

- [ ] **Step 3: Implement the formatter and one gauge path**

Change `humanizeMinutes` to:

```ts
function humanizeMinutes(totalMinutes: number): string {
  if (totalMinutes < 60) return `${totalMinutes}m`;
  const totalHours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  if (totalHours < 24) return `${totalHours}h ${minutes}m`;
  const days = Math.floor(totalHours / 24);
  return `${days}d ${totalHours % 24}h ${minutes}m`;
}
```

Remove the absolute/relative prop branch from `UsageGauge` and remove
`resetFormat="absolute"` from the weekly `UsagePanel` call. Every valid reset
renders:

```svelte
<time datetime={usage.resetsAt}>resets in {resetLabel}</time>
```

- [ ] **Step 4: Run GREEN tests**

Run the Task 1 command again. Expected: all selected tests pass.

---

### Task 2: Compact source-free freshness and explicit primary action

**Files:**

- Modify: `src/lib/components/UsagePanel.test.ts`
- Modify: `src/lib/components/UsagePanel.svelte`
- Modify: `src/App.test.ts`
- Modify: `src/App.svelte`

**Interfaces:**

- Consumes: `onSelect(provider)` and `onPrimary(provider)` component callbacks.
- Produces: a tab selection that only changes selected provider and a button click that persists primary provider.

- [ ] **Step 1: Write failing component and App tests**

Update the panel expectation to:

```ts
expect(freshness.textContent?.replace(/\s+/g, ' ').trim()).toBe(
  '● Fresh · captured 2 min ago',
);
expect(freshness.textContent).not.toContain('oauth_api');
expect(getComputedStyle(freshness).whiteSpace).toBe('nowrap');
```

Update the App interaction test:

```ts
await fireEvent.click(await screen.findByRole('tab', { name: 'Codex' }));
expect(gateway.updateSettings).not.toHaveBeenCalled();
await fireEvent.click(screen.getByRole('button', { name: 'Set as primary' }));
await waitFor(() =>
  expect(gateway.updateSettings).toHaveBeenCalledWith(
    expect.objectContaining({ primaryProvider: 'codex' }),
  ),
);
```

- [ ] **Step 2: Run RED tests**

Run:

```bash
corepack pnpm test src/lib/components/UsagePanel.test.ts src/App.test.ts --run
```

Expected: collector source remains visible, nowrap is absent, and tab selection still writes settings.

- [ ] **Step 3: Implement minimal UI and interaction changes**

Render freshness without source/cache suffixes:

```svelte
<small class:stale={current.stale} class="freshness">
  ● {current.stale ? 'Stale' : 'Fresh'}
  {#if current.capturedAt && captured}
    <span>&nbsp;· captured <time datetime={current.capturedAt}>{captured}</time></span>
  {/if}
</small>
```

Use the compact UI-plan treatment:

```css
.freshness {
  overflow: hidden;
  color: var(--sev-ok);
  font-family: var(--font-mono);
  font-size: 0.6875rem;
  white-space: nowrap;
}
```

In `App.svelte`, make `onSelect` only call:

```ts
providersStore.selectTab(provider);
```

Leave `onPrimary` on the existing immutable `changeSettings` path.

- [ ] **Step 4: Run GREEN tests**

Run the Task 2 command again. Expected: all selected tests pass.

---

### Task 3: Browser regression and full verification

**Files:**

- Modify: `tests/e2e/renderer.spec.ts`
- Modify: `src/lib/api/fixtureGateway.ts`

**Interfaces:**

- Consumes: renderer fixture panel and fixture gateway.
- Produces: browser-level coverage for one-line freshness, explicit primary action, and relative weekly reset.

- [ ] **Step 1: Add failing renderer assertions**

In the panel flow, assert that:

```ts
const freshness = await $('.freshness');
expect(await freshness.getText()).toMatch(/^● Fresh · captured /);
expect(await freshness.getText()).not.toMatch(/oauth_api|cli_rpc|cached/);
```

Measure the line and click the explicit button:

```ts
const lineCount = await freshness.execute((element) => {
  const style = getComputedStyle(element);
  return {
    whiteSpace: style.whiteSpace,
    height: element.getBoundingClientRect().height,
    lineHeight: Number.parseFloat(style.lineHeight),
  };
});
expect(lineCount.whiteSpace).toBe('nowrap');

const codexTab = await $('button[role="tab"][aria-label="Codex"]');
await codexTab.click();
const primaryButton = await $('button=Set as primary');
expect(await primaryButton.isEnabled()).toBe(true);
await primaryButton.click();
await browser.waitUntil(async () => !(await primaryButton.isEnabled()));
```

Also assert the weekly gauge contains `resets in` when the fixture supplies a
future reset timestamp.

- [ ] **Step 2: Run renderer E2E**

Run the Windows Vite server on port 4173, then:

```bash
corepack pnpm exec wdio run ./wdio.browser.conf.ts --baseUrl http://127.0.0.1:4173
```

Expected: all renderer specs pass.

- [ ] **Step 3: Run full validation**

Run:

```bash
corepack pnpm test:ci
cargo test --manifest-path src-tauri/Cargo.toml --all-features
```

Expected: type checks, lint, formatting, 80%+ coverage, renderer build, and all
native tests pass.

- [ ] **Step 4: Review**

Run `git diff --check`, inspect only task-owned hunks, and request a read-only
code review focused on the three approved requirements.
