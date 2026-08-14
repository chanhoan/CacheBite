import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import ProviderColumn from './ProviderColumn.svelte';

import type { PanelProviderModel } from './panelModels';

const NOW = Date.parse('2026-07-16T12:02:00Z');

const model = (
  overrides: Partial<PanelProviderModel> = {},
): PanelProviderModel => ({
  provider: 'claude',
  system: 'active',
  stale: false,
  planType: 'pro',
  session: { usedPercent: 74, severity: 'warn', resetsAt: null },
  weekly: { usedPercent: 20, severity: 'ok', resetsAt: null },
  capturedAt: '2026-07-16T12:00:00Z',
  source: 'oauth_api',
  isCached: false,
  ...overrides,
});

describe('ProviderColumn', () => {
  afterEach(cleanup);

  it('names the primary column and keeps its star decorative', () => {
    const { container } = render(ProviderColumn, {
      props: { model: model(), isPrimary: true, nowMs: NOW },
    });

    expect(screen.getByLabelText('Claude usage (primary)')).toBeTruthy();
    expect(
      container.querySelector('.primary-star')?.getAttribute('aria-hidden'),
    ).toBe('true');
  });

  it('leaves a non-primary column unstarred', () => {
    const { container } = render(ProviderColumn, {
      props: {
        model: model({ provider: 'codex', source: 'cli_rpc' }),
        isPrimary: false,
        nowMs: NOW,
      },
    });

    expect(screen.getByLabelText('Codex usage')).toBeTruthy();
    expect(container.querySelector('.primary-star')).toBeNull();
  });

  it('shows the skeleton only while loading', () => {
    render(ProviderColumn, {
      props: {
        model: model({ system: 'loading' }),
        isPrimary: true,
        nowMs: NOW,
      },
    });

    expect(screen.getByTestId('usage-skeleton')).toBeTruthy();
    expect(screen.queryAllByTestId('usage-gauge')).toHaveLength(0);
  });

  // A failing provider keeps its column (it still counts as connected), so its
  // gauges must render — degraded to Unknown — rather than collapsing away.
  it('renders both gauges for a non-loading system state', () => {
    render(ProviderColumn, {
      props: {
        model: model({ system: 'offline' }),
        isPrimary: true,
        nowMs: NOW,
      },
    });

    expect(screen.queryByTestId('usage-skeleton')).toBeNull();
    expect(screen.getAllByTestId('usage-gauge')).toHaveLength(2);
  });

  it('omits source and cache details from stale freshness copy', () => {
    const { container } = render(ProviderColumn, {
      props: {
        model: model({ stale: true, isCached: true }),
        isPrimary: true,
        nowMs: NOW,
      },
    });

    const freshness = container.querySelector('.freshness');
    expect(freshness?.textContent?.replace(/\s+/g, ' ').trim()).toBe(
      '● Stale · captured 2 min ago',
    );
    expect(freshness?.textContent).not.toMatch(/oauth_api|cli_rpc|cached/);
  });

  it('carries its own provider name in the recovery guidance', () => {
    render(ProviderColumn, {
      props: {
        model: model({
          provider: 'codex',
          source: 'cli_rpc',
          system: 'auth_required',
        }),
        isPrimary: false,
        nowMs: NOW,
      },
    });

    expect(screen.getByRole('status').textContent).toBe(
      'Sign in to the Codex CLI: codex login',
    );
  });

  it('keeps the guidance live region empty while usage is displayable', () => {
    render(ProviderColumn, {
      props: { model: model(), isPrimary: true, nowMs: NOW },
    });

    expect(screen.getByRole('status').textContent).toBe('');
  });

  it('drops the plan chip when the provider reports no plan', () => {
    const { container } = render(ProviderColumn, {
      props: { model: model({ planType: null }), isPrimary: true, nowMs: NOW },
    });

    expect(container.querySelector('.plan-chip')).toBeNull();
  });
});
