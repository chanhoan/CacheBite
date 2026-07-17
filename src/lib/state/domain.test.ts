import { get } from 'svelte/store';
import { describe, expect, it } from 'vitest';

import {
  applyProviderUpdate,
  createProviderState,
  derivePetUiState,
  tickResetTimers,
} from './engine';
import type { ProviderUiSnapshot } from '../contracts/domain';
import { fromProviderUiSnapshotWire } from '../api/providerSnapshot';
import { createProvidersStore } from '../stores/providers';

const NOW = Date.parse('2026-07-16T12:00:00Z');

function snapshot(
  overrides: Partial<ProviderUiSnapshot> = {},
): ProviderUiSnapshot {
  return {
    provider: 'claude',
    planType: 'pro',
    session: { usedPercent: 69, windowMinutes: 300, resetsAt: null },
    weekly: { usedPercent: 90, windowMinutes: 10080, resetsAt: null },
    capturedAt: '2026-07-16T11:40:00Z',
    source: 'oauth_api',
    isCached: false,
    revision: 1,
    failureClass: null,
    unavailableReason: null,
    ...overrides,
  };
}

describe('domain presentation rules', () => {
  it('maps the snake_case IPC DTO into the renderer model', () => {
    expect(
      fromProviderUiSnapshotWire({
        provider: 'codex',
        plan_type: null,
        session: { used_percent: 70, window_minutes: 300, resets_at: null },
        weekly: null,
        captured_at: '2026-07-16T12:00:00Z',
        source: 'cli_rpc',
        is_cached: true,
        revision: 4,
        failure_class: null,
        unavailable_reason: null,
      }),
    ).toMatchObject({
      provider: 'codex',
      planType: null,
      capturedAt: '2026-07-16T12:00:00Z',
      isCached: true,
      session: { usedPercent: 70 },
    });
  });
  it.each([
    [69, 'ok'],
    [70, 'warn'],
    [89, 'warn'],
    [90, 'critical'],
    [99, 'critical'],
    [100, 'exhausted'],
    [101, 'exhausted'],
  ] as const)('maps %s percent to %s', (usedPercent, expected) => {
    const state = createProviderState('claude');
    const updated = applyProviderUpdate(
      state,
      snapshot({
        session: { usedPercent, windowMinutes: 300, resetsAt: null },
      }),
      NOW,
    );
    expect(derivePetUiState(updated.state, NOW).sessionSeverity).toBe(expected);
  });

  it.each([
    ['2026-07-16T11:40:00Z', false, 'active'],
    ['2026-07-16T11:39:59.999Z', true, 'active'],
    ['2026-07-16T11:30:00Z', true, 'active'],
    ['2026-07-16T11:29:59.999Z', false, 'error'],
  ] as const)('derives freshness at %s', (capturedAt, stale, system) => {
    const updated = applyProviderUpdate(
      createProviderState('claude'),
      snapshot({ capturedAt }),
      NOW,
    );
    expect(derivePetUiState(updated.state, NOW)).toMatchObject({
      stale,
      system,
    });
  });

  it('keeps an active snapshot on fetch failure and expires using the failure class', () => {
    const initial = applyProviderUpdate(
      createProviderState('claude'),
      snapshot(),
      NOW,
    ).state;
    const failed = applyProviderUpdate(
      initial,
      { kind: 'fetch_failed', failureClass: 'network', revision: 2 },
      NOW,
    ).state;
    expect(derivePetUiState(failed, NOW).system).toBe('active');
    expect(derivePetUiState(failed, NOW + 30 * 60_000 + 1).system).toBe(
      'offline',
    );
  });

  it.each([
    [{ kind: 'credentials_missing', revision: 2 } as const, 'auth_required'],
    [{ kind: 'cli_missing', revision: 2 } as const, 'unavailable'],
  ])(
    'gives blocking outcomes precedence over a cached snapshot',
    (outcome, expected) => {
      const initial = applyProviderUpdate(
        createProviderState('claude'),
        snapshot(),
        NOW,
      ).state;
      const updated = applyProviderUpdate(initial, outcome, NOW).state;
      expect(derivePetUiState(updated, NOW).system).toBe(expected);
    },
  );

  it.each([
    ['loading', 'network', 'offline'],
    ['loading', 'parse', 'error'],
    ['offline', 'provider', 'error'],
    ['error', 'network', 'offline'],
    ['auth_required', 'network', 'auth_required'],
    ['auth_required', 'internal', 'auth_required'],
    ['unavailable', 'network', 'unavailable'],
    ['unavailable', 'parse', 'unavailable'],
  ] as const)(
    'transitions %s on %s failure to %s',
    (status, failureClass, expected) => {
      const initial = { ...createProviderState('claude'), status };
      const updated = applyProviderUpdate(
        initial,
        { kind: 'fetch_failed', failureClass, revision: 1 },
        NOW,
      );
      expect(derivePetUiState(updated.state, NOW).system).toBe(expected);
    },
  );

  it.each([Number.NaN, Number.POSITIVE_INFINITY, Number.NEGATIVE_INFINITY])(
    'treats invalid numeric value %s as unknown',
    (usedPercent) => {
      const updated = applyProviderUpdate(
        createProviderState('claude'),
        snapshot({
          session: { usedPercent, windowMinutes: 300, resetsAt: null },
        }),
        NOW,
      );
      expect(derivePetUiState(updated.state, NOW).sessionSeverity).toBe(
        'unknown',
      );
    },
  );

  it('uses an internal error when a snapshot expires without a recorded failure', () => {
    const state = applyProviderUpdate(
      createProviderState('claude'),
      snapshot(),
      NOW,
    ).state;
    expect(derivePetUiState(state, NOW + 30 * 60_000 + 1)).toMatchObject({
      system: 'error',
    });
  });

  it('discards updates whose revision is not newer', () => {
    const revisionSeven = applyProviderUpdate(
      createProviderState('claude'),
      snapshot({ revision: 7 }),
      NOW,
    ).state;
    const result = applyProviderUpdate(
      revisionSeven,
      { kind: 'credentials_missing', revision: 6 },
      NOW,
    );
    expect(result.accepted).toBe(false);
    expect(result.state).toBe(revisionSeven);
  });

  it('emits one optimistic reset and leaves the window unknown until refresh', () => {
    const due = snapshot({
      session: {
        usedPercent: 99,
        windowMinutes: 300,
        resetsAt: '2026-07-16T12:00:00Z',
      },
    });
    const initial = applyProviderUpdate(
      createProviderState('claude'),
      due,
      NOW - 1,
    ).state;
    const first = tickResetTimers(initial, NOW);
    const second = tickResetTimers(first.state, NOW + 1);
    expect(first.events).toEqual([{ kind: 'window_reset', window: 'session' }]);
    expect(derivePetUiState(first.state, NOW).sessionSeverity).toBe('unknown');
    expect(second.events).toEqual([]);
  });

  it('marks retained provider usage unknown when the backend reports reset pending', () => {
    const store = createProvidersStore();
    store.apply(snapshot({ revision: 4 }), NOW);

    expect(store.markResetPending('claude', 5)).toEqual([
      { kind: 'window_reset', window: 'session', provider: 'claude' },
      { kind: 'window_reset', window: 'weekly', provider: 'claude' },
    ]);

    const state = get(store).claude;
    expect(state.revision).toBe(5);
    expect(derivePetUiState(state, NOW)).toMatchObject({
      sessionSeverity: 'unknown',
      weeklySeverity: 'unknown',
    });
  });

  it('ignores reset-pending revisions older than retained provider state', () => {
    const store = createProvidersStore();
    store.apply(snapshot({ revision: 4 }), NOW);
    const retained = get(store).claude;

    expect(store.markResetPending('claude', 3)).toEqual([]);
    expect(get(store).claude).toBe(retained);
  });

  it('deduplicates an equal-revision reset-pending transition', () => {
    const store = createProvidersStore();
    store.apply(snapshot({ revision: 4 }), NOW);

    expect(store.markResetPending('claude', 4)).toHaveLength(2);
    expect(store.markResetPending('claude', 4)).toEqual([]);
  });

  it('derives a newly selected provider without changing or refetching either provider', () => {
    const claude = applyProviderUpdate(
      createProviderState('claude'),
      snapshot(),
      NOW,
    ).state;
    const codex = applyProviderUpdate(
      createProviderState('codex'),
      snapshot({
        provider: 'codex',
        source: 'cli_rpc',
        session: null,
        weekly: null,
      }),
      NOW,
    ).state;
    expect(derivePetUiState(claude, NOW).petMood).toBe('critical');
    expect(derivePetUiState(codex, NOW).petMood).toBe('ok');
    expect(codex.revision).toBe(1);
  });
});
