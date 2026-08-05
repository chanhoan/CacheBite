<script>
  /** @type {{ settings: import('../state/presentation').SettingsStoreState; theme?: import('../state/theme').ThemePreference; autostartAvailable?: boolean; hideShowHotkeyLabel?: string; hideShowHotkeyAvailable?: boolean; pets?: readonly import('../api/gateway').PetSummaryModel[]; currentVersion?: string; updateLine?: string; availableUpdateVersion?: string | null; updateBusy?: boolean; onChange?: (settings: import('../state/presentation').SettingsStoreState) => void; onThemeChange?: (theme: import('../state/theme').ThemePreference) => void; onCheckUpdate?: () => void; onInstallUpdate?: () => void }} */
  let {
    settings,
    theme = 'system',
    autostartAvailable = true,
    hideShowHotkeyLabel = 'Ctrl+Shift+H',
    hideShowHotkeyAvailable = true,
    pets = [],
    currentVersion = '?',
    updateLine = '',
    availableUpdateVersion = null,
    updateBusy = false,
    onChange = () => {},
    onThemeChange = () => {},
    onCheckUpdate = () => {},
    onInstallUpdate = () => {},
  } = $props();

  // A failed enumeration must still show what is currently selected — an empty
  // <select> would hide the active pet entirely.
  const petOptions = $derived(
    pets.some((pet) => pet.id === settings.selectedPetId)
      ? pets
      : [
          { id: settings.selectedPetId, displayName: settings.selectedPetId },
          ...pets,
        ],
  );

  // The <select> only ever holds the two option values below, but its DOM type
  // is plain string. Narrow at the boundary rather than trusting the markup.
  /** @param {string} value @returns {import('../contracts/domain').Provider} */
  const asProvider = (value) => (value === 'codex' ? 'codex' : 'claude');

  /** @param {string} value @returns {import('../state/theme').ThemePreference} */
  const asTheme = (value) =>
    value === 'light' ? 'light' : value === 'dark' ? 'dark' : 'system';

  const hideShowHotkeyHelpId = 'hide-show-hotkey-help';
  const hideShowHotkeyLabelId = 'hide-show-hotkey-label';
  const versionLabelId = 'app-version-label';
  const updateLabelId = 'app-update-label';
  const updateHelpId = 'app-update-help';
</script>

