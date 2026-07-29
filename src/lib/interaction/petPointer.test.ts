import { describe, expect, it } from 'vitest';
import {
  beginPointer,
  pointerButtonsReleased,
  updatePointer,
} from './petPointer';

describe('pet pointer policy', () => {
  it('stays undragged below the four pixel boundary', () => {
    const state = updatePointer(beginPointer({ x: 10, y: 10 }), {
      x: 13.99,
      y: 10,
    });
    expect(state.dragging).toBe(false);
  });

  it('starts dragging at exactly four pixels', () => {
    const state = updatePointer(beginPointer({ x: -4, y: 2 }), { x: 0, y: 2 });
    expect(state.dragging).toBe(true);
    expect(state.current).toEqual({ x: 0, y: 2 });
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

  it('treats a button-free pointer event as the end of the gesture', () => {
    expect(pointerButtonsReleased(0)).toBe(true);
    expect(pointerButtonsReleased(1)).toBe(false);
    expect(pointerButtonsReleased(3)).toBe(false);
  });
});
