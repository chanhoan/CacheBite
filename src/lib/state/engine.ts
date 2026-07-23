import type {
  FailureClass,
  Provider,
  ProviderUiSnapshot,
  UnavailableReason,
} from '../contracts/domain';

export const FRESH_MAX_AGE_MS = 20 * 60_000;
// Must mirror the backend TTL (`refresh/actor.rs`). The backend is the single
// authority on expiry: it emits `snapshot_expired`, and the renderer never
// recomputes age against this constant. Two clocks judging the same deadline
// would oscillate under drift.
export const SNAPSHOT_TTL_MS = 30 * 60_000;

export type SystemState =
  | 'auth_required'
  | 'unavailable'
  | 'error'
  | 'offline'
  | 'loading'
  | 'active';
export type Severity = 'ok' | 'warn' | 'critical' | 'exhausted' | 'unknown';
export type WindowName = 'session' | 'weekly';

type StatusUpdate =
  | {
      readonly kind: 'fetch_failed';
      readonly failureClass: FailureClass;
      readonly revision: number;
    }
  | { readonly kind: 'credentials_missing'; readonly revision: number }
  | { readonly kind: 'cli_missing'; readonly revision: number }
  | {
      readonly kind: 'snapshot_expired';
      readonly revision: number;
      // The backend reports expiry and the last outcome independently, so an
      // expired snapshot can arrive alongside `not_signed_in`. Carry both or
      // the degradation loses the reason the fetch failed.
      readonly unavailableReason: UnavailableReason | null;
      readonly failureClass: FailureClass | null;
    };

export interface ProviderState {
  readonly provider: Provider;
  readonly revision: number;
  readonly snapshot: ProviderUiSnapshot | null;
  readonly status: Exclude<SystemState, 'active'>;
  readonly lastFailure: FailureClass | null;
  readonly resetWindows: ReadonlySet<WindowName>;
}

export interface PetUiState {
  readonly system: SystemState;
  readonly stale: boolean;
  readonly sessionSeverity: Severity;
  readonly weeklySeverity: Severity;
  readonly petMood: Exclude<Severity, 'unknown'>;
}

/**
 * A raise only ever lands above `ok`, but the comparison that produces one is
 * arithmetic on a rank table, which the type system cannot follow. The
 * predicate below carries that proof so no caller has to cast.
 */
export type RaisedSeverity = Exclude<Severity, 'ok' | 'unknown'>;

const isRaisedSeverity = (value: Severity): value is RaisedSeverity =>
  value === 'warn' || value === 'critical' || value === 'exhausted';

export type DomainEvent =
  | {
      readonly kind: 'severity_raised';
      readonly window: WindowName;
      readonly severity: RaisedSeverity;
    }
  | { readonly kind: 'window_reset'; readonly window: WindowName };

export function createProviderState(provider: Provider): ProviderState {
  return {
    provider,
    revision: 0,
    snapshot: null,
    status: 'loading',
    lastFailure: null,
    resetWindows: new Set(),
  };
}

export function severity(usedPercent: number | null): Severity {
  if (usedPercent === null) return 'unknown';
  if (!Number.isFinite(usedPercent)) return 'unknown';
  const percent = Math.min(100, Math.max(0, usedPercent));
  if (percent >= 100) return 'exhausted';
  if (percent >= 90) return 'critical';
  if (percent >= 70) return 'warn';
  return 'ok';
}

function windowSeverity(state: ProviderState, window: WindowName): Severity {
  if (state.resetWindows.has(window)) return 'unknown';
  return severity(state.snapshot?.[window]?.usedPercent ?? null);
}

const rank: Record<Severity, number> = {
  unknown: -1,
  ok: 0,
  warn: 1,
  critical: 2,
  exhausted: 3,
};

