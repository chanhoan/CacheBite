import type { Provider } from '../contracts/domain';
import {
  eventMessage,
  reduceEventEligibility,
  type UsageTransitionEvent,
} from './eventPolicy';

export type PermissionState = 'prompt' | 'granted' | 'denied';
export type NotificationDiagnostic =
  | { readonly status: 'available' }
  | { readonly status: 'permission_denied' }
  | { readonly status: 'unavailable'; readonly reason: string };

export interface NativeNotification {
  readonly title: string;
  readonly body: string;
}

export interface NotificationAdapter {
  capability(): Promise<NotificationDiagnostic>;
  permission(): Promise<PermissionState>;
  requestPermission(): Promise<PermissionState>;
  send(notification: NativeNotification): Promise<void>;
}

export interface NotificationPolicyState {
  readonly optedIn: boolean;
  readonly permission: PermissionState;
  readonly diagnostic: NotificationDiagnostic;
  readonly dedupe: ReadonlySet<string>;
}

export const createNotificationPolicy = (): NotificationPolicyState => ({
  optedIn: false,
  permission: 'prompt',
  diagnostic: { status: 'available' },
  dedupe: new Set(),
});

export async function configureNotifications(
  state: NotificationPolicyState,
  optedIn: boolean,
  adapter: NotificationAdapter,
): Promise<NotificationPolicyState> {
  if (!optedIn) return { ...state, optedIn: false };
  const capability = await adapter.capability();
  if (capability.status === 'unavailable')
    return { ...state, optedIn: true, diagnostic: capability };
  const current = await adapter.permission();
  const permission =
    current === 'prompt' ? await adapter.requestPermission() : current;
  return {
    ...state,
    optedIn: true,
    permission,
    diagnostic:
      permission === 'granted'
        ? { status: 'available' }
        : { status: 'permission_denied' },
  };
}

const notificationEligible = (event: UsageTransitionEvent): boolean =>
  event.kind === 'auth_required' ||
  (event.kind === 'severity_raised' &&
    (event.severity === 'critical' || event.severity === 'exhausted'));

export async function handleNotificationEvent(
  state: NotificationPolicyState,
  event: UsageTransitionEvent,
  primary: Provider,
  adapter: NotificationAdapter,
  secondaryNotificationsEnabled = false,
): Promise<NotificationPolicyState> {
  const result = reduceEventEligibility(
    state.dedupe,
    event,
    {
      primary,
      enabled:
        state.optedIn &&
        state.permission === 'granted' &&
        state.diagnostic.status === 'available',
      secondaryEnabled: secondaryNotificationsEnabled,
      trackIneligible: true,
    },
    notificationEligible,
  );
  const next = { ...state, dedupe: result.dedupe };
  if (!result.accepted) return next;
  const providerLabel = event.provider === 'claude' ? 'Claude' : 'Codex';
  await adapter.send({
    title: 'CacheBite',
    body: `${providerLabel}: ${eventMessage(event)}`,
  });
  return next;
}
