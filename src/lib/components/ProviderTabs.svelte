<script>
  /** @type {{ selected: import('../contracts/domain').Provider; onSelect?: (provider: import('../contracts/domain').Provider) => void }} */
  let { selected, onSelect = () => {} } = $props();
</script>

<div role="tablist" aria-label="Providers">
  <button
    role="tab"
    aria-selected={selected === 'claude'}
    onclick={() => onSelect('claude')}>Claude</button
  >
  <button
    role="tab"
    aria-selected={selected === 'codex'}
    onclick={() => onSelect('codex')}>Codex</button
  >
</div>
