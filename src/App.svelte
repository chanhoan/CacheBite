<script lang="ts">
  import { get } from 'svelte/store';
  import { onMount } from 'svelte';
  import PetOverlay from './lib/components/PetOverlay.svelte';
  import UsagePanel from './lib/components/UsagePanel.svelte';
  import SettingsPanel from './lib/components/SettingsPanel.svelte';
  import SpeechBubble from './lib/components/SpeechBubble.svelte';
  import {
    tauriGateway,
    type AppGateway,
    type AppSettings,
    type CollectorModeDiagnostic,
    type PetSummaryModel,
    type PlatformCapabilities,
    type ProviderBackendStateWire,
  } from './lib/api/gateway';
  import { rendererFixtureGateway } from './lib/api/fixtureGateway';
  import { fromProviderUiSnapshotWire } from './lib/api/providerSnapshot';
  import { nativeNotificationAdapter } from './lib/api/notificationAdapter';
  import {
    applyThemePreference,
    loadThemePreference,
    persistThemePreference,
    type ThemePreference,
  } from './lib/state/theme';
  import { createProvidersStore } from './lib/stores/providers';
  import { createSettingsStore } from './lib/stores/settings';
  import { createInteractionStore } from './lib/stores/interaction';
  import { derivePetUiState } from './lib/state/engine';
  import {
    toProviderPresentation,
    toSettingsStoreState,
  } from './lib/state/presentation';
  import {
    beginPointer,
    pointerButtonsReleased,
    updatePointer,
    type PetPointerState,
  } from './lib/interaction/petPointer';
  import {
    configureNotifications,
    createNotificationPolicy,
    handleNotificationEvent,
    type NotificationAdapter,
    type NotificationDiagnostic,
    type NotificationPolicyState,
  } from './lib/interaction/notificationPolicy';
  import type { PetOverlayViewModel } from './lib/components/models';
  import { validatePetManifest, type PetManifest } from './lib/assets/manifest';
  import {
    requestedAnimationKey,
    resolvePetAnimation,
  } from './lib/assets/resolver';

  const query = new URLSearchParams(window.location.search);
  const rendererFixtureEnabled =
    import.meta.env.DEV &&
    (window.location.hostname === '127.0.0.1' ||
      window.location.hostname === 'localhost') &&
    query.get('fixture') === 'e2e';
  const defaultGateway = rendererFixtureEnabled
    ? rendererFixtureGateway
    : tauriGateway;
  let {
    gateway = defaultGateway,
    notificationAdapter = nativeNotificationAdapter,
  }: { gateway?: AppGateway; notificationAdapter?: NotificationAdapter } =
    $props();
  const windowLabel = query.get('window') ?? 'overlay';
  const providersStore = createProvidersStore(
    (provider) => void gateway.refreshProvider(provider),
  );
  const settingsStore = createSettingsStore();
  const interactionStore = createInteractionStore();
  let startupState = $state<'loading' | 'error' | 'ready'>('loading');
  let collectorMode = $state<CollectorModeDiagnostic | null>(null);
  let appSettings = $state<AppSettings>({
    schemaVersion: 3,
    primaryProvider: 'claude',
    // Must match the Rust default (`store/settings.rs`). 'idle' is an animation
    // key, not a package id, so a getSettings() failure used to guarantee a
    // failed pet load.
    selectedPetId: 'tabby',
    bubblesEnabled: true,
    startAtLogin: false,
    notificationsEnabled: false,
    secondaryNotificationsEnabled: false,
    logicalPosition: { x: 0, y: 0 },
  });
  let showSettings = $state(false);
  let themePreference = $state<ThemePreference>(
    loadThemePreference(globalThis.localStorage),
  );
  const changeTheme = (preference: ThemePreference) => {
    themePreference = preference;
    persistThemePreference(globalThis.localStorage, preference);
    applyThemePreference(document.documentElement, preference);
  };
  let panelShell = $state<HTMLElement | null>(null);
  let pointer = $state<PetPointerState | null>(null);
  // `pointer` is replaced with a fresh object on every move, so an effect that
  // read it directly would tear down and re-register its listeners on every
  // frame of a drag. The window-level safety net only cares whether a gesture
  // is open, and a derived boolean wakes dependents only when that flips.
  const gestureOpen = $derived(pointer !== null);
  let petPackage = $state<{
    manifest: PetManifest;
    assetBaseUrl: string;
  } | null>(null);
  let petPackageError = $state(false);
  let petOptions = $state<readonly PetSummaryModel[]>([]);
  let platformCapabilities = $state<PlatformCapabilities | null>(null);
  let notificationState = $state<NotificationPolicyState>(
    createNotificationPolicy(),
  );
  let notificationDiagnostic = $state<NotificationDiagnostic>({
    status: 'available',
  });
  // `Date.now()` is not a reactive dependency, so a `$derived` that calls it
  // only recomputes when some unrelated `$state` happens to change. This ticker
  // is that dependency: it drives fresh→stale transitions and relative-time
  // labels when no snapshot arrives. One minute is fine for a 20-minute stale
  // boundary and for "N min ago"; anything shorter re-renders the always-on
  // overlay for no visible gain.
  const CLOCK_TICK_MS = 60_000;
  /** Overlay window edge in logical pixels — must match `tauri.conf.json`. */
  const OVERLAY_WINDOW_PX = 240;
  let nowMs = $state(Date.now());
  let settingsSaveFailed = $state(false);
  let positionSaveFailed = $state(false);
  let notificationQueue: Promise<void> = Promise.resolve();
  let settingsQueue: Promise<void> = Promise.resolve();
  let unlisten: (() => void) | undefined;
  let unlistenPosition: (() => void) | undefined;
  let unlistenSettings: (() => void) | undefined;
  let mounted = false;
  let startupAttempt = 0;
  const diagnosticsEnabled = import.meta.env.VITE_CACHEBITE_DIAGNOSTIC === '1';
  function trace(message: string, detail: unknown = '') {
    if (!diagnosticsEnabled) return;
    console.info(`[CacheBite:${windowLabel}] ${message}`, detail ?? '');
  }
  async function traceAsync<T>(label: string, operation: Promise<T>) {
    trace(`${label}:start`);
    try {
      const result = await operation;
      trace(`${label}:ok`);
      return result;
    } catch (error) {
      trace(`${label}:error`, error);
      throw error;
    }
  }

  const serializeNotification = (
    transition: (
      current: NotificationPolicyState,
    ) => Promise<NotificationPolicyState>,
  ): Promise<NotificationPolicyState> => {
    let result!: NotificationPolicyState;
    const operation = notificationQueue
      .catch(() => undefined)
      .then(async () => {
        result = await transition(notificationState);
        notificationState = result;
        notificationDiagnostic = result.diagnostic;
      });
    notificationQueue = operation.catch(() => undefined);
    return operation.then(() => result);
  };

  const statusUpdate = (wire: ProviderBackendStateWire) => {
    // Expiry outranks every other branch: an expired snapshot is not a reset
    // candidate, and its payload must not be applied as if it were current.
    if (wire.expired)
      return providersStore.applyExpiry(
        wire.provider,
        wire.revision,
        wire.unavailable_reason,
        wire.failure_class,
      );
    if (wire.reset_pending) {
      if (wire.snapshot)
        providersStore.apply(
          fromProviderUiSnapshotWire({
            ...wire.snapshot,
            failure_class: wire.failure_class,
            unavailable_reason: wire.unavailable_reason,
          }),
          Date.now(),
        );
      return providersStore.markResetPending(wire.provider, wire.revision);
    }
    if (wire.snapshot) {
      return providersStore.apply(
        fromProviderUiSnapshotWire({
          ...wire.snapshot,
          failure_class: wire.failure_class,
          unavailable_reason: wire.unavailable_reason,
        }),
        Date.now(),
      );
    }
    if (wire.unavailable_reason === 'not_signed_in')
      return providersStore.applyStatus(wire.provider, {
        kind: 'credentials_missing',
        revision: wire.revision,
      });
    if (wire.unavailable_reason === 'not_installed')
      return providersStore.applyStatus(wire.provider, {
        kind: 'cli_missing',
        revision: wire.revision,
      });
    return providersStore.applyStatus(wire.provider, {
      kind: 'fetch_failed',
      revision: wire.revision,
      failureClass: wire.failure_class ?? 'internal',
    });
  };

  const consume = (wire: ProviderBackendStateWire) => {
    const events = statusUpdate(wire);
    for (const event of events) {
      interactionStore.handleBubble(event, {
        primary: appSettings.primaryProvider,
        enabled: appSettings.bubblesEnabled,
        dragging: get(interactionStore).dragging,
        fullscreen: get(interactionStore).fullscreen,
        nowMs: Date.now(),
      });
      void serializeNotification((current) =>
        handleNotificationEvent(
          current,
          event,
          appSettings.primaryProvider,
          notificationAdapter,
          appSettings.secondaryNotificationsEnabled,
        ),
      );
    }
  };

  const callCleanup = (cleanup: (() => void) | undefined) => {
    try {
      cleanup?.();
    } catch {
      // Native listener cleanup is best-effort and must not block recovery.
    }
  };

  const cleanupListeners = () => {
    const providerCleanup = unlisten;
    const positionCleanup = unlistenPosition;
    const settingsCleanup = unlistenSettings;
    unlisten = undefined;
    unlistenPosition = undefined;
    unlistenSettings = undefined;
    callCleanup(providerCleanup);
    callCleanup(positionCleanup);
    callCleanup(settingsCleanup);
  };

  const loadPetPackage = async () => {
    const loadedPetPackage = await gateway.getPetPackage().catch(() => null);
    if (!loadedPetPackage) {
      petPackageError = true;
      return;
    }
    try {
      const manifest = validatePetManifest(loadedPetPackage.manifest);
      resolvePetAnimation(manifest, loadedPetPackage.assetBaseUrl, 'idle');
      petPackage = {
        manifest,
        assetBaseUrl: loadedPetPackage.assetBaseUrl,
      };
      petPackageError = false;
    } catch {
      petPackageError = true;
    }
  };

  const loadPetOptions = async () => {
    // Enumeration is presentation-only: a failure leaves the picker showing
    // just the active pet rather than blocking the panel.
    petOptions = await gateway.listPetPackages().catch(() => []);
  };

  const registerListeners = async (attempt: number) => {
    try {
      const providerUnlisten = await traceAsync(
        'listenProviderStates',
        gateway.listenProviderStates((state) => consume(state)),
      );
      if (!mounted || attempt !== startupAttempt) {
        callCleanup(providerUnlisten);
        return;
      }
      unlisten = providerUnlisten;
      const settingsUnlisten = await traceAsync(
        'listenSettings',
        gateway.listenSettings((settings) => {
          const petChanged =
            settings.selectedPetId !== appSettings.selectedPetId;
          appSettings = settings;
          settingsStore.replace(toSettingsStoreState(settings));
          if (petChanged && windowLabel === 'overlay') void loadPetPackage();
        }),
      );
      if (!mounted || attempt !== startupAttempt) {
        callCleanup(settingsUnlisten);
        return;
      }
      unlistenSettings = settingsUnlisten;
      if (windowLabel !== 'overlay') return;
      const positionUnlisten = await traceAsync(
        'listenPositionMoved',
        gateway.listenPositionMoved(
          (position) => {
            appSettings = { ...appSettings, logicalPosition: position };
          },
          () => {
            positionSaveFailed = true;
          },
        ),
      );
      if (!mounted || attempt !== startupAttempt) {
        callCleanup(positionUnlisten);
        return;
      }
      unlistenPosition = positionUnlisten;
    } catch (error) {
      trace('listener-registration:error', error);
      if (mounted && attempt === startupAttempt) cleanupListeners();
    }
  };

  const start = async () => {
    const attempt = ++startupAttempt;
    cleanupListeners();
    startupState = 'loading';
    trace('startup:begin', { attempt });
    try {
      const [states, settings, selectedCollectorMode] = await Promise.all([
        traceAsync('getProviderStates', gateway.getProviderStates()),
        traceAsync(
          'getSettings',
          gateway.getSettings().catch(() => appSettings),
        ),
        traceAsync('getCollectorMode', gateway.getCollectorMode()),
      ]);
      if (!mounted || attempt !== startupAttempt) return;
      appSettings = settings;
      collectorMode = selectedCollectorMode;
      if (windowLabel === 'panel') {
        const reconciled = await serializeNotification((current) =>
          configureNotifications(
            current,
            appSettings.notificationsEnabled,
            notificationAdapter,
          ),
        );
        if (
          appSettings.notificationsEnabled &&
          (reconciled.permission !== 'granted' ||
            reconciled.diagnostic.status !== 'available')
        ) {
          appSettings = await gateway.updateSettings({
            ...appSettings,
            notificationsEnabled: false,
          });
        }
      }
      settingsStore.replace(toSettingsStoreState(appSettings));
      const [, loadedCapabilities] = await Promise.all([
        windowLabel === 'overlay'
          ? traceAsync('loadPetPackage', loadPetPackage())
          : // `list_pet_packages` is authorized for the panel only.
            loadPetOptions(),
        gateway.getPlatformCapabilities().catch(() => null),
      ]);
      if (!mounted || attempt !== startupAttempt) return;
      platformCapabilities = loadedCapabilities;
      consume(states.claude);
      consume(states.codex);
      startupState = 'ready';
      trace('startup:ready');
      void registerListeners(attempt);
    } catch (error) {
      trace('startup:error', error);
      if (!mounted || attempt !== startupAttempt) return;
      cleanupListeners();
      startupState = 'error';
    }
  };

  onMount(() => {
    mounted = true;
    void start();
    return () => {
      mounted = false;
      startupAttempt += 1;
      cleanupListeners();
    };
  });

  $effect(() => {
    const ticker = window.setInterval(() => {
      nowMs = Date.now();
    }, CLOCK_TICK_MS);
    return () => window.clearInterval(ticker);
  });

  $effect(() => {
    if (
      windowLabel !== 'panel' ||
      !panelShell ||
      typeof ResizeObserver === 'undefined'
    ) {
      return;
    }
    let lastHeight = 0;
    const resize = () => {
      const height = Math.ceil(panelShell?.getBoundingClientRect().height ?? 0);
      if (height <= 0 || height === lastHeight) return;
      lastHeight = height;
      void gateway.resizePanel(height).catch((error) => {
        if (lastHeight === height) lastHeight = 0;
        trace('panel-resize:error', error);
      });
    };
    const observer = new ResizeObserver(resize);
    observer.observe(panelShell);
    return () => observer.disconnect();
  });

  // Contract §7.1-4: the bubble self-dismisses after BUBBLE_DISMISS_MS. The
  // timer is armed per bubble; a replacement bubble re-arms it via cleanup, and
  // a manual dismiss simply leaves the pending timer to no-op on an empty
  // policy. Only the overlay renders bubbles.
  $effect(() => {
    if (windowLabel !== 'overlay') return;
    const bubble = $interactionStore.bubblePolicy.bubble;
    if (!bubble) return;
    // Expire against the deadline, not the wall clock at fire time: a timer that
    // fires a millisecond early would leave the policy untouched, and since the
    // policy returns the same reference the effect never re-arms — the bubble
    // would stick forever.
    const timer = window.setTimeout(
      () => interactionStore.expireBubble(bubble.expiresAt),
      Math.max(0, bubble.expiresAt - Date.now()),
    );
    return () => window.clearTimeout(timer);
  });

  // The pet surface can lose the end of a gesture: `startDragging()` gives the
  // mouse loop to the window manager, and on Windows the accompanying
  // `ReleaseCapture()` breaks the captured-element path outright. Without this
  // net the drag latch stays set for the rest of the session, pinning the pet
  // to bare `idle` and suppressing every bubble. Armed only while a gesture is
  // open, so idle overlays add no listeners, and the registration happens once
  // per gesture rather than once per move (`gestureOpen`).
  $effect(() => {
    if (windowLabel !== 'overlay' || !gestureOpen) return;
    const end = () => endPointerInteraction();
    const moved = (event: PointerEvent) => {
      if (pointerButtonsReleased(event.buttons)) endPointerInteraction();
    };
    window.addEventListener('pointerup', end);
    window.addEventListener('pointercancel', end);
    window.addEventListener('pointermove', moved);
    window.addEventListener('blur', end);
    return () => {
      window.removeEventListener('pointerup', end);
      window.removeEventListener('pointercancel', end);
      window.removeEventListener('pointermove', moved);
      window.removeEventListener('blur', end);
    };
  });

  const panelProviders = $derived({
    claude: toProviderPresentation($providersStore.claude, nowMs),
    codex: toProviderPresentation($providersStore.codex, nowMs),
  });
  const primaryState = $derived($providersStore[appSettings.primaryProvider]);
  const primaryPresentation = $derived(
    toProviderPresentation(primaryState, nowMs),
  );
  const primaryUi = $derived(derivePetUiState(primaryState, nowMs));
  const resolvedAnimation = $derived(
    petPackage
      ? resolvePetAnimation(
          petPackage.manifest,
          petPackage.assetBaseUrl,
          requestedAnimationKey({
            system: primaryUi.system,
            mood: primaryUi.petMood,
            dragging: $interactionStore.dragging,
            draggingAvailable:
              petPackage.manifest.states.dragging !== undefined,
          }),
        )
      : null,
  );
  // The overlay window is a fixed square (`tauri.conf.json`), so a manifest that
  // declares something larger would simply be clipped. Clamp instead, and fall
  // back to the window edge when no package has loaded.
  const overlaySize = $derived(
    Math.min(
      OVERLAY_WINDOW_PX,
      petPackage?.manifest.defaultSize.width ?? OVERLAY_WINDOW_PX,
    ),
  );
  const overlayModel = $derived<PetOverlayViewModel | null>(
    resolvedAnimation
      ? {
          system: primaryPresentation.system,
          stale: primaryPresentation.stale,
          session: {
            usedPercent: primaryPresentation.session.usedPercent,
            severity: primaryPresentation.session.severity,
          },
          weekly: {
            usedPercent: primaryPresentation.weekly.usedPercent,
            severity: primaryPresentation.weekly.severity,
          },
          animation: resolvedAnimation,
          petName:
            petPackage?.manifest.displayName ?? appSettings.selectedPetId,
          size: overlaySize,
        }
      : null,
  );

  const changeSettings = (next: typeof $settingsStore) => {
    settingsStore.replace(next);
    const operation = settingsQueue
      .catch(() => undefined)
      .then(async () => {
        let merged = { ...appSettings, ...next };
        // The pet is chosen independently of the primary provider: changing
        // one must never silently rewrite the other.
        if (next.notificationsEnabled !== appSettings.notificationsEnabled) {
          notificationState = await serializeNotification((current) =>
            configureNotifications(
              current,
              next.notificationsEnabled,
              notificationAdapter,
            ),
          );
          notificationDiagnostic = notificationState.diagnostic;
          if (
            notificationState.permission !== 'granted' &&
            next.notificationsEnabled
          )
            merged = { ...merged, notificationsEnabled: false };
        }
        try {
          appSettings = await gateway.updateSettings(merged);
          settingsSaveFailed = false;
          // No overlay reload here: `changeSettings` only ever runs in the
          // panel window (`update_settings` is panel-authorized). The overlay
          // picks the change up through the `settings-updated` listener.
        } catch {
          const reconciled = await serializeNotification((current) =>
            configureNotifications(
              current,
              appSettings.notificationsEnabled,
              notificationAdapter,
            ),
          ).catch(() => notificationState);
          notificationState = reconciled;
          notificationDiagnostic = reconciled.diagnostic;
          settingsSaveFailed = true;
        }
        settingsStore.replace(toSettingsStoreState(appSettings));
      });
    settingsQueue = operation.catch(() => undefined);
    return operation;
  };
  // The single place the drag latch is released. `startDragging()` hands the
  // mouse loop to the OS, so `pointerup` on the surface is not guaranteed to
  // arrive — every signal that proves the gesture ended routes through here.
  const endPointerInteraction = () => {
    pointer = null;
    interactionStore.setDragging(false);
  };
  const pointerDown = (event: PointerEvent) => {
    // Last-resort recovery: if every release signal was swallowed by the OS
    // drag loop, the next gesture still starts from a clean latch.
    endPointerInteraction();
    (
      event.currentTarget as EventTarget & {
        setPointerCapture?: (pointerId: number) => void;
      }
    ).setPointerCapture?.(event.pointerId);
    pointer = beginPointer({ x: event.clientX, y: event.clientY });
  };
  const pointerMove = (event: PointerEvent) => {
    if (!pointer) return;
    if (pointerButtonsReleased(event.buttons)) {
      endPointerInteraction();
      return;
    }
    const wasDragging = pointer.dragging;
    pointer = updatePointer(pointer, { x: event.clientX, y: event.clientY });
    interactionStore.setDragging(pointer.dragging);
    if (!wasDragging && pointer.dragging) void gateway.startDragging();
  };
  const pointerUp = (event: PointerEvent) => {
    // Capture is released before the latch is consulted: a window-level signal
    // may have already cleared `pointer`, and an early return here would leave
    // the capture to the browser's implicit release.
    const surface = event.currentTarget as EventTarget & {
      hasPointerCapture?: (pointerId: number) => boolean;
      releasePointerCapture?: (pointerId: number) => void;
    };
    if (surface.hasPointerCapture?.(event.pointerId)) {
      surface.releasePointerCapture?.(event.pointerId);
    }
    endPointerInteraction();
  };
  const pointerCancel = (event: PointerEvent) => pointerUp(event);
