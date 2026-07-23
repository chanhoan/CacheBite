<script>
  /** @type {{ system: import('./models').BadgeState }} */
  let { system } = $props();

  /** @type {Record<import('./models').BadgeState, { color: string; label: string }>} */
  const badges = {
    auth_required: {
      color: 'var(--badge-lock)',
      label: 'Authentication required',
    },
    unavailable: { color: 'var(--badge-slash)', label: 'Provider unavailable' },
    error: {
      color: 'var(--badge-error)',
      label: 'Usage unavailable due to an error',
    },
    offline: { color: 'var(--badge-offline)', label: 'Network offline' },
    loading: { color: 'var(--badge-loading)', label: 'Loading usage' },
  };
  const badge = $derived(badges[system]);
</script>

<span
  class:loading={system === 'loading'}
  class="badge"
  role="status"
  aria-label={badge.label}
  style:background={badge.color}
>
  {#if system === 'auth_required'}
    <svg aria-hidden="true" viewBox="0 0 24 24"
      ><rect
        x="5"
        y="10"
        width="14"
        height="10"
        rx="2"
        fill="currentColor"
      /><path d="M8 10V7a4 4 0 0 1 8 0v3" /></svg
    >
  {:else if system === 'unavailable'}
    <svg aria-hidden="true" viewBox="0 0 24 24"
      ><circle cx="12" cy="12" r="8" /><line
        x1="6.5"
        y1="6.5"
        x2="17.5"
        y2="17.5"
      /></svg
    >
  {:else if system === 'error'}
    <svg aria-hidden="true" viewBox="0 0 24 24"
      ><path d="M12 4 21 20H3Z" /><line
        x1="12"
        y1="10"
        x2="12"
        y2="14"
      /><circle class="filled" cx="12" cy="17" r="1.1" /></svg
    >
  {:else if system === 'offline'}
    <svg aria-hidden="true" viewBox="0 0 24 24"
      ><path d="M6 16a4 4 0 0 1 .5-8 5 5 0 0 1 9.5 1 3.5 3.5 0 0 1 0 7Z" /><line
        x1="5"
        y1="4"
        x2="19"
        y2="20"
      /></svg
    >
  {:else}
    <svg aria-hidden="true" viewBox="0 0 24 24"
      ><path d="M12 4a8 8 0 1 1-5.6 2.3" stroke-linecap="round" /></svg
    >
  {/if}
</span>

<style>
  .badge {
    display: grid;
    width: 2rem;
    height: 2rem;
    place-items: center;
    border: 2.5px solid var(--color-surface);
    border-radius: 50%;
    color: var(--badge-icon);
    box-shadow: 0 2px 6px rgb(0 0 0 / 20%);
  }
  svg {
    width: 1.125rem;
    height: 1.125rem;
    fill: none;
    stroke: currentColor;
    stroke-width: 2.3;
    stroke-linejoin: round;
  }
  svg rect,
  svg .filled {
    stroke: none;
  }
  .loading {
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .loading {
      animation: none;
    }
  }
</style>
