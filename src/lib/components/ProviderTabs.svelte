<script>
  /** @type {{ selected: import('../contracts/domain').Provider; primary?: import('../contracts/domain').Provider; onSelect?: (provider: import('../contracts/domain').Provider) => void }} */
  let { selected, primary = selected, onSelect = () => {} } = $props();
</script>

<div class="tabs" role="tablist" aria-label="Providers">
  <button
    role="tab"
    aria-selected={selected === 'claude'}
    aria-label={primary === 'claude' ? 'Claude (primary)' : 'Claude'}
    onclick={() => onSelect('claude')}
    >Claude{#if primary === 'claude'}<span aria-hidden="true">
        ★</span
      >{/if}</button
  >
  <button
    role="tab"
    aria-selected={selected === 'codex'}
    aria-label={primary === 'codex' ? 'Codex (primary)' : 'Codex'}
    onclick={() => onSelect('codex')}
    >Codex{#if primary === 'codex'}<span aria-hidden="true">
        ★</span
      >{/if}</button
  >
</div>

<style>
  .tabs {
    display: flex;
    border-bottom: 1px solid var(--color-border);
  }
  button {
    flex: 1;
    padding: 0.6rem 0;
    border: 0;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: var(--color-text-faint);
    font: inherit;
    font-weight: 500;
    cursor: pointer;
  }
  button[aria-selected='true'] {
    border-bottom-color: var(--color-text);
    color: var(--color-text);
    font-weight: 600;
  }
</style>
