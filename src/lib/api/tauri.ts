import { invoke } from '@tauri-apps/api/core';

/** The only renderer boundary for native CacheBite commands. */
export const invokeNative = <T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> => invoke<T>(command, args);

export const getProviderStates = <T>(): Promise<T> =>
  invokeNative('get_provider_states');
export const refreshProvider = (provider: 'claude' | 'codex'): Promise<void> =>
  invokeNative('refresh_provider', { provider });
export const updateSettings = <T>(settings: T): Promise<T> =>
  invokeNative('update_settings', { settings });
export const showPanel = (): Promise<void> => invokeNative('show_panel');
export const quit = (): Promise<void> => invokeNative('quit');
