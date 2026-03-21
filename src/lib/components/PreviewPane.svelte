<script lang="ts">
  import { tick, untrack } from "svelte";
  import * as pdfjs from "pdfjs-dist";
  import type { PDFDocumentProxy } from "pdfjs-dist";
  import pdfjsWorker from "pdfjs-dist/build/pdf.worker.min.mjs?url";

  pdfjs.GlobalWorkerOptions.workerSrc = pdfjsWorker;

  let {
    pdfUrl,
    page = 1,
  }: {
    pdfUrl: string | null;
    /** 1-based page for scroll sync (pdf.js). */
    page?: number;
  } = $props();

  let slot = $state<HTMLDivElement | null>(null);
  let pagesRoot = $state<HTMLDivElement | null>(null);

  const safePage = $derived(Math.max(1, Math.floor(page)));
  let loadError = $state<string | null>(null);
  /** Full pdf.js re-render generation (URL, settle after resize, etc.). */
  let layoutRev = $state(0);
  /** Bumped after a full paint of all page canvases (for scroll sync). */
  let renderSerial = $state(0);

  let loaded = $state<{ doc: PDFDocumentProxy; url: string } | null>(null);

  /** Last CSS width used when canvases were rendered (pdf.js). */
  let renderedAtWidth = $state(0);
  /** Live width of the scroll viewport (ResizeObserver). */
  let frameClientWidth = $state(0);

  const displayScale = $derived.by(() => {
    const rw = renderedAtWidth;
    const cw = frameClientWidth;
    if (rw <= 0 || cw <= 0) return 1;
    return Math.min(4, Math.max(0.05, cw / rw));
  });

  const scaledContentHeight = $derived.by(() => {
    void renderSerial;
    void displayScale;
    const root = pagesRoot;
    if (!root) return 0;
    return Math.max(0, root.offsetHeight * displayScale);
  });

  /** For debounced settle callback (non-reactive read). */
  const renderedWidthRef = { current: 0 };
  $effect(() => {
    renderedWidthRef.current = renderedAtWidth;
  });

  $effect(() => {
    const url = pdfUrl;
    if (!url) {
      loadError = null;
      renderedAtWidth = 0;
      frameClientWidth = 0;
      const cur = untrack(() => loaded);
      if (cur) {
        loaded = null;
        void cur.doc.destroy().catch(() => {});
      }
      return;
    }

    let cancelled = false;
    let loadHandled = false;
    loadError = null;
    const loadingTask = pdfjs.getDocument({ url });

    const run = async () => {
      try {
        const pdf = await loadingTask.promise;
        if (cancelled) {
          await pdf.destroy().catch(() => {});
          loadHandled = true;
          return;
        }
        const prev = untrack(() => loaded);
        if (prev && prev.url !== url) {
          await prev.doc.destroy().catch(() => {});
        }
        loaded = { doc: pdf, url };
        loadHandled = true;
      } catch (e) {
        if (!cancelled) {
          loadError = e instanceof Error ? e.message : String(e);
          const cur = untrack(() => loaded);
          loaded = null;
          if (cur) await cur.doc.destroy().catch(() => {});
        }
        loadHandled = true;
        await loadingTask.destroy().catch(() => {});
      }
    };

    void run();

    return () => {
      cancelled = true;
      if (!loadHandled) {
        void loadingTask.destroy().catch(() => {});
      }
    };
  });

  $effect(() => {
    const docEntry = loaded;
    const wrap = slot;
    const root = pagesRoot;
    const _layout = layoutRev;
    if (!docEntry || !wrap || !root) {
      return;
    }

    const pdf = docEntry.doc;
    let cancelled = false;

    const run = async () => {
      await tick();
      root.replaceChildren();
      let cssW = Math.max(wrap.clientWidth, 1);
      if (cssW < 8) {
        await new Promise<void>((r) => requestAnimationFrame(() => r()));
        cssW = Math.max(wrap.clientWidth, 1);
      }
      const dpr = typeof window !== "undefined" ? window.devicePixelRatio || 1 : 1;

      try {
        for (let n = 1; n <= pdf.numPages; n++) {
          if (cancelled) return;
          const pdfPage = await pdf.getPage(n);
          if (cancelled) return;

          const base = pdfPage.getViewport({ scale: 1 });
          const cssScale = cssW / base.width;
          const viewport = pdfPage.getViewport({ scale: cssScale * dpr });

          const c = document.createElement("canvas");
          c.className = "pdf-canvas";
          c.dataset.page = String(n);
          c.width = Math.floor(viewport.width);
          c.height = Math.floor(viewport.height);
          c.style.width = `${cssW}px`;
          c.style.height = `${(base.height * cssScale).toFixed(2)}px`;

          const ctx = c.getContext("2d");
          if (!ctx) continue;

          const renderTask = pdfPage.render({ canvasContext: ctx, viewport });
          await renderTask.promise;
          if (cancelled) return;
          root.appendChild(c);
        }
        if (!cancelled) {
          renderedAtWidth = cssW;
          frameClientWidth = Math.round(wrap.clientWidth);
          renderSerial += 1;
        }
      } catch (e) {
        if (!cancelled) {
          loadError = e instanceof Error ? e.message : String(e);
        }
      }
    };

    void run();

    return () => {
      cancelled = true;
      root.replaceChildren();
    };
  });

  $effect(() => {
    const wrap = slot;
    const p = safePage;
    const _rs = renderSerial;
    const url = pdfUrl;
    if (!wrap || !url) return;

    void tick().then(() => {
      const el = wrap.querySelector(`[data-page="${p}"]`);
      el?.scrollIntoView({ block: "nearest", behavior: "auto" });
    });
  });

  /** Live width + debounced sharp re-render after resize settles (no pdf.js during drag). */
  const RESIZE_SETTLE_MS = 280;
  $effect(() => {
    const wrap = slot;
    if (!wrap) return;
    let settleTimer = 0;

    const ro = new ResizeObserver(() => {
      frameClientWidth = Math.round(wrap.clientWidth);
      clearTimeout(settleTimer);
      settleTimer = window.setTimeout(() => {
        settleTimer = 0;
        const cw = Math.round(wrap.clientWidth);
        const rw = renderedWidthRef.current;
        if (rw > 0 && Math.abs(cw - rw) > 2) {
          layoutRev += 1;
        }
      }, RESIZE_SETTLE_MS);
    });
    ro.observe(wrap);
    frameClientWidth = Math.round(wrap.clientWidth);
    return () => {
      ro.disconnect();
      clearTimeout(settleTimer);
    };
  });
</script>

<div class="preview">
  <div class="head">Preview</div>
  <div class="frame-wrap" bind:this={slot}>
    {#if pdfUrl}
      <div
        class="scale-viewport"
        style:min-height="{scaledContentHeight > 0 ? `${scaledContentHeight}px` : undefined}"
      >
        <div
          class="scale-inner"
          style:width={renderedAtWidth > 0 ? `${renderedAtWidth}px` : "100%"}
          style:transform="scale({displayScale})"
        >
          <div class="pages-stack" class:dim={!!loadError} bind:this={pagesRoot}></div>
        </div>
      </div>
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

  .scale-viewport {
    display: flex;
    justify-content: center;
    width: 100%;
    box-sizing: border-box;
  }

  .scale-inner {
    transform-origin: top center;
    flex-shrink: 0;
    will-change: transform;
  }

  .pages-stack {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 8px 0 16px;
  }

  .pages-stack.dim {
    opacity: 0.25;
  }

  :global(.pages-stack .pdf-canvas) {
    display: block;
    margin: 0 auto;
    flex-shrink: 0;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.35);
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
