<script lang="ts">
  import {
    buildGraphSegments,
    sampleGraphSegments,
    type GraphPoint,
    type HistorySample,
    type HistoryWindow,
    VISUAL_POINT_CAP,
  } from './historyGraph';

  type WindowName = HistoryWindow;
  // The tablist had no tabpanel to point at, leaving the ARIA pattern half
  // built. Only one panel exists, so a module constant is enough.
  const PANEL_ID = 'history-graph-panel';
  const SESSION_TAB_ID = 'history-graph-tab-session';
  const WEEKLY_TAB_ID = 'history-graph-tab-weekly';
  let {
    samples,
    window,
    onWindowChange = () => {},
  }: {
    samples: readonly HistorySample[];
    window: WindowName;
    onWindowChange?: (value: WindowName) => void;
  } = $props();
  const label = $derived(window === 'session' ? '5-hour' : 'Weekly');
  const segments = $derived(buildGraphSegments(samples, window));
  const sampledSegments = $derived(
    sampleGraphSegments(segments, VISUAL_POINT_CAP),
  );
  const points = $derived(sampledSegments.flatMap((segment) => segment.points));
  const rawPointCount = $derived(
    segments.reduce((total, segment) => total + segment.points.length, 0),
  );
  const isSampled = $derived(rawPointCount > points.length);
  const path = (segment: readonly GraphPoint[]) =>
    segment
      .map((point, index) => `${index ? 'L' : 'M'} ${point.x} ${point.y}`)
      .join(' ');
  const choose = (value: WindowName) => onWindowChange(value);
  let sessionTab: HTMLButtonElement;
  let weeklyTab: HTMLButtonElement;
  const navigate = (event: KeyboardEvent, value: WindowName) => {
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
    event.preventDefault();
    const next =
      event.key === 'Home'
        ? 'session'
        : event.key === 'End'
          ? 'weekly'
          : value === 'session'
            ? 'weekly'
            : 'session';
    (next === 'session' ? sessionTab : weeklyTab).focus();
    choose(next);
  };
</script>

<section class="history" aria-label="Usage history">
  <div role="tablist" aria-label="History window">
    <button
      bind:this={sessionTab}
      id={SESSION_TAB_ID}
      role="tab"
      aria-selected={window === 'session'}
      aria-controls={PANEL_ID}
      tabindex={window === 'session' ? 0 : -1}
      onclick={() => choose('session')}
      onkeydown={(event) => navigate(event, 'session')}>5-hour</button
    >
    <button
      bind:this={weeklyTab}
      id={WEEKLY_TAB_ID}
      role="tab"
      aria-selected={window === 'weekly'}
      aria-controls={PANEL_ID}
      tabindex={window === 'weekly' ? 0 : -1}
      onclick={() => choose('weekly')}
      onkeydown={(event) => navigate(event, 'weekly')}>Weekly</button
    >
  </div>
  <!-- The panel holds no focusable content, so it takes focus itself (APG
       tabs pattern) and borrows its name from the active tab rather than
       repeating the label the inner <svg> already announces. -->
  <div
    id={PANEL_ID}
    role="tabpanel"
    tabindex="0"
    aria-labelledby={window === 'session' ? SESSION_TAB_ID : WEEKLY_TAB_ID}
  >
    {#if rawPointCount === 0}
      <p>No {label.toLowerCase()} history yet</p>
    {:else}
      <svg
        role="img"
        aria-label={`${label} usage history`}
        viewBox="0 0 100 100"
      >
        {#if isSampled}
          <desc
            >Showing a sampled view of {rawPointCount.toLocaleString()} points.</desc
          >
        {/if}
        {#each sampledSegments as segment (segment.points[0]?.key)}
          <path
            data-testid="history-segment"
            d={path(segment.points)}
            fill="none"
            stroke="var(--color-accent)"
          />
        {/each}
        <!-- Up to VISUAL_POINT_CAP marks. Exposing each as its own image role
             floods the accessibility tree; the <svg> label and <desc> already
             represent the series. -->
        {#each points as point (point.key)}
          <circle
            cx={point.x}
            cy={point.y}
            r="2"
            fill="var(--color-accent)"
            aria-hidden="true"
          />
        {/each}
      </svg>
    {/if}
  </div>
</section>

<style>
  .history {
    display: grid;
    gap: var(--space-3);
    width: 100%;
    padding: var(--space-4);
    border-top: 1px solid var(--color-border);
    color: var(--color-text-muted);
  }
  [role='tablist'] {
    display: flex;
    gap: var(--space-1);
  }
  [role='tabpanel']:focus-visible {
    border-radius: 0.35rem;
    outline: 2px solid var(--color-accent);
    outline-offset: 2px;
  }
  button {
    border: 0;
    border-radius: 0.35rem;
    background: transparent;
    color: var(--color-text-faint);
    font: inherit;
  }
  button[aria-selected='true'] {
    background: var(--color-surface-sunken);
    color: var(--color-text);
  }
  svg {
    width: 100%;
    max-height: 8rem;
  }
  path {
    stroke-width: 2;
    vector-effect: non-scaling-stroke;
  }
</style>
