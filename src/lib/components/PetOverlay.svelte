<script>
  import PetAnimation from './PetAnimation.svelte';
  import SplitUsageRing from './SplitUsageRing.svelte';
  import SystemBadge from './SystemBadge.svelte';

  /** @type {{ model: import('./models').PetOverlayViewModel; onPointerDown?: (event: PointerEvent) => void; onPointerMove?: (event: PointerEvent) => void; onPointerUp?: (event: PointerEvent) => void; onPointerCancel?: (event: PointerEvent) => void; onToggle?: () => void; onShowMenu?: () => void }} */
  let {
    model,
    onPointerDown = () => {},
    onPointerMove = () => {},
    onPointerUp = () => {},
    onPointerCancel = () => {},
    onToggle = () => {},
    onShowMenu = () => {},
  } = $props();
  /** @param {MouseEvent} event */
  const contextMenu = (event) => {
    // The webview's own context menu is never wanted over the pet; the native
    // popup the gateway requests replaces it.
    event.preventDefault();
    onShowMenu();
  };
  /** @param {KeyboardEvent} event */
  const keydown = (event) => {
    if (event.key !== 'Enter' && event.key !== ' ') return;
    event.preventDefault();
    onToggle();
  };
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
  <div
    class="interaction-surface"
    data-testid="overlay-pointer-surface"
    role="button"
    tabindex="0"
    aria-label="Move pet; double-click or press Enter to show or hide usage; right-click for the menu"
    style="clip-path: circle(50% at 50% 50%)"
    onpointerdown={onPointerDown}
    onpointermove={onPointerMove}
    onpointerup={onPointerUp}
    onpointercancel={onPointerCancel}
    ondblclick={() => onToggle()}
    oncontextmenu={contextMenu}
    onkeydown={keydown}
  ></div>
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
  .interaction-surface {
    position: absolute;
    z-index: 1;
    inset: 0;
    border-radius: 50%;
    clip-path: circle(50% at 50% 50%);
    cursor: grab;
    touch-action: none;
  }
  .interaction-surface:active {
    cursor: grabbing;
  }
</style>
