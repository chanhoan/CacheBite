<script lang="ts">
  import { get } from 'svelte/store';
  import { onMount } from 'svelte';
  import PetOverlay from './lib/components/PetOverlay.svelte';
  import UsagePanel from './lib/components/UsagePanel.svelte';
  import SettingsPanel from './lib/components/SettingsPanel.svelte';
  import HistoryGraph from './lib/components/HistoryGraph.svelte';
  import SpeechBubble from './lib/components/SpeechBubble.svelte';
  import {
    tauriGateway,
    type AppGateway,
    type AppSettings,
    type CollectorModeDiagnostic,
    type HistoryModels,
    type PlatformCapabilities,
    type ProviderBackendStateWire,
  } from './lib/api/gateway';
  import { rendererFixtureGateway } from './lib/api/fixtureGateway';
  import { fromProviderUiSnapshotWire } from './lib/api/providerSnapshot';
  import { nativeNotificationAdapter } from './lib/api/notificationAdapter';
  import { createProvidersStore } from './lib/stores/providers';
  import { createSettingsStore } from './lib/stores/settings';
  import { createInteractionStore } from './lib/stores/interaction';
  import { derivePetUiState, type ProviderState } from './lib/state/engine';
  import {
    beginPointer,
    releasePointer,
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
  import type { PanelProviderModel } from './lib/components/panelModels';
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
    selectedPetId: 'idle',
    bubblesEnabled: true,
    startAtLogin: false,
    notificationsEnabled: false,
    secondaryNotificationsEnabled: false,
    logicalPosition: { x: 0, y: 0 },
  });
  let history = $state<HistoryModels>({ claude: [], codex: [] });
  let historyWindow = $state<'session' | 'weekly'>('session');
  let pointer = $state<PetPointerState | null>(null);
  let petPackage = $state<{
    manifest: PetManifest;
    assetBaseUrl: string;
  } | null>(null);
  let petPackageError = $state(false);
  let platformCapabilities = $state<PlatformCapabilities | null>(null);
  let notificationState = $state<NotificationPolicyState>(
    createNotificationPolicy(),
  );
  let notificationDiagnostic = $state<NotificationDiagnostic>({
    status: 'available',
  });
  let settingsSaveFailed = $state(false);
  let positionSaveFailed = $state(false);
  let notificationQueue: Promise<void> = Promise.resolve();
  let historyQueue: Promise<void> = Promise.resolve();
  let settingsQueue: Promise<void> = Promise.resolve();
  let unlisten: (() => void) | undefined;
  let unlistenPosition: (() => void) | undefined;
  let mounted = false;
  let startupAttempt = 0;

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

  const refreshHistory = () => {
    if (windowLabel !== 'panel') return;
    historyQueue = historyQueue
      .catch(() => undefined)
      .then(async () => {
        history = await gateway.getHistory().catch(() => history);
      });
  };

  const statusUpdate = (wire: ProviderBackendStateWire) => {
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

  const consume = (wire: ProviderBackendStateWire, live = false) => {
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
    if (live) refreshHistory();
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
    unlisten = undefined;
    unlistenPosition = undefined;
    callCleanup(providerCleanup);
    callCleanup(positionCleanup);
  };

  const start = async () => {
    const attempt = ++startupAttempt;
    cleanupListeners();
    startupState = 'loading';
    try {
      const [states, settings, selectedCollectorMode] = await Promise.all([
        gateway.getProviderStates(),
        gateway.getSettings().catch(() => appSettings),
        gateway.getCollectorMode(),
      ]);
      if (!mounted || attempt !== startupAttempt) return;
      appSettings = settings;
      collectorMode = selectedCollectorMode;
      if (windowLabel === 'panel') {
        const reconciled = await serializeNotification((current) =>
          configureNotifications(
            current,
            settings.notificationsEnabled,
            notificationAdapter,
          ),
        );
        if (
          settings.notificationsEnabled &&
          (reconciled.permission !== 'granted' ||
            reconciled.diagnostic.status !== 'available')
        ) {
          appSettings = await gateway.updateSettings({
            ...settings,
            notificationsEnabled: false,
          });
        }
      }
      settingsStore.replace({
        primaryProvider: appSettings.primaryProvider,
        bubblesEnabled: appSettings.bubblesEnabled,
        startAtLogin: appSettings.startAtLogin,
        notificationsEnabled: appSettings.notificationsEnabled,
        secondaryNotificationsEnabled:
          appSettings.secondaryNotificationsEnabled,
      });
      const [loadedPetPackage, loadedHistory, loadedCapabilities] =
        await Promise.all([
          gateway.getPetPackage().catch(() => null),
          windowLabel === 'panel'
            ? gateway.getHistory().catch(() => ({ claude: [], codex: [] }))
            : Promise.resolve({ claude: [], codex: [] }),
          windowLabel === 'panel'
            ? gateway.getPlatformCapabilities().catch(() => null)
            : Promise.resolve(null),
        ]);
      if (!mounted || attempt !== startupAttempt) return;
      if (loadedPetPackage) {
        try {
          petPackage = {
            manifest: validatePetManifest(loadedPetPackage.manifest),
            assetBaseUrl: loadedPetPackage.assetBaseUrl,
          };
          petPackageError = false;
        } catch {
          petPackageError = true;
        }
      } else {
        petPackageError = true;
      }
      history = loadedHistory;
      platformCapabilities = loadedCapabilities;
      consume(states.claude);
      consume(states.codex);
      const providerUnlisten = await gateway.listenProviderStates((state) =>
        consume(state, true),
      );
      if (!mounted || attempt !== startupAttempt) {
        callCleanup(providerUnlisten);
        return;
      }
      unlisten = providerUnlisten;
      if (windowLabel === 'overlay') {
        const positionUnlisten = await gateway.listenPositionMoved(
          (position) => {
            appSettings = { ...appSettings, logicalPosition: position };
          },
          () => {
            positionSaveFailed = true;
          },
        );
        if (!mounted || attempt !== startupAttempt) {
          callCleanup(positionUnlisten);
          return;
        }
        unlistenPosition = positionUnlisten;
      }
      startupState = 'ready';
    } catch {
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

  const panelModel = (state: ProviderState): PanelProviderModel => {
    const ui = derivePetUiState(state, Date.now());
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
  };
  const panelProviders = $derived({
    claude: panelModel($providersStore.claude),
    codex: panelModel($providersStore.codex),
  });
  const primaryState = $derived($providersStore[appSettings.primaryProvider]);
  const primaryUi = $derived(derivePetUiState(primaryState, Date.now()));
  const resolvedAnimation = $derived(
    petPackage
      ? resolvePetAnimation(
          petPackage.manifest,
          petPackage.assetBaseUrl,
          requestedAnimationKey({
            system: primaryUi.system,
            mood: primaryUi.petMood,
            dragging: $interactionStore.dragging,
          }),
        )
      : null,
  );
  const overlayModel = $derived<PetOverlayViewModel | null>(
    resolvedAnimation
      ? {
          system: primaryUi.system,
          stale: primaryUi.stale,
          session: {
            usedPercent:
              primaryUi.sessionSeverity === 'unknown'
                ? null
                : (primaryState.snapshot?.session?.usedPercent ?? null),
            severity: primaryUi.sessionSeverity,
          },
          weekly: {
            usedPercent:
              primaryUi.weeklySeverity === 'unknown'
                ? null
                : (primaryState.snapshot?.weekly?.usedPercent ?? null),
            severity: primaryUi.weeklySeverity,
          },
          animation: resolvedAnimation,
          petName:
            petPackage?.manifest.displayName ?? appSettings.selectedPetId,
        }
      : null,
  );

  const changeSettings = (next: typeof $settingsStore) => {
    settingsStore.replace(next);
    const operation = settingsQueue
      .catch(() => undefined)
      .then(async () => {
        let merged = { ...appSettings, ...next };
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
        settingsStore.replace({
          primaryProvider: appSettings.primaryProvider,
          bubblesEnabled: appSettings.bubblesEnabled,
          startAtLogin: appSettings.startAtLogin,
          notificationsEnabled: appSettings.notificationsEnabled,
          secondaryNotificationsEnabled:
            appSettings.secondaryNotificationsEnabled,
        });
      });
    settingsQueue = operation.catch(() => undefined);
    return operation;
  };
  const pointerDown = (event: PointerEvent) => {
    pointer = beginPointer({ x: event.clientX, y: event.clientY });
  };
  const pointerMove = (event: PointerEvent) => {
    if (!pointer) return;
    const wasDragging = pointer.dragging;
    pointer = updatePointer(pointer, { x: event.clientX, y: event.clientY });
    interactionStore.setDragging(pointer.dragging);
    if (!wasDragging && pointer.dragging) void gateway.startDragging();
  };
  const pointerUp = (event: PointerEvent) => {
    if (!pointer) return;
    const completed = releasePointer(
      updatePointer(pointer, { x: event.clientX, y: event.clientY }),
    );
    pointer = null;
    interactionStore.setDragging(false);
    if (completed.kind === 'toggle_panel') void gateway.showPanel();
  };
</script>

<main
  aria-label="CacheBite"
  class:panel={windowLabel === 'panel'}
  data-collector-mode-claude={collectorMode?.claude}
  data-collector-mode-codex={collectorMode?.codex}
  data-window-label={windowLabel}
>
  <h1 class="visually-hidden">CacheBite</h1>
  {#if startupState === 'loading'}
    <p>CacheBite is starting</p>
  {:else if startupState === 'error'}
    <p role="alert">CacheBite could not start</p>
    <button onclick={() => void start()}>Retry</button>
  {:else if windowLabel === 'panel'}
    <UsagePanel
      providers={panelProviders}
      selected={$providersStore.selected}
      refreshing={$providersStore.refreshing[$providersStore.selected]}
      onSelect={(provider) => providersStore.selectTab(provider)}
      onRefresh={(provider) => providersStore.requestRefresh(provider)}
      onPrimary={(provider) =>
        void changeSettings({ ...$settingsStore, primaryProvider: provider })}
    />
    <HistoryGraph
      samples={history[$providersStore.selected]}
      window={historyWindow}
      onWindowChange={(value) => {
        historyWindow = value;
      }}
    />
    <SettingsPanel
      settings={$settingsStore}
      autostartAvailable={platformCapabilities?.autostart.status !==
        'unavailable'}
      onChange={(settings) => void changeSettings(settings)}
    />
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
    <div
      data-testid="overlay-pointer-surface"
      onpointerdown={pointerDown}
      onpointermove={pointerMove}
      onpointerup={pointerUp}
    >
      {#if overlayModel}
        <PetOverlay model={overlayModel} />
      {:else if petPackageError}
        <p role="status">Pet package unavailable</p>
      {/if}
    </div>
    {#if $interactionStore.bubblePolicy.bubble}
      <SpeechBubble
        message={$interactionStore.bubblePolicy.bubble.message}
        onDismiss={() => interactionStore.dismissBubble()}
        onOpenPanel={() => void gateway.showPanel()}
      />
    {/if}
    {#if positionSaveFailed}<p role="status">
        Window position could not be saved
      </p>{/if}
  {/if}
</main>
