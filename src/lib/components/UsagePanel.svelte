<script>
  import ProviderTabs from './ProviderTabs.svelte';
  import UsageGauge from './UsageGauge.svelte';
  import { capturedAgo } from '../format/time.js';
  import { systemGuidance } from './systemGuidance.js';
  /** @typedef {import('./panelModels').PanelProviderModel} PanelProvider */
  /** @type {{ providers: { claude: PanelProvider; codex: PanelProvider }; selected: import('../contracts/domain').Provider; primary?: import('../contracts/domain').Provider; refreshing: boolean; nowMs?: number; onRefresh?: (provider: import('../contracts/domain').Provider) => void; onSelect?: (provider: import('../contracts/domain').Provider) => void; onPrimary?: (provider: import('../contracts/domain').Provider) => void; onSettings?: () => void; onClose?: () => void; onQuit?: () => void }} */
  let {
    providers,
    selected,
    primary = selected,
    refreshing,
    nowMs = Date.now(),
    onRefresh = () => {},
    onSelect = () => {},
    onPrimary = () => {},
    onSettings = () => {},
    onClose = () => {},
    onQuit = () => {},
  } = $props();
  const current = $derived(providers[selected]);
  const guidance = $derived(systemGuidance(current.system, selected));
  const captured = $derived(
    current.capturedAt === null ? null : capturedAgo(current.capturedAt, nowMs),
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
  <h2 class="visually-hidden">Usage panel</h2>
  <header>
    <ProviderTabs {selected} {primary} {onSelect} />
  </header>
  <div class="body">
    {#if current.system === 'loading'}
      <div
        class="skeleton"
        data-testid="usage-skeleton"
        aria-label="Loading usage"
      >
        Loading…
      </div>
    {:else}
      <div class="provider-heading">
        <strong>{selected === 'claude' ? 'Claude' : 'Codex'}</strong>
        {#if current.planType}<span class="plan-chip">{current.planType}</span
          >{/if}
      </div>
      <UsageGauge
        label="5-hour"
        window={current.session}
        stale={current.stale}
        {nowMs}
      />
      <UsageGauge
        label="Weekly"
        window={current.weekly}
        stale={current.stale}
        {nowMs}
      />
      <small class:stale={current.stale} class="freshness"
        >● {current.stale
          ? 'Stale'
          : 'Fresh'}{#if current.capturedAt && captured}<span
            >&nbsp;· captured <time datetime={current.capturedAt}
              >{captured}</time
            ></span
          >{/if}</small
      >
    {/if}
  </div>
  <!-- Stays mounted so a state change is announced rather than re-declared;
       only its content varies. Kept out of the grid so the empty case adds no
       gap, and collapsed to zero height by having no line box. -->
  <p class="guidance" role="status">{guidance ?? ''}</p>
  <footer>
    <div class="footer-row">
      <button
        class="primary-action"
        disabled={refreshing}
        onclick={() => onRefresh(selected)}>Refresh now</button
      >
      <button
        class="secondary-action"
        disabled={selected === primary}
        onclick={() => onPrimary(selected)}>Set as primary</button
      >
    </div>
    <div class="footer-row">
      <button class="ghost-action" onclick={() => onSettings()}>Settings</button
      >
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
  header {
    padding: var(--space-3) var(--space-4) 0;
  }
  .body {
    display: grid;
    gap: var(--space-4);
    padding: var(--space-4);
  }
  .provider-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .plan-chip {
    padding: 0.15rem 0.5rem;
    border-radius: 999px;
    background: var(--color-surface-sunken);
    color: var(--color-text-muted);
    font-size: 0.6875rem;
    text-transform: capitalize;
  }
  .freshness {
    overflow: hidden;
    color: var(--sev-ok);
    font-family: var(--font-mono);
    font-size: 0.6875rem;
    white-space: nowrap;
  }
  .freshness.stale {
    color: var(--color-text-faint);
  }
  .skeleton {
    padding: 2rem;
    color: var(--color-text-muted);
    text-align: center;
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
  button {
    min-height: 2.25rem;
    border-radius: 0.5rem;
    font: inherit;
    font-weight: 600;
    cursor: pointer;
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
    font-weight: 500;
  }
  .ghost-action:hover,
  .ghost-action:focus-visible {
    color: var(--color-text);
  }
  .ghost-action.quit:hover,
  .ghost-action.quit:focus-visible {
    color: var(--sev-exhausted);
  }
  .guidance {
    padding: 0 var(--space-4);
    margin: 0;
    color: var(--color-text-muted);
    font-size: 0.75rem;
    line-height: 1.45;
  }
</style>