</script>

<main
  bind:this={panelShell}
  aria-label="CacheBite"
  class:panel={windowLabel === 'panel'}
  data-collector-mode-claude={collectorMode?.claude}
  data-collector-mode-codex={collectorMode?.codex}
  data-platform={platformCapabilities?.os ?? 'linux'}
  data-window-label={windowLabel}
>
  <h1 class="visually-hidden">CacheBite</h1>
  {#if startupState === 'loading'}
    <p>CacheBite is starting</p>
  {:else if startupState === 'error'}
    <p role="alert">CacheBite could not start</p>
    <button onclick={() => void start()}>Retry</button>
  {:else if windowLabel === 'panel'}
    {#if showSettings}
      <div class="settings-view">
        <button class="settings-back" onclick={() => (showSettings = false)}
          >← Back</button
        >
        <SettingsPanel
          settings={$settingsStore}
          theme={themePreference}
          autostartAvailable={platformCapabilities?.autostart.status !==
            'unavailable'}
          pets={petOptions}
          onChange={(settings) => void changeSettings(settings)}
          onThemeChange={changeTheme}
        />
      </div>
    {:else}
      <UsagePanel
        providers={panelProviders}
        selected={$providersStore.selected}
        primary={$settingsStore.primaryProvider}
        refreshing={$providersStore.refreshing[$providersStore.selected]}
        {nowMs}
        onClose={() => void gateway.hidePanel()}
        onSettings={() => (showSettings = true)}
        onSelect={(provider) => {
          providersStore.selectTab(provider);
        }}
        onRefresh={(provider) => providersStore.requestRefresh(provider)}
        onPrimary={(provider) =>
          void changeSettings({ ...$settingsStore, primaryProvider: provider })}
      />
    {/if}
    {#if settingsSaveFailed}<p role="status">
        Settings could not be saved
      </p>{/if}
    {#if platformCapabilities?.autostart.status === 'unavailable'}
      <p role="status">{platformCapabilities.autostart.reason}</p>
    {/if}
    {#if platformCapabilities?.fullscreen_detection.status === 'unavailable'}
      <p role="status">{platformCapabilities.fullscreen_detection.reason}</p>
    {/if}
    {#if notificationDiagnostic.status === 'permission_denied'}<p role="status">
        Notification permission denied
      </p>{/if}
    {#if notificationDiagnostic.status === 'unavailable'}<p role="status">
        {notificationDiagnostic.reason}
      </p>{/if}
  {:else}
    {#if overlayModel}
      <PetOverlay
        model={overlayModel}
        onPointerDown={pointerDown}
        onPointerMove={pointerMove}
        onPointerUp={pointerUp}
        onPointerCancel={pointerCancel}
        onOpen={() => void gateway.showPanel()}
      />
    {:else if petPackageError}
      <p role="status">Pet package unavailable</p>
    {/if}
    {#if $interactionStore.bubblePolicy.bubble}
      <SpeechBubble
        message={$interactionStore.bubblePolicy.bubble.message}
        onDismiss={() => interactionStore.dismissBubble()}
      />
    {/if}
    {#if positionSaveFailed}<p role="status">
        Window position could not be saved
      </p>{/if}
  {/if}
</main>

<style>
  .settings-view {
    display: grid;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4) var(--space-4);
  }
  .settings-back {
    justify-self: start;
    min-height: 2rem;
    padding: 0 var(--space-3);
    border: 1px solid transparent;
    border-radius: 0.5rem;
    background: transparent;
    color: var(--color-text-muted);
    font: inherit;
    font-weight: 500;
    cursor: pointer;
  }
  .settings-back:hover,
  .settings-back:focus-visible {
    border-color: var(--color-border);
    color: var(--color-text);
  }
</style>
