<script>
  /** @type {{ settings: { primaryProvider: import('../contracts/domain').Provider; bubblesEnabled: boolean; startAtLogin: boolean; notificationsEnabled: boolean; secondaryNotificationsEnabled: boolean }; autostartAvailable?: boolean; onChange?: (settings: any) => void }} */
  let { settings, autostartAvailable = true, onChange = () => {} } = $props();
</script>

<fieldset>
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
