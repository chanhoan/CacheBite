import type { Provider } from '../contracts/domain';
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
}
