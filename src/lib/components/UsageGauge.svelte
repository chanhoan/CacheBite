<script>
  /** @type {{ label: string; window: { usedPercent: number | null; severity: string; resetsAt: string | null }; stale?: boolean }} */
  let { label, window: usage, stale = false } = $props();
  const percent = $derived(
    usage.usedPercent === null
      ? 0
      : Math.min(100, Math.max(0, usage.usedPercent)),
  );
</script>

<section class:stale aria-label={`${label} usage`} data-testid="usage-gauge">
  <span>{label}</span><progress max="100" value={percent}>{percent}%</progress>
  <strong
    >{usage.usedPercent === null
      ? 'Unknown'
      : `${Math.round(percent)}%`}</strong
  >
  {#if usage.resetsAt}<time datetime={usage.resetsAt}>{usage.resetsAt}</time
    >{/if}
</section>

<style>
  .stale progress {
    opacity: 0.45;
  }
</style>
