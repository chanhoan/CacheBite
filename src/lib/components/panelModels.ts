import type { FailureClass, Provider } from '../contracts/domain';
import type { Severity, SystemState } from '../state/engine';

export interface PanelProviderModel {
  readonly provider: Provider;
  readonly system: SystemState;
  readonly stale: boolean;
  readonly planType: string | null;
  readonly session: {
    readonly usedPercent: number | null;
    readonly severity: Severity;
    readonly resetsAt: string | null;
  };
  readonly weekly: {
    readonly usedPercent: number | null;
    readonly severity: Severity;
    readonly resetsAt: string | null;
  };
  readonly capturedAt: string | null;
  readonly source: string;
  readonly isCached: boolean;
  /** Drives the `error` guidance copy; see `systemGuidance`. */
  readonly failureClass: FailureClass | null;
}

/**
 * Fixed column order. The primary is deliberately *not* pulled to the front:
 * pressing `Set … as primary` would swap the two columns, throwing the figures
 * the user was just reading across to the other side. The ★ already says which
 * one is primary.
 */
const PANEL_ORDER: readonly Provider[] = ['claude', 'codex'];

/**
 * Whether a provider counts as connected. Same line the engine draws for its
 * blocking statuses — `error`/`offline` are a connected provider's transient
 * failure, so they keep their column. Judging them disconnected would make the
 * panel flip between one and two columns every time the network hiccups.
 */
export const isProviderConnected = (system: SystemState): boolean =>
  system !== 'auth_required' && system !== 'unavailable';

/**
 * The providers the panel draws. Connected ones only — unless none of them is
 * connected, in which case both stay: hiding is a judgement that only holds
 * while the *other* provider is connected, and hiding everything would leave
 * both sign-in instructions with nowhere to appear.
 */
export function visiblePanelProviders(
  providers: Readonly<Record<Provider, PanelProviderModel>>,
): readonly Provider[] {
  const connected = PANEL_ORDER.filter((provider) =>
    isProviderConnected(providers[provider].system),
  );
  // A copy, not the constant itself: `readonly` is a compile-time claim only,
  // and handing callers the module's own array would let one stray mutation
  // corrupt the order for the rest of the session.
  return connected.length === 0 ? [...PANEL_ORDER] : connected;
}

/**
 * What `Set … as primary` aims at: the visible provider that is not already the
 * primary. `null` when the only visible column is the primary itself, which
 * leaves the control with nothing to do and so disabled.
 */
export function primaryCandidate(
  visible: readonly Provider[],
  primary: Provider,
): Provider | null {
  const [only, ...rest] = visible.filter((provider) => provider !== primary);
  return rest.length === 0 ? (only ?? null) : null;
}
