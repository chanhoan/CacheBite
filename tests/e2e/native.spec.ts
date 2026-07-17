const expectedMode = process.env.CACHEBITE_EXPECTED_COLLECTOR_MODE;

if (expectedMode !== 'fixture' && expectedMode !== 'production') {
  throw new Error(
    'CACHEBITE_EXPECTED_COLLECTOR_MODE must be fixture or production',
  );
}

describe(`CacheBite native ${expectedMode} composition smoke`, () => {
  const switchToCacheBiteWindow = async (label: 'overlay' | 'panel') => {
    for (const handle of await browser.getWindowHandles()) {
      await browser.switchToWindow(handle);
      const main = $('main[aria-label="CacheBite"]');
      if (
        (await main.isExisting()) &&
        (await main.getAttribute('data-window-label')) === label
      )
        return;
    }
    throw new Error(`CacheBite ${label} window was not found`);
  };

  beforeEach(async () => {
    await switchToCacheBiteWindow('overlay');
  });

  const invokeFromCurrentWindow = async (command: string) =>
    browser.executeAsync(
      (
        requestedCommand: string,
        done: (result: { status: string; reason?: string }) => void,
      ) => {
        const internals = (
          window as Window & {
            __TAURI_INTERNALS__: {
              invoke<T>(command: string): Promise<T>;
            };
          }
        ).__TAURI_INTERNALS__;
        void internals
          .invoke(requestedCommand)
          .then(() => done({ status: 'resolved' }))
          .catch((reason: unknown) =>
            done({ status: 'rejected', reason: String(reason) }),
          );
      },
      command,
    );

  const bubblesToggle = () =>
    $('label*=Speech bubbles').$('input[type="checkbox"]');
  const reloadUntilBubblesSetting = async (expected: boolean) => {
    await browser.waitUntil(
      async () => {
        await browser.refresh();
        const toggle = bubblesToggle();
        await toggle.waitForExist();
        return (await toggle.isSelected()) === expected;
      },
      {
        timeout: 5_000,
        interval: 100,
        timeoutMsg: `speech-bubble setting was not persisted as ${expected}`,
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

  it('hydrates panel history and round-trips a representative setting', async () => {
    const overlayHistory = await invokeFromCurrentWindow('get_history');
    expect(overlayHistory).toEqual({
      status: 'rejected',
      reason: 'forbidden',
    });

    await $(
      'main[data-window-label="overlay"] [data-testid="overlay-pointer-surface"]',
    ).click();
    await switchToCacheBiteWindow('panel');

    await expect($('section[aria-label="Usage panel"]')).toExist();
    await expect($('section[aria-label="Usage history"]')).toExist();
    const bubbles = bubblesToggle();
    await expect(bubbles).toExist();
    const initial = await bubbles.isSelected();
    try {
      await bubbles.click();
      await reloadUntilBubblesSetting(!initial);
    } finally {
      const persisted = bubblesToggle();
      if ((await persisted.isSelected()) !== initial) await persisted.click();
      await reloadUntilBubblesSetting(initial);
    }
  });

  if (expectedMode === 'production') {
    it('shows credential-free production provider states after panel hydration', async () => {
      await switchToCacheBiteWindow('overlay');
      await $(
        'main[data-window-label="overlay"] [data-testid="overlay-pointer-surface"]',
      ).click();
      await switchToCacheBiteWindow('panel');

      const claudeTab = $('button[role="tab"]=Claude');
      await claudeTab.click();
      await expect(claudeTab).toHaveAttribute('aria-selected', 'true');
      await expect($('section[aria-label="Usage panel"]')).toHaveText(
        expect.stringContaining('auth_required'),
      );

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
        await expect($('body')).not.toHaveText(
          expect.stringContaining('fullscreen detection is unavailable'),
        );
        await expect($('section[aria-label="Usage panel"]')).toHaveText(
          expect.stringContaining('unavailable'),
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
