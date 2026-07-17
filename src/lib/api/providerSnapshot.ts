import type {
  FailureClass,
  Provider,
  ProviderUiSnapshot,
  Source,
  UnavailableReason,
  UsageWindow,
} from '../contracts/domain';

export interface UsageWindowWire {
  readonly used_percent: number;
  readonly window_minutes: number;
  readonly resets_at: string | null;
}

/** Credential-free IPC shape. Wire names deliberately mirror Rust serde fields. */
export interface ProviderUiSnapshotWire {
  readonly provider: Provider;
  readonly plan_type: string | null;
  readonly session: UsageWindowWire | null;
  readonly weekly: UsageWindowWire | null;
  readonly captured_at: string;
  readonly source: Source;
  readonly is_cached: boolean;
  readonly revision: number;
  readonly failure_class: FailureClass | null;
  readonly unavailable_reason: UnavailableReason | null;
}

export function fromProviderUiSnapshotWire(
  wire: ProviderUiSnapshotWire,
): ProviderUiSnapshot {
  const mapWindow = (window: UsageWindowWire | null): UsageWindow | null =>
    window === null
      ? null
      : {
          usedPercent: window.used_percent,
          windowMinutes: window.window_minutes,
          resetsAt: window.resets_at,
        };
  return {
    provider: wire.provider,
    planType: wire.plan_type,
    session: mapWindow(wire.session),
    weekly: mapWindow(wire.weekly),
    capturedAt: wire.captured_at,
    source: wire.source,
    isCached: wire.is_cached,
    revision: wire.revision,
    failureClass: wire.failure_class,
    unavailableReason: wire.unavailable_reason,
  };
}