<section class="settings" aria-labelledby="settings-heading">
  <h2 id="settings-heading" class="settings-heading">Settings</h2>
  {#if availableUpdateVersion}
    <div class="field update-available-row">
      <span>Update available</span>
      <button
        class="install-update-action"
        type="button"
        disabled={updateBusy}
        onclick={() => onInstallUpdate()}
      >
        Install and restart
      </button>
    </div>
  {/if}
  <label class="field"
    >Appearance <select
      value={theme}
      onchange={(event) => onThemeChange(asTheme(event.currentTarget.value))}
      ><option value="system">System</option><option value="light">Light</option
      ><option value="dark">Dark</option></select
    ></label
  >
  <label class="toggle"
    ><input
      type="checkbox"
      checked={settings.notificationsEnabled}
      onchange={(event) =>
        onChange({
          ...settings,
          notificationsEnabled: event.currentTarget.checked,
        })}
    /><span>Native notifications</span></label
  >
  <label class="toggle"
    ><input
      type="checkbox"
      checked={settings.secondaryNotificationsEnabled}
      onchange={(event) =>
        onChange({
          ...settings,
          secondaryNotificationsEnabled: event.currentTarget.checked,
        })}
    /><span>Secondary provider notifications</span></label
  >
  <label class="field"
    >Primary provider <select
      value={settings.primaryProvider}
      onchange={(event) =>
        onChange({
          ...settings,
          primaryProvider: asProvider(event.currentTarget.value),
        })}
      ><option value="claude">Claude</option><option value="codex">Codex</option
      ></select
    ></label
  >
  <label class="field"
    >Pet <select
      value={settings.selectedPetId}
      onchange={(event) =>
        onChange({ ...settings, selectedPetId: event.currentTarget.value })}
      >{#each petOptions as pet (pet.id)}<option value={pet.id}
          >{pet.displayName}</option
        >{/each}</select
    ></label
  >
  <label class="toggle"
    ><input
      type="checkbox"
      checked={settings.bubblesEnabled}
      onchange={(event) =>
        onChange({ ...settings, bubblesEnabled: event.currentTarget.checked })}
    /><span>Speech bubbles</span></label
  >
  <label class="toggle"
    ><input
      type="checkbox"
      checked={settings.startAtLogin}
      disabled={!autostartAvailable}
      onchange={(event) =>
        onChange({ ...settings, startAtLogin: event.currentTarget.checked })}
    /><span>Start at login</span></label
  >
  <!-- Read-only: the running version comes from the bundle, which CI derives
       from the git tag. There is nothing here for the user to change. -->
  <div class="field">
    <span id={versionLabelId}>Version</span>
    <kbd class="shortcut" aria-labelledby={versionLabelId}>{currentVersion}</kbd
    >
  </div>
  <div class="field">
    <span id={updateLabelId}>Updates</span>
    <button
      class="ghost-action"
      type="button"
      disabled={updateBusy}
      aria-describedby={updateHelpId}
      onclick={() => onCheckUpdate()}>Check for updates</button
    >
  </div>
  <p id={updateHelpId} class="field-help">{updateLine}</p>
  <!-- Read-only: the binding is a fixed native constant, so there is no form
       control here and no onChange to fire. -->
  <div class="field">
    <span id={hideShowHotkeyLabelId}>Hide/show shortcut</span>
    <kbd
      class="shortcut"
      aria-labelledby={hideShowHotkeyLabelId}
      aria-describedby={hideShowHotkeyHelpId}>{hideShowHotkeyLabel}</kbd
    >
  </div>
  <p id={hideShowHotkeyHelpId} class="field-help">
    Hides and shows the pet.<br />Usage keeps updating while hidden.
    {#if !hideShowHotkeyAvailable}
      <span class="field-state"
        >Another app is using this shortcut. Close it and restart CacheBite.</span
      >
    {/if}
  </p>
</section>

<style>
  .settings {
    display: grid;
    gap: var(--space-3);
    width: 100%;
    padding: var(--space-4);
    border-top: 1px solid var(--color-border);
    color: var(--color-text);
  }
  .settings-heading {
    margin: 0 0 var(--space-1);
    font-size: 0.9375rem;
    font-weight: 600;
    color: var(--color-text);
  }
  /* Checkbox rows: control sits directly beside its label so long text wraps
     cleanly instead of being flung to the opposite edge. */
  .toggle {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-text-muted);
    font-size: 0.8125rem;
  }
  .toggle input {
    flex: none;
    margin: 0;
    accent-color: var(--color-accent);
  }
  /* Select rows: label left, control pinned right. */
  .field {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    color: var(--color-text-muted);
    font-size: 0.8125rem;
  }
  .update-available-row {
    align-items: center;
  }
  select {
    padding: 0.35rem 1.75rem 0.35rem 0.5rem;
    border: 1px solid var(--color-border);
    border-radius: 0.4rem;
    background: var(--color-surface);
    color: var(--color-text);
    font: inherit;
  }
  /* Reads as a key cap, not an editable field — the binding is fixed. */
  .shortcut {
    flex: none;
    padding: 0.2rem 0.45rem;
    border: 1px solid var(--color-border);
    border-radius: 0.3rem;
    background: var(--color-surface);
    color: var(--color-text);
    font: inherit;
    white-space: nowrap;
  }
  .ghost-action {
    flex: none;
    min-height: 1.875rem;
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--color-border);
    border-radius: 0.4rem;
    background: var(--color-surface);
    color: var(--color-text);
    font: inherit;
    font-size: 0.75rem;
    white-space: nowrap;
    cursor: pointer;
  }
  .install-update-action {
    flex: none;
    min-height: 1.875rem;
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--color-text);
    border-radius: 0.5rem;
    background: var(--color-text);
    color: var(--color-surface);
    font: inherit;
    font-size: 0.75rem;
    font-weight: 600;
    line-height: 1;
    white-space: nowrap;
    cursor: pointer;
  }
  .ghost-action:disabled,
  .install-update-action:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .ghost-action:not(:disabled):hover,
  .install-update-action:not(:disabled):hover,
  .ghost-action:focus-visible,
  .install-update-action:focus-visible {
    opacity: 0.88;
  }
  .field-help {
    margin: 0;
    color: var(--color-text-muted);
    font-size: 0.75rem;
    line-height: 1.4;
  }
  .field-state {
    display: block;
    margin-top: 0.2rem;
    color: var(--color-text);
  }
</style>
