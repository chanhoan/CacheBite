<script>
  /** @type {{ settings: import('../state/presentation').SettingsStoreState; theme?: import('../state/theme').ThemePreference; autostartAvailable?: boolean; onChange?: (settings: import('../state/presentation').SettingsStoreState) => void; onThemeChange?: (theme: import('../state/theme').ThemePreference) => void }} */
  let {
    settings,
    theme = 'system',
    autostartAvailable = true,
    onChange = () => {},
    onThemeChange = () => {},
  } = $props();

  // The <select> only ever holds the two option values below, but its DOM type
  // is plain string. Narrow at the boundary rather than trusting the markup.
  /** @param {string} value @returns {import('../contracts/domain').Provider} */
  const asProvider = (value) => (value === 'codex' ? 'codex' : 'claude');

  /** @param {string} value @returns {import('../state/theme').ThemePreference} */
  const asTheme = (value) =>
    value === 'light' ? 'light' : value === 'dark' ? 'dark' : 'system';
</script>

<section class="settings" aria-labelledby="settings-heading">
  <h2 id="settings-heading" class="settings-heading">Settings</h2>
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
  select {
    padding: 0.35rem 1.75rem 0.35rem 0.5rem;
    border: 1px solid var(--color-border);
    border-radius: 0.4rem;
    background: var(--color-surface);
    color: var(--color-text);
    font: inherit;
  }
</style>
