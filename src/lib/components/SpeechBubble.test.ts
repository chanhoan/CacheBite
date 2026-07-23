import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import SpeechBubble from './SpeechBubble.svelte';

describe('SpeechBubble', () => {
  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });
  it('leaves timed dismissal to the policy layer', async () => {
    vi.useFakeTimers();
    const dismiss = vi.fn();
    render(SpeechBubble, {
      props: {
        message: 'Almost full',
        onDismiss: dismiss,
      },
    });
    // A mount-scoped timer here could not be re-armed when the bubble is
    // replaced, so expiry is driven from `expiresAt` in App.svelte instead.
    await vi.advanceTimersByTimeAsync(60_000);
    expect(dismiss).not.toHaveBeenCalled();
  });
  it('only dismisses when clicked', async () => {
    const dismiss = vi.fn();
    render(SpeechBubble, {
      props: { message: 'Reset', onDismiss: dismiss },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Reset' }));
    expect(dismiss).toHaveBeenCalledOnce();
  });
});
