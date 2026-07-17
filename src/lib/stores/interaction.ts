import { writable } from 'svelte/store';
import type { BubblePolicyState } from '../interaction/bubblePolicy';
import { createBubblePolicy } from '../interaction/bubblePolicy';
import {
  reduceBubble,
  type BubbleContext,
  type BubbleEvent,
} from '../interaction/bubblePolicy';

export interface InteractionState {
  readonly dragging: boolean;
  readonly fullscreen: boolean;
  readonly bubblePolicy: BubblePolicyState;
}
export function createInteractionStore() {
  const { subscribe, update } = writable<InteractionState>({
    dragging: false,
    fullscreen: false,
    bubblePolicy: createBubblePolicy(),
  });
  return {
    subscribe,
    setDragging(dragging: boolean) {
      update((state) => ({ ...state, dragging }));
    },
    setFullscreen(fullscreen: boolean) {
      update((state) => ({ ...state, fullscreen }));
    },
    dismissBubble() {
      update((state) => ({
        ...state,
        bubblePolicy: { ...state.bubblePolicy, bubble: null },
      }));
    },
    handleBubble(event: BubbleEvent, context: BubbleContext) {
      update((state) => ({
        ...state,
        bubblePolicy: reduceBubble(state.bubblePolicy, event, context),
      }));
    },
  };
}
