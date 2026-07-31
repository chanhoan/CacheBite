import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import SettingsPanel from './SettingsPanel.svelte';

describe('SettingsPanel', () => {
  afterEach(cleanup);
  it('emits immutable setting changes', async () => {
    const onChange = vi.fn();
    const onThemeChange = vi.fn();
    render(SettingsPanel, {
      props: {
        settings: {
          primaryProvider: 'claude',
          selectedPetId: 'tabby',
          bubblesEnabled: true,
          startAtLogin: false,
          notificationsEnabled: false,
          secondaryNotificationsEnabled: false,
          hideShowHotkey: null,
        },
        theme: 'system',
        pets: [
          { id: 'corgi', displayName: 'Corgi' },
          { id: 'tabby', displayName: 'Tabby' },
        ],
        onChange,
        onThemeChange,
      },
    });
    await fireEvent.change(screen.getByLabelText('Appearance'), {
      target: { value: 'dark' },
    });
    await fireEvent.change(screen.getByLabelText('Primary provider'), {
      target: { value: 'codex' },
    });
    await fireEvent.change(screen.getByLabelText('Pet'), {
      target: { value: 'corgi' },
    });
    await fireEvent.click(screen.getByLabelText('Speech bubbles'));
    await fireEvent.click(screen.getByLabelText('Native notifications'));
    await fireEvent.click(
      screen.getByLabelText('Secondary provider notifications'),
    );
    await fireEvent.change(screen.getByLabelText('Hide/show shortcut'), {
      target: { value: 'CmdOrCtrl+Shift+H' },
    });
    expect(onChange).toHaveBeenCalledWith(
      expect.objectContaining({ primaryProvider: 'codex' }),
    );
    // The pet is picked on its own; the primary provider rides along unchanged.
    expect(onChange).toHaveBeenCalledWith(
      expect.objectContaining({
        selectedPetId: 'corgi',
        primaryProvider: 'claude',
      }),
    );
    expect(onChange).toHaveBeenCalledWith(
      expect.objectContaining({ bubblesEnabled: false }),
    );
    expect(onChange).toHaveBeenCalledWith(
      expect.objectContaining({ notificationsEnabled: true }),
    );
    expect(onChange).toHaveBeenCalledWith(
      expect.objectContaining({ secondaryNotificationsEnabled: true }),
    );
    expect(onChange).toHaveBeenCalledWith(
      expect.objectContaining({ hideShowHotkey: 'CmdOrCtrl+Shift+H' }),
    );
    expect(onThemeChange).toHaveBeenCalledWith('dark');
  });

  it('clears the hotkey when the input is emptied', async () => {
    const onChange = vi.fn();
    render(SettingsPanel, {
      props: {
        settings: {
          primaryProvider: 'claude',
          selectedPetId: 'tabby',
          bubblesEnabled: true,
          startAtLogin: false,
          notificationsEnabled: false,
          secondaryNotificationsEnabled: false,
          hideShowHotkey: 'CommandOrControl+Shift+H',
        },
        pets: [{ id: 'tabby', displayName: 'Tabby' }],
        onChange,
      },
    });

    await fireEvent.change(screen.getByLabelText('Hide/show shortcut'), {
      target: { value: '   ' },
    });

    expect(onChange).toHaveBeenCalledWith(
      expect.objectContaining({ hideShowHotkey: null }),
    );
  });

  it('describes the preset mapping and inactive state', () => {
    render(SettingsPanel, {
      props: {
        settings: {
          primaryProvider: 'claude',
          selectedPetId: 'tabby',
          bubblesEnabled: true,
          startAtLogin: false,
          notificationsEnabled: false,
          secondaryNotificationsEnabled: false,
          hideShowHotkey: null,
        },
        pets: [{ id: 'tabby', displayName: 'Tabby' }],
      },
    });

    expect(
      screen.queryByText(
        'Default: Windows/Linux Ctrl+Shift+H, macOS Cmd+Shift+H.',
      ),
    ).not.toBeNull();
    expect(screen.queryByText('No shortcut active')).not.toBeNull();
  });
});
