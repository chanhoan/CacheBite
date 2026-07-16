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

export type PointerRelease =
  | { readonly kind: 'toggle_panel' }
  | { readonly kind: 'finish_drag'; readonly position: PointerPoint };

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

export function releasePointer(state: PetPointerState): PointerRelease {
  return state.dragging
    ? { kind: 'finish_drag', position: { ...state.current } }
    : { kind: 'toggle_panel' };
}
