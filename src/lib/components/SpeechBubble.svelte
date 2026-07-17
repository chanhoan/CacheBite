<script>
  import { onMount } from 'svelte';
  import { BUBBLE_DISMISS_MS } from '../interaction/bubblePolicy';
  /** @type {{ message: string; onDismiss?: () => void; onOpenPanel?: () => void }} */
  let { message, onDismiss = () => {}, onOpenPanel = () => {} } = $props();
  onMount(() => {
    const timer = window.setTimeout(onDismiss, BUBBLE_DISMISS_MS);
    return () => window.clearTimeout(timer);
  });
  const click = () => {
    onDismiss();
    onOpenPanel();
  };
</script>

<button class="bubble" aria-label={message} onclick={click}>{message}</button>

<style>
  .bubble {
    position: relative;
    max-width: 15rem;
    padding: 0.65rem 0.85rem;
    border: 1px solid var(--color-border);
    border-radius: 0.75rem;
    background: var(--color-surface);
    color: var(--color-text);
    box-shadow: var(--shadow-panel);
    font: inherit;
  }
  .bubble::after {
    position: absolute;
    right: 1.25rem;
    bottom: -0.4rem;
    width: 0.7rem;
    height: 0.7rem;
    border-right: 1px solid var(--color-border);
    border-bottom: 1px solid var(--color-border);
    background: var(--color-surface);
    content: '';
    transform: rotate(45deg);
  }
</style>
