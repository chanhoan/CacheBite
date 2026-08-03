const expectedMode = process.env.CACHEBITE_EXPECTED_COLLECTOR_MODE;

type ProviderStates = {
  claude: { unavailable_reason: string | null; snapshot: unknown | null };
  codex: { unavailable_reason: string | null; snapshot: unknown | null };
};

type HistoryStore = {
  claude: { samples: unknown[] };
  codex: { samples: unknown[] };
};

type NativeSettings = {
  primary_provider: 'claude' | 'codex';
};

type NativeWindowState = {
  is_visible: boolean;
  label?: string;
};

type NativeWindowStates =
  | NativeWindowState[]
  | Record<string, NativeWindowState>;

type InvokeResult<T> =
  | { status: 'resolved'; value: T }
  | { status: 'rejected'; reason: string };

if (expectedMode !== 'fixture' && expectedMode !== 'production') {
  throw new Error(
    'CACHEBITE_EXPECTED_COLLECTOR_MODE must be fixture or production',
  );
}

describe(`CacheBite native ${expectedMode} composition smoke`, () => {
  const switchToCacheBiteWindow = async (label: 'overlay' | 'panel') => {
    await browser.waitUntil(
      async () => {
        const handles = await browser.getWindowHandles();
        const labeledHandle = handles.find((handle) => handle === label);
        const candidates = labeledHandle
          ? [labeledHandle, ...handles.filter((handle) => handle !== label)]
          : handles;

        for (const handle of candidates) {
          await browser.switchToWindow(handle);
          const main = $('main[aria-label="CacheBite"]');
          if (
            (await main.isExisting()) &&
            (await main.getAttribute('data-window-label')) === label
          )
            return true;
        }
        return false;
      },
      { timeoutMsg: `CacheBite ${label} window was not found` },
    );
  };

  const invokeFromCurrentWindow = async <T = undefined>(command: string) =>
    browser.executeAsync(
      (requestedCommand: string, done: (result: InvokeResult<T>) => void) => {
        const internals = (
          window as Window & {
            __TAURI_INTERNALS__: {
              invoke<T>(command: string): Promise<T>;
            };
          }
        ).__TAURI_INTERNALS__;
        void internals
          .invoke<T>(requestedCommand)
          .then((value) => done({ status: 'resolved', value }))
          .catch((reason: unknown) =>
            done({ status: 'rejected', reason: String(reason) }),
          );
      },
      command,
    );

  const getNativeWindowStates = async () => {
    const result =
      await invokeFromCurrentWindow<NativeWindowStates>('get_window_states');

    if (result.status !== 'resolved')
      throw new Error(`get_window_states was rejected: ${result.reason}`);

    return result.value;
  };

  const getNativeWindowState = async (label: 'overlay' | 'panel') => {
    const windowStates = await getNativeWindowStates();
    const states = Array.isArray(windowStates)
      ? windowStates
      : Object.entries(windowStates).map(([entryLabel, state]) => ({
          ...state,
          label: state.label ?? entryLabel,
        }));
    const nativeWindowState = states.find((state) => state.label === label);

    if (!nativeWindowState)
      throw new Error(`Native window state for ${label} was not found`);

    return nativeWindowState;
  };

  const waitForPanelVisibility = async (expectedVisible: boolean) => {
    await browser.waitUntil(
      async () =>
        (await getNativeWindowState('panel')).is_visible === expectedVisible,
      {
        timeout: 10_000,
        interval: 100,
        timeoutMsg: `Panel window was not ${
          expectedVisible ? 'visible' : 'hidden'
        }`,
      },
    );
  };

  const hidePanelFromPanelWindow = async () => {
    await switchToCacheBiteWindow('panel');
    const hideResult = await invokeFromCurrentWindow('hide_panel');

    if (hideResult.status !== 'resolved')
      throw new Error(`hide_panel was rejected: ${hideResult.reason}`);

    await waitForPanelVisibility(false);
  };

  beforeEach(async () => {
    await hidePanelFromPanelWindow();
    await switchToCacheBiteWindow('overlay');
  });

  afterEach(async () => {
    try {
      await hidePanelFromPanelWindow();
    } finally {
      await switchToCacheBiteWindow('overlay');
    }
  });

  const waitForPersistedPrimary = async (
    provider: NativeSettings['primary_provider'],
  ) => {
    let settings: InvokeResult<NativeSettings> | undefined;
    await browser.waitUntil(
      async () => {
        settings =
          await invokeFromCurrentWindow<NativeSettings>('get_settings');
        return (
          settings.status === 'resolved' &&
          settings.value.primary_provider === provider
        );
      },
      {
        timeout: 10_000,
        interval: 100,
        timeoutMsg: `Primary provider was not persisted as ${provider}`,
      },
    );
  };

  it('hydrates through registered IPC and reports the selected collectors', async () => {
    const app = $('main[aria-label="CacheBite"]');
    await expect(app).toExist();
    await expect(app).toHaveAttribute(
      'data-collector-mode-claude',
      expectedMode,
    );
    await expect(app).toHaveAttribute(
      'data-collector-mode-codex',
      expectedMode,
    );
    const bodyText = await $('body').getText();
    expect(bodyText).not.toContain('CacheBite is starting');
    expect(bodyText).not.toContain('CacheBite could not start');
    expect(bodyText).not.toContain('Pet package unavailable');
  });

  it('authorizes history IPC only from the panel window', async () => {
    const overlayHistory = await invokeFromCurrentWindow('get_history');
    expect(overlayHistory).toEqual({
      status: 'rejected',
      reason: 'forbidden',
    });

    await $(
      'main[data-window-label="overlay"] [data-testid="overlay-pointer-surface"]',
    ).doubleClick();
    await switchToCacheBiteWindow('panel');

    await expect($('section[aria-label="Usage panel"]')).toExist();
    const panelHistory =
      await invokeFromCurrentWindow<HistoryStore>('get_history');
    expect(panelHistory.status).toBe('resolved');
    if (panelHistory.status !== 'resolved')
      throw new Error(`Panel history unavailable: ${panelHistory.reason}`);
    expect(Array.isArray(panelHistory.value.claude.samples)).toBe(true);
    expect(Array.isArray(panelHistory.value.codex.samples)).toBe(true);
  });

  it('toggles the panel from the pet and returns to the state it started in', async () => {
    await switchToCacheBiteWindow('overlay');

    await waitForPanelVisibility(false);

    const toggleAndWaitForVisibility = async (expectedVisible: boolean) => {
      const result = await invokeFromCurrentWindow<'shown' | 'hidden'>(
        'toggle_panel',
      );

      if (result.status !== 'resolved')
        throw new Error(`toggle_panel was rejected: ${result.reason}`);

      expect(result.value).toBe(expectedVisible ? 'shown' : 'hidden');
      await waitForPanelVisibility(expectedVisible);

      return result.value;
    };

    const toggleResults = [
      await toggleAndWaitForVisibility(true),
      await toggleAndWaitForVisibility(false),
      await toggleAndWaitForVisibility(true),
      await toggleAndWaitForVisibility(false),
    ];

    expect(toggleResults).toEqual(['shown', 'hidden', 'shown', 'hidden']);
    await waitForPanelVisibility(false);
  });

  it('authorizes the panel toggle only from the overlay window', async () => {
    await switchToCacheBiteWindow('overlay');
    await $(
      'main[data-window-label="overlay"] [data-testid="overlay-pointer-surface"]',
    ).doubleClick();
    await switchToCacheBiteWindow('panel');
    await expect($('section[aria-label="Usage panel"]')).toExist();

    // The panel dismisses itself with `hide_panel`; the toggle is the pet's.
    expect(await invokeFromCurrentWindow('toggle_panel')).toEqual({
      status: 'rejected',
      reason: 'forbidden',
    });
  });

  if (expectedMode === 'production') {
    it('shows credential-free production provider states after panel hydration', async () => {
      await switchToCacheBiteWindow('overlay');
      await $(
        'main[data-window-label="overlay"] [data-testid="overlay-pointer-surface"]',
      ).doubleClick();
      await switchToCacheBiteWindow('panel');

      const claudeTab = $('button[role="tab"]=Claude');
      await claudeTab.click();
      await expect(claudeTab).toHaveAttribute('aria-selected', 'true');
      const panelText = await $('section[aria-label="Usage panel"]').getText();
      expect(panelText).not.toMatch(/oauth_api|cli_rpc|cached/);

      let providerStates: InvokeResult<ProviderStates> | undefined;
      try {
        await browser.waitUntil(
          async () => {
            providerStates = await invokeFromCurrentWindow<ProviderStates>(
              'get_provider_states',
            );
            return (
              providerStates.status === 'resolved' &&
              providerStates.value.claude.unavailable_reason ===
                'not_signed_in' &&
              providerStates.value.codex.unavailable_reason ===
                'not_installed' &&
              providerStates.value.claude.snapshot === null &&
              providerStates.value.codex.snapshot === null
            );
          },
          {
            timeout: 15_000,
            interval: 250,
            timeoutMsg:
              'Provider collectors did not publish unavailable states',
          },
        );
      } catch (error) {
        throw new Error(
          `Provider states were not ready: ${JSON.stringify(providerStates)}`,
          { cause: error },
        );
      }
      if (!providerStates || providerStates.status !== 'resolved')
        throw new Error('Provider states were not available from the panel');
      expect(providerStates.value.claude.unavailable_reason).toBe(
        'not_signed_in',
      );
      expect(providerStates.value.codex.unavailable_reason).toBe(
        'not_installed',
      );
      expect(providerStates.value.claude.snapshot).toBeNull();
      expect(providerStates.value.codex.snapshot).toBeNull();

      const codexTab = $('button[role="tab"]=Codex');
      try {
        await codexTab.click();
        await expect(codexTab).toHaveAttribute('aria-selected', 'true');
        await expect(claudeTab).toHaveAttribute(
          'aria-label',
          'Claude (primary)',
        );
        const setPrimary = $('button=Set as primary');
        await expect(setPrimary).toBeEnabled();
        await setPrimary.click();
        await waitForPersistedPrimary('codex');
        await expect(codexTab).toHaveAttribute('aria-label', 'Codex (primary)');
        await browser.waitUntil(async () => !(await setPrimary.isEnabled()));
        await expect($('body')).not.toHaveText(
          expect.stringContaining('Settings could not be saved'),
        );
        await expect($('body')).not.toHaveText(
          expect.stringContaining('autostart integration is unavailable'),
        );
      } finally {
        await claudeTab.click();
        const setPrimary = $('button=Set as primary');
        if (await setPrimary.isEnabled()) await setPrimary.click();
        await waitForPersistedPrimary('claude');
        await expect(claudeTab).toHaveAttribute(
          'aria-label',
          'Claude (primary)',
        );
      }
    });
  }
});
