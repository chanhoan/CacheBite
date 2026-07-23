import type { AppGateway, ProviderBackendStateWire } from './gateway';

const provider = (name: 'claude' | 'codex'): ProviderBackendStateWire => ({
  provider: name,
  revision: 1,
  snapshot: {
    provider: name,
    plan_type: 'Fixture Pro',
    session: {
      used_percent: name === 'claude' ? 91 : 22,
      window_minutes: 300,
      resets_at: null,
    },
    weekly: { used_percent: 48, window_minutes: 10_080, resets_at: null },
    captured_at: '2026-07-17T00:00:00Z',
    source: name === 'claude' ? 'oauth_api' : 'cli_rpc',
    is_cached: false,
    revision: 1,
  },
  failure_class: null,
  unavailable_reason: null,
  expired: false,
  reset_pending: false,
});

export const rendererFixtureGateway: AppGateway = {
  getCollectorMode: async () => ({ claude: 'fixture', codex: 'fixture' }),
  getProviderStates: async () => ({
    claude: provider('claude'),
    codex: provider('codex'),
  }),
  listenProviderStates: async () => () => undefined,
  getSettings: async () => ({
    schemaVersion: 3,
    primaryProvider: 'claude',
    selectedPetId: 'fixture-pet',
    bubblesEnabled: true,
    startAtLogin: false,
    notificationsEnabled: false,
    secondaryNotificationsEnabled: false,
    logicalPosition: { x: 0, y: 0 },
  }),
  listenSettings: async () => () => undefined,
  getPetPackage: async () => ({
    manifest: {
      id: 'fixture-pet',
      displayName: 'Fixture Pet',
      defaultSize: { width: 160, height: 160 },
      animations: { idle: { type: 'image', source: 'idle.svg' } },
      states: {},
    },
    assetBaseUrl: 'asset://localhost/pets/fixture-pet/',
  }),
  getPlatformCapabilities: async () => ({
    os: 'linux',
    always_on_top: { status: 'available' },
    fullscreen_detection: { status: 'available' },
    autostart: { status: 'available' },
  }),
  updateSettings: async (settings) => settings,
  getHistory: async () => ({
    claude: [
      {
        capturedAt: '2026-07-17T00:00:00Z',
        session: { usedPercent: 91, startsNewSegment: false },
        weekly: null,
      },
    ],
    codex: [],
  }),
  refreshProvider: async () => undefined,
  startDragging: async () => undefined,
  listenPositionMoved: async () => () => undefined,
  showPanel: async () => undefined,
  hidePanel: async () => undefined,
  quit: async () => undefined,
};
