import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import UsagePanel from './UsagePanel.svelte';

import type { PanelProviderModel } from './panelModels';
import type { SystemState } from '../state/engine';

const NOW = Date.parse('2026-07-16T12:02:00Z');
const IDLE = { claude: false, codex: false } as const;

const provider = (
  system: SystemState,
  overrides: Partial<PanelProviderModel> = {},
): PanelProviderModel => ({
  provider: 'claude',
  system,
  stale: false,
  planType: 'pro',
  session: { usedPercent: 74, severity: 'warn', resetsAt: null },
  weekly: { usedPercent: 20, severity: 'ok', resetsAt: null },
  capturedAt: '2026-07-16T12:00:00Z',
  source: 'oauth_api',
  isCached: false,
  failureClass: null,
  ...overrides,
});

const bothProviders = (claude: SystemState, codex: SystemState = claude) => ({
  claude: provider(claude),
  codex: provider(codex, { provider: 'codex', source: 'cli_rpc' }),
});

const columns = (container: HTMLElement) => [
  ...container.querySelectorAll('[data-provider]'),
];

describe('UsagePanel', () => {
  afterEach(cleanup);

  it('renders one column per connected provider', () => {
    const { container } = render(UsagePanel, {
      props: {
        providers: bothProviders('active'),
        primary: 'claude',
        refreshing: IDLE,
        nowMs: NOW,
      },
    });

    expect(columns(container)).toHaveLength(2);
    expect(screen.getByLabelText('Claude usage (primary)')).toBeTruthy();
    expect(screen.getByLabelText('Codex usage')).toBeTruthy();
  });

  it('hides a provider whose CLI is not installed', () => {
    const { container } = render(UsagePanel, {
      props: {
        providers: bothProviders('active', 'unavailable'),
        primary: 'claude',
        refreshing: IDLE,
        nowMs: NOW,
      },
    });

    expect(columns(container)).toHaveLength(1);
    expect(columns(container)[0]?.getAttribute('data-provider')).toBe('claude');
    expect(screen.queryByLabelText('Codex usage')).toBeNull();
  });

  it('hides a provider that is not signed in', () => {
    const { container } = render(UsagePanel, {
      props: {
        providers: bothProviders('auth_required', 'active'),
        primary: 'claude',
        refreshing: IDLE,
        nowMs: NOW,
      },
    });

    expect(columns(container)).toHaveLength(1);
    expect(columns(container)[0]?.getAttribute('data-provider')).toBe('codex');
  });

  // Hiding only makes sense against a connected sibling. With neither connected
  // both stay, so the two sign-in instructions still have somewhere to appear.
  it('keeps both columns when neither provider is connected', () => {
    const { container } = render(UsagePanel, {
      props: {
        providers: bothProviders('auth_required', 'unavailable'),
        primary: 'claude',
        refreshing: IDLE,
        nowMs: NOW,
      },
    });

    expect(columns(container)).toHaveLength(2);
    expect(
      screen.getAllByRole('status').map((element) => element.textContent),
    ).toEqual([
      'Sign in to the Claude CLI: claude login',
      'The Codex CLI is not installed',
    ]);
  });

  it('keeps a column whose fetch failed', () => {
    const { container } = render(UsagePanel, {
      props: {
        providers: bothProviders('active', 'error'),
        primary: 'claude',
        refreshing: IDLE,
        nowMs: NOW,
      },
    });

    expect(columns(container)).toHaveLength(2);
  });

  it('shows the loading skeleton only for a loading provider', () => {
    render(UsagePanel, {
      props: {
        providers: bothProviders('loading', 'active'),
        primary: 'claude',
        refreshing: IDLE,
        nowMs: NOW,
      },
    });

    expect(screen.getAllByTestId('usage-skeleton')).toHaveLength(1);
  });

  it('replaces the tab strip with a visible panel heading', () => {
    render(UsagePanel, {
      props: {
        providers: bothProviders('active'),
        primary: 'claude',
        refreshing: IDLE,
        nowMs: NOW,
      },
    });

    expect(screen.queryAllByRole('tab')).toHaveLength(0);
    expect(screen.getByRole('heading', { name: 'Usage' })).toBeTruthy();
  });

  it('refreshes every visible provider from one control', async () => {
    const onRefresh = vi.fn();
    render(UsagePanel, {
      props: {
        providers: bothProviders('active'),
        primary: 'claude',
        refreshing: IDLE,
        nowMs: NOW,
        onRefresh,
      },
    });

    await fireEvent.click(screen.getByRole('button', { name: 'Refresh now' }));
    expect(onRefresh.mock.calls.map(([called]) => called)).toEqual([
      'claude',
      'codex',
    ]);
  });

  it('refreshes only the surviving provider when one is hidden', async () => {
    const onRefresh = vi.fn();
    render(UsagePanel, {
      props: {
        providers: bothProviders('active', 'unavailable'),
        primary: 'claude',
        refreshing: IDLE,
        nowMs: NOW,
        onRefresh,
      },
    });

    await fireEvent.click(screen.getByRole('button', { name: 'Refresh now' }));
    expect(onRefresh.mock.calls.map(([called]) => called)).toEqual(['claude']);
  });

  it('disables refresh while any visible provider is debounced', () => {
    render(UsagePanel, {
      props: {
        providers: bothProviders('active'),
        primary: 'claude',
        refreshing: { claude: false, codex: true },
        nowMs: NOW,
      },
    });

    expect(
      (
        screen.getByRole('button', {
          name: 'Refresh now',
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(true);
  });

  it('aims the primary control at the visible provider that is not primary', async () => {
    const onPrimary = vi.fn();
    render(UsagePanel, {
      props: {
        providers: bothProviders('active'),
        primary: 'claude',
        refreshing: IDLE,
        nowMs: NOW,
        onPrimary,
      },
    });

    await fireEvent.click(
      screen.getByRole('button', { name: 'Set Codex as primary' }),
    );
    expect(onPrimary).toHaveBeenCalledWith('codex');
  });

  it('disables the primary control when the only column is already primary', async () => {
    const onPrimary = vi.fn();
    render(UsagePanel, {
      props: {
        providers: bothProviders('active', 'unavailable'),
        primary: 'claude',
        refreshing: IDLE,
        nowMs: NOW,
        onPrimary,
      },
    });

    const button = screen.getByRole('button', {
      name: 'Set as primary',
    }) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
    await fireEvent.click(button);
    expect(onPrimary).not.toHaveBeenCalled();
  });

  // The primary is never auto-demoted, so it can point at a hidden provider.
  // The lone visible column is then exactly what the control should offer.
  it('offers the lone visible column when the primary is hidden', () => {
    render(UsagePanel, {
      props: {
        providers: bothProviders('active', 'unavailable'),
        primary: 'codex',
        refreshing: IDLE,
        nowMs: NOW,
      },
    });

    expect(
      (
        screen.getByRole('button', {
          name: 'Set Claude as primary',
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(false);
  });

  it('hides the panel through the close control and quits through the footer button', async () => {
    const onClose = vi.fn();
    const onQuit = vi.fn();
    render(UsagePanel, {
      props: {
        providers: bothProviders('active'),
        primary: 'claude',
        refreshing: IDLE,
        nowMs: NOW,
        onClose,
        onQuit,
      },
    });

    await fireEvent.click(
      screen.getByRole('button', { name: 'Close usage panel' }),
    );
    expect(onClose).toHaveBeenCalledTimes(1);
    expect(onQuit).not.toHaveBeenCalled();

    await fireEvent.click(screen.getByRole('button', { name: 'Quit' }));
    expect(onQuit).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('opens settings through its callback and exposes the panel close control', async () => {
    const onSettings = vi.fn();
    render(UsagePanel, {
      props: {
        providers: bothProviders('active'),
        primary: 'claude',
        refreshing: IDLE,
        nowMs: NOW,
        onSettings,
      },
    });

    expect(
      screen.getByRole('button', { name: 'Close usage panel' }),
    ).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
    expect(onSettings).toHaveBeenCalledTimes(1);
  });

  it('announces an available settings update with a decorative dot', () => {
    const { container } = render(UsagePanel, {
      props: {
        providers: bothProviders('active'),
        primary: 'claude',
        refreshing: IDLE,
        nowMs: NOW,
        updateAvailable: true,
      },
    });

    const dot = screen.getByTestId('settings-update-dot');
    expect(container.querySelector('.settings-label')?.contains(dot)).toBe(
      true,
    );
    expect(
      screen.getByRole('button', { name: 'Settings, update available' }),
    ).toBeTruthy();
  });

  it('keeps the default settings control queryable without an update dot', () => {
    render(UsagePanel, {
      props: {
        providers: bothProviders('active'),
        primary: 'claude',
        refreshing: IDLE,
        nowMs: NOW,
      },
    });

    expect(screen.queryByTestId('settings-update-dot')).toBeNull();
    expect(screen.getByRole('button', { name: 'Settings' })).toBeTruthy();
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
        primary: 'claude',
        refreshing: IDLE,
        nowMs: NOW,
      },
    });

    expect(screen.getAllByRole('status')[0]?.textContent).toBe(expected);
  });

  it('keeps the guidance live regions empty while usage is displayable', () => {
    render(UsagePanel, {
      props: {
        providers: bothProviders('active'),
        primary: 'claude',
        refreshing: IDLE,
        nowMs: NOW,
      },
    });

    expect(
      screen.getAllByRole('status').map((element) => element.textContent),
    ).toEqual(['', '']);
  });
});
