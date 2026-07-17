import { convertFileSrc } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type {
  Provider,
  FailureClass,
  UnavailableReason,
} from '../contracts/domain';
import type { ProviderUiSnapshotWire } from './providerSnapshot';
import { invokeNative } from './tauri';

export interface ProviderBackendStateWire {
  readonly provider: Provider;
  readonly revision: number;
  readonly snapshot: Omit<
    ProviderUiSnapshotWire,
    'failure_class' | 'unavailable_reason'
  > | null;
  readonly failure_class: FailureClass | null;
  readonly unavailable_reason: UnavailableReason | null;
  readonly expired: boolean;
  readonly reset_pending: boolean;
}
export interface AppSettings {
  readonly schemaVersion: number;
  readonly primaryProvider: Provider;
  readonly selectedPetId: string;
  readonly bubblesEnabled: boolean;
  readonly startAtLogin: boolean;
  readonly notificationsEnabled: boolean;
  readonly secondaryNotificationsEnabled: boolean;
  readonly logicalPosition: { readonly x: number; readonly y: number };
}
export interface HistoryPointModel {
  readonly usedPercent: number;
  readonly startsNewSegment: boolean;
}
export interface HistorySampleModel {
  readonly capturedAt: string;
  readonly session: HistoryPointModel | null;
  readonly weekly: HistoryPointModel | null;
}
export interface HistoryModels {
  readonly claude: readonly HistorySampleModel[];
  readonly codex: readonly HistorySampleModel[];
}
export interface PetPackageModel {
  readonly manifest: unknown;
  readonly assetBaseUrl: string;
}
export type CapabilityDiagnostic =
  | { readonly status: 'available' }
  | { readonly status: 'unavailable'; readonly reason: string };
export interface PlatformCapabilities {
  readonly os: 'macos' | 'windows' | 'linux';
  readonly always_on_top: CapabilityDiagnostic;
  readonly fullscreen_detection: CapabilityDiagnostic;
  readonly autostart: CapabilityDiagnostic;
}
export type CollectorMode = 'fixture' | 'production';
export interface CollectorModeDiagnostic {
  readonly claude: CollectorMode;
  readonly codex: CollectorMode;
}
export interface AppGateway {
  getCollectorMode(): Promise<CollectorModeDiagnostic>;
  getProviderStates(): Promise<{
    claude: ProviderBackendStateWire;
    codex: ProviderBackendStateWire;
  }>;
  listenProviderStates(
    next: (state: ProviderBackendStateWire) => void,
  ): Promise<() => void>;
  getSettings(): Promise<AppSettings>;
  listenSettings(next: (settings: AppSettings) => void): Promise<() => void>;
  getPetPackage(): Promise<PetPackageModel>;
  getPlatformCapabilities(): Promise<PlatformCapabilities>;
  updateSettings(settings: AppSettings): Promise<AppSettings>;
  getHistory(): Promise<HistoryModels>;
  refreshProvider(provider: Provider): Promise<void>;
  startDragging(): Promise<void>;
  listenPositionMoved(
    next: (position: AppSettings['logicalPosition']) => void,
    onSaveFailure?: (failure: 'position_save_failed') => void,
  ): Promise<() => void>;
  showPanel(): Promise<void>;
}

