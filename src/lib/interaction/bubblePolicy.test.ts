import { describe, expect, it } from 'vitest';
import {
  BUBBLE_DISMISS_MS,
  createBubblePolicy,
  expireBubble,
  reduceBubble,
} from './bubblePolicy';

const context = {
  primary: 'claude' as const,
  enabled: true,
  dragging: false,
  fullscreen: false,
  nowMs: 0,
};

describe('bubble policy', () => {
  it('deduplicates severity until that provider window resets', () => {
    const first = reduceBubble(
      createBubblePolicy(),
      {
        kind: 'severity_raised',
        provider: 'claude',
        window: 'session',
        severity: 'warn',
      },
      context,
    );
    const duplicate = reduceBubble(
      first,
      {
        kind: 'severity_raised',
        provider: 'claude',
        window: 'session',
        severity: 'warn',
      },
      context,
    );
    const reset = reduceBubble(
      duplicate,
      { kind: 'window_reset', provider: 'claude', window: 'session' },
      context,
    );
    const repeated = reduceBubble(
      { ...reset, bubble: null },
      {
        kind: 'severity_raised',
        provider: 'claude',
        window: 'session',
        severity: 'warn',
      },
      context,
    );
    expect(duplicate).toBe(first);
    expect(repeated.bubble?.kind).toBe('severity_raised');
  });

  it('drops events without queueing during drag/fullscreen or for secondary provider', () => {
    for (const blocked of [
      { ...context, dragging: true },
      { ...context, fullscreen: true },
    ]) {
      expect(
        reduceBubble(
          createBubblePolicy(),
          { kind: 'auth_required', provider: 'claude' },
          blocked,
        ).bubble,
      ).toBeNull();
    }
    expect(
      reduceBubble(
        createBubblePolicy(),
        { kind: 'auth_required', provider: 'codex' },
        context,
      ).bubble,
    ).toBeNull();
  });

  it('prioritizes exhausted over lower priority recovery and expires at eight seconds', () => {
    const exhausted = reduceBubble(
      createBubblePolicy(),
      {
        kind: 'severity_raised',
        provider: 'claude',
        window: 'weekly',
        severity: 'exhausted',
      },
      context,
    );
    const recovery = reduceBubble(
      exhausted,
      { kind: 'recovered', provider: 'claude' },
      { ...context, nowMs: 1 },
    );
    expect(recovery).toBe(exhausted);
    expect(exhausted.bubble?.expiresAt).toBe(8000);
  });

  it('returns the same state when the bubble has not expired yet', () => {
    const shown = reduceBubble(
      createBubblePolicy(),
      { kind: 'auth_required', provider: 'claude' },
      context,
    );
    expect(expireBubble(shown, BUBBLE_DISMISS_MS - 1)).toBe(shown);
  });

  it('clears the bubble once the dismissal deadline is reached', () => {
    const shown = reduceBubble(
      createBubblePolicy(),
      { kind: 'auth_required', provider: 'claude' },
      context,
    );
    const expired = expireBubble(shown, BUBBLE_DISMISS_MS);
    expect(expired.bubble).toBeNull();
    expect(expired.dedupe).toBe(shown.dedupe);
  });

  it('is a no-op when no bubble is showing', () => {
    const empty = createBubblePolicy();
    expect(expireBubble(empty, BUBBLE_DISMISS_MS * 10)).toBe(empty);
  });
});
