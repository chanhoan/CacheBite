import { describe, expect, it } from 'vitest';
import { systemGuidance } from './systemGuidance';

describe('systemGuidance', () => {
  it.each([
    ['claude' as const, 'Sign in to the Claude CLI: claude login'],
    ['codex' as const, 'Sign in to the Codex CLI: codex login'],
  ])('names the %s CLI sign-in command', (provider, expected) => {
    expect(systemGuidance('auth_required', provider)).toBe(expected);
  });

  it.each([
    ['claude' as const, 'The Claude CLI is not installed'],
    ['codex' as const, 'The Codex CLI is not installed'],
  ])('reports a missing %s CLI as an install gap', (provider, expected) => {
    expect(systemGuidance('unavailable', provider)).toBe(expected);
  });

  it.each(['claude' as const, 'codex' as const])(
    'gives provider-independent transport guidance for %s',
    (provider) => {
      expect(systemGuidance('error', provider)).toBe(
        'Could not fetch usage. Retrying shortly.',
      );
      expect(systemGuidance('offline', provider)).toBe(
        'Cannot reach the network',
      );
    },
  );

  it.each(['active' as const, 'loading' as const])(
    'stays silent for the non-actionable state %s',
    (system) => {
      expect(systemGuidance(system, 'claude')).toBeNull();
      expect(systemGuidance(system, 'codex')).toBeNull();
    },
  );
});
