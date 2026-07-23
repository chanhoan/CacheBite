import { mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { tauriGateway } from './gateway';

const windowMock = vi.hoisted(() => ({
  moved: undefined as
    | ((event: { payload: { x: number; y: number } }) => Promise<void>)
    | undefined,
  scaleFactor: vi.fn<() => Promise<number>>(),
  unlisten: vi.fn(),
}));

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    onMoved: vi.fn(
      async (
        listener: (event: {
          payload: { x: number; y: number };
        }) => Promise<void>,
      ) => {
        windowMock.moved = listener;
        return windowMock.unlisten;
      },
    ),
    scaleFactor: windowMock.scaleFactor,
    startDragging: vi.fn(),
  }),
}));

const settingsWire = {
  schema_version: 3,
  primary_provider: 'claude',
  selected_pet_id: 'pet',
  bubble_enabled: true,
  start_at_login: false,
  notification_enabled: false,
  secondary_notification_enabled: false,
  logical_position: { x: 1, y: 2 },
} as const;

describe('typed Tauri gateway', () => {
  beforeEach(() => {
    Object.assign(
      (
        window as unknown as Window & {
          __TAURI_INTERNALS__: {
            convertFileSrc(path: string, protocol: string): string;
          };
        }
      ).__TAURI_INTERNALS__,
      {
        convertFileSrc: vi.fn(() => 'asset://localhost/pets/pet/'),
      },
    );
    windowMock.moved = undefined;
    windowMock.scaleFactor.mockReset();
    windowMock.unlisten.mockReset();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('maps settings in both directions', async () => {
    const calls: Array<[string, unknown]> = [];
    mockIPC((command, args) => {
      calls.push([command, args]);
      if (command === 'get_settings') return settingsWire;
      if (command === 'update_settings')
        return (args as { settings: typeof settingsWire }).settings;
      throw new Error(command);
    });
    const loaded = await tauriGateway.getSettings();
    expect(loaded).toMatchObject({
      primaryProvider: 'claude',
      selectedPetId: 'pet',
    });
    await tauriGateway.updateSettings({
      ...loaded,
      notificationsEnabled: true,
    });
    expect(calls[1]?.[1]).toMatchObject({
      settings: { notification_enabled: true, selected_pet_id: 'pet' },
    });
  });

  it('maps bounded history and delegates narrow commands', async () => {
    const invoked = vi.fn();
    mockIPC((command, args) => {
      invoked(command, args);
      if (command === 'get_history')
        return {
          claude: {
            samples: [
              {
                captured_at: '2026-07-17T00:00:00Z',
                session: { used_percent: 90, starts_new_segment: true },
                weekly: null,
              },
            ],
          },
          codex: { samples: [] },
        };
      if (command === 'get_collector_mode')
        return { claude: 'fixture', codex: 'fixture' };
      if (command === 'get_provider_states') return { claude: {}, codex: {} };
      if (command === 'get_platform_capabilities')
        return {
          os: 'windows',
          always_on_top: { status: 'available' },
          fullscreen_detection: { status: 'available' },
          autostart: { status: 'available' },
        };
      if (command === 'get_pet_package')
        return {
          manifest: { id: 'pet' },
          asset_base_url: 'asset://localhost/pets/pet/',
        };
      return null;
    });
    expect((await tauriGateway.getHistory()).claude[0]).toMatchObject({
      capturedAt: '2026-07-17T00:00:00Z',
      session: { usedPercent: 90, startsNewSegment: true },
    });
    expect(await tauriGateway.getCollectorMode()).toEqual({
      claude: 'fixture',
      codex: 'fixture',
    });
    await tauriGateway.getProviderStates();
    expect(await tauriGateway.getPlatformCapabilities()).toMatchObject({
      os: 'windows',
      autostart: { status: 'available' },
    });
    expect(await tauriGateway.getPetPackage()).toEqual({
      manifest: { id: 'pet' },
      assetBaseUrl: 'asset://localhost/pets/pet/',
    });
    await tauriGateway.refreshProvider('codex');
    await tauriGateway.showPanel();
    await tauriGateway.hidePanel();
    expect(invoked).toHaveBeenCalledWith('refresh_provider', {
      provider: 'codex',
    });
    expect(invoked).toHaveBeenCalledWith('show_panel', {});
    expect(invoked).toHaveBeenCalledWith('hide_panel', {});
  });

  it('converts every move with its current scale factor and replaces the save timer', async () => {
    vi.useFakeTimers();
    const invoked = vi.fn();
    mockIPC((command, args) => invoked(command, args));
    windowMock.scaleFactor.mockResolvedValueOnce(2).mockResolvedValueOnce(4);
    const moved = vi.fn();

    const cleanup = await tauriGateway.listenPositionMoved(moved);
    await windowMock.moved?.({ payload: { x: 20, y: 40 } });
    await vi.advanceTimersByTimeAsync(200);
    await windowMock.moved?.({ payload: { x: 80, y: 120 } });

    expect(moved).toHaveBeenNthCalledWith(1, { x: 10, y: 20 });
    expect(moved).toHaveBeenNthCalledWith(2, { x: 20, y: 30 });
    await vi.advanceTimersByTimeAsync(249);
    expect(invoked).not.toHaveBeenCalledWith(
      'save_position',
      expect.anything(),
    );
    await vi.advanceTimersByTimeAsync(1);
    expect(invoked).toHaveBeenCalledTimes(1);
    expect(invoked).toHaveBeenCalledWith('save_position', {
      position: { x: 20, y: 30 },
    });
    cleanup();
  });

  it('discards an older move when its scale factor resolves after a newer move', async () => {
    vi.useFakeTimers();
    let resolveFirst!: (value: number) => void;
    windowMock.scaleFactor
      .mockReturnValueOnce(
        new Promise((resolve) => {
          resolveFirst = resolve;
        }),
      )
      .mockResolvedValueOnce(2);
    const invoked = vi.fn();
    mockIPC((command, args) => invoked(command, args));
    const moved = vi.fn();
    const cleanup = await tauriGateway.listenPositionMoved(moved);

    const first = windowMock.moved?.({ payload: { x: 100, y: 120 } });
    await windowMock.moved?.({ payload: { x: 40, y: 60 } });
    resolveFirst(10);
    await first;
    await vi.advanceTimersByTimeAsync(250);

    expect(moved).toHaveBeenCalledOnce();
    expect(moved).toHaveBeenCalledWith({ x: 20, y: 30 });
    expect(invoked).toHaveBeenCalledOnce();
    expect(invoked).toHaveBeenCalledWith('save_position', {
      position: { x: 20, y: 30 },
    });
    cleanup();
  });

  it.each([0, Number.NaN, Number.POSITIVE_INFINITY])(
    'rejects invalid scale factor %s without publishing or saving a position',
    async (scaleFactor) => {
      vi.useFakeTimers();
      const invoked = vi.fn();
      mockIPC((command, args) => invoked(command, args));
      windowMock.scaleFactor.mockResolvedValue(scaleFactor);
      const moved = vi.fn();
      const failure = vi.fn();
      const cleanup = await tauriGateway.listenPositionMoved(moved, failure);

      await windowMock.moved?.({ payload: { x: 12, y: 18 } });
      await vi.advanceTimersByTimeAsync(250);

      expect(moved).not.toHaveBeenCalled();
      expect(invoked).not.toHaveBeenCalled();
      expect(failure).toHaveBeenCalledOnce();
      expect(failure).toHaveBeenCalledWith('position_save_failed');
      cleanup();
    },
  );

  it.each([
    { x: Number.NaN, y: 18 },
    { x: 12, y: Number.POSITIVE_INFINITY },
  ])(
    'rejects invalid physical position $x,$y without publishing or saving it',
    async (payload) => {
      vi.useFakeTimers();
      const invoked = vi.fn();
      mockIPC((command, args) => invoked(command, args));
      windowMock.scaleFactor.mockResolvedValue(2);
      const moved = vi.fn();
      const failure = vi.fn();
      const cleanup = await tauriGateway.listenPositionMoved(moved, failure);

      await windowMock.moved?.({ payload });
      await vi.advanceTimersByTimeAsync(250);

      expect(moved).not.toHaveBeenCalled();
      expect(invoked).not.toHaveBeenCalled();
      expect(failure).toHaveBeenCalledOnce();
      expect(failure).toHaveBeenCalledWith('position_save_failed');
      cleanup();
    },
  );

  it('flushes the latest pending position and unlistens during cleanup', async () => {
    vi.useFakeTimers();
    const invoked = vi.fn();
    mockIPC((command, args) => invoked(command, args));
    windowMock.scaleFactor.mockResolvedValue(2);
    const cleanup = await tauriGateway.listenPositionMoved(vi.fn());

    await windowMock.moved?.({ payload: { x: 12, y: 18 } });
    cleanup();
    await vi.runAllTimersAsync();

    expect(invoked).toHaveBeenCalledTimes(1);
    expect(invoked).toHaveBeenCalledWith('save_position', {
      position: { x: 6, y: 9 },
    });
    expect(windowMock.unlisten).toHaveBeenCalledOnce();
  });

  it('reports a sanitized debounced position save failure', async () => {
    vi.useFakeTimers();
    mockIPC((command) => {
      if (command === 'save_position') throw new Error('write failed');
      return null;
    });
    windowMock.scaleFactor.mockResolvedValue(1);
    const failure = vi.fn();
    const cleanup = await tauriGateway.listenPositionMoved(vi.fn(), failure);

    await windowMock.moved?.({ payload: { x: 1, y: 2 } });
    await vi.advanceTimersByTimeAsync(250);

    expect(failure).toHaveBeenCalledOnce();
    expect(failure).toHaveBeenCalledWith('position_save_failed');
    cleanup();
  });

  it('reports a sanitized cleanup position save failure', async () => {
    vi.useFakeTimers();
    mockIPC((command) => {
      if (command === 'save_position') throw new Error('private raw details');
      return null;
    });
    windowMock.scaleFactor.mockResolvedValue(1);
    const failure = vi.fn();
    const cleanup = await tauriGateway.listenPositionMoved(vi.fn(), failure);

    await windowMock.moved?.({ payload: { x: 3, y: 4 } });
    cleanup();

    await vi.waitFor(() => expect(failure).toHaveBeenCalledOnce());
    expect(failure).toHaveBeenCalledWith('position_save_failed');
  });

  it('ignores a pending scale factor after idempotent cleanup', async () => {
    vi.useFakeTimers();
    let resolveScale!: (value: number) => void;
    windowMock.scaleFactor.mockReturnValue(
      new Promise((resolve) => {
        resolveScale = resolve;
      }),
    );
    const invoked = vi.fn();
    mockIPC((command, args) => invoked(command, args));
    const moved = vi.fn();
    const cleanup = await tauriGateway.listenPositionMoved(moved);

    const pendingMove = windowMock.moved?.({ payload: { x: 10, y: 20 } });
    cleanup();
    cleanup();
    resolveScale(2);
    await pendingMove;
    await vi.runAllTimersAsync();

    expect(moved).not.toHaveBeenCalled();
    expect(invoked).not.toHaveBeenCalled();
    expect(windowMock.unlisten).toHaveBeenCalledOnce();
  });
});
