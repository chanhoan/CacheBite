# Overlay Toast Notification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the speech-bubble visual with a readable, tail-free toast below the circular usage gauge without overlap or clipping.

**Architecture:** Keep the existing bubble policy and component props. Change only the overlay composition and presentation: an overlay-only stack owns the gauge/toast relationship, while the toast component owns readable wrapping. A fixture-only transition lets the real browser test measure the layout.

**Tech Stack:** Svelte 5, TypeScript, Vitest, Testing Library, WebdriverIO

## Global Constraints

- Work only in `C:\playground\CacheBite\.worktrees\fix-speech-bubble-anchor`.
- Do not modify native Tauri commands, window sizes, policy timing, or settings wire formats.
- Keep the existing 128 px bundled pet unchanged.
- The toast must be below the gauge with a visible gap and no overlapping rectangles.
- The toast must remain inside the 240 x 240 overlay viewport.
- Text may wrap naturally to two or three lines without truncation or ellipsis.
- Prefer whole-word wrapping; use emergency character wrapping only for an oversized unbroken token.
- Preserve eight-second expiry and click-to-dismiss behavior.
- Follow RED → GREEN → REFACTOR and report the observed failure before implementation.
- Do not create commits; the user authorized implementation in the worktree, not commits.

---

### Task 1: Tail-free overlay toast

**Files:**
- Modify: `src/lib/components/SpeechBubble.test.ts`
- Modify: `src/lib/components/SpeechBubble.svelte`
- Modify: `src/App.test.ts`
- Modify: `src/App.svelte`
- Modify: `src/lib/styles/global.css`
- Modify: `src/lib/api/fixtureGateway.ts`
- Modify: `tests/e2e/renderer.spec.ts`

**Interfaces:**
- Consumes: `SpeechBubble` props `{ message: string; onDismiss?: () => void }`
- Produces: `[data-testid="overlay-toast"]` inside `.overlay-stack`
- Produces: renderer fixture query `?window=overlay&fixture=e2e&toast=layout`

- [ ] **Step 1: Write the failing toast presentation test**

Add a test to `SpeechBubble.test.ts` that renders a long message and asserts the
button has `data-testid="overlay-toast"`, class `toast`, and a child
`.toast-message` containing the complete untruncated string:

```ts
it('renders a tail-free overlay toast with the complete message', () => {
  const message =
    'Weekly usage is nearly exhausted and will reset after the current window';
  render(SpeechBubble, { props: { message } });

  const toast = screen.getByTestId('overlay-toast');
  expect(toast.classList).toContain('toast');
  expect(toast.classList).not.toContain('bubble');
  expect(toast.querySelector('.toast-message')?.textContent).toBe(message);
});
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```powershell
corepack pnpm vitest run src/lib/components/SpeechBubble.test.ts
```

Expected: FAIL because `overlay-toast` and `.toast-message` do not exist.

- [ ] **Step 3: Implement the tail-free toast**

Change `SpeechBubble.svelte` to:

```svelte
<button
  class="toast"
  data-testid="overlay-toast"
  aria-label={message}
  onclick={() => onDismiss()}
>
  <span class="toast-message">{message}</span>
</button>
```

Replace the existing `.bubble` and `.bubble::after` rules with these toast
styles:

```css
.toast {
  display: block;
  inline-size: fit-content;
  max-inline-size: calc(100vw - var(--space-4));
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--color-border);
  border-radius: 0.625rem;
  background: var(--color-surface);
  color: var(--color-text);
  box-shadow: var(--shadow-panel);
  font: inherit;
  font-size: 0.75rem;
  line-height: 1.4;
  text-align: center;
  white-space: normal;
  cursor: pointer;
}
.toast-message {
  display: block;
  overflow-wrap: anywhere;
  text-wrap: pretty;
  word-break: keep-all;
}
```

Do not add a pseudo-element or any other tail.

- [ ] **Step 4: Run the focused test and verify GREEN**

Run:

```powershell
corepack pnpm vitest run src/lib/components/SpeechBubble.test.ts
```

Expected: 3 tests pass.

- [ ] **Step 5: Write the failing composition test**

Extend the existing App bubble test after the exhausted transition:

```ts
const toast = await screen.findByTestId('overlay-toast');
const stack = toast.closest('.overlay-stack');
expect(stack).not.toBeNull();
expect(stack?.getAttribute('data-toast-visible')).toBe('true');
expect(stack?.querySelector('[data-testid="usage-ring"]')).not.toBeNull();
```

Run:

```powershell
corepack pnpm vitest run src/App.test.ts
```

Expected: FAIL because `.overlay-stack` and `data-toast-visible` do not exist.

- [ ] **Step 6: Group the gauge and toast in one overlay stack**

In the overlay branch of `App.svelte`, wrap `PetOverlay`, its package-error
fallback, and `SpeechBubble` in:

```svelte
<div
  class:toast-visible={Boolean($interactionStore.bubblePolicy.bubble)}
  class="overlay-stack"
  data-toast-visible={Boolean($interactionStore.bubblePolicy.bubble)}
