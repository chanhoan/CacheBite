<script>
  import PetAnimation from './PetAnimation.svelte';
  import ProviderLogo from './ProviderLogo.svelte';
  import SatelliteRing from './SatelliteRing.svelte';
  import SplitUsageRing from './SplitUsageRing.svelte';
  import SystemBadge from './SystemBadge.svelte';
  import {
    orbitPath,
    satelliteOrbitPath,
    SATELLITE_CENTER,
    SATELLITE_READOUT_RATIO,
    SATELLITE_SIZE,
    SATELLITE_WALK_DURATION_S,
    SATELLITE_WALKER_SIZE,
    WALK_DURATION_S,
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
  // is a square of `model.size`, so both loops are rebuilt whenever the size
  // changes. Neither depends on the ring mode: each circle is its own object,
  // and the satellite simply adds a second mark rather than reshaping the first.
  const walkPath = $derived(orbitPath(model.size));
  const satelliteWalkPath = $derived(satelliteOrbitPath(model.size));
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
  style:--satellite-center-x={`${SATELLITE_CENTER.x}%`}
  style:--satellite-center-y={`${SATELLITE_CENTER.y}%`}
  style:--satellite-readout-size={`${(model.size * SATELLITE_SIZE * SATELLITE_READOUT_RATIO) / 100}px`}
  style:--walk-duration={`${WALK_DURATION_S}s`}
  style:--satellite-walk-duration={`${SATELLITE_WALK_DURATION_S.toFixed(3)}s`}
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
    <!-- The secondary's mark, walking the satellite. A direct child of the
         overlay, not of `.satellite`: `offset-path` is absolute pixels against
         this box, so nesting it inside a 36%-wide, translated element would put
         it somewhere else entirely.

         Rendered whatever the satellite's system state — identity matters most
         when a provider is the one that failed.

         `aria-hidden` on purpose: the satellite's own ring label already says
         "Codex usage: …", so naming this would read the same provider a third
         time. The big mark below is named because the big ring's label is
         generic and nothing else says which provider drives the overlay. -->
    <div
      class="satellite-walker"
      class:reverse={model.orbit.direction === 'reverse'}
      data-testid="satellite-orbit-walker"
      data-direction={model.orbit.direction}
      aria-hidden="true"
      style:offset-path={`path("${satelliteWalkPath}")`}
      style:width={`${SATELLITE_WALKER_SIZE}%`}
    >
      <ProviderLogo provider={model.satellite.provider} />
    </div>
  {/if}
  <!-- Walks the outside of the big ring, in both modes. `offset-rotate: auto`
       turns the mark with the path, so its feet stay on the circle and it goes
       upside down along the bottom, the way a walker on a globe looks from
       outside. -->
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
  /* Layer 1 is the near end of the cone — the big ring and everything that
     belongs to it. The satellite's whole group stays on layer 0 behind it; see
     the depth note further down. */
  .overlay > :global(.ring) {
    position: absolute;
    z-index: 1;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  .pet {
    position: absolute;
    z-index: 1;
    inset: 16%;
  }
  .badge-position {
    position: absolute;
    z-index: 1;
    right: 2%;
    bottom: 2%;
  }
  .interaction-surface {
    position: absolute;
    z-index: 2;
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
     chip exactly. Both read the same custom properties, set from `orbitPath`,
     including the `translate` that turns a centre into a box. */
  .satellite-surface {
    position: absolute;
    z-index: 2;
    left: var(--satellite-center-x);
    top: var(--satellite-center-y);
    width: var(--satellite-size);
    height: var(--satellite-size);
    border-radius: 50%;
    transform: translate(-50%, -50%);
    clip-path: circle(50% at 50% 50%);
    cursor: grab;
    touch-action: none;
  }
  .satellite-surface:active {
    cursor: grabbing;
  }
  /* Both marks are decorative duplicates of information the panel already
     carries; neither may steal the drag surface it is walking over. */
  .walker,
  .satellite-walker {
    position: absolute;
    top: 0;
    left: 0;
    aspect-ratio: 1;
    pointer-events: none;
    offset-anchor: 50% 50%;
    offset-rotate: auto;
  }
  /* Depth order carries the perspective, and it applies to the whole of each
     group rather than to the marks alone. The big ring is the near end of the
     cone, so its mark passes in *front* of the satellite; the satellite is the
     far end, so its mark passes *behind* the big ring. Lifting the satellite's
     mark above the big ring to stop the arc clipping it — the obvious thing to
     want — puts the far object in front of the near one and the recede stops
     reading. It costs almost nothing to do it properly: the mark's centre line
     never comes closer than 47.5 to the overlay centre against the arc's outer
     edge of 45.25, so only about a quarter of its body tucks behind the stroke,
     and it never reaches the pet at all (its nearest x is 87.5, the pet's box
     ends at 84). */
  .walker {
    z-index: 3;
    animation: walk-orbit var(--walk-duration) linear infinite;
  }
  .satellite-walker {
    z-index: 0;
    animation: walk-orbit var(--satellite-walk-duration) linear infinite;
  }
  .walker.reverse,
  .satellite-walker.reverse {
    animation-direction: reverse;
  }
  @keyframes walk-orbit {
    to {
      offset-distance: 100%;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .walker,
    .satellite-walker {
      animation: none;
    }
  }
  /* Motion paths landed in WebKit 16 / WebKitGTK 2.38, and Linux is a supported
     target. Without `offset-path` the marks do not merely stop moving — they
     stay at the `top: 0; left: 0` they were positioned at, i.e. two logos
     stacked in the overlay's corner well outside both rings. They duplicate
     what the panel already shows, so dropping them is the honest degradation. */
  @supports not (offset-path: path('M 0 0')) {
    .walker,
    .satellite-walker {
      display: none;
    }
  }
</style>
