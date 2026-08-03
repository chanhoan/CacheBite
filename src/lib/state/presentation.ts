import type { AppSettings, PlatformCapabilities } from '../api/gateway';
import type { PanelProviderModel } from '../components/panelModels';
import { derivePetUiState, type ProviderState } from './engine';

export type SettingsStoreState = Pick<
  AppSettings,
  | 'primaryProvider'
  | 'selectedPetId'
  | 'bubblesEnabled'
  | 'startAtLogin'
  | 'notificationsEnabled'
  | 'secondaryNotificationsEnabled'
>;

/**
 * The hide/show shortcut as the running platform spells it. The binding itself
 * is a fixed native constant (`CommandOrControl+Shift+H`); only its rendering
 * differs. The OS comes from the native capability report, never from
 * `navigator` — the renderer does not sniff its own environment.
 */
export function hideShowHotkeyLabel(os: PlatformCapabilities['os']): string {
  return os === 'macos' ? 'Cmd+Shift+H' : 'Ctrl+Shift+H';
}

export function toSettingsStoreState(
  settings: AppSettings,
): SettingsStoreState {
  return {
    primaryProvider: settings.primaryProvider,
    selectedPetId: settings.selectedPetId,
    bubblesEnabled: settings.bubblesEnabled,
    startAtLogin: settings.startAtLogin,
    notificationsEnabled: settings.notificationsEnabled,
    secondaryNotificationsEnabled: settings.secondaryNotificationsEnabled,
  };
}

export function toProviderPresentation(
  state: ProviderState,
  nowMs: number,
): PanelProviderModel {
  const ui = derivePetUiState(state, nowMs);
  return {
    provider: state.provider,
    system: ui.system,
    stale: ui.stale,
    planType: state.snapshot?.planType ?? null,
    session: {
      usedPercent:
        ui.sessionSeverity === 'unknown'
          ? null
          : (state.snapshot?.session?.usedPercent ?? null),
      severity: ui.sessionSeverity,
      resetsAt: state.snapshot?.session?.resetsAt ?? null,
    },
    weekly: {
      usedPercent:
        ui.weeklySeverity === 'unknown'
          ? null
          : (state.snapshot?.weekly?.usedPercent ?? null),
      severity: ui.weeklySeverity,
      resetsAt: state.snapshot?.weekly?.resetsAt ?? null,
    },
    capturedAt: state.snapshot?.capturedAt ?? null,
    source:
      state.snapshot?.source ??
      (state.provider === 'claude' ? 'oauth_api' : 'cli_rpc'),
    isCached: state.snapshot?.isCached ?? false,
  };
}
