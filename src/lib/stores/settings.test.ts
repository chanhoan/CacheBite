import { get } from 'svelte/store';
import { describe, expect, it } from 'vitest';
import { createSettingsStore } from './settings';
import { createInteractionStore } from './interaction';

describe('settings and interaction stores', () => {
  it('updates settings with immutable snapshots', () => {
    const store = createSettingsStore();
    const initial = get(store);
    expect(initial.notificationsEnabled).toBe(false);
    expect(initial.secondaryNotificationsEnabled).toBe(false);
    store.setPrimary('codex');
    store.setBubbles(false);
    store.setStartAtLogin(true);
    store.setNotifications(true);
    store.setSecondaryNotifications(true);
    expect(get(store)).toEqual({
      primaryProvider: 'codex',
      bubblesEnabled: false,
      startAtLogin: true,
      notificationsEnabled: true,
      secondaryNotificationsEnabled: true,
    });
    expect(get(store)).not.toBe(initial);
    store.replace(initial);
    expect(get(store)).toEqual(initial);
  });
  it('tracks drag, fullscreen, and bubble dismissal independently', () => {
    const store = createInteractionStore();
    store.setDragging(true);
    store.setFullscreen(true);
    store.dismissBubble();
    expect(get(store)).toMatchObject({
      dragging: true,
      fullscreen: true,
      bubblePolicy: { bubble: null },
    });
  });
});
