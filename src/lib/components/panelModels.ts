import type { Provider } from '../contracts/domain';

export interface PanelProviderModel {
  readonly provider: Provider;
  readonly system: string;
  readonly stale: boolean;
  readonly planType: string | null;
  readonly session: {
    readonly usedPercent: number | null;
    readonly severity: string;
    readonly resetsAt: string | null;
  };
  readonly weekly: {
    readonly usedPercent: number | null;
    readonly severity: string;
    readonly resetsAt: string | null;
  };
  readonly capturedAt: string | null;
  readonly source: string;
  readonly isCached: boolean;
}
