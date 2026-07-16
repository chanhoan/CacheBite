<script>
  import ProviderTabs from './ProviderTabs.svelte';
  import UsageGauge from './UsageGauge.svelte';
  /** @typedef {{ provider: import('../contracts/domain').Provider; system: string; stale: boolean; planType: string | null; session: { usedPercent: number | null; severity: string; resetsAt: string | null }; weekly: { usedPercent: number | null; severity: string; resetsAt: string | null }; capturedAt: string | null; source: string; isCached: boolean }} PanelProvider */
  /** @type {{ providers: { claude: PanelProvider; codex: PanelProvider }; selected: import('../contracts/domain').Provider; refreshing: boolean; onRefresh?: (provider: import('../contracts/domain').Provider) => void; onSelect?: (provider: import('../contracts/domain').Provider) => void; onPrimary?: (provider: import('../contracts/domain').Provider) => void }} */
  let {
    providers,
    selected,
    refreshing,
    onRefresh = () => {},
    onSelect = () => {},
    onPrimary = () => {},
  } = $props();
  const current = $derived(providers[selected]);
  const other = $derived(selected === 'claude' ? 'codex' : 'claude');
</script>

<section aria-label="Usage panel">
  <h2>Usage panel</h2>
  <ProviderTabs {selected} {onSelect} />
  {#if current.system === 'loading'}<div
      data-testid="usage-skeleton"
      aria-label="Loading usage"
    >
      Loading…
    </div>
  {:else}
    <p>{current.planType ?? 'Unknown plan'} · {current.system}</p>
    <UsageGauge label="5-hour" window={current.session} stale={current.stale} />
    <UsageGauge label="Weekly" window={current.weekly} stale={current.stale} />
    <small
      >{current.capturedAt} · {current.source}{current.isCached
        ? ' · cached'
        : ''}</small
    >
  {/if}
  <button disabled={refreshing} onclick={() => onRefresh(selected)}
    >Refresh now</button
  >
  <button
    aria-label={`Make ${other === 'claude' ? 'Claude' : 'Codex'} primary`}
    onclick={() => onPrimary(other)}>Make {other} primary</button
  >
</section>
