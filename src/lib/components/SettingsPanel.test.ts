import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import SettingsPanel from './SettingsPanel.svelte';

const settings = {
  primaryProvider: 'claude',
  selectedPetId: 'tabby',
  bubblesEnabled: true,
  startAtLogin: false,
  notificationsEnabled: false,
  secondaryNotificationsEnabled: false,
} as const;

describe('SettingsPanel', () => {
  afterEach(cleanup);

  it('emits immutable setting changes', async () => {
    const onChange = vi.fn();
    const onThemeChange = vi.fn();
    render(SettingsPanel, {
      props: {
        settings,
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

  it('shows the fixed shortcut the way the running platform spells it', () => {
    render(SettingsPanel, {
      props: {
        settings,
        pets: [{ id: 'tabby', displayName: 'Tabby' }],
        hideShowHotkeyLabel: 'Cmd+Shift+H',
      },
    });

    expect(screen.getByLabelText('Hide/show shortcut').textContent).toBe(
      'Cmd+Shift+H',
    );
    // The two sentences sit on their own lines, split by a <br>, so match each
    // against the paragraph rather than expecting one exact text node.
    expect(screen.queryByText(/Hides and shows the pet\./)).not.toBeNull();
    expect(
      screen.queryByText(/Usage keeps updating while hidden\./),
    ).not.toBeNull();
    // Guards the regression this screen exists to prevent: an editable field
    // here is what let one failed registration persist as "no shortcut active".
    expect(screen.queryByRole('textbox')).toBeNull();
  });

  it('explains how to recover when another app owns the shortcut', () => {
    const conflictMessage =
      'Another app is using this shortcut. Close it and restart CacheBite.';
    const { unmount } = render(SettingsPanel, {
      props: {
        settings,
        pets: [{ id: 'tabby', displayName: 'Tabby' }],
        hideShowHotkeyAvailable: false,
      },
    });

    expect(screen.queryByText(conflictMessage)).not.toBeNull();

    unmount();
    render(SettingsPanel, {
      props: {
        settings,
        pets: [{ id: 'tabby', displayName: 'Tabby' }],
        hideShowHotkeyAvailable: true,
      },
    });

    expect(screen.queryByText(conflictMessage)).toBeNull();
  });

  it('reports the running version and the update status', () => {
    render(SettingsPanel, {
      props: {
        settings,
        pets: [{ id: 'tabby', displayName: 'Tabby' }],
        currentVersion: '0.1.0-beta.4',
        updateLine: 'Up to date',
      },
    });

    expect(screen.queryByText('0.1.0-beta.4')).not.toBeNull();
    expect(screen.queryByText('Up to date')).not.toBeNull();
  });

  it('shows the available update row before Appearance and installs once per click', async () => {
    const onInstallUpdate = vi.fn();
    const { container } = render(SettingsPanel, {
      props: {
        settings,
        pets: [{ id: 'tabby', displayName: 'Tabby' }],
        availableUpdateVersion: '0.1.0-beta.5',
        onInstallUpdate,
      },
    });

    const updateRow = container.querySelector('.update-available-row');
    const appearanceField = screen.getByText('Appearance');

    if (!updateRow) {
      throw new Error('Expected update row to exist');
    }
    expect(
      updateRow.compareDocumentPosition(appearanceField) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(screen.getByText('Update available')).toBeTruthy();
    expect(screen.queryByText('Later')).toBeNull();
    expect(screen.queryByText('Fixes a crash.')).toBeNull();

    await fireEvent.click(
      screen.getByRole('button', { name: 'Install and restart' }),
    );

    expect(onInstallUpdate).toHaveBeenCalledTimes(1);
  });

  it('hides the available update row when no version is available', () => {
    render(SettingsPanel, {
      props: {
        settings,
        pets: [{ id: 'tabby', displayName: 'Tabby' }],
        availableUpdateVersion: null,
      },
    });

    expect(
      screen.queryByRole('button', { name: 'Install and restart' }),
    ).toBeNull();
    expect(screen.queryByText('Update available')).toBeNull();
  });

  it('disables the install button while the update action is busy', () => {
    render(SettingsPanel, {
      props: {
        settings,
        pets: [{ id: 'tabby', displayName: 'Tabby' }],
        availableUpdateVersion: '0.1.0-beta.5',
        updateBusy: true,
      },
    });

    expect(
      screen
        .getByRole('button', { name: 'Install and restart' })
        .hasAttribute('disabled'),
    ).toBe(true);
  });

  it('invokes onCheckUpdate once per click', async () => {
    const onCheckUpdate = vi.fn();
    render(SettingsPanel, {
      props: {
        settings,
        pets: [{ id: 'tabby', displayName: 'Tabby' }],
        onCheckUpdate,
      },
    });

    await fireEvent.click(
      screen.getByRole('button', { name: 'Check for updates' }),
    );

    expect(onCheckUpdate).toHaveBeenCalledTimes(1);
  });

  it('disables the manual check while one is already in flight', () => {
    render(SettingsPanel, {
      props: {
        settings,
        pets: [{ id: 'tabby', displayName: 'Tabby' }],
        updateBusy: true,
        updateLine: 'Checking…',
      },
    });

    expect(
      screen
        .getByRole('button', { name: 'Check for updates' })
        .hasAttribute('disabled'),
    ).toBe(true);
  });

  it('keeps the version row read-only', () => {
    render(SettingsPanel, {
      props: {
        settings,
        pets: [{ id: 'tabby', displayName: 'Tabby' }],
        currentVersion: '0.1.0-beta.4',
      },
    });

    // The version comes from the bundle, which CI derives from the git tag.
    // An editable field here would invite a value the updater cannot honour.
    expect(screen.queryByRole('textbox')).toBeNull();
  });
});
