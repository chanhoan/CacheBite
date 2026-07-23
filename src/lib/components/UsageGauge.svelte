<script lang="ts">
  import { absoluteShort, relativeFromNow } from '../format/time';
  import type { Severity } from '../state/engine';

  let {
    label,
    window: usage,
    stale = false,
    nowMs = Date.now(),
    resetFormat = 'relative',
  }: {
    label: string;
    window: {
      usedPercent: number | null;
      severity: Severity;
      resetsAt: string | null;
    };
    stale?: boolean;
    nowMs?: number;
    /** How the reset time reads. Chosen by the caller, not inferred from copy. */
    resetFormat?: 'absolute' | 'relative';
  } = $props();
  const percent = $derived(
    usage.usedPercent === null || !Number.isFinite(usage.usedPercent)
      ? 0
      : Math.min(100, Math.max(0, usage.usedPercent)),
  );
  // A reset days out reads better as a wall-clock time than as a countdown, and
  // vice versa for a short window — but that is the caller's call, so it comes
  // in as `resetFormat` rather than being read off the display label. An
  // unparsable timestamp yields null and the <time> element is dropped entirely.
  const resetLabel = $derived(
    usage.resetsAt === null
      ? null
      : resetFormat === 'absolute'
        ? absoluteShort(usage.resetsAt)
        : relativeFromNow(usage.resetsAt, nowMs),
  );
</script>

<section class="gauge" aria-label={`${label} usage`} data-testid="usage-gauge">
  <div class="gauge-heading">
    <span>{label} window</span>
    <strong data-severity={usage.severity}
      >{usage.usedPercent === null
        ? 'Unknown'
        : `${Math.round(percent)}%`}</strong
    >
  </div>
  <div
    class:stale
    class="gauge-track"
    role="progressbar"
    aria-label={`${label} usage`}
    aria-valuemin="0"
    aria-valuemax="100"
    aria-valuenow={percent}
  >
    <div
      class="gauge-fill"
      data-severity={usage.severity}
      style:width={`${percent}%`}
    ></div>
  </div>
  {#if usage.resetsAt && resetLabel}<time datetime={usage.resetsAt}
      >{resetFormat === 'absolute' ? 'resets' : 'resets in'} {resetLabel}</time
    >{/if}
</section>

<style>
  .gauge {
    display: grid;
    gap: 0.35rem;
  }
  .gauge-heading {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    color: var(--color-text-muted);
    font-size: 0.8125rem;
  }
  strong {
    font-family: var(--font-mono);
    font-size: 0.9375rem;
  }
  .gauge-track {
    height: 0.5rem;
    overflow: hidden;
    border-radius: 999px;
    background: var(--color-surface-sunken);
  }
  .gauge-track.stale {
    opacity: var(--overlay-stale-dim);
  }
  .gauge-fill {
    height: 100%;
    border-radius: inherit;
    background: var(--sev-unknown);
  }
  strong[data-severity='ok'] {
    color: var(--sev-ok);
  }
  strong[data-severity='warn'] {
    color: var(--sev-warn);
  }
  strong[data-severity='critical'] {
    color: var(--sev-critical);
  }
  strong[data-severity='exhausted'] {
    color: var(--sev-exhausted);
  }
  strong[data-severity='unknown'] {
    color: var(--sev-unknown);
  }
  .gauge-fill[data-severity='ok'] {
    background: var(--sev-ok);
  }
  .gauge-fill[data-severity='warn'] {
    background: var(--sev-warn);
  }
  .gauge-fill[data-severity='critical'] {
    background: var(--sev-critical);
  }
  .gauge-fill[data-severity='exhausted'] {
    background: var(--sev-exhausted);
  }
  .gauge-fill[data-severity='unknown'] {
    background: var(--sev-unknown);
  }
  time {
    color: var(--color-text-faint);
    font-family: var(--font-mono);
    font-size: 0.6875rem;
  }
</style>
