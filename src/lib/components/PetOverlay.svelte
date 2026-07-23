<script>
  import PetAnimation from './PetAnimation.svelte';
  import SplitUsageRing from './SplitUsageRing.svelte';
  import SystemBadge from './SystemBadge.svelte';

  /** @type {{ model: import('./models').PetOverlayViewModel }} */
  let { model } = $props();
</script>

<section
  class="overlay"
  aria-label="CacheBite pet status"
  style:width={`${model.size}px`}
>
  {#if model.system === 'active'}
    <SplitUsageRing
      session={model.session}
      weekly={model.weekly}
      stale={model.stale}
    />
  {/if}
  <div class="pet">
    <PetAnimation animation={model.animation} label={model.petName} />
  </div>
  {#if model.system !== 'active'}
    <div class="badge-position">
      <SystemBadge system={model.system} />
    </div>
  {/if}
</section>

<style>
  .overlay {
    position: relative;
    /* Width comes from the manifest via `model.size`; the square ratio and the
       max-width keep an oversized package inside the overlay window. */
    max-width: 100%;
    aspect-ratio: 1;
  }
  .overlay :global(.ring) {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  .pet {
    position: absolute;
    inset: 16%;
  }
  .badge-position {
    position: absolute;
    right: 2%;
    bottom: 2%;
  }
</style>
