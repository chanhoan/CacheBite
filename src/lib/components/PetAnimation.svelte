<script>
  /** @type {{ animation: import('../assets/resolver').ResolvedAnimation; label: string }} */
  let { animation, label } = $props();
  let frameIndex = $state(0);

  const source = $derived(
    animation.type === 'image'
      ? animation.source
      : animation.sources[frameIndex % animation.sources.length],
  );

  $effect(() => {
    frameIndex = 0;
    if (animation.type !== 'frames' || animation.sources.length < 2) return;
    const sources = animation.sources;
    const frameDurationMs = animation.frameDurationMs;
    const timer = window.setInterval(() => {
      frameIndex = (frameIndex + 1) % sources.length;
    }, frameDurationMs);
    return () => window.clearInterval(timer);
  });
</script>

<img class="pet-animation" src={source} alt={label} draggable="false" />

<style>
  .pet-animation {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: contain;
    user-select: none;
  }
</style>
