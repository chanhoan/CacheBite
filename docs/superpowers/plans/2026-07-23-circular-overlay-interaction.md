# Circular Overlay Interaction Implementation Plan

> Execute with test-driven development: add failing tests first, confirm the
> expected failures, implement the minimum behavior, then refactor and verify.

## Global constraints

- `5H` and `WK` render at 9 px with stronger contrast.
- The visual pet/ring remains unclipped.
- Pointer interaction is limited to a transparent circular surface.
- Drag still moves the overlay; single-click does nothing; double-click opens
  the panel.
- Speech-bubble click only dismisses the bubble.
- Closing the panel hides only that window through an authorized native
  `hide_panel` command.
- Do not modify or commit unrelated files.

## Task 1: Lock renderer behavior with failing tests

**Files**

- Modify: `src/lib/components/SplitUsageRing.test.ts`
- Modify: `src/lib/components/PetOverlay.test.ts`
- Modify: `src/lib/components/UsagePanel.test.ts`
- Modify: `src/lib/components/SpeechBubble.test.ts`
- Modify: `src/App.test.ts`
- Modify: `src/lib/api/gateway.test.ts`

Add assertions for the 9 px labels, circular hit surface, double-click opening,
single-click no-op, preserved dragging, bubble-only dismissal, panel close, and
the `hide_panel` gateway invocation. Run the focused Vitest files and confirm
the new assertions fail for the intended missing behavior.

## Task 2: Implement renderer interaction and styling

**Files**

- Modify: `src/lib/components/SplitUsageRing.svelte`
- Modify: `src/lib/components/PetOverlay.svelte`
- Modify: `src/lib/components/UsagePanel.svelte`
- Modify: `src/lib/components/SpeechBubble.svelte`
- Modify: `src/App.svelte`
- Modify: `src/lib/api/gateway.ts`
- Modify: `src/lib/api/fixtureGateway.ts`

Implement the tested behavior. Keep pointer state management in `App.svelte`,
attach it only to the circular surface, use `dblclick` for panel opening, and
wire the panel close control to `gateway.hidePanel()`. Run the focused tests
until green.

## Task 3: Add the native hide-panel boundary

**Files**

- Modify: `src-tauri/src/window/mod.rs`
- Modify: `src-tauri/src/window/tests.rs`
- Modify: `src-tauri/src/refresh/ipc.rs`
- Modify: `src-tauri/src/lib.rs`

Add `NativeCommand::HidePanel`, authorize it only for the panel window, register
the Tauri command, and hide the invoking panel window. Add the authorization
tests first, confirm failure, implement, then run focused Rust tests.

## Task 4: Update E2E behavior and verify

**Files**

- Modify: `tests/e2e/native.spec.ts`

Change panel-opening interactions from click to double-click. Run formatting,
type checks, focused unit tests, Rust tests, and the renderer build. Review the
complete diff for scope and regressions.
