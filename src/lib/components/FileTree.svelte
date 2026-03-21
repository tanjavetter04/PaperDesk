<script lang="ts">
  let {
    files,
    selectedPath,
    onSelect,
  }: {
    files: string[];
    selectedPath: string | null;
    onSelect: (path: string) => void;
  } = $props();
</script>

<div class="tree">
  <div class="tree-head">Files</div>
  <ul>
    {#each files as f (f)}
      <li>
        <button
          type="button"
          class:sel={f === selectedPath}
          onclick={() => onSelect(f)}
        >
          {f}
        </button>
      </li>
    {/each}
  </ul>
</div>

<style>
  .tree {
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-right: 1px solid var(--pd-border);
    background: var(--pd-surface);
  }

  .tree-head {
    padding: 0.5rem 0.75rem;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--pd-muted);
    border-bottom: 1px solid var(--pd-border);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0.35rem 0;
    overflow: auto;
    flex: 1;
  }

  li button {
    display: block;
    width: 100%;
    text-align: left;
    padding: 0.35rem 0.75rem;
    border: none;
    background: transparent;
    color: var(--pd-text);
    font-size: 0.82rem;
    font-family: var(--pd-mono);
    cursor: pointer;
  }

  li button:hover {
    background: color-mix(in srgb, var(--pd-text) 6%, transparent);
  }

  li button.sel {
    background: color-mix(in srgb, var(--pd-accent) 22%, transparent);
    color: var(--pd-text);
  }
</style>
