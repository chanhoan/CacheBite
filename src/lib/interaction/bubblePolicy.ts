import type { Provider } from '../contracts/domain';
import {
  eventMessage,
  eventPriority,
  reduceEventEligibility,
  type UsageTransitionEvent,
} from './eventPolicy';

export const BUBBLE_DISMISS_MS = 8_000;
export type BubbleEvent = UsageTransitionEvent;
export interface Bubble {
  readonly kind: BubbleEvent['kind'];
  readonly message: string;
  readonly priority: number;
  readonly expiresAt: number;
}
export interface BubblePolicyState {
  readonly bubble: Bubble | null;
  readonly dedupe: ReadonlySet<string>;
}
export interface BubbleContext {
  readonly primary: Provider;
  readonly enabled: boolean;
  readonly dragging: boolean;
  readonly fullscreen: boolean;
  readonly nowMs: number;
}

export const createBubblePolicy = (): BubblePolicyState => ({
  bubble: null,
  dedupe: new Set(),
});
export function expireBubble(
  state: BubblePolicyState,
  nowMs: number,
): BubblePolicyState {
  if (state.bubble === null || nowMs < state.bubble.expiresAt) return state;
  return { ...state, bubble: null };
}
export function reduceBubble(
  state: BubblePolicyState,
  event: BubbleEvent,
  context: BubbleContext,
): BubblePolicyState {
  const nextPriority = eventPriority(event);
  const result = reduceEventEligibility(
    state.dedupe,
    event,
    { primary: context.primary, enabled: context.enabled },
    () =>
      !context.dragging &&
      !context.fullscreen &&
      (state.bubble === null || nextPriority >= state.bubble.priority),
  );
  if (!result.accepted)
    return result.dedupe === state.dedupe
      ? state
      : { ...state, dedupe: result.dedupe };
  return {
    dedupe: result.dedupe,
    bubble: {
      kind: event.kind,
      message: eventMessage(event),
      priority: nextPriority,
      expiresAt: context.nowMs + BUBBLE_DISMISS_MS,
    },
  };
}
