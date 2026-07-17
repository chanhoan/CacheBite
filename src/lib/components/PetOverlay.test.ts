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
        },
      },
    });

    expect(screen.getByLabelText('5-hour usage: 74%')).toBeTruthy();
    expect(screen.getByLabelText('Weekly usage: 93%')).toBeTruthy();
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
        },
      },
    });

    const unknown = screen.getByLabelText('5-hour usage: unknown');
    expect(unknown.getAttribute('data-severity')).toBe('unknown');
    expect(unknown.getAttribute('stroke-dasharray')).toBe('0 100');
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
        },
      },
    });

    expect(screen.getByRole('status').getAttribute('aria-label')).toBe(label);
    expect(container.querySelector('[data-testid="usage-ring"]')).toBeNull();
  });
});
