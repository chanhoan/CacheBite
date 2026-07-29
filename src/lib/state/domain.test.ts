import { get } from 'svelte/store';
import { describe, expect, it } from 'vitest';

import {
  applyProviderUpdate,
  createProviderState,
  derivePetUiState,
  FRESH_MAX_AGE_MS,
  SNAPSHOT_TTL_MS,
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
    ['2026-07-16T11:29:59.999Z', true, 'active'],
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

  it('keeps cached usage visible and stale after a fetch failure', () => {
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
    expect(derivePetUiState(failed, NOW + 30 * 60_000 + 1)).toMatchObject({
      system: 'active',
      stale: true,
      sessionSeverity: 'ok',
    });
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

  it('keeps a snapshot visible as stale until the backend declares it expired', () => {
    const state = applyProviderUpdate(
      createProviderState('claude'),
      snapshot(),
      NOW,
    ).state;
    expect(derivePetUiState(state, NOW + FRESH_MAX_AGE_MS + 1)).toMatchObject({
      system: 'active',
      stale: true,
    });
  });

  it('drops an expired snapshot and degrades to error when no network failure was recorded', () => {
    const EXPIRED_AT = NOW + SNAPSHOT_TTL_MS + 1;
    const state = applyProviderUpdate(
      createProviderState('claude'),
      snapshot(),
      NOW,
    ).state;
    const result = applyProviderUpdate(
      state,
      {
        kind: 'snapshot_expired',
        revision: 2,
        unavailableReason: null,
        failureClass: null,
      },
      EXPIRED_AT,
    );
    expect(result.accepted).toBe(true);
    expect(result.state.snapshot).toBeNull();
    expect(derivePetUiState(result.state, EXPIRED_AT)).toMatchObject({
      system: 'error',
      stale: false,
    });
  });

  it('degrades an expired snapshot to offline when the last failure was network', () => {
    const EXPIRED_AT = NOW + SNAPSHOT_TTL_MS + 1;
    const state = applyProviderUpdate(
      createProviderState('claude'),
      snapshot({ failureClass: 'network' }),
      NOW,
    ).state;
    const result = applyProviderUpdate(
      state,
      {
        kind: 'snapshot_expired',
        revision: 2,
        unavailableReason: null,
        failureClass: null,
      },
      EXPIRED_AT,
    );
    expect(derivePetUiState(result.state, EXPIRED_AT).system).toBe('offline');
  });

  it('keeps auth_required when a snapshot expires', () => {
    const EXPIRED_AT = NOW + SNAPSHOT_TTL_MS + 1;
    const withSnapshot = applyProviderUpdate(
      createProviderState('claude'),
      snapshot(),
      NOW,
    ).state;
    const authRequired = applyProviderUpdate(
      withSnapshot,
      { kind: 'credentials_missing', revision: 2 },
      NOW,
    ).state;
    const result = applyProviderUpdate(
      authRequired,
      {
        kind: 'snapshot_expired',
        revision: 3,
        unavailableReason: null,
        failureClass: null,
      },
      EXPIRED_AT,
    );
    expect(result.state.snapshot).toBeNull();
    expect(derivePetUiState(result.state, EXPIRED_AT).system).toBe(
      'auth_required',
    );
  });

  it('degrades a hydrated expiry by the backend-reported reason, not the empty renderer state', () => {
    // On boot `provider_state_from_record` fills `expired` from snapshot age and
    // `unavailable_reason` from the last outcome independently, so both can
    // arrive in the first event the renderer ever sees. Falling back to
    // `state.lastFailure` there would show retry guidance to a logged-out user.
    const result = applyProviderUpdate(
      createProviderState('claude'),
      {
        kind: 'snapshot_expired',
        revision: 4,
        unavailableReason: 'not_signed_in',
        failureClass: null,
      },
      NOW,
    );
    expect(result.accepted).toBe(true);
    expect(derivePetUiState(result.state, NOW).system).toBe('auth_required');
  });

  it('degrades a hydrated expiry to unavailable when the CLI is missing', () => {
    const result = applyProviderUpdate(
      createProviderState('codex'),
      {
        kind: 'snapshot_expired',
        revision: 4,
        unavailableReason: 'not_installed',
        failureClass: null,
      },
      NOW,
    );
    expect(derivePetUiState(result.state, NOW).system).toBe('unavailable');
  });

  it('degrades a hydrated expiry to offline from the reported failure class', () => {
    const result = applyProviderUpdate(
      createProviderState('claude'),
      {
        kind: 'snapshot_expired',
        revision: 4,
        unavailableReason: null,
        failureClass: 'network',
      },
      NOW,
    );
    expect(derivePetUiState(result.state, NOW).system).toBe('offline');
    expect(result.state.lastFailure).toBe('network');
  });

  it('ignores an expiry whose revision is not newer', () => {
    const state = applyProviderUpdate(
      createProviderState('claude'),
      snapshot({ revision: 7 }),
      NOW,
    ).state;
    const result = applyProviderUpdate(
      state,
      {
        kind: 'snapshot_expired',
        revision: 6,
        unavailableReason: null,
        failureClass: null,
      },
      NOW + SNAPSHOT_TTL_MS + 1,
    );
    expect(result.accepted).toBe(false);
    expect(result.state).toBe(state);
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
