import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import UsagePanel from './UsagePanel.svelte';

import type { SystemState } from '../state/engine';

const NOW = Date.parse('2026-07-16T12:02:00Z');

const provider = (system: SystemState, stale = false) => ({
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

const bothProviders = (system: SystemState) => ({
  claude: provider(system),
  codex: { ...provider(system), provider: 'codex' as const, source: 'cli_rpc' },
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
        primary: 'claude',
        refreshing: false,
      },
    });
    expect(screen.getByRole('tab', { name: 'Claude (primary)' })).toBeTruthy();
    expect(screen.getByRole('tab', { name: 'Codex' })).toBeTruthy();
    expect(screen.getByTestId('usage-skeleton')).toBeTruthy();
    await rerender({
      providers: {
        claude: provider('offline'),
        codex: { ...provider('active'), provider: 'codex', source: 'cli_rpc' },
      },
      selected: 'claude',
      primary: 'claude',
      refreshing: false,
    });
    expect(screen.queryByTestId('usage-skeleton')).toBeNull();
  });

  it('disables refresh only while debounced and sets the selected tab primary without fetching', async () => {
    const onRefresh = vi.fn();
    const onSelect = vi.fn();
    const onPrimary = vi.fn();
    const { container } = render(UsagePanel, {
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
        primary: 'codex',
        refreshing: true,
        nowMs: NOW,
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
      screen.getByRole('button', { name: 'Set as primary' }),
    );
    expect(onPrimary).toHaveBeenCalledWith('claude');
    expect(onRefresh).not.toHaveBeenCalled();
    const freshness = container.querySelector<HTMLElement>('.freshness');
    expect(freshness?.textContent?.replace(/\s+/g, ' ').trim()).toBe(
      '● Fresh · captured 2 min ago',
    );
    expect(freshness?.textContent).not.toMatch(/oauth_api|cli_rpc|cached/);
    expect(
      container.querySelector('.freshness time')?.getAttribute('datetime'),
    ).toBe('2026-07-16T12:00:00Z');
  });

  it('omits source and cache details from stale freshness copy', () => {
    const staleCached = {
      ...provider('active', true),
      isCached: true,
    };
    const { container } = render(UsagePanel, {
      props: {
        providers: {
          claude: staleCached,
          codex: {
            ...staleCached,
            provider: 'codex',
            source: 'cli_rpc',
          },
        },
        selected: 'claude',
        primary: 'claude',
        refreshing: false,
        nowMs: NOW,
      },
    });

    const freshness = container.querySelector('.freshness');
    expect(freshness?.textContent?.replace(/\s+/g, ' ').trim()).toBe(
      '● Stale · captured 2 min ago',
    );
    expect(freshness?.textContent).not.toMatch(/oauth_api|cli_rpc|cached/);
  });

  it('closes the panel when Quit is pressed rather than acting on the window itself', async () => {
    const onClose = vi.fn();
    render(UsagePanel, {
      props: {
        providers: bothProviders('active'),
        selected: 'claude',
        primary: 'claude',
        refreshing: false,
        nowMs: NOW,
        onClose,
      },
    });

    await fireEvent.click(screen.getByRole('button', { name: 'Quit' }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('opens settings through its callback and exposes no window-close control', async () => {
    const onSettings = vi.fn();
    render(UsagePanel, {
      props: {
        providers: bothProviders('active'),
        selected: 'claude',
        primary: 'claude',
        refreshing: false,
        nowMs: NOW,
        onSettings,
      },
    });

    expect(
      screen.queryByRole('button', { name: 'Close usage panel' }),
    ).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
    expect(onSettings).toHaveBeenCalledTimes(1);
  });

  it.each([
    ['auth_required' as const, 'Sign in to the Claude CLI: claude login'],
    ['unavailable' as const, 'The Claude CLI is not installed'],
    ['error' as const, 'Could not fetch usage. Retrying shortly.'],
    ['offline' as const, 'Cannot reach the network'],
  ])('shows recovery guidance for %s', (system, expected) => {
    render(UsagePanel, {
      props: {
        providers: bothProviders(system),
        selected: 'claude',
        primary: 'claude',
        refreshing: false,
        nowMs: NOW,
      },
    });

    expect(screen.getByRole('status').textContent).toBe(expected);
  });

  it('keeps the guidance live region empty while usage is displayable', () => {
    render(UsagePanel, {
      props: {
        providers: bothProviders('active'),
        selected: 'claude',
        primary: 'claude',
        refreshing: false,
        nowMs: NOW,
      },
    });

    expect(screen.getByRole('status').textContent).toBe('');
  });
});
