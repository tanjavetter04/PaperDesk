<script lang="ts">
  let { pdfUrl }: { pdfUrl: string | null } = $props();
</script>

<div class="preview">
  <div class="head">Preview</div>
  <div class="frame-wrap">
    {#if pdfUrl}
      <iframe class="frame" title="PDF preview" src={pdfUrl}></iframe>
    {:else}
      <p class="placeholder">Compile to see PDF preview</p>
    {/if}
  </div>
</div>

<style>
  .preview {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    border-left: 1px solid var(--pd-border);
    background: var(--pd-surface);
  }

  .head {
    padding: 0.5rem 0.75rem;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--pd-muted);
    border-bottom: 1px solid var(--pd-border);
  }

  .frame-wrap {
    flex: 1;
    min-height: 0;
    position: relative;
  }

  .frame {
    border: none;
    width: 100%;
    height: 100%;
    background: #525659;
  }

  .placeholder {
    margin: 2rem 1rem;
    color: var(--pd-muted);
    font-size: 0.9rem;
  }
</style>
