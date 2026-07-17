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

  it('hydrates provider panel, history, and settings', async () => {
    await browser.url('/?window=panel&fixture=e2e');
    await expect($('section[aria-label="Usage panel"]')).toHaveText(
      expect.stringContaining('Fixture Pro'),
    );
    await expect($('svg[aria-label="5-hour usage history"]')).toBeDisplayed();
    await expect($('input[type="checkbox"]')).toExist();
  });
});
