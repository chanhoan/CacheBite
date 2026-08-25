import { describe, expect, it } from 'vitest';
import {
  isProviderConnected,
  primaryCandidate,
  visiblePanelProviders,
  type PanelProviderModel,
} from './panelModels';
import type { Provider } from '../contracts/domain';
import type { SystemState } from '../state/engine';

const model = (
  provider: Provider,
  system: SystemState,
): PanelProviderModel => ({
  provider,
  system,
  stale: false,
  planType: null,
  session: { usedPercent: null, severity: 'unknown', resetsAt: null },
  weekly: { usedPercent: null, severity: 'unknown', resetsAt: null },
  failureClass: null,
  capturedAt: null,
  source: provider === 'claude' ? 'oauth_api' : 'cli_rpc',
  isCached: false,
});

const pair = (claude: SystemState, codex: SystemState) => ({
  claude: model('claude', claude),
  codex: model('codex', codex),
});

describe('isProviderConnected', () => {
  it.each([
    ['auth_required' as const, false],
    ['unavailable' as const, false],
    ['error' as const, true],
    ['offline' as const, true],
    ['loading' as const, true],
    ['active' as const, true],
  ])('treats %s as connected=%s', (system, expected) => {
    expect(isProviderConnected(system)).toBe(expected);
  });
});

describe('visiblePanelProviders', () => {
  it('keeps both columns when both providers are connected', () => {
    expect(visiblePanelProviders(pair('active', 'active'))).toEqual([
      'claude',
      'codex',
    ]);
  });

  it('drops a provider whose CLI is not installed', () => {
    expect(visiblePanelProviders(pair('active', 'unavailable'))).toEqual([
      'claude',
    ]);
  });

  it('drops a provider that is not signed in', () => {
    expect(visiblePanelProviders(pair('auth_required', 'active'))).toEqual([
      'codex',
    ]);
  });

  // Hiding is a judgement that only holds while the *other* provider is
  // connected. With neither connected there is nothing to hide against, and an
  // empty panel would strand both sign-in instructions.
  it('keeps both columns when neither provider is connected', () => {
    expect(visiblePanelProviders(pair('auth_required', 'unavailable'))).toEqual(
      ['claude', 'codex'],
    );
  });

  it('keeps a column whose fetch failed', () => {
    expect(visiblePanelProviders(pair('active', 'error'))).toEqual([
      'claude',
      'codex',
    ]);
  });

  it('keeps a column that is offline', () => {
    expect(visiblePanelProviders(pair('offline', 'active'))).toEqual([
      'claude',
      'codex',
    ]);
  });

  // Both providers report `loading` on boot. Treating that as disconnected
  // would open the panel at one column and pop it to two moments later.
  it('keeps both columns while the providers are still loading', () => {
    expect(visiblePanelProviders(pair('loading', 'loading'))).toEqual([
      'claude',
      'codex',
    ]);
  });
});

describe('primaryCandidate', () => {
  it('targets the visible provider that is not primary', () => {
    expect(primaryCandidate(['claude', 'codex'], 'claude')).toBe('codex');
  });

  it('targets the other way round just as well', () => {
    expect(primaryCandidate(['claude', 'codex'], 'codex')).toBe('claude');
  });

  it('has no target when the only visible column is already primary', () => {
    expect(primaryCandidate(['claude'], 'claude')).toBeNull();
  });

  // The primary can point at a hidden provider: the ring never auto-demotes it.
  it('targets the lone visible column when the primary is hidden', () => {
    expect(primaryCandidate(['claude'], 'codex')).toBe('claude');
  });
});
