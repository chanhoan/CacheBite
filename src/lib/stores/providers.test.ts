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
});
