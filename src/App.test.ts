import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import App from './App.svelte';
import type { AppGateway, ProviderBackendStateWire } from './lib/api/gateway';
import type { NotificationAdapter } from './lib/interaction/notificationPolicy';

const active = (
  provider: 'claude' | 'codex',
  revision = 1,
  usedPercent = provider === 'claude' ? 91 : 20,
): ProviderBackendStateWire => ({
  provider,
  revision,
  snapshot: {
    provider,
    plan_type: 'Pro',
    session: {
      used_percent: usedPercent,
      window_minutes: 300,
      resets_at: null,
    },
    weekly: { used_percent: 40, window_minutes: 10_080, resets_at: null },
    captured_at: new Date().toISOString(),
    source: provider === 'claude' ? 'oauth_api' : 'cli_rpc',
    is_cached: false,
    revision,
  },
  failure_class: null,
  unavailable_reason: null,
  expired: false,
  reset_pending: false,
});

const fixture = () => {
  let listener: (state: ProviderBackendStateWire) => void = () => undefined;
  let settingsListener: (
    settings: Awaited<ReturnType<AppGateway['getSettings']>>,
  ) => void = () => undefined;
  const gateway: AppGateway = {
    getCollectorMode: vi.fn().mockResolvedValue({
      claude: 'fixture',
      codex: 'fixture',
    }),
    getProviderStates: vi.fn(async () => ({
      claude: active('claude'),
      codex: active('codex'),
    })),
    listenProviderStates: vi.fn(async (next) => {
      listener = next;
      return () => undefined;
    }),
    getSettings: vi.fn(async () => ({
      schemaVersion: 3,
      primaryProvider: 'claude' as const,
      selectedPetId: 'cat',
      bubblesEnabled: true,
      startAtLogin: false,
      notificationsEnabled: false,
      secondaryNotificationsEnabled: false,
      logicalPosition: { x: 0, y: 0 },
    })),
    listenSettings: vi.fn(async (next) => {
      settingsListener = next;
      return () => undefined;
    }),
    getPetPackage: vi.fn(async () => ({
      manifest: {
        id: 'fixture-pet',
        displayName: 'Fixture Pet',
        defaultSize: { width: 160, height: 160 },
        animations: { idle: { type: 'image', source: 'idle.svg' } },
        states: {},
      },
      assetBaseUrl: 'asset://localhost/pets/fixture-pet/',
    })),
    getPlatformCapabilities: vi.fn(async () => ({
      os: 'linux' as const,
      always_on_top: { status: 'available' as const },
      fullscreen_detection: { status: 'available' as const },
      autostart: { status: 'available' as const },
    })),
    updateSettings: vi.fn(async (settings) => settings),
    getHistory: vi.fn(async () => ({
      claude: [
        {
          capturedAt: '2026-07-17T00:00:00Z',
          session: { usedPercent: 91, startsNewSegment: false },
          weekly: null,
        },
      ],
      codex: [],
    })),
    refreshProvider: vi.fn(async () => undefined),
    startDragging: vi.fn(async () => undefined),
    listenPositionMoved: vi.fn(async () => () => undefined),
    showPanel: vi.fn(async () => undefined),
  };
  return {
    gateway,
    emit: (state: ProviderBackendStateWire) => listener(state),
    emitSettings: (settings: Awaited<ReturnType<AppGateway['getSettings']>>) =>
      settingsListener(settings),
  };
};

const notifications: NotificationAdapter = {
  capability: vi.fn(async () => ({ status: 'available' as const })),
  permission: vi.fn(async () => 'granted' as const),
  requestPermission: vi.fn(async () => 'granted' as const),
  send: vi.fn(async () => undefined),
};

