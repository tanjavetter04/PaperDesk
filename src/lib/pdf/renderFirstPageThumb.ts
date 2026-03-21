import * as pdfjs from "pdfjs-dist";
import pdfjsWorker from "pdfjs-dist/build/pdf.worker.min.mjs?url";

let workerConfigured = false;

function ensurePdfWorker(): void {
  if (!workerConfigured) {
    pdfjs.GlobalWorkerOptions.workerSrc = pdfjsWorker;
    workerConfigured = true;
  }
}

/**
 * Renders the first PDF page to a JPEG data URL for small hub thumbnails.
 */
export async function renderFirstPageThumbFromBase64(
  pdfBase64: string,
  maxCssWidthPx: number,
): Promise<string | null> {
  ensurePdfWorker();
  let bytes: Uint8Array;
  try {
    bytes = Uint8Array.from(atob(pdfBase64), (c) => c.charCodeAt(0));
  } catch {
    return null;
  }
  const blobUrl = URL.createObjectURL(
    new Blob([bytes], { type: "application/pdf" }),
  );
  try {
    const loadingTask = pdfjs.getDocument({ url: blobUrl });
    const pdf = await loadingTask.promise;
    try {
      const page = await pdf.getPage(1);
      const base = page.getViewport({ scale: 1 });
      const scale = Math.min(1, maxCssWidthPx / base.width);
      const viewport = page.getViewport({ scale });
      const canvas = document.createElement("canvas");
      const dpr = Math.min(typeof window !== "undefined" ? window.devicePixelRatio : 1, 2);
      canvas.width = Math.max(1, Math.floor(viewport.width * dpr));
      canvas.height = Math.max(1, Math.floor(viewport.height * dpr));
      const ctx = canvas.getContext("2d");
      if (!ctx) return null;
      ctx.scale(dpr, dpr);
      await page.render({ canvasContext: ctx, viewport }).promise;
      return canvas.toDataURL("image/jpeg", 0.82);
    } finally {
      await pdf.destroy().catch(() => {});
    }
  } catch {
    return null;
  } finally {
    URL.revokeObjectURL(blobUrl);
  }
}
