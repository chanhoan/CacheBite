import { get } from 'svelte/store';
import { describe, expect, it, vi } from 'vitest';
import { createProvidersStore } from './providers';
import type { ProviderUiSnapshot } from '../contracts/domain';

const snapshot = (
  provider: 'claude' | 'codex',
  revision: number,
): ProviderUiSnapshot => ({
  provider,
  revision,
  planType: 'pro',
  session: null,
  weekly: null,
  capturedAt: '2026-07-16T12:00:00Z',
  source: provider === 'claude' ? 'oauth_api' : 'cli_rpc',
  isCached: false,
  failureClass: null,
  unavailableReason: null,
});

describe('providers store', () => {
  it('keeps providers independent and rejects older revisions', () => {
    const store = createProvidersStore();
    store.apply(snapshot('claude', 4), 0);
    store.apply(snapshot('codex', 2), 0);
    store.apply(snapshot('claude', 3), 0);
    expect(get(store).claude.revision).toBe(4);
    expect(get(store).codex.revision).toBe(2);
  });

  it('switches the viewed tab locally without refreshing', () => {
    const refresh = vi.fn();
    const store = createProvidersStore(refresh);
    store.selectTab('codex');
    expect(get(store).selected).toBe('codex');
    expect(refresh).not.toHaveBeenCalled();
  });

  it('marks a provider refreshing until its next state event arrives', () => {
    const store = createProvidersStore(vi.fn());
    store.requestRefresh('claude');
    expect(get(store).refreshing).toMatchObject({
      claude: true,
      codex: false,
    });

    store.apply(snapshot('claude', 1), 0);
    expect(get(store).refreshing.claude).toBe(false);
  });

  it('does not let one provider clear the other provider flag', () => {
    const store = createProvidersStore(vi.fn());
    store.requestRefresh('claude');
    store.requestRefresh('codex');
    store.applyStatus('codex', { kind: 'cli_missing', revision: 1 });
    expect(get(store).refreshing).toMatchObject({
      claude: true,
      codex: false,
    });
  });

  it.each([
    [
      'an expiry',
      (store: ReturnType<typeof createProvidersStore>) =>
        store.applyExpiry('claude', 5),
    ],
    [
      'a pending reset',
      (store: ReturnType<typeof createProvidersStore>) =>
        store.markResetPending('claude', 5),
    ],
  ])('clears the refreshing flag on %s', (_label, settle) => {
    const store = createProvidersStore(vi.fn());
    store.requestRefresh('claude');
    settle(store);
    expect(get(store).refreshing.claude).toBe(false);
  });

  it('releases the flag after the timeout when collection never reports', () => {
    vi.useFakeTimers();
    try {
      const store = createProvidersStore(vi.fn());
      store.requestRefresh('claude');
      vi.advanceTimersByTime(29_999);
      expect(get(store).refreshing.claude).toBe(true);
      vi.advanceTimersByTime(1);
      expect(get(store).refreshing.claude).toBe(false);
    } finally {
      vi.useRealTimers();
    }
  });
});
