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

  it('hydrates provider panel, history, and settings', async () => {
    await browser.url('/?window=panel&fixture=e2e');
    await expect($('section[aria-label="Usage panel"]')).toHaveText(
      expect.stringContaining('Fixture Pro'),
    );
    await expect($('svg[aria-label="5-hour usage history"]')).toBeDisplayed();
    await expect($('input[type="checkbox"]')).toExist();
  });
});
