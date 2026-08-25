import type { FailureClass, Provider } from '../contracts/domain';
import type { SystemState } from '../state/engine';

const CLI_NAME: Record<Provider, string> = {
  claude: 'Claude',
  codex: 'Codex',
};
const SIGN_IN_COMMAND: Record<Provider, string> = {
  claude: 'claude login',
  codex: 'codex login',
};

/**
 * Recovery guidance for the panel status line (ui-contract §4.2).
 *
 * `active` and `loading` need no instruction: the gauges and the loading
 * skeleton already say everything there is to say.
 */
export function systemGuidance(
  system: SystemState,
  provider: Provider,
  failureClass: FailureClass | null = null,
): string | null {
  switch (system) {
    case 'auth_required':
      return `Sign in to the ${CLI_NAME[provider]} CLI: ${SIGN_IN_COMMAND[provider]}`;
    case 'unavailable':
      return `The ${CLI_NAME[provider]} CLI is not installed`;
    case 'error':
      // A CLI that rejected our invocation never recovers on retry, so the
      // generic line would be a lie — and it is the line that made this read as
      // an auth problem when codex-cli dropped an argument value.
      return failureClass === 'cli_incompatible'
        ? `The ${CLI_NAME[provider]} CLI rejected this CacheBite build. Update CacheBite.`
        : 'Could not fetch usage. Retrying shortly.';
    case 'offline':
      return 'Cannot reach the network';
    default:
      return null;
  }
}
