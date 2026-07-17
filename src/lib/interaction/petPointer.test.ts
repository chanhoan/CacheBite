import { describe, expect, it } from 'vitest';
import { beginPointer, releasePointer, updatePointer } from './petPointer';

describe('pet pointer policy', () => {
  it('toggles the panel below the four pixel boundary', () => {
    const state = updatePointer(beginPointer({ x: 10, y: 10 }), {
      x: 13.99,
      y: 10,
    });
    expect(releasePointer(state)).toEqual({ kind: 'toggle_panel' });
  });

  it('starts and completes dragging at exactly four pixels', () => {
    const state = updatePointer(beginPointer({ x: -4, y: 2 }), { x: 0, y: 2 });
    expect(state.dragging).toBe(true);
    expect(releasePointer(state)).toEqual({
      kind: 'finish_drag',
      position: { x: 0, y: 2 },
    });
  });

  it('uses radial distance and keeps immutable snapshots', () => {
    const started = beginPointer({ x: 0, y: 0 });
    const updated = updatePointer(started, { x: 3, y: 3 });
    expect(updated.dragging).toBe(true);
    expect(started).toEqual({
      origin: { x: 0, y: 0 },
      current: { x: 0, y: 0 },
      dragging: false,
    });
  });
});
