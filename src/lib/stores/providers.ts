import { writable } from 'svelte/store';
import type {
  FailureClass,
  Provider,
  ProviderUiSnapshot,
  UnavailableReason,
} from '../contracts/domain';
import type { UsageTransitionEvent } from '../interaction/eventPolicy';
import {
  applyProviderUpdate,
  createProviderState,
  type ProviderState,
} from '../state/engine';

/** Ceiling for the refreshing flag when collection never reports back. */
const REFRESH_TIMEOUT_MS = 30_000;

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
  const refreshTimers = new Map<Provider, ReturnType<typeof setTimeout>>();
  const setRefreshing = (provider: Provider, value: boolean) => {
    update((state) =>
      state.refreshing[provider] === value
        ? state
        : {
            ...state,
            refreshing: Object.freeze({
              ...state.refreshing,
              [provider]: value,
            }),
          },
    );
  };
  // `refresh_provider` only arms a debounce and returns, so the promise says
  // nothing about collection. The flag is released by the next state event for
  // that provider, with a ceiling in case collection never reports back.
  const settleRefresh = (provider: Provider) => {
    const timer = refreshTimers.get(provider);
    if (timer === undefined) return;
    clearTimeout(timer);
    refreshTimers.delete(provider);
    setRefreshing(provider, false);
  };
  return {
    subscribe,
    apply(
      snapshot: ProviderUiSnapshot,
      nowMs: number,
    ): readonly UsageTransitionEvent[] {
      settleRefresh(snapshot.provider);
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
          }));
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
      settleRefresh(provider);
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
    applyExpiry(
      provider: Provider,
      revision: number,
      unavailableReason: UnavailableReason | null = null,
      failureClass: FailureClass | null = null,
    ): readonly UsageTransitionEvent[] {
      settleRefresh(provider);
      update((state) => {
        const result = applyProviderUpdate(
          state[provider],
          {
            kind: 'snapshot_expired',
            revision,
            unavailableReason,
            failureClass,
          },
          Date.now(),
        );
        return result.accepted ? { ...state, [provider]: result.state } : state;
      });
      return [];
    },
    markResetPending(
      provider: Provider,
      revision: number,
    ): readonly UsageTransitionEvent[] {
      settleRefresh(provider);
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
      const pending = refreshTimers.get(provider);
      if (pending !== undefined) clearTimeout(pending);
      setRefreshing(provider, true);
      refreshTimers.set(
        provider,
        setTimeout(() => {
          refreshTimers.delete(provider);
          setRefreshing(provider, false);
        }, REFRESH_TIMEOUT_MS),
      );
      refresh(provider);
    },
  };
}
