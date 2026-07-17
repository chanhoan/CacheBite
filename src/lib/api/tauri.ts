import { invoke } from '@tauri-apps/api/core';

/** The only renderer boundary for native CacheBite commands. */
export const invokeNative = <T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> => invoke<T>(command, args);
