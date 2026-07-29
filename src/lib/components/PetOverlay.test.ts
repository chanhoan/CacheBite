import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import PetOverlay from './PetOverlay.svelte';

const animation = {
  type: 'frames' as const,
  sources: ['/fixtures/idle-01.svg', '/fixtures/idle-02.svg'],
  frameDurationMs: 120,
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
      name: 'Move pet; double-click or press Enter for usage',
    });
    expect(surface.getAttribute('data-testid')).toBe('overlay-pointer-surface');
    expect(surface.style.clipPath).toBe('circle(50% at 50% 50%)');
    expect(screen.queryByRole('status')).toBeNull();
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
        },
      },
    });

    expect(screen.getByRole('status').getAttribute('aria-label')).toBe(label);
    expect(screen.getByRole('status').querySelector('svg')).toBeTruthy();
    expect(container.querySelector('[data-testid="usage-ring"]')).toBeNull();
  });
});
