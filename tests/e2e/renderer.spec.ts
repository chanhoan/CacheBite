describe('CacheBite renderer fixture flows', () => {
  it('hydrates the overlay without production collectors', async () => {
    await browser.url('/?window=overlay&fixture=e2e');
    await expect($('main[aria-label="CacheBite"]')).toBeDisplayed();
    await expect(
      $('section[aria-label="CacheBite pet status"]'),
    ).toBeDisplayed();
    await expect($('body')).not.toHaveText(
      expect.stringContaining('CacheBite is starting'),
    );
  });

  it('limits overlay hit testing to the circular surface', async () => {
    await browser.url('/?window=overlay&fixture=e2e');
    await expect($('[data-testid="overlay-pointer-surface"]')).toBeDisplayed();

    const hits = await browser.execute(() => {
      const surface = document.querySelector<HTMLElement>(
        '[data-testid="overlay-pointer-surface"]',
      );
      if (!surface) throw new Error('overlay pointer surface missing');
      const rect = surface.getBoundingClientRect();
      return {
        center: document
          .elementFromPoint(
            rect.left + rect.width / 2,
            rect.top + rect.height / 2,
          )
          ?.getAttribute('data-testid'),
        corner: document
          .elementFromPoint(rect.left + 1, rect.top + 1)
          ?.getAttribute('data-testid'),
      };
    });

    expect(hits.center).toBe('overlay-pointer-surface');
    expect(hits.corner).not.toBe('overlay-pointer-surface');
  });

  it('hydrates provider panel and reaches settings via the footer button', async () => {
    await browser.url('/?window=panel&fixture=e2e');
    await expect($('section[aria-label="Usage panel"]')).toHaveText(
      expect.stringContaining('Fixture Pro'),
    );
    const freshness = await $('.freshness');
    const freshnessText = await freshness.getText();
    expect(freshnessText).toMatch(/^● Fresh · captured /);
    expect(freshnessText).not.toMatch(/oauth_api|cli_rpc|cached/);
    expect(
      await freshness.getCSSProperty('white-space').then(({ value }) => value),
    ).toBe('nowrap');
    const visualLines = await browser.execute(() => {
      const element = document.querySelector<HTMLElement>('.freshness');
      if (!element) throw new Error('freshness line missing');
      const range = document.createRange();
      range.selectNodeContents(element);
      return new Set(
        [...range.getClientRects()].map((rect) => Math.round(rect.top)),
      ).size;
    });
    expect(visualLines).toBe(1);

    const weeklyReset = await $('section[aria-label="Weekly usage"] time');
    expect(await weeklyReset.getText()).toMatch(/^resets in \d+d \d+h \d+m$/);

    await $('button[role="tab"][aria-label="Codex"]').click();
    const primaryButton = await $('button=Set as primary');
    expect(await primaryButton.isEnabled()).toBe(true);
    await primaryButton.click();
    await browser.waitUntil(async () => !(await primaryButton.isEnabled()));

    await $('button=Settings').click();
    await expect($('input[type="checkbox"]')).toExist();
  });

  it('uses the unified 312px vibrancy panel shell', async () => {
    await browser.url('/?window=panel&fixture=e2e');
    await expect($('section[aria-label="Usage panel"]')).toBeDisplayed();

    const layout = await browser.execute(() => {
      const panel = document.querySelector<HTMLElement>('main.panel');
      const header = panel?.querySelector<HTMLElement>('.usage-panel > header');
      const body = panel?.querySelector<HTMLElement>('.usage-panel > .body');
      const footer = panel?.querySelector<HTMLElement>('.usage-panel > footer');
      if (!panel || !header || !body || !footer) {
        throw new Error('panel layout missing');
      }
      const shellStyle = getComputedStyle(panel);
      const headerStyle = getComputedStyle(header);
      const bodyStyle = getComputedStyle(body);
      const footerStyle = getComputedStyle(footer);
      return {
        platform: panel.dataset.platform,
        outerWidth: panel.getBoundingClientRect().width,
        outerHeight: panel.getBoundingClientRect().height,
        minHeight: shellStyle.minHeight,
        borderRadius: shellStyle.borderRadius,
        backdropFilter: shellStyle.backdropFilter,
        shellPadding: [
          shellStyle.paddingTop,
          shellStyle.paddingRight,
          shellStyle.paddingBottom,
          shellStyle.paddingLeft,
        ],
        headerPadding: [
          headerStyle.paddingTop,
          headerStyle.paddingRight,
          headerStyle.paddingBottom,
          headerStyle.paddingLeft,
        ],
        bodyPadding: [
          bodyStyle.paddingTop,
          bodyStyle.paddingRight,
          bodyStyle.paddingBottom,
          bodyStyle.paddingLeft,
        ],
        footerPadding: [
          footerStyle.paddingTop,
          footerStyle.paddingRight,
          footerStyle.paddingBottom,
          footerStyle.paddingLeft,
        ],
      };
    });

    expect(layout).toMatchObject({
      platform: 'linux',
      outerWidth: 312,
      minHeight: '0px',
      borderRadius: '14px',
      backdropFilter: 'blur(20px)',
      shellPadding: ['0px', '0px', '0px', '0px'],
      headerPadding: ['12px', '16px', '0px', '16px'],
      bodyPadding: ['16px', '16px', '16px', '16px'],
      footerPadding: ['12px', '16px', '14px', '16px'],
    });
    expect(layout.outerHeight).toBeLessThan(520);
  });
});
