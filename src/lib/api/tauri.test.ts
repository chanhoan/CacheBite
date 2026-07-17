import { beforeEach, describe, expect, it, vi } from 'vitest';
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
import {
  getProviderStates,
  quit,
  refreshProvider,
  showPanel,
  updateSettings,
} from './tauri';

describe('typed tauri gateway', () => {
  beforeEach(() => invoke.mockResolvedValue(undefined));
  it('uses only narrow allowlisted commands', async () => {
    await getProviderStates();
    await refreshProvider('codex');
    await updateSettings({ bubbles: true });
    await showPanel();
    await quit();
    expect(invoke.mock.calls.map(([command]) => command)).toEqual([
      'get_provider_states',
      'refresh_provider',
      'update_settings',
      'show_panel',
      'quit',
    ]);
    expect(invoke).toHaveBeenCalledWith('refresh_provider', {
      provider: 'codex',
    });
  });
});
