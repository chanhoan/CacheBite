<script>
  /** @type {{ system: import('./models').BadgeState }} */
  let { system } = $props();

  /** @type {Record<import('./models').BadgeState, { icon: string; label: string }>} */
  const badges = {
    auth_required: { icon: '🔒', label: 'Authentication required' },
    unavailable: { icon: '⊘', label: 'Provider unavailable' },
    error: { icon: '⚠', label: 'Usage unavailable due to an error' },
    offline: { icon: '☁', label: 'Network offline' },
    loading: { icon: '◌', label: 'Loading usage' },
  };
  const badge = $derived(badges[system]);
</script>

<span
  class:loading={system === 'loading'}
  class="badge"
  role="status"
  aria-label={badge.label}
>
  <span aria-hidden="true">{badge.icon}</span>
</span>

<style>
  .badge {
    display: grid;
    width: 2rem;
    height: 2rem;
    place-items: center;
    border: 2px solid var(--color-text);
    border-radius: 50%;
    background: var(--color-surface-raised);
    color: var(--color-text);
    box-shadow: var(--shadow-panel);
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
