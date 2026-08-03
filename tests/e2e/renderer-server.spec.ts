import { expect } from '@wdio/globals';

describe('renderer E2E server lifecycle', () => {
  it('serves a warmed fixture renderer without a separately started server', async () => {
    await browser.url('/?window=panel&fixture=e2e');
    const panel = await $('section[aria-label="Usage panel"]');
    await expect(panel).toBeDisplayed();
  });
});
