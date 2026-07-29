<script>
  // Dismissal timing lives with the policy that produced `expiresAt`, not here.
  // A mount-scoped timer fired once per component instance, so a replacement
  // bubble inherited the previous bubble's remaining time instead of a full
  // interval. See the bubble `$effect` in App.svelte.
  /** @type {{ message: string; onDismiss?: () => void }} */
  let { message, onDismiss = () => {} } = $props();
</script>

<button
  class="toast"
  data-testid="overlay-toast"
  aria-label={message}
  onclick={() => onDismiss()}
>
  <span class="toast-message">{message}</span>
</button>

<style>
  .toast {
    display: block;
    inline-size: fit-content;
    max-inline-size: calc(100vw - var(--space-4));
    padding: 0.3rem 0.65rem;
    border: 1px solid var(--color-border);
    border-radius: 0.625rem;
    background: var(--color-surface);
    color: var(--color-text);
    box-shadow: var(--shadow-panel);
    font: inherit;
    font-size: 0.625rem;
    line-height: 1.15;
    text-align: center;
    white-space: normal;
    cursor: pointer;
  }
  .toast-message {
    display: block;
    overflow-wrap: anywhere;
    text-wrap: pretty;
    word-break: keep-all;
  }
</style>
