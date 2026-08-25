export type Provider = 'claude' | 'codex';

/**
 * The provider bound to the overlay's small ring. The user never assigns it by
 * hand: the big ring is always the primary, so the satellite is whatever is
 * left over.
 */
export const secondaryProvider = (primary: Provider): Provider =>
  primary === 'claude' ? 'codex' : 'claude';

/**
 * Human-readable provider names, for accessible labels and panel copy.
 *
 * A map rather than the `p === 'claude' ? 'Claude' : 'Codex'` ternary this
 * replaced: the ternary compiles fine when a third provider is added and simply
 * starts labelling it 'Codex', whereas the map fails to typecheck until every
 * member has a name.
 */
export const PROVIDER_NAME: Readonly<Record<Provider, string>> = {
  claude: 'Claude',
  codex: 'Codex',
};

/**
 * How many rings the overlay draws. `double` adds a satellite ring for the
 * secondary provider; the big ring stays bound to the primary either way.
 * Purely a display preference — it never reaches collection or notifications.
 */
export type RingMode = 'single' | 'double';

export type FailureClass =
  | 'network'
  | 'provider'
  | 'parse'
  | 'internal'
  | 'cli_incompatible';
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
