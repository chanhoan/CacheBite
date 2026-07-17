import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import UsagePanel from './UsagePanel.svelte';

const provider = (system: string, stale = false) => ({
  provider: 'claude' as const,
  system,
  stale,
  planType: 'pro',
  session: { usedPercent: 74, severity: 'warn' as const, resetsAt: null },
  weekly: { usedPercent: 20, severity: 'ok' as const, resetsAt: null },
  capturedAt: '2026-07-16T12:00:00Z',
  source: 'oauth_api' as const,
  isCached: false,
});

describe('UsagePanel', () => {
  afterEach(cleanup);
  it('always shows both provider tabs and loading skeleton only for loading', async () => {
    const { rerender } = render(UsagePanel, {
      props: {
        providers: {
          claude: provider('loading'),
          codex: {
            ...provider('active'),
            provider: 'codex',
            source: 'cli_rpc',
          },
        },
        selected: 'claude',
        refreshing: false,
      },
    });
    expect(screen.getByRole('tab', { name: 'Claude' })).toBeTruthy();
    expect(screen.getByRole('tab', { name: 'Codex' })).toBeTruthy();
    expect(screen.getByTestId('usage-skeleton')).toBeTruthy();
    await rerender({
      providers: {
        claude: provider('offline'),
        codex: { ...provider('active'), provider: 'codex', source: 'cli_rpc' },
      },
      selected: 'claude',
      refreshing: false,
    });
    expect(screen.queryByTestId('usage-skeleton')).toBeNull();
  });

  it('disables refresh only while debounced and selects primary without fetching', async () => {
    const onRefresh = vi.fn();
    const onSelect = vi.fn();
    const onPrimary = vi.fn();
    render(UsagePanel, {
      props: {
        providers: {
          claude: provider('active'),
          codex: {
            ...provider('active'),
            provider: 'codex',
            source: 'cli_rpc',
          },
        },
        selected: 'claude',
        refreshing: true,
        onRefresh,
        onSelect,
        onPrimary,
      },
    });
    expect(
      (screen.getByRole('button', { name: 'Refresh now' }) as HTMLButtonElement)
        .disabled,
    ).toBe(true);
    await fireEvent.click(
      screen.getByRole('button', { name: 'Make Codex primary' }),
    );
    expect(onPrimary).toHaveBeenCalledWith('codex');
    expect(onRefresh).not.toHaveBeenCalled();
  });
});
