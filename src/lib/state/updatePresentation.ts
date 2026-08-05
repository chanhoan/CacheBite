import type { UpdateFailureReason, UpdateStateWire } from '../api/gateway';

export interface UpdateViewModel {
  /** `false` → the panel renders nothing above the tabs. */
  readonly visible: boolean;
  readonly headline: string;
  readonly detail: string | null;
  /** `null` → no action button (the banner is not visible in those states). */
  readonly primaryLabel: string | null;
  /**
   * Which command the primary button issues. `failed` shows `Try again`, which
   * has to re-*check* — the native service refuses to install from anything but
   * `available`, so wiring the retry to `installUpdate` makes it a no-op.
   */
  readonly primaryAction: 'install' | 'check' | null;
  readonly primaryEnabled: boolean;
  readonly dismissible: boolean;
  /** Always present — the Settings view reports the true status even after "Later". */
  readonly settingsLine: string;
  /** `true` while a check or an install is in flight, so Settings can disable its button. */
  readonly busy: boolean;
}

/**
 * One sentence per failure class. Deliberately free of URLs, hostnames and
 * paths: the native side classifies the failure precisely so the renderer never
 * has to echo a transport detail back at the user.
 */
const FAILURE_COPY: Record<UpdateFailureReason, string> = {
  offline: 'CacheBite could not reach GitHub. Check your connection.',
  rate_limited: 'GitHub is rate limiting downloads. Try again later.',
  metadata_invalid: 'The update information could not be read.',
  artifact_missing: 'No update is published for this platform yet.',
  download_failed: 'The download did not finish.',
  verification_failed:
    'The update signature did not verify. It was not installed.',
  install_failed:
    'The update could not be installed. Your current version is unchanged.',
};

const percentage = (received: number, total: number | null): string | null => {
  if (total === null || total <= 0) return null;
  const ratio = Math.min(Math.max(received / total, 0), 1);
  return `${Math.round(ratio * 100)}%`;
};

const hidden = (settingsLine: string): UpdateViewModel => ({
  visible: false,
  headline: '',
  detail: null,
  primaryLabel: null,
  primaryAction: null,
  primaryEnabled: false,
  dismissible: false,
  settingsLine,
  busy: false,
});

/**
 * Turns the native update state into everything both surfaces render.
 *
 * The two dismissals are separate on purpose. `dismissedVersion` suppresses an
 * *offer* and is keyed by version, so a newer release re-offers itself. A
 * failure has no version to key on, so `failureDismissed` is a plain session
 * flag the caller clears on the next check. Neither can hide a download or an
 * install: work already under way is always visible.
 */
export function updateViewModel(
  state: UpdateStateWire,
  dismissedVersion: string | null,
  failureDismissed = false,
): UpdateViewModel {
  const status = state.status;
  switch (status.status) {
    case 'idle':
      return hidden('Not checked yet');
    case 'checking':
      return { ...hidden('Checking…'), busy: true };
    case 'up_to_date':
      return hidden('Up to date');
    case 'available': {
      const settingsLine = `Update available — ${status.version}`;
      return {
        visible: dismissedVersion !== status.version,
        headline: settingsLine,
        detail: status.notes,
        primaryLabel: 'Install and restart',
        primaryAction: 'install',
        primaryEnabled: true,
        dismissible: true,
        settingsLine,
        busy: false,
      };
    }
    case 'downloading': {
      const progress = percentage(status.received, status.total);
      return {
        visible: true,
        headline: 'Downloading update',
        detail: progress ?? 'Downloading…',
        primaryLabel: 'Install and restart',
        primaryAction: 'install',
        primaryEnabled: false,
        dismissible: false,
        settingsLine: 'Downloading…',
        busy: true,
      };
    }
    case 'installing':
      return {
        visible: true,
        headline: `Installing ${status.version}…`,
        detail: 'CacheBite will restart.',
        primaryLabel: 'Install and restart',
        primaryAction: 'install',
        primaryEnabled: false,
        dismissible: false,
        settingsLine: 'Installing…',
        busy: true,
      };
    case 'failed':
      return {
        // Dismissible, but only through `failureDismissed` — a version the user
        // dismissed earlier must never suppress a failure they have not seen.
        visible: !failureDismissed,
        headline: 'Update failed',
        detail: FAILURE_COPY[status.reason],
        primaryLabel: 'Try again',
        primaryAction: 'check',
        primaryEnabled: true,
        dismissible: true,
        settingsLine: 'Update failed',
        busy: false,
      };
  }
}
