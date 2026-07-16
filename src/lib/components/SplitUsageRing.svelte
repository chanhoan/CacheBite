<script>
  /** @type {{ session: import('./models').RingWindowModel; weekly: import('./models').RingWindowModel; stale: boolean }} */
  let { session, weekly, stale } = $props();

  /** @param {import('./models').RingWindowModel} window */
  const percent = (window) =>
    window.usedPercent === null || !Number.isFinite(window.usedPercent)
      ? 0
      : Math.min(100, Math.max(0, window.usedPercent));
  /** @param {string} name @param {import('./models').RingWindowModel} window */
  const label = (name, window) =>
    `${name} usage: ${window.severity === 'unknown' ? 'unknown' : `${Math.round(percent(window))}%`}`;
</script>

<svg
  data-testid="usage-ring"
  data-stale={stale}
  class:stale
  class="ring"
  viewBox="0 0 100 100"
  aria-label="Provider usage"
>
  <path class="track" d="M 8 50 A 42 42 0 0 1 92 50" pathLength="100" />
  <path
    class="usage"
    data-severity={session.severity}
    d="M 8 50 A 42 42 0 0 1 92 50"
    pathLength="100"
    stroke-dasharray={`${percent(session)} 100`}
    aria-label={label('5-hour', session)}
  />
  <path class="track" d="M 92 50 A 42 42 0 0 1 8 50" pathLength="100" />
  <path
    class="usage"
    data-severity={weekly.severity}
    d="M 92 50 A 42 42 0 0 1 8 50"
    pathLength="100"
    stroke-dasharray={`${percent(weekly)} 100`}
    aria-label={label('Weekly', weekly)}
  />
</svg>

<style>
  .ring {
    overflow: visible;
    fill: none;
    stroke-linecap: round;
  }
  .ring.stale {
    opacity: var(--overlay-stale-dim, 0.45);
  }
  path {
    stroke-width: 6;
  }
  .track {
    stroke: var(--color-status-unknown, #777);
    opacity: 0.28;
  }
  .usage {
    stroke: var(--color-status-unknown, #777);
  }
  .usage[data-severity='ok'] {
    stroke: var(--color-status-ok);
  }
  .usage[data-severity='warn'] {
    stroke: var(--color-status-warning);
  }
  .usage[data-severity='critical'] {
    stroke: var(--color-status-critical);
  }
  .usage[data-severity='exhausted'] {
    stroke: var(--color-status-error);
  }
</style>
