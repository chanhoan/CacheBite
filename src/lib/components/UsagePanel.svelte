<script>
  import ProviderColumn from './ProviderColumn.svelte';
  import { PROVIDER_NAME } from '../contracts/domain';
  import { primaryCandidate, visiblePanelProviders } from './panelModels.js';
  /** @typedef {import('./panelModels').PanelProviderModel} PanelProvider */
  /** @typedef {import('../contracts/domain').Provider} Provider */
  /** @type {{ providers: { claude: PanelProvider; codex: PanelProvider }; primary: Provider; refreshing: Readonly<Record<Provider, boolean>>; nowMs?: number; updateAvailable?: boolean; onRefresh?: (provider: Provider) => void; onPrimary?: (provider: Provider) => void; onSettings?: () => void; onClose?: () => void; onQuit?: () => void }} */
  let {
    providers,
    primary,
    refreshing,
    nowMs = Date.now(),
    updateAvailable = false,
    onRefresh = () => {},
    onPrimary = () => {},
    onSettings = () => {},
    onClose = () => {},
    onQuit = () => {},
  } = $props();
  const visible = $derived(visiblePanelProviders(providers));
  const candidate = $derived(primaryCandidate(visible, primary));
  // One button that re-reads everything on screen — the label stays `Refresh
  // now` whether that is one provider or both, because the control means the
  // same thing either way and a label that changes with the column count makes
  // the footer twitch as providers connect. Disabled while any of them is still
  // debounced: a request to the rest would only be dropped by the native
  // debounce, so letting it fire buys nothing and hides that the previous press
  // is still in flight.
  const refreshBusy = $derived(
    visible.some((provider) => refreshing[provider]),
  );
  const primaryLabel = $derived(
    candidate === null
      ? 'Set as primary'
      : `Set ${PROVIDER_NAME[candidate]} as primary`,
  );
</script>

<section class="usage-panel" aria-label="Usage panel">
  <button
    class="close-panel"
    type="button"
    aria-label="Close usage panel"
    title="Close usage panel"
    onclick={() => onClose()}>×</button
  >
  <header><h2>Usage</h2></header>
  <div class="body" style="--panel-columns: {visible.length};">
    {#each visible as provider (provider)}
      <ProviderColumn
        model={providers[provider]}
        isPrimary={provider === primary}
        {nowMs}
      />
    {/each}
  </div>
  <footer>
    <div class="footer-row">
      <button
        class="primary-action"
        disabled={refreshBusy}
        onclick={() => {
          for (const provider of visible) onRefresh(provider);
        }}>Refresh now</button
      >
      <button
        class="secondary-action"
        disabled={candidate === null}
        onclick={() => {
          if (candidate !== null) onPrimary(candidate);
        }}>{primaryLabel}</button
      >
    </div>
    <div class="footer-row">
      <button
        class="ghost-action settings-action"
        type="button"
        aria-label={updateAvailable ? 'Settings, update available' : undefined}
        onclick={() => onSettings()}
      >
        <span class="settings-label">
          Settings
          {#if updateAvailable}
            <span
              class="settings-update-dot"
              data-testid="settings-update-dot"
              aria-hidden="true"
            ></span>
          {/if}
        </span>
      </button>
      <button class="ghost-action quit" onclick={() => onQuit()}>Quit</button>
    </div>
  </footer>
</section>

<style>
  .usage-panel {
    position: relative;
    width: 100%;
    color: var(--color-text);
  }
  /* Out of flow on purpose: the close control layers over the header instead of
     reserving a column, so adding it leaves every existing box — and the height
     the ResizeObserver reports to `resize_panel` — untouched. */
  .close-panel {
    position: absolute;
    z-index: 2;
    top: 0.375rem;
    right: 0.375rem;
    display: grid;
    width: 1.5rem;
    height: 1.5rem;
    min-height: 0;
    place-items: center;
    padding: 0;
    border: 1px solid transparent;
    border-radius: 0.375rem;
    background: transparent;
    color: var(--color-text-muted);
    font-size: 1rem;
    font-weight: 500;
    line-height: 1;
  }
  .close-panel:hover,
  .close-panel:focus-visible {
    border-color: var(--color-border);
    background: var(--color-surface-sunken);
    color: var(--color-text);
  }
  /* The tab strip used to draw this rule; with the tabs gone the header owns
     it, so the seam between header and body stays where it always was. */
  header {
    padding: var(--space-3) var(--space-4);
    border-bottom: 1px solid var(--color-border);
  }
  header h2 {
    margin: 0;
    font-size: 0.8125rem;
    font-weight: 700;
  }
  /* Padding lives on the columns, not here — otherwise the divider stops short
     of the body's edges instead of splitting it top to bottom. */
  .body {
    display: grid;
    grid-template-columns: repeat(var(--panel-columns, 1), minmax(0, 1fr));
    padding: 0;
  }
  /* `:global` is required: the columns are a child component's markup, which
     this component's scoped styles cannot reach. */
  .body :global(.column + .column) {
    border-left: 1px solid var(--color-border);
  }
  footer {
    display: grid;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4) 0.875rem;
    border-top: 1px solid var(--color-border);
  }
  .footer-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-2);
  }
  /* The panel reads at 11–13px everywhere else (gauge headings, plan chip,
     freshness), so buttons inheriting the 16px document default were the
     outlier — and at 16px `Set Claude as primary` does not fit half of a 380px
     footer and wraps to two lines. `font-size` after the `font` shorthand on
     purpose: the shorthand resets size back to the inherited value. */
  button {
    min-height: 2.25rem;
    border-radius: 0.5rem;
    font: inherit;
    font-size: 0.8125rem;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }
  button:disabled {
    cursor: default;
    opacity: 0.45;
  }
  /* UI-plan canonical panel: Refresh is a solid high-contrast button (near-black
     on light, inverted on dark), Set as primary is a filled surface with a
     border. Mapped to tokens so both themes stay consistent. */
  .primary-action {
    border: 1px solid var(--color-text);
    background: var(--color-text);
    color: var(--color-surface);
  }
  .primary-action:not(:disabled):hover,
  .primary-action:focus-visible {
    opacity: 0.88;
  }
  .secondary-action {
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-text);
  }
  .secondary-action:not(:disabled):hover,
  .secondary-action:focus-visible {
    background: var(--color-surface-sunken);
  }
  .ghost-action {
    min-height: 1.875rem;
    border: 1px solid transparent;
    background: transparent;
    color: var(--color-text-muted);
    font-size: 0.75rem;
    font-weight: 500;
  }
  .settings-action {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .settings-label {
    position: relative;
    display: inline-flex;
    align-items: center;
  }
  .settings-update-dot {
    position: absolute;
    top: 0.15rem;
    right: -0.6rem;
    width: 0.45rem;
    height: 0.45rem;
    border-radius: 999px;
    background: var(--sev-exhausted);
    pointer-events: none;
  }
  .ghost-action:hover,
  .ghost-action:focus-visible {
    color: var(--color-text);
  }
  .ghost-action.quit:hover,
  .ghost-action.quit:focus-visible {
    color: var(--sev-exhausted);
  }
</style>
