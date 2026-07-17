import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import UsageGauge from './UsageGauge.svelte';

describe('UsageGauge', () => {
  afterEach(cleanup);

  it('exposes clamped progress and severity on the fill', () => {
    const { container } = render(UsageGauge, {
      props: {
        label: '5-hour',
        window: { usedPercent: 150, severity: 'exhausted', resetsAt: null },
      },
    });

    expect(screen.getByRole('progressbar').getAttribute('aria-valuenow')).toBe(
      '100',
    );
    expect(container.querySelector('[data-severity="exhausted"]')).toBeTruthy();
    expect(screen.getByText('100%')).toBeTruthy();
  });

  it('renders reset context and dims only a stale bar', () => {
    const { container } = render(UsageGauge, {
      props: {
        label: 'Weekly',
        window: {
          usedPercent: 68,
          severity: 'warn',
          resetsAt: '2026-07-20T09:00:00Z',
        },
        stale: true,
      },
    });

    expect(screen.getByText(/resets 2026-07-20T09:00:00Z/)).toBeTruthy();
    expect(container.querySelector('.gauge-track')?.classList).toContain(
      'stale',
    );
  });
});
