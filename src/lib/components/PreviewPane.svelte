<script lang="ts">
  import { tick, untrack } from "svelte";
  import { t } from "$lib/i18n/locale.svelte";
  import * as pdfjs from "pdfjs-dist";
  import type { PDFDocumentProxy } from "pdfjs-dist";
  import pdfjsWorker from "pdfjs-dist/build/pdf.worker.min.mjs?url";

  pdfjs.GlobalWorkerOptions.workerSrc = pdfjsWorker;

  let {
    previewUrl,
    pdfUrl = null,
    page = 1,
    error = null,
  }: {
    previewUrl: string | null;
    pdfUrl?: string | null;
    /** 1-based page for scroll sync (pdf.js). */
    page?: number;
    error?: string | null;
  } = $props();

  const showPdf = $derived(!previewUrl && !!pdfUrl);

  let slot = $state<HTMLDivElement | null>(null);
  let pagesRoot = $state<HTMLDivElement | null>(null);

  const safePage = $derived(Math.max(1, Math.floor(page)));
  let loadError = $state<string | null>(null);
  let layoutRev = $state(0);
  let renderSerial = $state(0);

  let loaded = $state<{ doc: PDFDocumentProxy; url: string } | null>(null);

  let renderedAtWidth = $state(0);

  /** rAF-throttled PDF re-layout on resize (avoid transform-scale, which breaks scrollIntoView). */
  let resizeRaf = 0;

  $effect(() => {
    const url = showPdf ? pdfUrl : null;
    if (!url) {
      loadError = null;
      renderedAtWidth = 0;
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
    if (!showPdf) return;

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
    if (!showPdf) return;

    const wrap = slot;
    const p = safePage;
    const _rs = renderSerial;
    const url = pdfUrl;
    if (!wrap || !url) return;

    void tick().then(() => {
      const el = wrap.querySelector(`[data-page="${p}"]`);
      if (!el) return;
      el.scrollIntoView({ block: "center", inline: "nearest", behavior: "auto" });
    });
  });

  $effect(() => {
    if (!showPdf) return;

    const wrap = slot;
    if (!wrap) return;

    const ro = new ResizeObserver(() => {
      cancelAnimationFrame(resizeRaf);
      resizeRaf = requestAnimationFrame(() => {
        resizeRaf = 0;
        const cw = Math.round(wrap.clientWidth);
        const rw = renderedAtWidth;
        if (cw > 0 && rw > 0 && Math.abs(cw - rw) > 2) {
          layoutRev += 1;
        }
      });
    });
    ro.observe(wrap);
    return () => {
      ro.disconnect();
      cancelAnimationFrame(resizeRaf);
    };
  });
</script>

<div class="preview">
  <div class="head">{t("previewPane.head")}</div>
  <div class="frame-wrap" class:frame-wrap--scroll={showPdf} bind:this={slot}>
    {#if previewUrl}
      <iframe
        class="preview-frame"
        src={previewUrl}
        title={t("previewPane.iframeTitle")}
        allow="fullscreen"
      ></iframe>
      {#if error}
        <p class="err-overlay">{error}</p>
      {/if}
    {:else if pdfUrl}
      <div class="scale-viewport">
        <div class="pages-stack" class:dim={!!loadError} bind:this={pagesRoot}></div>
      </div>
      {#if loadError}
        <p class="err-overlay">{loadError}</p>
      {/if}
    {:else}
      <p class="placeholder">{error ?? t("previewPane.startingPlaceholder")}</p>
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
    font-size: 1rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--pd-muted);
    border-bottom: 1px solid var(--pd-border);
  }

  .frame-wrap {
    flex: 1;
    min-height: 0;
    position: relative;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: #525659;
  }

  .frame-wrap--scroll {
    overflow: auto;
  }

  .preview-frame {
    flex: 1;
    min-height: 0;
    width: 100%;
    border: none;
    background: #525659;
  }

  .scale-viewport {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 100%;
    box-sizing: border-box;
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
    top: 0.9rem;
    transform: translateX(-50%);
    max-width: 90%;
    margin: 0;
    padding: 0.65rem 0.85rem;
    border-radius: 6px;
    background: var(--pd-surface);
    border: 1px solid var(--pd-error);
    color: var(--pd-error);
    font-size: 1rem;
    z-index: 1;
  }

  .placeholder {
    margin: 2rem 1rem;
    color: var(--pd-muted);
    font-size: 1rem;
  }
</style>
