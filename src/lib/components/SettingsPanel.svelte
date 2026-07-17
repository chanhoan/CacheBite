<script>
  /** @type {{ settings: { primaryProvider: import('../contracts/domain').Provider; bubblesEnabled: boolean; startAtLogin: boolean; notificationsEnabled: boolean; secondaryNotificationsEnabled: boolean }; autostartAvailable?: boolean; onChange?: (settings: any) => void }} */
  let { settings, autostartAvailable = true, onChange = () => {} } = $props();
</script>

<fieldset class="settings">
  <legend>Settings</legend>
  <label
    ><input
      type="checkbox"
      checked={settings.notificationsEnabled}
      onchange={(event) =>
        onChange({
          ...settings,
          notificationsEnabled: event.currentTarget.checked,
        })}
    /> Native notifications</label
  >
  <label
    ><input
      type="checkbox"
      checked={settings.secondaryNotificationsEnabled}
      onchange={(event) =>
        onChange({
          ...settings,
          secondaryNotificationsEnabled: event.currentTarget.checked,
        })}
    /> Include secondary provider notifications</label
  >
  <label
    >Primary provider <select
      value={settings.primaryProvider}
      onchange={(event) =>
        onChange({ ...settings, primaryProvider: event.currentTarget.value })}
      ><option value="claude">Claude</option><option value="codex">Codex</option
      ></select
    ></label
  >
  <label
    ><input
      type="checkbox"
      checked={settings.bubblesEnabled}
      onchange={(event) =>
        onChange({ ...settings, bubblesEnabled: event.currentTarget.checked })}
    /> Speech bubbles</label
  >
  <label
    ><input
      type="checkbox"
      checked={settings.startAtLogin}
      disabled={!autostartAvailable}
      onchange={(event) =>
        onChange({ ...settings, startAtLogin: event.currentTarget.checked })}
    /> Start at login</label
  >
</fieldset>

<style>
  .settings {
    display: grid;
    gap: var(--space-3);
    width: 100%;
    padding: var(--space-4);
    border: 0;
    border-top: 1px solid var(--color-border);
    color: var(--color-text);
  }
  legend {
    padding: 0 0 var(--space-2);
    font-weight: 600;
  }
  label {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    color: var(--color-text-muted);
    font-size: 0.8125rem;
  }
  input {
    accent-color: var(--color-accent);
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
