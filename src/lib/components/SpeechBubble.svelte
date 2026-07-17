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
