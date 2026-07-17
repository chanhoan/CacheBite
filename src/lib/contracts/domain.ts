export type Provider = 'claude' | 'codex';
export type FailureClass = 'network' | 'provider' | 'parse' | 'internal';
export type Source = 'oauth_api' | 'cli_rpc';
export type UnavailableReason = 'not_installed' | 'not_signed_in';

export interface UsageWindow {
  readonly usedPercent: number;
  readonly windowMinutes: number;
  readonly resetsAt: string | null;
}

export interface ProviderUiSnapshot {
  readonly provider: Provider;
  readonly planType: string | null;
  readonly session: UsageWindow | null;
  readonly weekly: UsageWindow | null;
  readonly capturedAt: string;
  readonly source: Source;
  readonly isCached: boolean;
  readonly revision: number;
  readonly failureClass: FailureClass | null;
  readonly unavailableReason: UnavailableReason | null;
}
