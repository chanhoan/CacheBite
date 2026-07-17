import type { AppSettings } from '../api/gateway';
import type { PanelProviderModel } from '../components/panelModels';
import { derivePetUiState, type ProviderState } from './engine';

export type SettingsStoreState = Pick<
  AppSettings,
  | 'primaryProvider'
  | 'bubblesEnabled'
  | 'startAtLogin'
  | 'notificationsEnabled'
  | 'secondaryNotificationsEnabled'
>;

export function toSettingsStoreState(
  settings: AppSettings,
): SettingsStoreState {
  return {
    primaryProvider: settings.primaryProvider,
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
