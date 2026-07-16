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

const loading = (provider: Provider): PanelProviderModel => ({
  provider,
  system: 'loading',
  stale: false,
  planType: null,
  session: { usedPercent: null, severity: 'unknown', resetsAt: null },
  weekly: { usedPercent: null, severity: 'unknown', resetsAt: null },
  capturedAt: null,
  source: provider === 'claude' ? 'oauth_api' : 'cli_rpc',
  isCached: false,
});
export const initialPanelProviders = Object.freeze({
  claude: loading('claude'),
  codex: loading('codex'),
});
