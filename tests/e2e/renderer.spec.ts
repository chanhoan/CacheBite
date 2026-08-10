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

  it('keeps the overlay toast below the usage ring and inside the viewport', async () => {
    await browser.url('/?window=overlay&fixture=e2e&toast=layout');
    const originalViewport = await browser.execute(() => ({
      width: window.innerWidth,
      height: window.innerHeight,
      devicePixelRatio: window.devicePixelRatio,
    }));
    try {
      await browser.setViewport({
        width: 240,
        height: 240,
        devicePixelRatio: 1,
      });
      const toast = await $('[data-testid="overlay-toast"]');
      await toast.waitForExist();

      const expectedMessage =
        'Weekly usage is nearly exhausted and will reset after the current window';
      const layout = await browser.execute((message) => {
        const toast = document.querySelector<HTMLElement>(
          '[data-testid="overlay-toast"]',
        );
        const ring = document.querySelector<HTMLElement>(
          '[data-testid="usage-ring"]',
        );
        if (!toast || !ring) {
          throw new Error('overlay layout missing');
        }
        const messageNode = toast.querySelector<HTMLElement>('.toast-message');
        if (!messageNode) {
          throw new Error('toast message missing');
        }
        const ringRect = ring.getBoundingClientRect();
        const visualBottom = Math.max(
          ringRect.bottom,
          ...[...ring.querySelectorAll<HTMLElement>('.ring-label')].map(
            (label) => label.getBoundingClientRect().bottom,
          ),
        );
        messageNode.textContent = message;
        const toastRect = toast.getBoundingClientRect();
        return {
          ringBottom: ringRect.bottom,
          visualBottom,
          toastTop: toastRect.top,
          toastBottom: toastRect.bottom,
          toastLeft: toastRect.left,
          toastRight: toastRect.right,
          viewportHeight: window.innerHeight,
          viewportWidth: window.innerWidth,
          messageScrollWidth: messageNode.scrollWidth,
          messageClientWidth: messageNode.clientWidth,
          message: messageNode.textContent,
          expectedMessage: message,
        };
      }, expectedMessage);

      expect(layout.toastTop).toBeGreaterThanOrEqual(layout.visualBottom + 8);
      expect(layout.toastBottom).toBeLessThanOrEqual(layout.viewportHeight);
      expect(layout.toastLeft).toBeGreaterThanOrEqual(0);
      expect(layout.toastRight).toBeLessThanOrEqual(layout.viewportWidth);
      expect(layout.messageScrollWidth).toBeLessThanOrEqual(
        layout.messageClientWidth,
      );
      expect(layout.message).toBe(layout.expectedMessage);
    } finally {
      await browser.setViewport({
        width: originalViewport.width,
        height: originalViewport.height,
        devicePixelRatio: originalViewport.devicePixelRatio,
      });
    }
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

  // jsdom computes no motion paths and no stacking, so the unit tests can only
  // prove the geometry module's arithmetic. These two specs are the only place
  // a real engine is asked whether it draws that arithmetic.
  it('rides both marks on their own circles and stacks them for depth', async () => {
    await browser.url('/?window=overlay&fixture=e2e&ring=double');
    await expect($('[data-testid="satellite-ring"]')).toBeDisplayed();
    await expect($('[data-testid="satellite-orbit-walker"]')).toBeDisplayed();

    const scene = await browser.execute(() => {
      const measure = (selector: string) => {
        const node = document.querySelector<HTMLElement>(selector);
        if (!node) throw new Error(`${selector} missing`);
        const rect = node.getBoundingClientRect();
        return {
          x: rect.left + rect.width / 2,
          y: rect.top + rect.height / 2,
          left: rect.left,
          top: rect.top,
          right: rect.right,
          bottom: rect.bottom,
          width: rect.width,
          zIndex: getComputedStyle(node).zIndex,
        };
      };
      return {
        overlay: measure('section[aria-label="CacheBite pet status"]'),
        bigMark: measure('[data-testid="orbit-walker"]'),
        smallMark: measure('[data-testid="satellite-orbit-walker"]'),
        satellite: measure('[data-testid="satellite-ring"]'),
        ring: measure('section[aria-label="CacheBite pet status"] > .ring'),
      };
    });

    const size = scene.overlay.width;
    const from = (
      a: { x: number; y: number },
      b: { x: number; y: number },
    ): number => Math.hypot(a.x - b.x, a.y - b.y) / size;

    // Orbit radii from `orbitPath.ts`, as fractions of the overlay box:
    // 45.25 + 7 for the primary, 18 + 4.5 for the satellite. Getting these
    // means `offset-path` ran — without it both marks stay pinned at the
    // overlay's top-left corner, which is the documented degradation.
    expect(from(scene.bigMark, scene.overlay)).toBeCloseTo(0.5225, 2);
    expect(from(scene.smallMark, scene.satellite)).toBeCloseTo(0.225, 2);

    // The size clamp has to survive the marks' rotation: `offset-rotate: auto`
    // turns each square, so its axis-aligned corner reaches further than half
    // its side. `OVERLAY_BOUNDS_FACTOR / 2` is the half-extent that budgets for.
    const reach = size * 0.9186;
    for (const mark of [scene.bigMark, scene.smallMark]) {
      expect(mark.left).toBeGreaterThanOrEqual(scene.overlay.x - reach - 1);
      expect(mark.right).toBeLessThanOrEqual(scene.overlay.x + reach + 1);
      expect(mark.top).toBeGreaterThanOrEqual(scene.overlay.y - reach - 1);
      expect(mark.bottom).toBeLessThanOrEqual(scene.overlay.y + reach + 1);
    }

    // Depth runs by group, not by element: the near ring's mark passes in front
    // of the far ring, and the far ring's mark passes behind the near one.
    // Scoped-CSS breakage would show up here as an `auto` where a number
    // belongs, which no unit test can see.
    expect(scene.bigMark.zIndex).toBe('3');
    expect(scene.ring.zIndex).toBe('1');
    expect(scene.smallMark.zIndex).toBe('0');
  });

  it('routes a press on the satellite puck to its own drag surface', async () => {
    await browser.url('/?window=overlay&fixture=e2e&ring=double');
    await expect(
      $('[data-testid="overlay-satellite-pointer-surface"]'),
    ).toBeExisting();

    const hits = await browser.execute(() => {
      const satellite = document.querySelector<HTMLElement>(
        '[data-testid="satellite-ring"]',
      );
      if (!satellite) throw new Error('satellite ring missing');
      const rect = satellite.getBoundingClientRect();
      return {
        centre: document
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

    // The puck sits outside the pet's circular surface, so without its own hit
    // area it would be a dead zone in the middle of the drag gesture.
    expect(hits.centre).toBe('overlay-satellite-pointer-surface');
    // Clipped to a circle like the pet's, so the bounding box corner misses.
    expect(hits.corner).not.toBe('overlay-satellite-pointer-surface');
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

  it('layers the close control over the header without reserving space', async () => {
    await browser.url('/?window=panel&fixture=e2e');
    await expect($('section[aria-label="Usage panel"]')).toBeDisplayed();

    const geometry = await browser.execute(() => {
      const panel = document.querySelector<HTMLElement>('main.panel');
      const close = panel?.querySelector<HTMLElement>('.close-panel');
      const header = panel?.querySelector<HTMLElement>('.usage-panel > header');
      if (!panel || !close || !header) {
        throw new Error('panel close control missing');
      }
      const panelBox = panel.getBoundingClientRect();
      const closeBox = close.getBoundingClientRect();
      const headerBox = header.getBoundingClientRect();
      return {
        position: getComputedStyle(close).position,
        // Out of flow: the header still starts at the shell's content edge and
        // spans its full inner width, exactly as it did without the control.
        // `clientTop`/`clientWidth` exclude the shell's 1px border, which
        // `getBoundingClientRect` includes.
        headerTopOffset: Math.round(
          headerBox.top - panelBox.top - panel.clientTop,
        ),
        headerWidth: Math.round(headerBox.width),
        panelInnerWidth: panel.clientWidth,
        closeInsideShell:
          closeBox.right <= panelBox.right && closeBox.top >= panelBox.top,
      };
    });

    expect(geometry.position).toBe('absolute');
    expect(geometry.headerTopOffset).toBe(0);
    // Compared with a 1px tolerance: `getBoundingClientRect` returns fractional
    // widths under a fractional device pixel ratio while `clientWidth` is
    // integral, so an exact equality would fail on scaled displays for a header
    // that in fact still spans the full shell.
    expect(
      Math.abs(geometry.headerWidth - geometry.panelInnerWidth),
    ).toBeLessThanOrEqual(1);
    expect(geometry.closeInsideShell).toBe(true);
  });

  // The close control deliberately layers over the second tab rather than making
  // the tab strip yield width (ui-contract.md §5). That trade-off is only
  // acceptable while the overlap stays small, so the measured extent is pinned
  // here: growing the icon or shrinking the header padding fails this test
  // instead of silently eating more of the tab.
  it('keeps the close control overlap over the second tab within contract', async () => {
    await browser.url('/?window=panel&fixture=e2e');
    await expect($('section[aria-label="Usage panel"]')).toBeDisplayed();

    const overlap = await browser.execute(() => {
      const panel = document.querySelector<HTMLElement>('main.panel');
      const close = panel?.querySelector<HTMLElement>('.close-panel');
      const codex = [
        ...(panel?.querySelectorAll<HTMLElement>('[role="tab"]') ?? []),
      ].find((tab) => tab.getAttribute('aria-label')?.startsWith('Codex'));
      if (!panel || !close || !codex) {
        throw new Error('panel close control or Codex tab missing');
      }
      const c = close.getBoundingClientRect();
      const t = codex.getBoundingClientRect();
      const hit = (x: number, y: number) =>
        document.elementFromPoint(x, y)?.getAttribute('aria-label') ?? '';
      return {
        width: Math.max(
          0,
          Math.min(c.right, t.right) - Math.max(c.left, t.left),
        ),
        height: Math.max(
          0,
          Math.min(c.bottom, t.bottom) - Math.max(c.top, t.top),
        ),
        tabWidth: t.width,
        // The tab must stay usable everywhere the control does not cover.
        hitAtTabCentre: hit(t.left + t.width / 2, t.top + t.height / 2),
        hitAtTabLeftEdge: hit(t.left + 4, t.top + t.height / 2),
      };
    });

    expect(Math.round(overlap.width)).toBeLessThanOrEqual(14);
    expect(Math.round(overlap.height)).toBeLessThanOrEqual(18);
    // 14px of a ~139px tab measures 10.1%; the cap sits just above that so the
    // contract figure is pinned without failing on sub-pixel tab widths.
    expect(overlap.width / overlap.tabWidth).toBeLessThanOrEqual(0.105);
    expect(overlap.hitAtTabCentre).toBe('Codex');
    expect(overlap.hitAtTabLeftEdge).toBe('Codex');
  });
});