export function derivePetUiState(
  state: ProviderState,
  nowMs: number,
): PetUiState {
  const age =
    state.snapshot === null
      ? null
      : nowMs - Date.parse(state.snapshot.capturedAt);
  let system: SystemState = state.status;
  const isBlockingStatus =
    state.status === 'auth_required' || state.status === 'unavailable';
  if (state.snapshot !== null && !isBlockingStatus) system = 'active';
  const sessionSeverity =
    system === 'active' ? windowSeverity(state, 'session') : 'unknown';
  const weeklySeverity =
    system === 'active' ? windowSeverity(state, 'weekly') : 'unknown';
  const known = [sessionSeverity, weeklySeverity].filter(
    (value): value is Exclude<Severity, 'unknown'> => value !== 'unknown',
  );
  const petMood = known.reduce<Exclude<Severity, 'unknown'>>(
    (highest, value) => (rank[value] > rank[highest] ? value : highest),
    'ok',
  );
  return {
    system,
    stale:
      system === 'active' &&
      age !== null &&
      (!Number.isFinite(age) || age > FRESH_MAX_AGE_MS),
    sessionSeverity,
    weeklySeverity,
    petMood,
  };
}

export function applyProviderUpdate(
  state: ProviderState,
  update: ProviderUiSnapshot | StatusUpdate,
  nowMs: number,
): { state: ProviderState; accepted: boolean; events: readonly DomainEvent[] } {
  if (update.revision <= state.revision)
    return { state, accepted: false, events: [] };
  if (!('kind' in update)) {
    const previous = derivePetUiState(state, nowMs);
    const nextState: ProviderState = {
      ...state,
      revision: update.revision,
      snapshot: update,
      status: 'loading',
      lastFailure: update.failureClass,
      resetWindows: new Set(),
    };
    const next = derivePetUiState(nextState, nowMs);
    const events = (['session', 'weekly'] as const).flatMap(
      (window): DomainEvent[] => {
        const before =
          window === 'session'
            ? previous.sessionSeverity
            : previous.weeklySeverity;
        const after =
          window === 'session' ? next.sessionSeverity : next.weeklySeverity;
        if (
          rank[after] > rank[before] &&
          before !== 'unknown' &&
          isRaisedSeverity(after)
        )
          return [{ kind: 'severity_raised', window, severity: after }];
        if (rank[after] < rank[before] && after !== 'unknown')
          return [{ kind: 'window_reset', window }];
        return [];
      },
    );
    return { state: nextState, accepted: true, events };
  }
  if (update.kind === 'snapshot_expired') {
    // Contract §2.2, `active` row / SNAPSHOT_EXPIRED column: discard the
    // snapshot and degrade by the reason the backend last recorded. On boot a
    // hydrated state arrives expired before the renderer has accumulated
    // anything, so the backend-reported reason must outrank `state`; only when
    // it is absent does a previously observed blocking status apply.
    const lastFailure = update.failureClass ?? state.lastFailure;
    const degraded =
      update.unavailableReason === 'not_signed_in' ||
      state.status === 'auth_required'
        ? 'auth_required'
        : update.unavailableReason === 'not_installed' ||
            state.status === 'unavailable'
          ? 'unavailable'
          : lastFailure === 'network'
            ? 'offline'
            : 'error';
    return {
      state: {
        ...state,
        revision: update.revision,
        snapshot: null,
        status: degraded,
        lastFailure,
        resetWindows: new Set(),
      },
      accepted: true,
      events: [],
    };
  }
  const status =
    update.kind === 'credentials_missing'
      ? 'auth_required'
      : update.kind === 'cli_missing'
        ? 'unavailable'
        : state.status === 'auth_required' ||
            state.status === 'unavailable' ||
            state.snapshot !== null
          ? state.status
          : update.failureClass === 'network'
            ? 'offline'
            : 'error';
  const lastFailure =
    update.kind === 'fetch_failed' ? update.failureClass : state.lastFailure;
  return {
    state: { ...state, revision: update.revision, status, lastFailure },
    accepted: true,
    events: [],
  };
}
