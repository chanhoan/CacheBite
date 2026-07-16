import { writable } from 'svelte/store';
import type { Provider } from '../contracts/domain';

export interface SettingsState {
  readonly primaryProvider: Provider;
  readonly bubblesEnabled: boolean;
  readonly startAtLogin: boolean;
  readonly notificationsEnabled: boolean;
  readonly secondaryNotificationsEnabled: boolean;
}
export const defaultSettings: SettingsState = Object.freeze({
  primaryProvider: 'claude',
  bubblesEnabled: true,
  startAtLogin: false,
  notificationsEnabled: false,
  secondaryNotificationsEnabled: false,
});
export function createSettingsStore(initial: SettingsState = defaultSettings) {
  const { subscribe, update, set } = writable(Object.freeze({ ...initial }));
  return {
    subscribe,
    replace(value: SettingsState) {
      set(Object.freeze({ ...value }));
    },
    setPrimary(primaryProvider: Provider) {
      update((state) => Object.freeze({ ...state, primaryProvider }));
    },
    setBubbles(bubblesEnabled: boolean) {
      update((state) => Object.freeze({ ...state, bubblesEnabled }));
    },
    setStartAtLogin(startAtLogin: boolean) {
      update((state) => Object.freeze({ ...state, startAtLogin }));
    },
    setNotifications(notificationsEnabled: boolean) {
      update((state) => Object.freeze({ ...state, notificationsEnabled }));
    },
    setSecondaryNotifications(secondaryNotificationsEnabled: boolean) {
      update((state) =>
        Object.freeze({ ...state, secondaryNotificationsEnabled }),
      );
    },
  };
}
