<script lang="ts">
  type WindowName = 'session' | 'weekly';
  type Point = {
    readonly usedPercent: number;
    readonly startsNewSegment: boolean;
  };
  type Sample = {
    readonly capturedAt: string;
    readonly session: Point | null;
    readonly weekly: Point | null;
  };
  let {
    samples,
    window,
    onWindowChange = () => {},
  }: {
    samples: readonly Sample[];
    window: WindowName;
    onWindowChange?: (value: WindowName) => void;
  } = $props();
  const label = $derived(window === 'session' ? '5-hour' : 'Weekly');
  const points = $derived(
    samples.flatMap((sample, index) => {
      const point = sample[window];
      return point
        ? [
            {
              ...point,
              capturedAt: sample.capturedAt,
              x:
                samples.length === 1
                  ? 50
                  : 5 + (index * 90) / (samples.length - 1),
              y: 95 - point.usedPercent * 0.9,
            },
          ]
        : [];
    }),
  );
  const segments = $derived(
    points.reduce<(typeof points)[]>((all, point) => {
      if (all.length === 0 || point.startsNewSegment) return [...all, [point]];
      const prior = all.slice(0, -1);
      const current = all.at(-1) ?? [];
      return [...prior, [...current, point]];
    }, []),
  );
  const path = (segment: typeof points) =>
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

<section aria-label="Usage history">
  <div role="tablist" aria-label="History window">
    <button
      bind:this={sessionTab}
      role="tab"
      aria-selected={window === 'session'}
      tabindex={window === 'session' ? 0 : -1}
      onclick={() => choose('session')}
      onkeydown={(event) => navigate(event, 'session')}>5-hour</button
    >
    <button
      bind:this={weeklyTab}
      role="tab"
      aria-selected={window === 'weekly'}
      tabindex={window === 'weekly' ? 0 : -1}
      onclick={() => choose('weekly')}
      onkeydown={(event) => navigate(event, 'weekly')}>Weekly</button
    >
  </div>
  {#if points.length === 0}
    <p>No {label.toLowerCase()} history yet</p>
  {:else}
    <svg role="img" aria-label={`${label} usage history`} viewBox="0 0 100 100">
      {#each segments as segment (segment)}
        <path
          data-testid="history-segment"
          d={path(segment)}
          fill="none"
          stroke="currentColor"
        />
      {/each}
      {#each points as point (point.capturedAt)}
        <circle
          role="img"
          cx={point.x}
          cy={point.y}
          r="2"
          aria-label={`${point.usedPercent}% at ${point.capturedAt}`}
        />
      {/each}
    </svg>
  {/if}
</section>
