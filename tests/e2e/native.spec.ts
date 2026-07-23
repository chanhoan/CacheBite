const expectedMode = process.env.CACHEBITE_EXPECTED_COLLECTOR_MODE;

type ProviderStates = {
  claude: { unavailable_reason: string | null; snapshot: unknown | null };
  codex: { unavailable_reason: string | null; snapshot: unknown | null };
};

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

  beforeEach(async () => {
    await switchToCacheBiteWindow('overlay');
  });

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

  it('hydrates panel history through registered IPC', async () => {
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
    await expect($('section[aria-label="Usage history"]')).toExist();
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
      await expect($('section[aria-label="Usage panel"]')).toHaveText(
        expect.stringContaining('oauth_api'),
      );

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
        await expect(codexTab).toHaveAttribute('aria-label', 'Codex (primary)');
        await expect($('body')).not.toHaveText(
          expect.stringContaining('Settings could not be saved'),
        );
        await expect($('body')).not.toHaveText(
          expect.stringContaining('autostart integration is unavailable'),
        );
      } finally {
        await claudeTab.click();
        await expect(claudeTab).toHaveAttribute(
          'aria-label',
          'Claude (primary)',
        );
      }
    });
  }
});
