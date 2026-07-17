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
  <text class="ring-label" x="50" y="4" text-anchor="middle" aria-hidden="true"
    >5H</text
  >
  <text class="ring-label" x="50" y="99" text-anchor="middle" aria-hidden="true"
    >WK</text
  >
</svg>

<style>
  .ring {
    overflow: visible;
    fill: none;
    stroke-linecap: round;
  }
  .ring.stale {
    opacity: var(--overlay-stale-dim);
  }
  path {
    stroke-width: 6.5;
  }
  .track {
    stroke: var(--sev-unknown);
    opacity: 0.28;
  }
  .usage {
    stroke: var(--sev-unknown);
  }
  .usage[data-severity='ok'] {
    stroke: var(--sev-ok);
  }
  .usage[data-severity='warn'] {
    stroke: var(--sev-warn);
  }
  .usage[data-severity='critical'] {
    stroke: var(--sev-critical);
  }
  .usage[data-severity='exhausted'] {
    stroke: var(--sev-exhausted);
  }
  .ring-label {
    fill: var(--color-text-faint);
    stroke: none;
    font-family: var(--font-mono);
    font-size: 4px;
    font-weight: 600;
    letter-spacing: 0.08em;
  }
</style>
