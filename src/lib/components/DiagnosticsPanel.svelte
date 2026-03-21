<script lang="ts">
  import type { CompileDiagnostic } from "$lib/tauri/api";

  let {
    diagnostics,
    onJumpTo,
  }: {
    diagnostics: CompileDiagnostic[];
    /** Focus the editor at this diagnostic (same file only). */
    onJumpTo?: (d: CompileDiagnostic) => void;
  } = $props();
</script>

<div class="panel">
  <div class="head">Diagnostics</div>
  <div class="body">
    {#if diagnostics.length === 0}
      <p class="empty">No issues</p>
    {:else}
      <ul>
        {#each diagnostics as d, i (i)}
          <li class={d.severity}>
            <button
              type="button"
              class="diag-hit"
              disabled={!onJumpTo}
              onclick={() => onJumpTo?.(d)}
            >
              {#if d.path}
                <span class="loc">{d.path}{#if d.line != null}:{d.line}{/if}{#if d.column != null}:{d.column}{/if}</span>
              {/if}
              <span class="msg">{d.message}</span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    border-top: 1px solid var(--pd-border);
    min-height: 7rem;
    max-height: 10rem;
    background: var(--pd-bg);
  }

  .head {
    padding: 0.35rem 0.65rem;
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--pd-muted);
    border-bottom: 1px solid var(--pd-border);
  }

  .body {
    overflow: auto;
    flex: 1;
    padding: 0.35rem 0.5rem;
  }

  .empty {
    margin: 0.35rem 0.25rem;
    color: var(--pd-muted);
    font-size: 0.85rem;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  li {
    font-size: 0.8rem;
    margin-bottom: 0.45rem;
    padding: 0;
    border-radius: 6px;
    background: var(--pd-surface);
    overflow: hidden;
  }

  .diag-hit {
    display: block;
    width: 100%;
    margin: 0;
    padding: 0.35rem 0.45rem;
    border: none;
    background: transparent;
    color: inherit;
    text-align: left;
    cursor: pointer;
    font: inherit;
    border-radius: inherit;
  }

  .diag-hit:hover:not(:disabled) {
    background: color-mix(in srgb, var(--pd-accent) 8%, transparent);
  }

  .diag-hit:disabled {
    cursor: default;
  }

  li.error {
    border-left: 3px solid var(--pd-error);
  }

  li.warning {
    border-left: 3px solid var(--pd-warning);
  }

  .loc {
    display: block;
    font-family: var(--pd-mono);
    color: var(--pd-muted);
    font-size: 0.75rem;
    margin-bottom: 0.2rem;
  }

  .msg {
    white-space: pre-wrap;
  }
</style>
