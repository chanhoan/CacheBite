import { writable } from 'svelte/store';
import type { Provider, ProviderUiSnapshot } from '../contracts/domain';
import type { UsageTransitionEvent } from '../interaction/eventPolicy';
import {
  applyProviderUpdate,
  createProviderState,
  type ProviderState,
} from '../state/engine';

export interface ProvidersStoreState {
  readonly claude: ProviderState;
  readonly codex: ProviderState;
  readonly selected: Provider;
  readonly refreshing: Readonly<Record<Provider, boolean>>;
}

export function createProvidersStore(
  refresh: (provider: Provider) => void = () => undefined,
) {
  const initial: ProvidersStoreState = {
    claude: createProviderState('claude'),
    codex: createProviderState('codex'),
    selected: 'claude',
    refreshing: Object.freeze({ claude: false, codex: false }),
  };
  const { subscribe, update } = writable(initial);
  return {
    subscribe,
    apply(
      snapshot: ProviderUiSnapshot,
      nowMs: number,
    ): readonly UsageTransitionEvent[] {
      let events: readonly UsageTransitionEvent[] = [];
      update((state) => ({
        ...state,
        [snapshot.provider]: (() => {
          const result = applyProviderUpdate(
            state[snapshot.provider],
            snapshot,
            nowMs,
          );
          events = result.events.map((event) => ({
            ...event,
            provider: snapshot.provider,
          })) as readonly UsageTransitionEvent[];
          return result.state;
        })(),
      }));
      return events;
    },
    applyStatus(
      provider: Provider,
      backendUpdate: {
        readonly kind: 'credentials_missing' | 'cli_missing' | 'fetch_failed';
        readonly revision: number;
        readonly failureClass?: 'network' | 'provider' | 'parse' | 'internal';
      },
    ): readonly UsageTransitionEvent[] {
      let events: readonly UsageTransitionEvent[] = [];
      update((state) => {
        const previous = state[provider].status;
        const result =
          backendUpdate.kind === 'fetch_failed'
            ? applyProviderUpdate(
                state[provider],
                {
                  kind: 'fetch_failed',
                  revision: backendUpdate.revision,
                  failureClass: backendUpdate.failureClass ?? 'internal',
                },
                Date.now(),
              )
            : backendUpdate.kind === 'credentials_missing'
              ? applyProviderUpdate(
                  state[provider],
                  {
                    kind: 'credentials_missing',
                    revision: backendUpdate.revision,
                  },
                  Date.now(),
                )
              : applyProviderUpdate(
                  state[provider],
                  { kind: 'cli_missing', revision: backendUpdate.revision },
                  Date.now(),
                );
        if (
          result.accepted &&
          backendUpdate.kind === 'credentials_missing' &&
          previous !== 'auth_required'
        )
          events = [{ kind: 'auth_required', provider }];
        return { ...state, [provider]: result.state };
      });
      return events;
    },
    markResetPending(
      provider: Provider,
      revision: number,
    ): readonly UsageTransitionEvent[] {
      let events: readonly UsageTransitionEvent[] = [];
      update((state) => {
        const current = state[provider];
        if (revision < current.revision) return state;
        const windows = (['session', 'weekly'] as const).filter(
          (window) =>
            current.snapshot?.[window] !== null &&
            current.snapshot?.[window] !== undefined &&
            !current.resetWindows.has(window),
        );
        const resetWindows = new Set(current.resetWindows);
        for (const window of windows) resetWindows.add(window);
        events = windows.map((window) => ({
          kind: 'window_reset',
          window,
          provider,
        }));
        return {
          ...state,
          [provider]: { ...current, revision, resetWindows },
        };
      });
      return events;
    },
    selectTab(selected: Provider) {
      update((state) => ({ ...state, selected }));
    },
    requestRefresh(provider: Provider) {
      refresh(provider);
    },
    setRefreshing(provider: Provider, value: boolean) {
      update((state) => ({
        ...state,
        refreshing: Object.freeze({ ...state.refreshing, [provider]: value }),
      }));
    },
  };
}
