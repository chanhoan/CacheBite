<script>
  import { onMount } from 'svelte';

  /** @type {{ animation: import('../assets/resolver').ResolvedAnimation; label: string }} */
  let { animation, label } = $props();
  let frameIndex = $state(0);

  const source = $derived(
    animation.type === 'image'
      ? animation.source
      : animation.sources[frameIndex % animation.sources.length],
  );

  onMount(() => {
    if (animation.type !== 'frames' || animation.sources.length < 2) return;
    const timer = window.setInterval(() => {
      frameIndex = (frameIndex + 1) % animation.sources.length;
    }, animation.frameDurationMs);
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
