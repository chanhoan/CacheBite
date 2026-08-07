import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent } from '@testing-library/svelte';

import PetOverlay from './PetOverlay.svelte';
import type {
  OrbitModel,
  PetOverlayViewModel,
  SatelliteRingModel,
} from './models';

const animation = {
  type: 'frames' as const,
  sources: ['/fixtures/idle-01.svg', '/fixtures/idle-02.svg'],
  frameDurationMs: 120,
};

const orbit: OrbitModel = {
  provider: 'claude',
  providerName: 'Claude',
  direction: 'forward',
};

describe('PetOverlay', () => {
  afterEach(cleanup);

  it('shows two accessible usage arcs for active usage', () => {
    render(PetOverlay, {
      props: {
        model: {
          system: 'active',
          stale: false,
          session: { usedPercent: 74, severity: 'warn' },
          weekly: { usedPercent: 93, severity: 'critical' },
          animation,
          petName: 'Geometric pet',
          size: 160,
          satellite: null,
          orbit,
        },
      },
    });

    // One composed label on the <svg>: a bare <path> has no implicit role, so
    // per-arc labels never reach the accessibility tree.
    expect(
      screen.getByRole('img', {
        name: 'Provider usage: 5-hour 74%, Weekly 93%',
      }),
    ).toBeTruthy();
    expect(screen.getByText('5H')).toBeTruthy();
    expect(screen.getByText('WK')).toBeTruthy();
    expect(screen.getByText('5H').getAttribute('font-size')).toBe('9');
    expect(screen.getByText('WK').getAttribute('font-size')).toBe('9');
    expect(screen.getByText('WK').getAttribute('y')).toBe('104');
    const surface = screen.getByRole('button', {
      name: 'Move pet; double-click or press Enter to show or hide usage; right-click for the menu',
    });
    expect(surface.getAttribute('data-testid')).toBe('overlay-pointer-surface');
    expect(surface.style.clipPath).toBe('circle(50% at 50% 50%)');
    expect(screen.queryByRole('status')).toBeNull();
  });

  it('requests the native menu on right-click instead of the browser menu', async () => {
    const onShowMenu = vi.fn();
    render(PetOverlay, {
      props: {
        model: {
          system: 'active',
          stale: false,
          session: { usedPercent: 74, severity: 'warn' },
          weekly: { usedPercent: 93, severity: 'critical' },
          animation,
          petName: 'Geometric pet',
          size: 160,
          satellite: null,
          orbit,
        },
        onShowMenu,
      },
    });

    const surface = screen.getByTestId('overlay-pointer-surface');
    const contextMenu = new MouseEvent('contextmenu', {
      bubbles: true,
      cancelable: true,
    });
    surface.dispatchEvent(contextMenu);

    expect(onShowMenu).toHaveBeenCalledOnce();
    // The webview's own context menu must never appear over the pet.
    expect(contextMenu.defaultPrevented).toBe(true);
  });

  it('routes both the double-click and the Enter key to a single toggle request', async () => {
    const onToggle = vi.fn();
    render(PetOverlay, {
      props: {
        model: {
          system: 'active',
          stale: false,
          session: { usedPercent: 74, severity: 'warn' },
          weekly: { usedPercent: 93, severity: 'critical' },
          animation,
          petName: 'Geometric pet',
          size: 160,
          satellite: null,
          orbit,
        },
        onToggle,
      },
    });

    const surface = screen.getByTestId('overlay-pointer-surface');

    await fireEvent.dblClick(surface);
    expect(onToggle).toHaveBeenCalledOnce();
    await fireEvent.keyDown(surface, { key: 'Enter' });
    expect(onToggle).toHaveBeenCalledTimes(2);
  });

  it('renders an unknown window as a neutral unfilled track', () => {
    render(PetOverlay, {
      props: {
        model: {
          system: 'active',
          stale: false,
          session: { usedPercent: null, severity: 'unknown' },
          weekly: { usedPercent: 15, severity: 'ok' },
          animation,
          petName: 'Geometric pet',
          size: 160,
          satellite: null,
          orbit,
        },
      },
    });

    expect(
      screen.getByRole('img', {
        name: 'Provider usage: 5-hour unknown, Weekly 15%',
      }),
    ).toBeTruthy();
    const unknown = screen.getByTestId('usage-ring').querySelector('.usage');
    expect(unknown?.getAttribute('data-severity')).toBe('unknown');
    expect(unknown?.getAttribute('stroke-dasharray')).toBe('0 100');
  });

  it('renders at the size the manifest declared', () => {
    const { container } = render(PetOverlay, {
      props: {
        model: {
          system: 'active',
          stale: false,
          session: { usedPercent: 10, severity: 'ok' },
          weekly: { usedPercent: 10, severity: 'ok' },
          animation,
          petName: 'Geometric pet',
          size: 192,
          satellite: null,
          orbit,
        },
      },
    });

    expect(
      (container.querySelector('.overlay') as HTMLElement).style.width,
    ).toBe('192px');
  });

  it('dims only the ring when usage is stale', () => {
    const { container } = render(PetOverlay, {
      props: {
        model: {
          system: 'active',
          stale: true,
          session: { usedPercent: 42, severity: 'ok' },
          weekly: { usedPercent: 55, severity: 'ok' },
          animation,
          petName: 'Geometric pet',
          size: 160,
          satellite: null,
          orbit,
        },
      },
    });

    expect(
      container
        .querySelector('[data-testid="usage-ring"]')
        ?.getAttribute('data-stale'),
    ).toBe('true');
    expect(
      screen
        .getByRole('img', { name: 'Geometric pet' })
        .hasAttribute('data-stale'),
    ).toBe(false);
  });

  it.each([
    ['auth_required', 'Authentication required'],
    ['unavailable', 'Provider unavailable'],
    ['error', 'Usage unavailable due to an error'],
    ['offline', 'Network offline'],
    ['loading', 'Loading usage'],
  ] as const)('shows the %s badge and hides the ring', (system, label) => {
    const { container } = render(PetOverlay, {
      props: {
        model: {
          system,
          stale: false,
          session: { usedPercent: null, severity: 'unknown' },
          weekly: { usedPercent: null, severity: 'unknown' },
          animation,
          petName: 'Geometric pet',
          size: 160,
          satellite: null,
          orbit,
        },
      },
    });

    expect(screen.getByRole('status').getAttribute('aria-label')).toBe(label);
    expect(screen.getByRole('status').querySelector('svg')).toBeTruthy();
    expect(container.querySelector('[data-testid="usage-ring"]')).toBeNull();
  });

  describe('double ring mode', () => {
    const satellite = (
      overrides: Partial<SatelliteRingModel> = {},
    ): SatelliteRingModel => ({
      provider: 'codex',
      providerName: 'Codex',
      system: 'active',
      stale: false,
      session: { usedPercent: 41, severity: 'ok' },
      weekly: { usedPercent: 58, severity: 'ok' },
      ...overrides,
    });
    const model = (
      satelliteModel: SatelliteRingModel | null,
    ): PetOverlayViewModel => ({
      system: 'active',
      stale: false,
      session: { usedPercent: 74, severity: 'warn' },
      weekly: { usedPercent: 93, severity: 'critical' },
      animation,
      petName: 'Geometric pet',
      size: 160,
      satellite: satelliteModel,
      orbit,
    });

    it('names the satellite by its provider while the big ring stays generic', () => {
      render(PetOverlay, { props: { model: model(satellite()) } });

      // The big ring is whatever `primaryProvider` points at and the panel
      // already names it; only the odd one out needs disambiguating.
      expect(
        screen.getByRole('img', {
          name: 'Provider usage: 5-hour 74%, Weekly 93%',
        }),
      ).toBeTruthy();
      expect(
        screen.getByRole('img', {
          name: 'Codex usage: 5-hour 41%, Weekly 58%',
        }),
      ).toBeTruthy();
      expect(screen.getByTestId('provider-logo-codex')).toBeTruthy();
    });

    it('nests the satellite ring below the overlay rule', () => {
      const { container } = render(PetOverlay, {
        props: { model: model(satellite()) },
      });

      // `.overlay` scopes its full-bleed `.ring` geometry to a direct child so
      // it cannot reach the satellite's ring, which sizes itself against the
      // puck. Flattening this nesting would silently hand both rings the big
      // one's geometry — at equal specificity, so nothing would warn.
      const overlay = container.querySelector('.overlay');
      const directRings = [...(overlay?.children ?? [])].filter((child) =>
        child.classList.contains('ring'),
      );
      expect(directRings).toHaveLength(1);
      expect(
        container.querySelector(
          '[data-testid="satellite-ring"] [data-testid="usage-ring"]',
        )?.parentElement,
      ).not.toBe(overlay);
    });

    it('omits the satellite entirely in single mode', () => {
      render(PetOverlay, { props: { model: model(null) } });

      expect(screen.queryByTestId('satellite-ring')).toBeNull();
      expect(
        screen.queryByTestId('overlay-satellite-pointer-surface'),
      ).toBeNull();
    });

    it('routes gestures over the satellite to the same handlers', async () => {
      const onToggle = vi.fn();
      const onShowMenu = vi.fn();
      render(PetOverlay, {
        props: { model: model(satellite()), onToggle, onShowMenu },
      });

      const surface = screen.getByTestId('overlay-satellite-pointer-surface');
      await fireEvent.dblClick(surface);
      const contextMenu = new MouseEvent('contextmenu', {
        bubbles: true,
        cancelable: true,
      });
      surface.dispatchEvent(contextMenu);

      expect(onToggle).toHaveBeenCalledOnce();
      expect(onShowMenu).toHaveBeenCalledOnce();
      expect(contextMenu.defaultPrevented).toBe(true);
    });

    it('keeps the satellite surface out of the accessibility tree', () => {
      render(PetOverlay, { props: { model: model(satellite()) } });

      // A second control with the same name would make every
      // getByRole('button', { name }) query in this file ambiguous.
      expect(screen.getAllByRole('button')).toHaveLength(1);
      expect(
        screen
          .getByTestId('overlay-satellite-pointer-surface')
          .getAttribute('aria-hidden'),
      ).toBe('true');
    });

    it('dims only the ring that is stale', () => {
      const { container } = render(PetOverlay, {
        props: { model: model(satellite({ stale: true })) },
      });

      const rings = [
        ...container.querySelectorAll('[data-testid="usage-ring"]'),
      ];
      expect(rings.map((ring) => ring.getAttribute('data-stale'))).toEqual([
        'false',
        'true',
      ]);
    });

    it('badges a failing satellite without blanking the big ring', () => {
      const { container } = render(PetOverlay, {
        props: { model: model(satellite({ system: 'auth_required' })) },
      });

      // Provider independence (contract section 5): the satellite drops to a
      // badge while the primary keeps both arcs.
      expect(screen.getByRole('status').getAttribute('aria-label')).toBe(
        'Authentication required',
      );
      expect(
        container.querySelectorAll('[data-testid="usage-ring"]'),
      ).toHaveLength(1);
      expect(
        screen.getByRole('img', {
          name: 'Provider usage: 5-hour 74%, Weekly 93%',
        }),
      ).toBeTruthy();
      // The logo still identifies which provider needs attention.
      expect(screen.getByTestId('provider-logo-codex')).toBeTruthy();
    });

    it('badges a failing big ring without blanking the satellite', () => {
      const offlinePrimary: PetOverlayViewModel = {
        ...model(satellite()),
        system: 'offline',
      };
      const { container } = render(PetOverlay, {
        props: { model: offlinePrimary },
      });

      expect(screen.getByRole('status').getAttribute('aria-label')).toBe(
        'Network offline',
      );
      expect(
        container.querySelectorAll('[data-testid="usage-ring"]'),
      ).toHaveLength(1);
      expect(
        screen.getByRole('img', {
          name: 'Codex usage: 5-hour 41%, Weekly 58%',
        }),
      ).toBeTruthy();
    });

    it('renders the Claude mark when Claude is the satellite', () => {
      render(PetOverlay, {
        props: {
          model: model(
            satellite({ provider: 'claude', providerName: 'Claude' }),
          ),
        },
      });

      // Claude appears twice: once in the satellite, once on the walker, which
      // always carries the primary mark.
      expect(screen.getAllByTestId('provider-logo-claude')).toHaveLength(2);
      expect(screen.queryByTestId('provider-logo-codex')).toBeNull();
    });
  });

  describe('orbiting primary mark', () => {
    const model = (
      overrides: Partial<PetOverlayViewModel> = {},
    ): PetOverlayViewModel => ({
      system: 'active',
      stale: false,
      session: { usedPercent: 74, severity: 'warn' },
      weekly: { usedPercent: 93, severity: 'critical' },
      animation,
      petName: 'Geometric pet',
      size: 160,
      satellite: null,
      orbit,
      ...overrides,
    });

    it('names the primary provider it is carrying', () => {
      render(PetOverlay, { props: { model: model() } });

      // The pet is cosmetic and the ring label is generic, so without this the
      // overlay never says which provider it is actually reporting.
      expect(
        screen.getByRole('img', { name: 'Primary provider: Claude' }),
      ).toBeTruthy();
    });

    it('walks in both ring modes, on a longer loop once there is a satellite', () => {
      const single = render(PetOverlay, { props: { model: model() } });
      const singlePath = screen.getByTestId('orbit-walker').style.offsetPath;

      expect(singlePath.startsWith('path(')).toBe(true);

      single.unmount();
      render(PetOverlay, {
        props: {
          model: model({
            satellite: {
              provider: 'codex',
              providerName: 'Codex',
              system: 'active',
              stale: false,
              session: { usedPercent: 41, severity: 'ok' },
              weekly: { usedPercent: 58, severity: 'ok' },
            },
          }),
        },
      });

      // The satellite adds its own arc to the outline, so the loop the walker
      // follows genuinely changes rather than staying the big ring alone.
      expect(screen.getByTestId('orbit-walker').style.offsetPath).not.toBe(
        singlePath,
      );
    });

    it('takes the direction the model chose', () => {
      const { unmount } = render(PetOverlay, { props: { model: model() } });
      expect(
        screen.getByTestId('orbit-walker').getAttribute('data-direction'),
      ).toBe('forward');
      expect(
        screen.getByTestId('orbit-walker').classList.contains('reverse'),
      ).toBe(false);

      unmount();
      render(PetOverlay, {
        props: {
          model: model({ orbit: { ...orbit, direction: 'reverse' } }),
        },
      });

      expect(
        screen.getByTestId('orbit-walker').classList.contains('reverse'),
      ).toBe(true);
    });
  });
});
