<script>
  import ProviderTabs from './ProviderTabs.svelte';
  import UsageGauge from './UsageGauge.svelte';
  import { capturedAgo } from '../format/time.js';
  import { systemGuidance } from './systemGuidance.js';
  /** @typedef {import('./panelModels').PanelProviderModel} PanelProvider */
  /** @type {{ providers: { claude: PanelProvider; codex: PanelProvider }; selected: import('../contracts/domain').Provider; primary?: import('../contracts/domain').Provider; refreshing: boolean; nowMs?: number; onRefresh?: (provider: import('../contracts/domain').Provider) => void; onSelect?: (provider: import('../contracts/domain').Provider) => void; onPrimary?: (provider: import('../contracts/domain').Provider) => void; onQuit?: () => void }} */
  let {
    providers,
    selected,
    primary = selected,
    refreshing,
    nowMs = Date.now(),
    onRefresh = () => {},
    onSelect = () => {},
    onPrimary = () => {},
    onQuit = () => {},
  } = $props();
  const current = $derived(providers[selected]);
  const guidance = $derived(systemGuidance(current.system, selected));
  const captured = $derived(
    current.capturedAt === null ? null : capturedAgo(current.capturedAt, nowMs),
  );
</script>

<section class="usage-panel" aria-label="Usage panel">
  <h2 class="visually-hidden">Usage panel</h2>
  <header><ProviderTabs {selected} {primary} {onSelect} /></header>
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
        resetFormat="absolute"
      />
      <small class:stale={current.stale} class="freshness"
        >● {current.stale
          ? 'Stale'
          : 'Fresh'}{#if current.capturedAt && captured}<span
            >&nbsp;· captured <time datetime={current.capturedAt}
              >{captured}</time
            ></span
          >{/if} · {current.source}{current.isCached ? ' · cached' : ''}</small
      >
    {/if}
  </div>
  <!-- Stays mounted so a state change is announced rather than re-declared;
       only its content varies. Kept out of the grid so the empty case adds no
       gap, and collapsed to zero height by having no line box. -->
  <p class="guidance" role="status">{guidance ?? ''}</p>
  <footer>
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
    <button class="tertiary-action" onclick={() => onQuit()}
      >Quit CacheBite</button
    >
  </footer>
</section>

<style>
  .usage-panel {
    width: 100%;
    color: var(--color-text);
  }
  header {
    padding: 0 var(--space-4);
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
    color: var(--sev-ok);
    font-family: var(--font-mono);
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
    grid-template-columns: 1fr 1fr;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4) var(--space-4);
    border-top: 1px solid var(--color-border);
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
  .primary-action {
    border: 1px solid var(--color-accent);
    background: var(--color-accent);
    color: var(--badge-icon);
  }
  .secondary-action {
    border: 1px solid var(--color-border);
    background: transparent;
    color: var(--color-text);
  }
  .tertiary-action {
    grid-column: 1 / -1;
    border: 1px solid transparent;
    background: transparent;
    color: var(--color-text-muted);
    font-weight: 500;
  }
  .tertiary-action:hover,
  .tertiary-action:focus-visible {
    border-color: var(--color-border);
    color: var(--color-text);
  }
  .guidance {
    padding: 0 var(--space-4);
    margin: 0;
    color: var(--color-text-muted);
    font-size: 0.75rem;
    line-height: 1.45;
  }
</style>
