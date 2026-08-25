<script lang="ts">
  import UsageGauge from './UsageGauge.svelte';
  import { capturedAgo } from '../format/time';
  import { PROVIDER_NAME } from '../contracts/domain';
  import { systemGuidance } from './systemGuidance';
  import type { PanelProviderModel } from './panelModels';

  let {
    model,
    isPrimary,
    nowMs = Date.now(),
  }: {
    model: PanelProviderModel;
    isPrimary: boolean;
    nowMs?: number;
  } = $props();

  const name = $derived(PROVIDER_NAME[model.provider]);
  const guidance = $derived(
    systemGuidance(model.system, model.provider, model.failureClass),
  );
  const captured = $derived(
    model.capturedAt === null ? null : capturedAgo(model.capturedAt, nowMs),
  );
</script>

<!-- `role="group"` rather than the implicit `region` a named <section> would
     take: the panel is one landmark, and a column is a labelled cluster inside
     it, not a second place to navigate to. Two columns each holding two gauges
     would otherwise put seven regions in a 380px popup. -->
<section
  class="column"
  role="group"
  data-provider={model.provider}
  aria-label={isPrimary ? `${name} usage (primary)` : `${name} usage`}
>
  {#if model.system === 'loading'}
    <div
      class="skeleton"
      data-testid="usage-skeleton"
      aria-label="Loading usage"
    >
      Loading…
    </div>
  {:else}
    <div class="provider-heading">
      <strong
        >{name}{#if isPrimary}<span class="primary-star" aria-hidden="true">
            ★</span
          >{/if}</strong
      >
      {#if model.planType}<span class="plan-chip">{model.planType}</span>{/if}
    </div>
    <UsageGauge
      label="5-hour"
      window={model.session}
      stale={model.stale}
      {nowMs}
    />
    <UsageGauge
      label="Weekly"
      window={model.weekly}
      stale={model.stale}
      {nowMs}
    />
    <!-- Two lines, not one clipped line. At a column's width `● Fresh ·
         captured just now` does not fit, and the old single nowrap line simply
         cut it off mid-word. Stacking keeps the whole sentence readable and
         costs one line of height that both columns pay equally. -->
    <small class:stale={model.stale} class="freshness">
      <span>● {model.stale ? 'Stale' : 'Fresh'}</span>
      {#if model.capturedAt && captured}
        <span>captured <time datetime={model.capturedAt}>{captured}</time></span
        >
      {/if}
    </small>
  {/if}
  <!-- Stays mounted so a state change is announced rather than re-declared;
       only its content varies. Collapsed to zero height by having no line box
       when empty, so a healthy column gains no trailing space. -->
  <p class="guidance" role="status">{guidance ?? ''}</p>
</section>

<style>
  /* `align-content: start` matters once two columns share a row: without it a
     short column (a lone skeleton) stretches to the tall one's height and its
     gauges drift apart. */
  .column {
    display: grid;
    align-content: start;
    gap: var(--space-4);
    padding: var(--space-4);
  }
  .provider-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  /* UI-plan spec: the primary-provider star is amber, independent of the
     heading text color (which is near-black). */
  .primary-star {
    color: #f59e0b;
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
    display: grid;
    gap: 0.1rem;
    color: var(--sev-ok);
    font-family: var(--font-mono);
    font-size: 0.6875rem;
  }
  .freshness.stale {
    color: var(--color-text-faint);
  }
  .skeleton {
    padding: 2rem;
    color: var(--color-text-muted);
    text-align: center;
  }
  /* No horizontal padding of its own — the column already carries it. */
  .guidance {
    padding: 0;
    margin: 0;
    color: var(--color-text-muted);
    font-size: 0.75rem;
    line-height: 1.45;
  }
</style>