type SettingsWire = {
  schema_version: number;
  primary_provider: Provider;
  selected_pet_id: string;
  bubble_enabled: boolean;
  start_at_login: boolean;
  notification_enabled: boolean;
  secondary_notification_enabled: boolean;
  logical_position: { x: number; y: number };
};
type HistoryWire = {
  claude: { samples: HistorySampleWire[] };
  codex: { samples: HistorySampleWire[] };
};
type PetPackageWire = { manifest: unknown; asset_base_url: string };
type HistorySampleWire = {
  captured_at: string;
  session: HistoryPointWire | null;
  weekly: HistoryPointWire | null;
};
type HistoryPointWire = { used_percent: number; starts_new_segment: boolean };
const fromSettings = (wire: SettingsWire): AppSettings => ({
  schemaVersion: wire.schema_version,
  primaryProvider: wire.primary_provider,
  selectedPetId: wire.selected_pet_id,
  bubblesEnabled: wire.bubble_enabled,
  startAtLogin: wire.start_at_login,
  notificationsEnabled: wire.notification_enabled,
  secondaryNotificationsEnabled: wire.secondary_notification_enabled,
  logicalPosition: wire.logical_position,
});
const toSettings = (settings: AppSettings): SettingsWire => ({
  schema_version: settings.schemaVersion,
  primary_provider: settings.primaryProvider,
  selected_pet_id: settings.selectedPetId,
  bubble_enabled: settings.bubblesEnabled,
  start_at_login: settings.startAtLogin,
  notification_enabled: settings.notificationsEnabled,
  secondary_notification_enabled: settings.secondaryNotificationsEnabled,
  logical_position: settings.logicalPosition,
});
const historySamples = (samples: HistorySampleWire[]): HistorySampleModel[] =>
  samples.map((sample) => ({
    capturedAt: sample.captured_at,
    session: sample.session && {
      usedPercent: sample.session.used_percent,
      startsNewSegment: sample.session.starts_new_segment,
    },
    weekly: sample.weekly && {
      usedPercent: sample.weekly.used_percent,
      startsNewSegment: sample.weekly.starts_new_segment,
    },
  }));

export const tauriGateway: AppGateway = {
  getCollectorMode: () => invokeNative('get_collector_mode'),
  getProviderStates: () => invokeNative('get_provider_states'),
  async listenProviderStates(next) {
    return listen<ProviderBackendStateWire>('provider-state', (event) =>
      next(event.payload),
    );
  },
  async getSettings() {
    return fromSettings(await invokeNative<SettingsWire>('get_settings'));
  },
  async listenSettings(next) {
    return listen<SettingsWire>('settings-updated', (event) =>
      next(fromSettings(event.payload)),
    );
  },
  async getPetPackage() {
    const wire = await invokeNative<PetPackageWire>('get_pet_package');
    return {
      manifest: wire.manifest,
      assetBaseUrl: `${convertFileSrc(wire.asset_base_url).replace(/\/$/, '')}/`,
    };
  },
  getPlatformCapabilities: () => invokeNative('get_platform_capabilities'),
  async updateSettings(settings) {
    return fromSettings(
      await invokeNative<SettingsWire>('update_settings', {
        settings: toSettings(settings),
      }),
    );
  },
  async getHistory() {
    const wire = await invokeNative<HistoryWire>('get_history');
    return {
      claude: historySamples(wire.claude.samples),
      codex: historySamples(wire.codex.samples),
    };
  },
  refreshProvider: (provider) => invokeNative('refresh_provider', { provider }),
  startDragging: () => getCurrentWindow().startDragging(),
  async listenPositionMoved(next, onSaveFailure) {
    const appWindow = getCurrentWindow();
    let pendingPosition: AppSettings['logicalPosition'] | null = null;
    let saveTimer: ReturnType<typeof setTimeout> | null = null;
    let latestMove = 0;
    let disposed = false;
    const persistPending = () => {
      if (!pendingPosition) return;
      const position = pendingPosition;
      pendingPosition = null;
      void invokeNative('save_position', { position }).catch(() =>
        onSaveFailure?.('position_save_failed'),
      );
    };
    const unlisten = await appWindow.onMoved(async ({ payload }) => {
      const move = ++latestMove;
      const scaleFactor = await appWindow.scaleFactor();
      if (disposed || move !== latestMove) return;
      if (
        !Number.isFinite(scaleFactor) ||
        scaleFactor <= 0 ||
        !Number.isFinite(payload.x) ||
        !Number.isFinite(payload.y)
      ) {
        onSaveFailure?.('position_save_failed');
        return;
      }
      const position = {
        x: payload.x / scaleFactor,
        y: payload.y / scaleFactor,
      };
      next(position);
      pendingPosition = position;
      if (saveTimer) clearTimeout(saveTimer);
      saveTimer = setTimeout(() => {
        saveTimer = null;
        persistPending();
      }, 250);
    });
    return () => {
      if (disposed) return;
      disposed = true;
      if (saveTimer) clearTimeout(saveTimer);
      // Cleanup is synchronous: this starts the final native write but cannot
      // await it, so an immediate process exit may still lose the last move.
      persistPending();
      unlisten();
    };
  },
  showPanel: () => invokeNative('show_panel'),
};
