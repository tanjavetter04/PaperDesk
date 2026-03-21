<script lang="ts">
  import * as pdfjs from "pdfjs-dist";
  import pdfjsWorker from "pdfjs-dist/build/pdf.worker.min.mjs?url";

  pdfjs.GlobalWorkerOptions.workerSrc = pdfjsWorker;

  let {
    pdfUrl,
    page = 1,
  }: {
    pdfUrl: string | null;
    /** 1-based page (pdf.js; native iframe `#page=` is unreliable in WebKitGTK). */
    page?: number;
  } = $props();

  let canvas = $state<HTMLCanvasElement | null>(null);
  let slot = $state<HTMLDivElement | null>(null);

  const safePage = $derived(Math.max(1, Math.floor(page)));
  let loadError = $state<string | null>(null);

  $effect(() => {
    const url = pdfUrl;
    const pg = safePage;
    const c = canvas;
    const wrap = slot;
    if (!url || !c || !wrap) {
      loadError = null;
      return;
    }

    let cancelled = false;
    loadError = null;

    const loadingTask = pdfjs.getDocument({ url });

    const run = async () => {
      try {
        const pdf = await loadingTask.promise;
        if (cancelled) {
          await pdf.destroy().catch(() => {});
          return;
        }

        const n = Math.min(pg, pdf.numPages);
        const pdfPage = await pdf.getPage(n);
        if (cancelled) {
          await pdf.destroy().catch(() => {});
          return;
        }

        const base = pdfPage.getViewport({ scale: 1 });
        const cssW = Math.max(wrap.clientWidth, 1);
        const cssScale = cssW / base.width;
        const dpr = typeof window !== "undefined" ? window.devicePixelRatio || 1 : 1;
        const viewport = pdfPage.getViewport({ scale: cssScale * dpr });

        c.width = Math.floor(viewport.width);
        c.height = Math.floor(viewport.height);
        c.style.width = `${cssW}px`;
        c.style.height = `${(base.height * cssScale).toFixed(2)}px`;

        const ctx = c.getContext("2d");
        if (!ctx) {
          await pdf.destroy().catch(() => {});
          return;
        }

        const renderTask = pdfPage.render({ canvasContext: ctx, viewport });
        await renderTask.promise;
        await pdf.destroy().catch(() => {});
      } catch (e) {
        if (!cancelled) {
          loadError = e instanceof Error ? e.message : String(e);
        }
      } finally {
        await loadingTask.destroy().catch(() => {});
      }
    };

    void run();

    return () => {
      cancelled = true;
      void loadingTask.destroy().catch(() => {});
    };
  });
</script>

<div class="preview">
  <div class="head">Preview</div>
  <div class="frame-wrap" bind:this={slot}>
    {#if pdfUrl}
      <canvas class="pdf-canvas" class:dim={!!loadError} bind:this={canvas}></canvas>
      {#if loadError}
        <p class="err-overlay">{loadError}</p>
      {/if}
    {:else}
      <p class="placeholder">Compile to see PDF preview</p>
    {/if}
  </div>
</div>

<style>
  .preview {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    min-height: 0;
    height: 100%;
    border-left: 1px solid var(--pd-border);
    background: var(--pd-surface);
  }

  .head {
    flex-shrink: 0;
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
    overflow: auto;
    background: #525659;
  }

  .pdf-canvas {
    display: block;
    margin: 0 auto;
    max-width: 100%;
    height: auto;
  }

  .pdf-canvas.dim {
    opacity: 0.25;
  }

  .err-overlay {
    position: absolute;
    left: 50%;
    top: 1rem;
    transform: translateX(-50%);
    max-width: 90%;
    margin: 0;
    padding: 0.65rem 0.85rem;
    border-radius: 6px;
    background: var(--pd-surface);
    border: 1px solid var(--pd-error);
    color: var(--pd-error);
    font-size: 0.82rem;
    z-index: 1;
  }

  .placeholder {
    margin: 2rem 1rem;
    color: var(--pd-muted);
    font-size: 0.9rem;
  }
</style>
