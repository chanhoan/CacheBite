<script>
  import SplitUsageRing from './SplitUsageRing.svelte';
  import SystemBadge from './SystemBadge.svelte';
  import { satelliteReading } from './ringReadout';

  /** @type {{ model: import('./models').SatelliteRingModel }} */
  let { model } = $props();

  const reading = $derived(satelliteReading(model));
  // An em dash, not `0` — reached only when neither window reported anything.
  // The ring is already drawing empty tracks in that case, and a `0` in the
  // middle of them would read as "nothing used yet" rather than "no reading".
  const readingText = $derived(
    reading.percent === null ? '—' : String(Math.round(reading.percent)),
  );
</script>

<div class="satellite" data-testid="satellite-ring">
  <div class="puck"></div>
  <!-- Same active/badge split as the big ring (`PetOverlay`): one provider
       failing must never blank the other. -->
  {#if model.system === 'active'}
    <SplitUsageRing
      session={model.session}
      weekly={model.weekly}
      stale={model.stale}
      name={model.providerName}
    />
    <!-- The 5-hour window, falling back to the weekly one only when 5H has no
         reading at all — see `satelliteReading`. Never "whichever is worse":
         at this size the `5H`/`WK` labels are hidden, so a number that switched
         on severity would leave no way to tell which limit it meant, while one
         that switches only on absence is disambiguated by the empty arc beside
         it. The ring's own aria-label already carries the figure, so this is
         `aria-hidden`: the visual duplicate, not a second fact. -->
    <div
      class="readout"
      class:stale={model.stale}
      data-testid="satellite-readout"
      data-severity={reading.severity}
      data-window={reading.window}
      aria-hidden="true"
    >
      {readingText}
    </div>
  {:else}
    <div class="satellite-badge"><SystemBadge system={model.system} /></div>
  {/if}
</div>

<style>
  /* 36% of the big ring, centred on the horizontal axis a full
     `SATELLITE_DISTANCE` to the right — far enough that the two circles no
     longer touch, and placed so their outer common tangents converge to the
     right. Both coordinates come from `orbitPath.ts` through custom properties;
     the numbers must not be repeated here or the walkers would orbit a circle
     the ring no longer occupies. Positioning by centre rather than by edge
     offsets is what lets the ring and its hit surface share one source. */
  .satellite {
    position: absolute;
    left: var(--satellite-center-x);
    top: var(--satellite-center-y);
    width: var(--satellite-size);
    height: var(--satellite-size);
    transform: translate(-50%, -50%);
  }
  .puck {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background: var(--satellite-puck);
    box-shadow: 0 3px 8px rgb(20 25 30 / 22%);
  }
  /* Geometry only. Stroke weight, track opacity and severity colours are
     inherited from SplitUsageRing untouched, so both rings read identically.

     The ring fills the puck: its arcs already stop at 45.25% of the box (radius
     42 plus half the 6.5 stroke), leaving the puck to read as a thin rim the way
     the v2 mock draws it. Insetting it further would open a gap the design does
     not have. `PetOverlay` scopes its own `.ring` rule to a direct child, so it
     cannot reach in here and silently override this. */
  .satellite :global(.ring) {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  /* The one exception: 5H / WK render at about 4px here, and they sit outside
     the viewBox, so they would spill over the big ring rather than label
     anything. Hiding them is what forces the readout to mean a fixed window. */
  .satellite :global(.ring-label) {
    display: none;
  }
  /* Sized in pixels from `model.size`, not in `%` or `em`: a percentage
     font-size resolves against the inherited font size, which knows nothing
     about how wide the overlay was clamped to. */
  .readout {
    position: absolute;
    display: grid;
    inset: 0;
    place-items: center;
    color: var(--sev-unknown);
    /* `ui-rounded` resolves to SF Pro Rounded on macOS without shipping a font
       file — Apple's licence does not cover redistribution on Windows or Linux,
       which both fall through to the mono stack. Tabular figures keep the glyph
       advance identical across all three so the number does not shift width as
       the value changes. */
    font-family: ui-rounded, var(--font-mono);
    font-size: var(--satellite-readout-size);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.02em;
    line-height: 1;
  }
  /* The number and the upper arc report the same window, so they carry the same
     colour. A neutral readout over a coloured arc would read as two facts. */
  .readout[data-severity='ok'] {
    color: var(--sev-ok);
  }
  .readout[data-severity='warn'] {
    color: var(--sev-warn);
  }
  .readout[data-severity='critical'] {
    color: var(--sev-critical);
  }
  .readout[data-severity='exhausted'] {
    color: var(--sev-exhausted);
  }
  .readout.stale {
    opacity: var(--overlay-stale-dim);
  }
  /* Centred on exactly the disc the readout occupies, because it is the same
     fact in another form: the puck's middle says what the provider is doing,
     and swapping a number for a badge must not move that statement. Anchored to
     a corner it landed outside the circle the puck actually draws, so it read as
     something stuck onto the satellite rather than as the satellite's own state.

     The size comes from `orbitPath` through `PetOverlay` for the same reason the
     readout's does: `SystemBadge`'s own 2rem chip knows nothing about how wide
     the overlay was clamped to, and at 36% of the box it would nearly fill the
     puck. */
  .satellite-badge {
    --badge-size: var(--satellite-badge-size);

    position: absolute;
    display: grid;
    inset: 0;
    place-items: center;
  }
</style>
