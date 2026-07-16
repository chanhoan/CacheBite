import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import SettingsPanel from './SettingsPanel.svelte';

describe('SettingsPanel', () => {
  afterEach(cleanup);
  it('emits immutable setting changes', async () => {
    const onChange = vi.fn();
    render(SettingsPanel, {
      props: {
        settings: {
          primaryProvider: 'claude',
          bubblesEnabled: true,
          startAtLogin: false,
          notificationsEnabled: false,
          secondaryNotificationsEnabled: false,
        },
        onChange,
      },
    });
    await fireEvent.change(screen.getByLabelText('Primary provider'), {
      target: { value: 'codex' },
    });
    await fireEvent.click(screen.getByLabelText('Speech bubbles'));
    await fireEvent.click(screen.getByLabelText('Native notifications'));
    await fireEvent.click(
      screen.getByLabelText('Include secondary provider notifications'),
    );
    expect(onChange).toHaveBeenCalledWith(
      expect.objectContaining({ primaryProvider: 'codex' }),
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
  });
});
