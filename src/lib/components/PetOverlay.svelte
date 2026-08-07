<script>
  import PetAnimation from './PetAnimation.svelte';
  import ProviderLogo from './ProviderLogo.svelte';
  import SatelliteRing from './SatelliteRing.svelte';
  import SplitUsageRing from './SplitUsageRing.svelte';
  import SystemBadge from './SystemBadge.svelte';
  import {
    orbitPath,
    SATELLITE_BOTTOM,
    SATELLITE_RIGHT,
    SATELLITE_SIZE,
    WALKER_SIZE,
  } from './orbitPath';

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
  // `offset-path` measures in pixels against the overlay box, and the overlay
  // is a square of `model.size`, so the loop is rebuilt whenever either the
  // size or the presence of a satellite changes.
  const walkPath = $derived(orbitPath(model.size, model.satellite !== null));
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
  style:--satellite-size={`${SATELLITE_SIZE}%`}
  style:--satellite-right={`${SATELLITE_RIGHT}%`}
  style:--satellite-bottom={`${SATELLITE_BOTTOM}%`}
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
  {#if model.satellite}
    <SatelliteRing model={model.satellite} />
    <!-- The chip sits outside the circular drag surface above, so without this
         it would be a dead zone in the middle of the pet's own gestures. It is
         a redundant hit target for an already-labelled control, so it stays out
         of the accessibility tree: a second element with the same name would
         make `getByRole('button', { name })` ambiguous, and it cannot take
         focus, so the keyboard path stays on the main surface. -->
    <div
      class="satellite-surface"
      data-testid="overlay-satellite-pointer-surface"
      aria-hidden="true"
      onpointerdown={onPointerDown}
      onpointermove={onPointerMove}
      onpointerup={onPointerUp}
      onpointercancel={onPointerCancel}
      ondblclick={() => onToggle()}
      oncontextmenu={contextMenu}
    ></div>
  {/if}
  <!-- Walks the outside of the ring — the union outline once a satellite is
       there. `offset-rotate: auto` turns the mark with the path, so its feet
       stay on whichever circle it is currently rounding and it goes upside
       down along the bottom, the way a walker on a globe looks from outside. -->
  <div
    class="walker"
    class:reverse={model.orbit.direction === 'reverse'}
    data-testid="orbit-walker"
    data-direction={model.orbit.direction}
    role="img"
    aria-label={`Primary provider: ${model.orbit.providerName}`}
    style:offset-path={`path("${walkPath}")`}
    style:width={`${WALKER_SIZE}%`}
  >
    <ProviderLogo provider={model.orbit.provider} />
  </div>
</section>

<style>
  .overlay {
    position: relative;
    /* Width comes from the manifest via `model.size`; the square ratio and the
       max-width keep an oversized package inside the overlay window. */
    max-width: 100%;
    aspect-ratio: 1;
  }
  /* Child combinator, not descendant: the satellite's ring is one level deeper
     and owns its own geometry. A descendant selector here matches that one too,
     at equal specificity, so whichever rule the bundler happens to emit last
     would win — and `SatelliteRing`'s geometry would become dead code. */
  .overlay > :global(.ring) {
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
  /* Mirrors `.satellite` in SatelliteRing.svelte so the hit area tracks the
     chip exactly. Both read the same custom properties, set from `orbitPath`. */
  .satellite-surface {
    position: absolute;
    z-index: 1;
    right: var(--satellite-right);
    bottom: var(--satellite-bottom);
    width: var(--satellite-size);
    height: var(--satellite-size);
    border-radius: 50%;
    clip-path: circle(50% at 50% 50%);
    cursor: grab;
    touch-action: none;
  }
  .satellite-surface:active {
    cursor: grabbing;
  }
  .walker {
    position: absolute;
    z-index: 2;
    top: 0;
    left: 0;
    aspect-ratio: 1;
    /* Decorative duplicate of information the panel already carries; it must
       never steal the drag surface it is walking over. */
    pointer-events: none;
    offset-anchor: 50% 50%;
    offset-rotate: auto;
    animation: walk-orbit 26s linear infinite;
  }
  .walker.reverse {
    animation-direction: reverse;
  }
  @keyframes walk-orbit {
    to {
      offset-distance: 100%;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .walker {
      animation: none;
    }
  }
  /* Motion paths landed in WebKit 16 / WebKitGTK 2.38, and Linux is a supported
     target. Without `offset-path` the mark does not merely stop moving — it
     stays at the `top: 0; left: 0` it was positioned at, i.e. a logo stranded in
     the overlay's corner well outside the ring. It is a decorative duplicate of
     what the panel already shows, so dropping it is the honest degradation. */
  @supports not (offset-path: path('M 0 0')) {
    .walker {
      display: none;
    }
  }
</style>
