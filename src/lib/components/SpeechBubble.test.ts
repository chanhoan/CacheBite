import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import SpeechBubble from './SpeechBubble.svelte';

describe('SpeechBubble', () => {
  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });
  it('dismisses after eight seconds', async () => {
    vi.useFakeTimers();
    const dismiss = vi.fn();
    render(SpeechBubble, {
      props: {
        message: 'Almost full',
        onDismiss: dismiss,
        onOpenPanel: vi.fn(),
      },
    });
    await vi.advanceTimersByTimeAsync(8000);
    expect(dismiss).toHaveBeenCalledOnce();
  });
  it('opens the panel and dismisses when clicked', async () => {
    const dismiss = vi.fn();
    const open = vi.fn();
    render(SpeechBubble, {
      props: { message: 'Reset', onDismiss: dismiss, onOpenPanel: open },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Reset' }));
    expect(open).toHaveBeenCalledOnce();
    expect(dismiss).toHaveBeenCalledOnce();
  });
});