>
  {#if overlayModel}
    <PetOverlay
      model={overlayModel}
      onPointerDown={pointerDown}
      onPointerMove={pointerMove}
      onPointerUp={pointerUp}
      onPointerCancel={pointerCancel}
      onOpen={() => void gateway.showPanel()}
    />
  {:else if petPackageError}
    <p role="status">Pet package unavailable</p>
  {/if}
  {#if $interactionStore.bubblePolicy.bubble}
    <SpeechBubble
      message={$interactionStore.bubblePolicy.bubble.message}
      onDismiss={() => interactionStore.dismissBubble()}
    />
  {/if}
</div>
```

Add component styles:

```css
.overlay-stack {
  display: grid;
  max-width: 100%;
  max-height: 100vh;
  align-items: center;
  justify-items: center;
  gap: var(--space-2);
}
.overlay-stack.toast-visible :global(.overlay) {
  max-width: min(10rem, 100vw);
}
```

Add this overlay-only rule to `src/lib/styles/global.css` so the fixed overlay
window uses its full area while panel styling remains unchanged:

```css
main[data-window-label='overlay'] {
  padding: 0;
}
```

- [ ] **Step 7: Run application and component tests**

Run:

```powershell
corepack pnpm vitest run src/App.test.ts src/lib/components/SpeechBubble.test.ts
```

Expected: both files pass and the existing expiry/click behavior remains green.

- [ ] **Step 8: Write the failing real-browser layout test**

Add a WebdriverIO test that opens
`/?window=overlay&fixture=e2e&toast=layout`, waits for
`[data-testid="overlay-toast"]`, replaces `.toast-message` text with a long
synthetic sentence, and returns these measurements from `browser.execute`:

```ts
const ring = document
  .querySelector<HTMLElement>('[data-testid="usage-ring"]')
  ?.getBoundingClientRect();
const toast = document
  .querySelector<HTMLElement>('[data-testid="overlay-toast"]')
  ?.getBoundingClientRect();
const message = document.querySelector<HTMLElement>('.toast-message');
```

Assert:

```ts
expect(layout.toastTop).toBeGreaterThanOrEqual(layout.ringBottom);
expect(layout.toastBottom).toBeLessThanOrEqual(layout.viewportHeight);
expect(layout.toastLeft).toBeGreaterThanOrEqual(0);
expect(layout.toastRight).toBeLessThanOrEqual(layout.viewportWidth);
expect(layout.messageScrollWidth).toBeLessThanOrEqual(
  layout.messageClientWidth,
);
expect(layout.message).toBe(layout.expectedMessage);
```

Run:

```powershell
corepack pnpm test:e2e:renderer
```

Expected: FAIL because the renderer fixture never emits a transition that
creates the toast.

- [ ] **Step 9: Add a fixture-only transition and verify GREEN**

Change `rendererFixtureGateway.listenProviderStates` to the following. This
transition occurs after the initial 91% state and therefore creates the existing
exhausted message through the real policy:

```ts
listenProviderStates: async (next) => {
  if (
    new URLSearchParams(window.location.search).get('toast') === 'layout'
  ) {
    queueMicrotask(() => {
      const exhausted = provider('claude');
      next({
        ...exhausted,
        revision: 2,
        snapshot: exhausted.snapshot
          ? {
              ...exhausted.snapshot,
              revision: 2,
              session: {
                ...exhausted.snapshot.session,
                used_percent: 100,
              },
            }
          : null,
      });
    });
  }
  return () => undefined;
},
```

Run:

```powershell
corepack pnpm test:e2e:renderer
```

Expected: all renderer E2E tests pass, including rectangle and overflow checks.

- [ ] **Step 10: Run full verification**

Run:

```powershell
corepack pnpm test
corepack pnpm check
corepack pnpm lint
corepack pnpm build
git diff --check
git status --short
```

Expected: all commands exit 0; status contains only the design, plan, and
intended renderer/test changes.

- [ ] **Step 11: Self-review**

Inspect the complete diff and confirm:

- no `::before` or `::after` speech tail remains;
- no native Rust or Tauri configuration changed;
- the default fixture flow remains unchanged without `toast=layout`;
- expiry and click dismissal tests remain intact;
- no unrelated files changed.