describe('application composition root', () => {
  afterEach(() => {
    cleanup();
    window.history.replaceState({}, '', '/');
  });

  it('hydrates overlay, ignores old revisions, and routes click pointer to panel', async () => {
    const { gateway, emit } = fixture();
    render(App, { props: { gateway, notificationAdapter: notifications } });
    expect(await screen.findByLabelText('CacheBite pet status')).toBeTruthy();
    expect(
      screen
        .getByLabelText('CacheBite')
        .getAttribute('data-collector-mode-claude'),
    ).toBe('fixture');
    expect(screen.queryByText('CacheBite is starting')).toBeNull();
    expect(gateway.getHistory).not.toHaveBeenCalled();
    emit(active('claude', 0));
    const overlay = screen.getByTestId('overlay-pointer-surface');
    await fireEvent.pointerDown(overlay, { clientX: 10, clientY: 10 });
    await fireEvent.pointerUp(overlay, { clientX: 12, clientY: 10 });
    expect(gateway.showPanel).toHaveBeenCalledOnce();
    expect(
      screen.getByLabelText('CacheBite').getAttribute('data-platform'),
    ).toBe('linux');
    expect(gateway.getPlatformCapabilities).toHaveBeenCalledOnce();
  });

  it('does not perform a forbidden settings write during overlay startup', async () => {
    const { gateway } = fixture();
    vi.mocked(gateway.getSettings).mockResolvedValue({
      ...(await gateway.getSettings()),
      primaryProvider: 'codex',
      selectedPetId: 'cat',
    });

    render(App, { props: { gateway, notificationAdapter: notifications } });

    await screen.findByLabelText('CacheBite pet status');
    expect(gateway.updateSettings).not.toHaveBeenCalled();
    expect(gateway.getPetPackage).toHaveBeenCalledOnce();
  });

  it('shows a Pet diagnostic instead of crashing on an invalid package root', async () => {
    const { gateway } = fixture();
    vi.mocked(gateway.getPetPackage).mockResolvedValue({
      ...(await gateway.getPetPackage()),
      assetBaseUrl: 'https://example.com/pets/cat/',
    });

    render(App, { props: { gateway, notificationAdapter: notifications } });

    expect(await screen.findByText('Pet package unavailable')).toBeTruthy();
    expect(screen.queryByText('CacheBite is starting')).toBeNull();
  });

  it('reloads the running overlay pet when another window changes the primary provider', async () => {
    const { gateway, emitSettings } = fixture();
    render(App, { props: { gateway, notificationAdapter: notifications } });
    await screen.findByLabelText('CacheBite pet status');
    await waitFor(() => expect(gateway.listenSettings).toHaveBeenCalledOnce());

    emitSettings({
      ...(await gateway.getSettings()),
      primaryProvider: 'codex',
      selectedPetId: 'corgi',
    });

    await waitFor(() => expect(gateway.getPetPackage).toHaveBeenCalledTimes(2));
  });

  it('renders a retry action when provider startup fails and recovers on retry', async () => {
    const { gateway } = fixture();
    vi.mocked(gateway.getProviderStates)
      .mockRejectedValueOnce(new Error('native startup failed'))
      .mockResolvedValueOnce({
        claude: active('claude'),
        codex: active('codex'),
      });

    render(App, { props: { gateway, notificationAdapter: notifications } });

    expect(await screen.findByText('CacheBite could not start')).toBeTruthy();
    expect(screen.queryByText('CacheBite is starting')).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'Retry' }));
    expect(await screen.findByLabelText('CacheBite pet status')).toBeTruthy();
    expect(gateway.getProviderStates).toHaveBeenCalledTimes(2);
  });

  it('becomes ready even when a nonessential native listener never resolves', async () => {
    const { gateway } = fixture();
    vi.mocked(gateway.listenProviderStates).mockImplementation(
      () => new Promise(() => undefined),
    );

    render(App, { props: { gateway, notificationAdapter: notifications } });

    expect(await screen.findByLabelText('CacheBite pet status')).toBeTruthy();
    expect(screen.queryByText('CacheBite is starting')).toBeNull();
  });

  it('cleans the provider listener without blocking readiness when position listener registration fails', async () => {
    const { gateway } = fixture();
    const providerCleanup = vi.fn();
    vi.mocked(gateway.listenProviderStates).mockResolvedValue(providerCleanup);
    vi.mocked(gateway.listenPositionMoved).mockRejectedValue(
      new Error('position listener failed'),
    );

    render(App, { props: { gateway, notificationAdapter: notifications } });

    expect(await screen.findByLabelText('CacheBite pet status')).toBeTruthy();
    await waitFor(() => expect(providerCleanup).toHaveBeenCalledOnce());
  });

  it('isolates listener cleanup failures after startup', async () => {
    const { gateway } = fixture();
    const order: string[] = [];
    const providerCleanup = vi.fn(() => {
      order.push('cleanup');
      throw new Error('provider cleanup failed');
    });
    vi.mocked(gateway.listenProviderStates)
      .mockResolvedValueOnce(providerCleanup)
      .mockResolvedValueOnce(() => undefined);
    vi.mocked(gateway.listenPositionMoved)
      .mockRejectedValueOnce(new Error('position listener failed'))
      .mockResolvedValueOnce(() => undefined);
    vi.mocked(gateway.getProviderStates).mockImplementation(async () => {
      order.push('startup');
      return { claude: active('claude'), codex: active('codex') };
    });

    render(App, { props: { gateway, notificationAdapter: notifications } });

    expect(await screen.findByLabelText('CacheBite pet status')).toBeTruthy();
    await waitFor(() => expect(order).toEqual(['startup', 'cleanup']));
  });

  it('cleans a provider listener that resolves after unmount', async () => {
    const { gateway } = fixture();
    let resolveListener!: (cleanup: () => void) => void;
    vi.mocked(gateway.listenProviderStates).mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveListener = resolve;
        }),
    );
    const view = render(App, {
      props: { gateway, notificationAdapter: notifications },
    });
    await waitFor(() =>
      expect(gateway.listenProviderStates).toHaveBeenCalled(),
    );
    view.unmount();
    const lateCleanup = vi.fn();

    resolveListener(lateCleanup);

    await waitFor(() => expect(lateCleanup).toHaveBeenCalledOnce());
  });

  it('attempts position cleanup even when provider cleanup throws', async () => {
    const { gateway } = fixture();
    const providerCleanup = vi.fn(() => {
      throw new Error('provider cleanup failed');
    });
    const positionCleanup = vi.fn();
    vi.mocked(gateway.listenProviderStates).mockResolvedValue(providerCleanup);
    vi.mocked(gateway.listenPositionMoved).mockResolvedValue(positionCleanup);
    const view = render(App, {
      props: { gateway, notificationAdapter: notifications },
    });
    await screen.findByLabelText('CacheBite pet status');
    await waitFor(() =>
      expect(gateway.listenPositionMoved).toHaveBeenCalledOnce(),
    );

    view.unmount();

    expect(providerCleanup).toHaveBeenCalledOnce();
    expect(positionCleanup).toHaveBeenCalledOnce();
  });

  it('shows a fixed diagnostic when position persistence fails', async () => {
    const { gateway } = fixture();
    let reportFailure!: (failure: 'position_save_failed') => void;
    vi.mocked(gateway.listenPositionMoved).mockImplementation(
      async (_next, failure) => {
        reportFailure = failure!;
        return () => undefined;
      },
    );
    render(App, { props: { gateway, notificationAdapter: notifications } });
    await screen.findByLabelText('CacheBite pet status');
    await waitFor(() =>
      expect(gateway.listenPositionMoved).toHaveBeenCalledOnce(),
    );

    reportFailure('position_save_failed');

    expect(
      await screen.findByText('Window position could not be saved'),
    ).toBeTruthy();
  });

  it('treats reset-pending backend state as unknown before retained snapshot data', async () => {
    window.history.replaceState({}, '', '/?window=panel');
    const { gateway } = fixture();
    const retained = active('claude', 2, 91);
    vi.mocked(gateway.getProviderStates).mockResolvedValue({
      claude: { ...retained, reset_pending: true },
      codex: active('codex'),
    });

    render(App, { props: { gateway, notificationAdapter: notifications } });

    expect(await screen.findByText('Pro')).toBeTruthy();
    expect(screen.getAllByText('Unknown').length).toBeGreaterThanOrEqual(2);
  });

  it('starts native dragging once when pointer movement crosses the threshold', async () => {
    const { gateway } = fixture();
    render(App, { props: { gateway, notificationAdapter: notifications } });
    const overlay = await screen.findByTestId('overlay-pointer-surface');
    const pointer = (type: string, x: number, y: number) => {
      const event = new Event(type, { bubbles: true });
      Object.defineProperties(event, {
        clientX: { value: x },
        clientY: { value: y },
      });
      return event;
    };
    await fireEvent(overlay, pointer('pointerdown', 10, 10));
    await fireEvent(overlay, pointer('pointermove', 20, 10));
    await fireEvent(overlay, pointer('pointermove', 30, 10));
    expect(gateway.startDragging).toHaveBeenCalledOnce();
  });

  it('hydrates panel history and persists settings through typed gateway', async () => {
    window.history.replaceState({}, '', '/?window=panel');
    const { gateway } = fixture();
    render(App, { props: { gateway, notificationAdapter: notifications } });
    expect(await screen.findByText('Pro')).toBeTruthy();
    expect(
      screen.getByRole('img', { name: '5-hour usage history' }),
    ).toBeTruthy();
    await fireEvent.click(screen.getByLabelText('Native notifications'));
    await waitFor(() => expect(gateway.updateSettings).toHaveBeenCalled());
    expect(gateway.getPetPackage).not.toHaveBeenCalled();
  });

  it('makes a provider primary when its tab is selected', async () => {
    window.history.replaceState({}, '', '/?window=panel');
    const { gateway } = fixture();
    render(App, { props: { gateway, notificationAdapter: notifications } });

    await fireEvent.click(await screen.findByRole('tab', { name: 'Codex' }));

    await waitFor(() =>
      expect(gateway.updateSettings).toHaveBeenCalledWith(
        expect.objectContaining({
          primaryProvider: 'codex',
          selectedPetId: 'corgi',
        }),
      ),
    );
    expect(screen.getByRole('tab', { name: 'Codex (primary)' })).toBeTruthy();
  });

  it('refreshes panel history after live provider revisions', async () => {
    window.history.replaceState({}, '', '/?window=panel');
    const { gateway, emit } = fixture();
    render(App, { props: { gateway, notificationAdapter: notifications } });
    await screen.findByText('Pro');
    emit(active('claude', 2, 95));
    await waitFor(() => expect(gateway.getHistory).toHaveBeenCalledTimes(2));
  });

  it('coalesces a burst of live history refreshes into one follow-up request', async () => {
    window.history.replaceState({}, '', '/?window=panel');
    const { gateway, emit } = fixture();
    let resolveLiveHistory!: (history: Awaited<ReturnType<AppGateway['getHistory']>>) => void;
    vi.mocked(gateway.getHistory)
      .mockResolvedValueOnce({ claude: [], codex: [] })
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveLiveHistory = resolve;
          }),
      )
      .mockResolvedValue({ claude: [], codex: [] });

    render(App, { props: { gateway, notificationAdapter: notifications } });
    await screen.findByText('Pro');
    await waitFor(() => expect(gateway.listenProviderStates).toHaveBeenCalledOnce());

    emit(active('claude', 2, 92));
    await waitFor(() => expect(gateway.getHistory).toHaveBeenCalledTimes(2));
    emit(active('claude', 3, 93));
    emit(active('claude', 4, 94));
    expect(gateway.getHistory).toHaveBeenCalledTimes(2);

    resolveLiveHistory({ claude: [], codex: [] });
    await waitFor(() => expect(gateway.getHistory).toHaveBeenCalledTimes(3));
  });

  it('reconciles persisted notification opt-in with granted permission', async () => {
    window.history.replaceState({}, '', '/?window=panel');
    const { gateway } = fixture();
    vi.mocked(gateway.getSettings).mockResolvedValue({
      ...(await gateway.getSettings()),
      notificationsEnabled: true,
    });
    render(App, { props: { gateway, notificationAdapter: notifications } });
    expect(
      (
        (await screen.findByLabelText(
          'Native notifications',
        )) as HTMLInputElement
      ).checked,
    ).toBe(true);
    expect(gateway.updateSettings).not.toHaveBeenCalled();
  });

  it('serializes live notification transitions without losing dedupe state', async () => {
    window.history.replaceState({}, '', '/?window=panel');
    const { gateway, emit } = fixture();
    vi.mocked(gateway.getSettings).mockResolvedValue({
      ...(await gateway.getSettings()),
      notificationsEnabled: true,
    });
    let activeSends = 0;
    let peakSends = 0;
    const serialized: NotificationAdapter = {
      ...notifications,
      send: vi.fn(async () => {
        activeSends += 1;
        peakSends = Math.max(peakSends, activeSends);
        await Promise.resolve();
        activeSends -= 1;
      }),
    };
    render(App, { props: { gateway, notificationAdapter: serialized } });
    await screen.findByText('Pro');
    await waitFor(() =>
      expect(gateway.listenProviderStates).toHaveBeenCalledOnce(),
    );
    emit(active('claude', 2, 20));
    emit(active('claude', 3, 91));
    emit(active('claude', 4, 100));
    await waitFor(() => expect(serialized.send).toHaveBeenCalledTimes(2));
    expect(peakSends).toBe(1);
  });

  it('shows permission denial diagnostic and rolls preference back', async () => {
    window.history.replaceState({}, '', '/?window=panel');
    const { gateway } = fixture();
    const denied = {
      ...notifications,
      permission: vi.fn(async () => 'denied' as const),
    };
    render(App, { props: { gateway, notificationAdapter: denied } });
    await fireEvent.click(await screen.findByLabelText('Native notifications'));
    expect(
      await screen.findByText('Notification permission denied'),
    ).toBeTruthy();
    await waitFor(() =>
      expect(
        (screen.getByLabelText('Native notifications') as HTMLInputElement)
          .checked,
      ).toBe(false),
    );
  });

  it('applies rapid settings responses in request order', async () => {
    window.history.replaceState({}, '', '/?window=panel');
    const { gateway } = fixture();
    let activeWrites = 0;
    let peakWrites = 0;
    vi.mocked(gateway.updateSettings)
      .mockImplementationOnce(async (settings) => {
        activeWrites += 1;
        peakWrites = Math.max(peakWrites, activeWrites);
        await new Promise((resolve) => setTimeout(resolve, 10));
        activeWrites -= 1;
        return settings;
      })
      .mockImplementationOnce(async (settings) => {
        activeWrites += 1;
        peakWrites = Math.max(peakWrites, activeWrites);
        activeWrites -= 1;
        return settings;
      });
    render(App, { props: { gateway, notificationAdapter: notifications } });
    const bubbles = (await screen.findByLabelText(
      'Speech bubbles',
    )) as HTMLInputElement;
    const secondary = screen.getByLabelText(
      'Include secondary provider notifications',
    ) as HTMLInputElement;

    await fireEvent.click(bubbles);
    await fireEvent.click(secondary);
    await waitFor(() =>
      expect(gateway.updateSettings).toHaveBeenCalledTimes(2),
    );
    await waitFor(() => expect(secondary.checked).toBe(true));
    expect(peakWrites).toBe(1);
    expect(gateway.updateSettings).toHaveBeenNthCalledWith(
      1,
      expect.objectContaining({ bubblesEnabled: false }),
    );
    expect(gateway.updateSettings).toHaveBeenNthCalledWith(
      2,
      expect.objectContaining({
        bubblesEnabled: false,
        secondaryNotificationsEnabled: true,
      }),
    );
  });

  it('rolls back a rejected settings write and saves later queued intent', async () => {
    window.history.replaceState({}, '', '/?window=panel');
    const { gateway } = fixture();
    vi.mocked(gateway.updateSettings)
      .mockRejectedValueOnce(new Error('/private/settings.json secret payload'))
      .mockImplementationOnce(async (settings) => settings);
    render(App, { props: { gateway, notificationAdapter: notifications } });
    const nativeNotifications = (await screen.findByLabelText(
      'Native notifications',
    )) as HTMLInputElement;

    await fireEvent.click(nativeNotifications);
    expect(await screen.findByText('Settings could not be saved')).toBeTruthy();
    await waitFor(() => expect(nativeNotifications.checked).toBe(false));

    const secondary = screen.getByLabelText(
      'Include secondary provider notifications',
    ) as HTMLInputElement;
    await fireEvent.click(secondary);

    await waitFor(() =>
      expect(gateway.updateSettings).toHaveBeenCalledTimes(2),
    );
    await waitFor(() =>
      expect(screen.queryByText('Settings could not be saved')).toBeNull(),
    );
    expect(gateway.updateSettings).toHaveBeenNthCalledWith(
      2,
      expect.objectContaining({
        notificationsEnabled: false,
        secondaryNotificationsEnabled: true,
      }),
    );
  });

  it('rolls back persisted notification opt-in denied at startup', async () => {
    window.history.replaceState({}, '', '/?window=panel');
    const { gateway } = fixture();
    vi.mocked(gateway.getSettings).mockResolvedValue({
      ...(await gateway.getSettings()),
      notificationsEnabled: true,
    });
    const denied = {
      ...notifications,
      permission: vi.fn(async () => 'denied' as const),
    };
    render(App, { props: { gateway, notificationAdapter: denied } });
    expect(
      await screen.findByText('Notification permission denied'),
    ).toBeTruthy();
    expect(gateway.updateSettings).toHaveBeenCalledWith(
      expect.objectContaining({ notificationsEnabled: false }),
    );
  });

  it('surfaces unavailable platform capabilities and disables autostart', async () => {
    window.history.replaceState({}, '', '/?window=panel');
    const { gateway } = fixture();
    vi.mocked(gateway.getPlatformCapabilities).mockResolvedValue({
      os: 'linux',
      always_on_top: { status: 'available' },
      fullscreen_detection: {
        status: 'unavailable',
        reason: 'fullscreen detection unavailable',
      },
      autostart: {
        status: 'unavailable',
        reason: 'autostart unavailable',
      },
    });
    render(App, { props: { gateway, notificationAdapter: notifications } });
    expect(
      await screen.findByText('fullscreen detection unavailable'),
    ).toBeTruthy();
    expect(screen.getByText('autostart unavailable')).toBeTruthy();
    expect(
      (screen.getByLabelText('Start at login') as HTMLInputElement).disabled,
    ).toBe(true);
  });
});
