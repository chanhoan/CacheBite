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

  it('renders a day-aware weekly countdown and dims only a stale bar', () => {
    const { container } = render(UsageGauge, {
      props: {
        label: 'Weekly',
        window: {
          usedPercent: 68,
          severity: 'warn',
          resetsAt: '2026-07-20T09:00:00Z',
        },
        stale: true,
        nowMs: Date.parse('2026-07-16T12:00:00Z'),
      },
    });

    const time = container.querySelector('time');
    expect(time?.getAttribute('datetime')).toBe('2026-07-20T09:00:00Z');
    expect(time?.textContent).toBe('resets in 3d 21h 0m');
    expect(container.querySelector('.gauge-track')?.classList).toContain(
      'stale',
    );
  });

  it('counts down the five-hour window instead of printing an ISO string', () => {
    const { container } = render(UsageGauge, {
      props: {
        label: '5-hour',
        window: {
          usedPercent: 42,
          severity: 'ok',
          resetsAt: '2026-07-16T13:12:00Z',
        },
        nowMs: Date.parse('2026-07-16T12:00:00Z'),
      },
    });

    expect(screen.getByText('resets in 1h 12m')).toBeTruthy();
    expect(container.querySelector('time')?.getAttribute('datetime')).toBe(
      '2026-07-16T13:12:00Z',
    );
  });

  it('drops the reset element when the timestamp cannot be parsed', () => {
    const { container } = render(UsageGauge, {
      props: {
        label: '5-hour',
        window: { usedPercent: 10, severity: 'ok', resetsAt: 'not-a-date' },
      },
    });

    expect(container.querySelector('time')).toBeNull();
  });
});
