import type { Provider } from '../contracts/domain';
import type { Severity, WindowName } from '../state/engine';

export type UsageTransitionEvent =
  | {
      readonly kind: 'severity_raised';
      readonly provider: Provider;
      readonly window: WindowName;
      readonly severity: Exclude<Severity, 'ok' | 'unknown'>;
    }
  | {
      readonly kind: 'window_reset';
      readonly provider: Provider;
      readonly window: WindowName;
    }
  | {
      readonly kind: 'auth_required' | 'recovered';
      readonly provider: Provider;
    };

export interface EventEligibilityContext {
  readonly primary: Provider;
  readonly enabled: boolean;
  readonly secondaryEnabled?: boolean;
  readonly trackIneligible?: boolean;
}

export interface EventEligibilityResult {
  readonly dedupe: ReadonlySet<string>;
  readonly accepted: boolean;
}

export const eventKey = (event: UsageTransitionEvent): string =>
  event.kind === 'severity_raised'
    ? `${event.provider}:${event.window}:${event.severity}`
    : `${event.provider}:${event.kind}:${'window' in event ? event.window : ''}`;

export const eventPriority = (event: UsageTransitionEvent): number =>
  event.kind === 'severity_raised'
    ? { warn: 1, critical: 3, exhausted: 4 }[event.severity]
    : event.kind === 'auth_required'
      ? 3
      : event.kind === 'window_reset'
        ? 2
        : 1;

export const eventMessage = (event: UsageTransitionEvent): string =>
  event.kind === 'severity_raised'
    ? `${event.window === 'session' ? '5-hour' : 'Weekly'} usage is ${event.severity}`
    : event.kind === 'window_reset'
      ? `${event.window === 'session' ? '5-hour' : 'Weekly'} window reset`
      : event.kind === 'auth_required'
        ? 'Sign in required'
        : 'Connected again';

export function reduceEventEligibility(
  dedupeState: ReadonlySet<string>,
  event: UsageTransitionEvent,
  context: EventEligibilityContext,
  deliverable: (event: UsageTransitionEvent) => boolean,
): EventEligibilityResult {
  const providerAllowed =
    event.provider === context.primary || context.secondaryEnabled === true;
  const providerTracked = providerAllowed || context.trackIneligible === true;
  if (!providerTracked) return { dedupe: dedupeState, accepted: false };

  let dedupe = dedupeState;
  if (event.kind === 'window_reset') {
    const next = new Set(dedupe);
    for (const key of next)
      if (key.startsWith(`${event.provider}:${event.window}:`))
        next.delete(key);
    dedupe = next;
  }

  if (!deliverable(event)) return { dedupe, accepted: false };
  if (
    (!providerAllowed || !context.enabled) &&
    context.trackIneligible !== true
  )
    return { dedupe, accepted: false };

  const key = eventKey(event);
  if (dedupe.has(key)) return { dedupe, accepted: false };
  const next = new Set(dedupe);
  next.add(key);
  return {
    dedupe: next,
    accepted: providerAllowed && context.enabled,
  };
}
