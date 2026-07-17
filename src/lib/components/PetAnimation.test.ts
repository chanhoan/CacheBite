import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import PetAnimation from './PetAnimation.svelte';

const frames = (duration = 100) => ({
  type: 'frames' as const,
  sources: ['/pets/frame-01.png', '/pets/frame-02.png'],
  frameDurationMs: duration,
});

const image = { type: 'image' as const, source: '/pets/sleep.png' };

describe('PetAnimation', () => {
  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it('owns a timer for frame animations and resets it when the animation changes', async () => {
    vi.useFakeTimers();
    const { rerender, unmount } = render(PetAnimation, {
      props: { animation: frames(), label: 'Pet' },
    });
    const pet = screen.getByRole('img', { name: 'Pet' });

    await vi.advanceTimersByTimeAsync(100);
    expect(pet.getAttribute('src')).toContain('frame-02.png');

    await rerender({ animation: image, label: 'Pet' });
    expect(pet.getAttribute('src')).toContain('sleep.png');
    await vi.advanceTimersByTimeAsync(500);
    expect(pet.getAttribute('src')).toContain('sleep.png');

    await rerender({ animation: frames(50), label: 'Pet' });
    expect(pet.getAttribute('src')).toContain('frame-01.png');
    await vi.advanceTimersByTimeAsync(49);
    expect(pet.getAttribute('src')).toContain('frame-01.png');
    await vi.advanceTimersByTimeAsync(1);
    expect(pet.getAttribute('src')).toContain('frame-02.png');

    unmount();
    await vi.advanceTimersByTimeAsync(500);
    expect(vi.getTimerCount()).toBe(0);
  });

  it('does not create a timer for a single frame animation', async () => {
    vi.useFakeTimers();
    render(PetAnimation, {
      props: {
        animation: {
          type: 'frames',
          sources: ['/pets/idle.png'],
          frameDurationMs: 100,
        },
        label: 'Pet',
      },
    });

    expect(vi.getTimerCount()).toBe(0);
  });
});
