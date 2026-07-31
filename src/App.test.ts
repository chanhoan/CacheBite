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

// jsdom has no `PointerEvent`, so gestures are driven with plain events that
// carry the fields the overlay reads. `buttons` defaults to a held primary
// button: a real pointer event always carries a number, and leaving it
// undefined would let `pointerButtonsReleased` pass for the wrong reason.
const pointerEvent = (type: string, x: number, y: number, buttons = 1) => {
  const event = new Event(type, { bubbles: true });
  Object.defineProperties(event, {
    clientX: { value: x },
    clientY: { value: y },
    buttons: { value: buttons },
  });
  return event;
};

// A package with a distinct asset per key. The shared fixture declares only
// `idle`, so every key would resolve to the same src and a stuck drag latch
// would be invisible in the rendered image — `dragging` above all.
const distinctPetPackage: Awaited<ReturnType<AppGateway['getPetPackage']>> = {
  manifest: {
    id: 'fixture-pet',
    displayName: 'Fixture Pet',
    defaultSize: { width: 160, height: 160 },
    animations: {
      idle: { type: 'image', source: 'idle.svg' },
      idle_critical: { type: 'image', source: 'idle_critical.svg' },
      dragging: { type: 'image', source: 'dragging.svg' },
    },
    states: {
      idle: 'idle',
      idle_critical: 'idle_critical',
      dragging: 'dragging',
    },
  },
  assetBaseUrl: 'asset://localhost/pets/fixture-pet/',
};

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
      schemaVersion: 4,
      primaryProvider: 'claude' as const,
      selectedPetId: 'tabby',
      bubblesEnabled: true,
      startAtLogin: false,
      notificationsEnabled: false,
      secondaryNotificationsEnabled: false,
      logicalPosition: { x: 0, y: 0 },
      hideShowHotkey: null,
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
    listPetPackages: vi.fn(async () => [
      { id: 'corgi', displayName: 'Corgi' },
      { id: 'tabby', displayName: 'Tabby' },
    ]),
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
    resizePanel: vi.fn(async () => undefined),
    hidePanel: vi.fn(async () => undefined),
    quit: vi.fn(async () => undefined),
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
    vi.unstubAllGlobals();
    window.history.replaceState({}, '', '/');
  });

  it('hydrates overlay and opens the panel only on a circular-surface double-click', async () => {
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
    expect(gateway.showPanel).not.toHaveBeenCalled();
    await fireEvent.dblClick(overlay);
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
      selectedPetId: 'tabby',
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
    await fireEvent(overlay, pointerEvent('pointerdown', 10, 10));
    await fireEvent(overlay, pointerEvent('pointermove', 20, 10));
    await fireEvent(overlay, pointerEvent('pointermove', 30, 10));
    expect(gateway.startDragging).toHaveBeenCalledOnce();
  });

  // Drives a gesture past the drag threshold and leaves it there: from this
  // point the OS drag loop owns the mouse and no pointerup reaches the
  // surface. Each caller then exercises a different release signal.
  const dragUntilLatched = async () => {
    const { gateway } = fixture();
    vi.mocked(gateway.getPetPackage).mockResolvedValue(distinctPetPackage);
    render(App, { props: { gateway, notificationAdapter: notifications } });
    const overlay = await screen.findByTestId('overlay-pointer-surface');
    expect(
      (screen.getByAltText('Fixture Pet') as HTMLImageElement).src,
    ).toContain('idle_critical.svg');
    await fireEvent(overlay, pointerEvent('pointerdown', 10, 10));
    await fireEvent(overlay, pointerEvent('pointermove', 40, 10));
    expect(gateway.startDragging).toHaveBeenCalledOnce();
    await waitFor(() =>
      expect(
        (screen.getByAltText('Fixture Pet') as HTMLImageElement).src,
      ).toContain('dragging.svg'),
    );
    return { gateway, overlay };
  };

  const expectLatchReleased = () =>
    waitFor(() =>
      expect(
        (screen.getByAltText('Fixture Pet') as HTMLImageElement).src,
      ).toContain('idle_critical.svg'),
    );

  it('restores the usage animation when the native drag swallows pointerup', async () => {
    await dragUntilLatched();
    // The first button-free move after the drop must release the latch on its
    // own.
    await fireEvent(window, pointerEvent('pointermove', 60, 10, 0));
    await expectLatchReleased();
  });

  it('releases the drag latch when the overlay loses focus after the drop', async () => {
    await dragUntilLatched();
    // Dropping the pet and clicking another window: no pointer event comes
    // back to the overlay at all, so `blur` is the only proof the gesture
    // ended.
    await fireEvent(window, new Event('blur'));
    await expectLatchReleased();
  });

  it('starts the next gesture from a clean latch when every release signal was swallowed', async () => {
    const { overlay } = await dragUntilLatched();
    // Last-resort recovery: with no window-level signal arriving at all, the
    // next pointerdown still has to clear the latch before opening a gesture.
    await fireEvent(overlay, pointerEvent('pointerdown', 10, 10));
    await expectLatchReleased();
  });

  it('keeps the usage animation during a drag when the package has no dragging asset', async () => {
    const { gateway } = fixture();
    vi.mocked(gateway.getPetPackage).mockResolvedValue({
      manifest: {
        id: 'fixture-pet',
        displayName: 'Fixture Pet',
        defaultSize: { width: 160, height: 160 },
        animations: {
          idle: { type: 'image', source: 'idle.svg' },
          idle_critical: { type: 'image', source: 'idle_critical.svg' },
        },
        states: { idle: 'idle', idle_critical: 'idle_critical' },
      },
      assetBaseUrl: 'asset://localhost/pets/fixture-pet/',
    });
    render(App, { props: { gateway, notificationAdapter: notifications } });
    const overlay = await screen.findByTestId('overlay-pointer-surface');
    await fireEvent(overlay, pointerEvent('pointerdown', 10, 10));
    await fireEvent(overlay, pointerEvent('pointermove', 40, 10));
    // Without this the test passes even if both handlers do nothing: the
    // initial src is already the asserted one, so the drag has to be proven.
    expect(gateway.startDragging).toHaveBeenCalledOnce();
    expect(
      (screen.getByAltText('Fixture Pet') as HTMLImageElement).src,
    ).toContain('idle_critical.svg');
  });

  it('persists settings through the typed gateway from the settings view', async () => {
    window.history.replaceState({}, '', '/?window=panel');
    const { gateway } = fixture();
    render(App, { props: { gateway, notificationAdapter: notifications } });
    expect(await screen.findByText('Pro')).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
    await fireEvent.click(screen.getByLabelText('Native notifications'));
    await waitFor(() => expect(gateway.updateSettings).toHaveBeenCalled());
    expect(gateway.getPetPackage).not.toHaveBeenCalled();
  });

  it('opens the settings view from the panel and returns to usage', async () => {
    window.history.replaceState({}, '', '/?window=panel');
    const { gateway } = fixture();
    render(App, { props: { gateway, notificationAdapter: notifications } });
    await screen.findByText('Pro');
    await fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
    expect(screen.getByLabelText('Native notifications')).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: '← Back' }));
    expect(await screen.findByText('Pro')).toBeTruthy();
    expect(screen.queryByLabelText('Native notifications')).toBeNull();
  });

  it('resizes to rendered content, dedupes success, and retries failure', async () => {
    window.history.replaceState({}, '', '/?window=panel');
    let notifyResize = () => {};
    const disconnect = vi.fn();
    vi.stubGlobal(
      'ResizeObserver',
      class {
        constructor(callback: ResizeObserverCallback) {
          notifyResize = () => callback([], this);
        }

        observe() {}
        unobserve() {}
        disconnect() {
          disconnect();
        }
      },
    );
    const { gateway } = fixture();
    const resizePanel = vi.mocked(gateway.resizePanel);
    let measuredHeight = 383.2;

    render(App, { props: { gateway, notificationAdapter: notifications } });
    await screen.findByLabelText('Usage panel');
    const shell = screen.getByLabelText('CacheBite');
    vi.spyOn(shell, 'getBoundingClientRect').mockReturnValue({
      bottom: 384,
      get height() {
        return measuredHeight;
      },
      left: 0,
      right: 312,
      top: 0,
      width: 312,
      x: 0,
      y: 0,
      toJSON: () => ({}),
    });

    notifyResize();

    await waitFor(() => expect(resizePanel).toHaveBeenCalledWith(384));
    notifyResize();
    expect(resizePanel).toHaveBeenCalledOnce();

    resizePanel.mockRejectedValueOnce(new Error('transient resize failure'));
    measuredHeight = 384.2;
    notifyResize();
    await waitFor(() => expect(resizePanel).toHaveBeenCalledTimes(2));
    await Promise.resolve();
    notifyResize();
    await waitFor(() => expect(resizePanel).toHaveBeenCalledTimes(3));
    expect(resizePanel).toHaveBeenLastCalledWith(385);
    expect(disconnect).not.toHaveBeenCalled();
  });

  it('changes primary only when Set as primary is clicked', async () => {
    window.history.replaceState({}, '', '/?window=panel');
    const { gateway } = fixture();
    render(App, { props: { gateway, notificationAdapter: notifications } });

    await fireEvent.click(await screen.findByRole('tab', { name: 'Codex' }));
    expect(gateway.updateSettings).not.toHaveBeenCalled();
    expect(screen.getByRole('tab', { name: 'Claude (primary)' })).toBeTruthy();
    expect(
      screen.getByRole('tab', { name: 'Codex' }).getAttribute('aria-selected'),
    ).toBe('true');

    const setPrimary = screen.getByRole('button', {
      name: 'Set as primary',
    }) as HTMLButtonElement;
    expect(setPrimary.disabled).toBe(false);
    await fireEvent.click(setPrimary);
    await waitFor(() =>
      expect(gateway.updateSettings).toHaveBeenCalledWith(
        // The pet is the user's choice — switching the primary provider
        // changes the data source, never the pet on screen.
        expect.objectContaining({
          primaryProvider: 'codex',
          selectedPetId: 'tabby',
        }),
      ),
    );
    expect(screen.getByRole('tab', { name: 'Codex (primary)' })).toBeTruthy();
    expect(setPrimary.disabled).toBe(true);
  });

  it('changes the pet from settings without touching the primary provider', async () => {
    window.history.replaceState({}, '', '/?window=panel');
    const { gateway } = fixture();
    render(App, { props: { gateway, notificationAdapter: notifications } });

    await fireEvent.click(
      await screen.findByRole('button', { name: 'Settings' }),
    );
    const picker = await screen.findByLabelText('Pet');
    expect(
      [...(picker as HTMLSelectElement).options].map((option) => option.value),
    ).toEqual(['corgi', 'tabby']);

    await fireEvent.change(picker, { target: { value: 'corgi' } });

    await waitFor(() =>
      expect(gateway.updateSettings).toHaveBeenCalledWith(
        expect.objectContaining({
          selectedPetId: 'corgi',
          primaryProvider: 'claude',
        }),
      ),
    );
  });

  it('keeps the settings pet picker usable when enumeration fails', async () => {
    window.history.replaceState({}, '', '/?window=panel');
    const { gateway } = fixture();
    vi.mocked(gateway.listPetPackages).mockRejectedValue(
      new Error('pets unavailable'),
    );
    render(App, { props: { gateway, notificationAdapter: notifications } });

    await fireEvent.click(
      await screen.findByRole('button', { name: 'Settings' }),
    );

    // Falls back to the active pet rather than rendering an empty <select>.
    const picker = (await screen.findByLabelText('Pet')) as HTMLSelectElement;
    expect([...picker.options].map((option) => option.value)).toEqual([
      'tabby',
    ]);
  });

  it('restores the previous primary when saving a primary change fails', async () => {
    window.history.replaceState({}, '', '/?window=panel');
    const { gateway } = fixture();
    vi.mocked(gateway.updateSettings).mockRejectedValueOnce(
      new Error('settings unavailable'),
    );
    render(App, { props: { gateway, notificationAdapter: notifications } });

    await fireEvent.click(await screen.findByRole('tab', { name: 'Codex' }));
    const setPrimary = screen.getByRole('button', {
      name: 'Set as primary',
    }) as HTMLButtonElement;
    await fireEvent.click(setPrimary);

    await screen.findByText('Settings could not be saved');
    expect(screen.getByRole('tab', { name: 'Claude (primary)' })).toBeTruthy();
    expect(
      screen.getByRole('tab', { name: 'Codex' }).getAttribute('aria-selected'),
    ).toBe('true');
    expect(setPrimary.disabled).toBe(false);
  });

  it('shows a distinct message when the hotkey fails to register', async () => {
    window.history.replaceState({}, '', '/?window=panel');
    const { gateway } = fixture();
    // NOT `new Error(...)` — Err(IpcError::HotkeyUnavailable) rejects the real
    // gateway's promise with the raw string, not a wrapped Error.
    vi.mocked(gateway.updateSettings).mockRejectedValueOnce(
      'hotkey_unavailable',
    );
    render(App, { props: { gateway, notificationAdapter: notifications } });

    await fireEvent.click(
      await screen.findByRole('button', { name: 'Settings' }),
    );
    await fireEvent.change(screen.getByLabelText('Hide/show shortcut'), {
      target: { value: 'CmdOrCtrl+Shift+H' },
    });

    await screen.findByText(
      'Global shortcut could not be registered — it may already be in use',
    );
    expect(screen.queryByText('Settings could not be saved')).toBeNull();
  });

  it('reconciles persisted notification opt-in with granted permission', async () => {
    window.history.replaceState({}, '', '/?window=panel');
    const { gateway } = fixture();
    vi.mocked(gateway.getSettings).mockResolvedValue({
      ...(await gateway.getSettings()),
      notificationsEnabled: true,
    });
    render(App, { props: { gateway, notificationAdapter: notifications } });
    await screen.findByText('Pro');
    await fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
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
    await screen.findByText('Pro');
    await fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
    await fireEvent.click(screen.getByLabelText('Native notifications'));
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

  it('pins the appearance theme and persists it', async () => {
    window.history.replaceState({}, '', '/?window=panel');
    const { gateway } = fixture();
    render(App, { props: { gateway, notificationAdapter: notifications } });
    await screen.findByText('Pro');
    await fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
    await fireEvent.change(await screen.findByLabelText('Appearance'), {
      target: { value: 'dark' },
    });
    await waitFor(() =>
      expect(document.documentElement.getAttribute('data-theme')).toBe('dark'),
    );
    expect(localStorage.getItem('cachebite:theme')).toBe('dark');
    document.documentElement.removeAttribute('data-theme');
    localStorage.removeItem('cachebite:theme');
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
    await screen.findByText('Pro');
    await fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
    const bubbles = (await screen.findByLabelText(
      'Speech bubbles',
    )) as HTMLInputElement;
    const secondary = screen.getByLabelText(
      'Secondary provider notifications',
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
    await screen.findByText('Pro');
    await fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
    const nativeNotifications = (await screen.findByLabelText(
      'Native notifications',
    )) as HTMLInputElement;

    await fireEvent.click(nativeNotifications);
    expect(await screen.findByText('Settings could not be saved')).toBeTruthy();
    await waitFor(() => expect(nativeNotifications.checked).toBe(false));

    const secondary = screen.getByLabelText(
      'Secondary provider notifications',
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

  it('retires the speech bubble once its expiry deadline passes', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    try {
      const { gateway, emit } = fixture();
      render(App, { props: { gateway, notificationAdapter: notifications } });
      await screen.findByLabelText('CacheBite pet status');

      // 91% -> 100% crosses critical into exhausted, which is what fires a
      // bubble; the first snapshot cannot, since it has no prior severity.
      emit(active('claude', 2, 100));
      const bubble = await screen.findByRole('button', {
        name: '5-hour usage is exhausted',
      });
      expect(bubble).toBeTruthy();

      await vi.advanceTimersByTimeAsync(7_999);
      expect(
        screen.queryByRole('button', { name: '5-hour usage is exhausted' }),
      ).toBeTruthy();

      await vi.advanceTimersByTimeAsync(1);
      await waitFor(() =>
        expect(
          screen.queryByRole('button', { name: '5-hour usage is exhausted' }),
        ).toBeNull(),
      );
    } finally {
      vi.useRealTimers();
    }
  });

  it('anchors the exhausted toast below the usage ring inside the overlay stack', async () => {
    const { gateway, emit } = fixture();
    render(App, { props: { gateway, notificationAdapter: notifications } });
    await screen.findByLabelText('CacheBite pet status');
    emit(active('claude', 2, 100));

    const toast = await screen.findByTestId('overlay-toast');
    const stack = toast.closest('.overlay-stack');
    expect(stack).not.toBeNull();
    expect(stack?.getAttribute('data-toast-visible')).toBe('true');
    expect(stack?.querySelector('[data-testid="usage-ring"]')).not.toBeNull();
  });

  it('dismisses a clicked speech bubble without opening the panel', async () => {
    const { gateway, emit } = fixture();
    render(App, { props: { gateway, notificationAdapter: notifications } });
    await screen.findByLabelText('CacheBite pet status');
    emit(active('claude', 2, 100));
    const bubble = await screen.findByRole('button', {
      name: '5-hour usage is exhausted',
    });

    await fireEvent.click(bubble);

    expect(
      screen.queryByRole('button', { name: '5-hour usage is exhausted' }),
    ).toBeNull();
    expect(gateway.showPanel).not.toHaveBeenCalled();
  });

  it('ages a fresh snapshot into stale without any new provider event', async () => {
    window.history.replaceState({}, '', '/?window=panel');
    vi.useFakeTimers({ shouldAdvanceTime: true });
    try {
      const { gateway } = fixture();
      render(App, { props: { gateway, notificationAdapter: notifications } });
      expect(await screen.findByText(/Fresh/)).toBeTruthy();

      // 21 minutes clears FRESH_MAX_AGE_MS with no snapshot in between: the
      // transition can only come from the clock ticker.
      await vi.advanceTimersByTimeAsync(21 * 60_000);
      await waitFor(() => expect(screen.getByText(/Stale/)).toBeTruthy());
    } finally {
      vi.useRealTimers();
    }
  });

  it('degrades to error guidance when the backend reports the snapshot expired', async () => {
    window.history.replaceState({}, '', '/?window=panel');
    const { gateway, emit } = fixture();
    render(App, { props: { gateway, notificationAdapter: notifications } });
    await screen.findByText('Pro');

    emit({ ...active('claude', 3), snapshot: null, expired: true });

    expect(
      await screen.findByText('Could not fetch usage. Retrying shortly.'),
    ).toBeTruthy();
  });

  it('keeps sign-in guidance when an expired snapshot arrives with the reason', async () => {
    // `provider_state_from_record` fills `expired` and `unavailable_reason`
    // independently, so a logged-out user with a stale cache gets both at once.
    // Degrading on renderer state alone would print retry guidance instead.
    window.history.replaceState({}, '', '/?window=panel');
    const { gateway, emit } = fixture();
    render(App, { props: { gateway, notificationAdapter: notifications } });
    await screen.findByText('Pro');

    emit({
      ...active('claude', 3),
      snapshot: null,
      expired: true,
      unavailable_reason: 'not_signed_in',
    });

    expect(
      await screen.findByText('Sign in to the Claude CLI: claude login'),
    ).toBeTruthy();
  });

  it('hides the panel through the close control without quitting CacheBite', async () => {
    window.history.replaceState({}, '', '/?window=panel');
    const { gateway } = fixture();
    render(App, { props: { gateway, notificationAdapter: notifications } });

    await fireEvent.click(
      await screen.findByRole('button', { name: 'Close usage panel' }),
    );
    expect(gateway.hidePanel).toHaveBeenCalledOnce();
    expect(gateway.quit).not.toHaveBeenCalled();
  });

  it('exits CacheBite through the footer Quit button', async () => {
    window.history.replaceState({}, '', '/?window=panel');
    const { gateway } = fixture();
    render(App, { props: { gateway, notificationAdapter: notifications } });

    await fireEvent.click(await screen.findByRole('button', { name: 'Quit' }));
    expect(gateway.quit).toHaveBeenCalledOnce();
    expect(gateway.hidePanel).not.toHaveBeenCalled();
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
    await fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
    expect(
      (screen.getByLabelText('Start at login') as HTMLInputElement).disabled,
    ).toBe(true);
  });
});
