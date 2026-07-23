import type { Provider } from '../contracts/domain';
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
): string | null {
  switch (system) {
    case 'auth_required':
      return `Sign in to the ${CLI_NAME[provider]} CLI: ${SIGN_IN_COMMAND[provider]}`;
    case 'unavailable':
      return `The ${CLI_NAME[provider]} CLI is not installed`;
    case 'error':
      return 'Could not fetch usage. Retrying shortly.';
    case 'offline':
      return 'Cannot reach the network';
    default:
      return null;
  }
}
