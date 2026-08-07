<script>
  import ProviderLogo from './ProviderLogo.svelte';
  import SplitUsageRing from './SplitUsageRing.svelte';
  import SystemBadge from './SystemBadge.svelte';

  /** @type {{ model: import('./models').SatelliteRingModel }} */
  let { model } = $props();
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
  {/if}
  <div class="logo"><ProviderLogo provider={model.provider} /></div>
  {#if model.system !== 'active'}
    <div class="satellite-badge"><SystemBadge system={model.system} /></div>
  {/if}
</div>

<style>
  /* 40% of the big ring, pinned lower-right and overlapping it by about a
     third. The negative offsets deliberately push it outside the overlay box —
     the overlay window has room, and nothing clips. */
  .satellite {
    position: absolute;
    right: var(--satellite-right);
    bottom: var(--satellite-bottom);
    width: var(--satellite-size);
    height: var(--satellite-size);
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
     anything. */
  .satellite :global(.ring-label) {
    display: none;
  }
  .logo {
    position: absolute;
    inset: 26%;
  }
  .satellite-badge {
    position: absolute;
    right: 0;
    bottom: 0;
  }
</style>
