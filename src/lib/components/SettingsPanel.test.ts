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
    expect(onThemeChange).toHaveBeenCalledWith('dark');
  });
});
