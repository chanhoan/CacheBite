export const DRAG_THRESHOLD_PX = 4;

export interface PointerPoint {
  readonly x: number;
  readonly y: number;
}

export interface PetPointerState {
  readonly origin: PointerPoint;
  readonly current: PointerPoint;
  readonly dragging: boolean;
}

export function beginPointer(origin: PointerPoint): PetPointerState {
  return { origin: { ...origin }, current: { ...origin }, dragging: false };
}

export function updatePointer(
  state: PetPointerState,
  current: PointerPoint,
): PetPointerState {
  const distance = Math.hypot(
    current.x - state.origin.x,
    current.y - state.origin.y,
  );
  return {
    ...state,
    current: { ...current },
    dragging: state.dragging || distance >= DRAG_THRESHOLD_PX,
  };
}

/**
 * A native window drag (`startDragging()`) hands the mouse loop to the OS, so
 * the webview may never receive `pointerup` for that gesture. Any later
 * pointer event with no button held is proof the gesture ended, and is the
 * signal that releases the drag latch when `pointerup` was swallowed.
 */
export function pointerButtonsReleased(buttons: number): boolean {
  return buttons === 0;
}
