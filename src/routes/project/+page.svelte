<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import { goto } from "$app/navigation";
  import FileTree from "$lib/components/FileTree.svelte";
  import EditorPane from "$lib/components/EditorPane.svelte";
  import PreviewPane from "$lib/components/PreviewPane.svelte";
  import DiagnosticsPanel from "$lib/components/DiagnosticsPanel.svelte";
  import {
    getOpenProject,
    listProjectFiles,
    writeTextFile,
    compileProject,
    exportPdf,
    closeProject,
    startTinymistPreview,
    restartTinymistPreview,
  } from "$lib/tauri/api";
  import type { CompileDiagnostic, PreviewSource } from "$lib/tauri/api";

  let rootPath = $state<string | null>(null);
  let files = $state<string[]>([]);
  let selectedPath = $state<string | null>(null);
  let buffer = $state("");
  let saveLabel = $state<"saved" | "dirty" | "saving">("saved");
  let previewLabel = $state<"idle" | "starting" | "live" | "err">("idle");
  let diagnostics = $state<CompileDiagnostic[]>([]);
  let previewUrl = $state<string | null>(null);
  let previewError = $state<string | null>(null);

  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  /** `buffer` is only a safe preview overlay when it matches the open tab (see EditorPane onReady). */
  let editorBufferPath = $state<string | null>(null);
  const LIVE_SAVE_DEBOUNCE_MS = 140;

  const PREVIEW_WIDTH_STORAGE = "paperdesk.previewWidthPx";
  const SIDEBAR_W = 220;
  const SPLITTER_W = 6;
  const MIN_PREVIEW_W = 200;
  const MIN_EDITOR_W = 200;

  let mainEl = $state<HTMLDivElement | null>(null);
  let previewWidthPx = $state(360);
  /** Bumped on window resize so aria / max width stay in sync with the grid. */
  let layoutMeasure = $state(0);

  let mainGridColumns = $derived(
    `${SIDEBAR_W}px minmax(0, 1fr) ${SPLITTER_W}px ${previewWidthPx}px`,
  );

  const previewWidthMaxPx = $derived.by(() => {
    void layoutMeasure;
    if (!mainEl) return 800;
    const total = mainEl.getBoundingClientRect().width;
    return Math.max(
      MIN_PREVIEW_W,
      Math.floor(total - SIDEBAR_W - SPLITTER_W - MIN_EDITOR_W),
    );
  });

  function defaultPreviewWidth(): number {
    if (typeof window === "undefined") return 360;
    return Math.round(
      Math.min(560, Math.max(240, window.innerWidth * 0.38)),
    );
  }

  function clampPreviewWidth(next: number): number {
    const max = mainEl
      ? Math.floor(
          mainEl.getBoundingClientRect().width -
            SIDEBAR_W -
            SPLITTER_W -
            MIN_EDITOR_W,
        )
      : 560;
    return Math.round(
      Math.max(MIN_PREVIEW_W, Math.min(Math.max(MIN_PREVIEW_W, max), next)),
    );
  }

  let splitDragStartX = 0;
  let splitDragStartW = 0;
  /** While dragging, avoid pointer hit-testing inside the tinymist iframe (reduces jank / odd scroll jumps). */
  let splitDragging = $state(false);

  function onSplitPointerDown(e: PointerEvent) {
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    splitDragStartX = e.clientX;
    splitDragStartW = previewWidthPx;
    splitDragging = true;
    e.preventDefault();
  }

  function onSplitPointerMove(e: PointerEvent) {
    if (!(e.currentTarget as HTMLElement).hasPointerCapture(e.pointerId)) return;
    const dx = e.clientX - splitDragStartX;
    previewWidthPx = clampPreviewWidth(splitDragStartW - dx);
  }

  function onSplitPointerUp(e: PointerEvent) {
    const el = e.currentTarget as HTMLElement;
    if (el.hasPointerCapture(e.pointerId)) {
      el.releasePointerCapture(e.pointerId);
    }
    splitDragging = false;
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(PREVIEW_WIDTH_STORAGE, String(previewWidthPx));
    }
  }

  onMount(() => {
    if (typeof localStorage !== "undefined") {
      const raw = localStorage.getItem(PREVIEW_WIDTH_STORAGE);
      if (raw) {
        const n = Number(raw);
        if (Number.isFinite(n) && n >= MIN_PREVIEW_W && n <= 2000) {
          previewWidthPx = n;
        }
      } else {
        previewWidthPx = defaultPreviewWidth();
      }
    }
    const onResize = () => {
      layoutMeasure += 1;
      previewWidthPx = clampPreviewWidth(previewWidthPx);
    };
    window.addEventListener("resize", onResize);
    void tick().then(() => {
      previewWidthPx = clampPreviewWidth(previewWidthPx);
    });
    return () => window.removeEventListener("resize", onResize);
  });

  async function refreshFiles() {
    try {
      files = await listProjectFiles();
      if (files.length && !selectedPath) {
        const main = files.find((f) => f === "main.typ") ?? files[0];
        selectedPath = main;
      }
    } catch {
      files = [];
    }
  }

  $effect(() => {
    let gone = false;
    (async () => {
      const open = await getOpenProject();
      if (gone) return;
      if (!open) {
        await goto("/");
        return;
      }
      rootPath = open;
      await refreshFiles();
      await ensurePreview();
    })();
    return () => {
      gone = true;
    };
  });

  async function ensurePreview(restart = false) {
    previewError = null;
    const hadUrl = previewUrl !== null;
    if (restart || !hadUrl) {
      previewLabel = "starting";
    }
    try {
      const url = restart
        ? await restartTinymistPreview(null)
        : await startTinymistPreview(null);
      if (previewUrl !== url) {
        previewUrl = url;
      }
      previewLabel = "live";
    } catch (e) {
      previewUrl = null;
      previewLabel = "err";
      previewError = String(e);
    }
  }

  onDestroy(() => {
    if (saveTimer) clearTimeout(saveTimer);
  });

  async function persistFile(path: string, text: string) {
    saveLabel = "saving";
    await writeTextFile(path, text);
    if (path === "paperdesk.json") {
      await ensurePreview(true);
      await refreshFiles();
    }
    saveLabel = "saved";
  }

  function scheduleSave() {
    if (!selectedPath) return;
    saveLabel = "dirty";
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(async () => {
      const path = selectedPath;
      const text = buffer;
      if (!path) return;
      try {
        await persistFile(path, text);
      } catch {
        saveLabel = "dirty";
      }
    }, LIVE_SAVE_DEBOUNCE_MS);
  }

  function previewSourceForCompile(): PreviewSource | null {
    if (!selectedPath || editorBufferPath !== selectedPath) return null;
    return {
      path: selectedPath,
      text: buffer,
    };
  }

  function onEditorChange(text: string) {
    buffer = text;
    scheduleSave();
  }

  async function selectFile(p: string) {
    if (p === selectedPath) return;
    if (saveTimer) clearTimeout(saveTimer);
    if (selectedPath) {
      try {
        await persistFile(selectedPath, buffer);
      } catch {
        saveLabel = "dirty";
      }
    }
    selectedPath = p;
    editorBufferPath = null;
    diagnostics = [];
  }

  async function goHub() {
    if (saveTimer) clearTimeout(saveTimer);
    if (selectedPath) {
      try {
        await persistFile(selectedPath, buffer);
      } catch {
        /* keep going */
      }
    }
    await closeProject();
    await goto("/");
  }

  async function doExport() {
    try {
      await exportPdf(null);
    } catch {
      /* dialog plugin surfaces errors */
    }
  }

  async function compileNow() {
    try {
      const r = await compileProject(null, previewSourceForCompile());
      diagnostics = r.diagnostics;
      await refreshFiles();
    } catch (e) {
      diagnostics = [
        {
          severity: "error",
          message: String(e),
          path: null,
          line: null,
          column: null,
        },
      ];
    }
  }
</script>

<div class="ide">
  <header class="bar">
    <button type="button" class="ghost" onclick={goHub}>← Projects</button>
    <span class="proj" title={rootPath ?? ""}>{rootPath ?? ""}</span>
    <span class="status">
      <span class="pill" data-state={saveLabel}>{saveLabel}</span>
      <span class="pill" data-state={previewLabel}>{previewLabel}</span>
    </span>
    <span class="spacer"></span>
    <button type="button" class="action" onclick={compileNow}>Compile</button>
    <button type="button" class="action" onclick={doExport}>Export PDF</button>
  </header>

  <div
    class="main"
    bind:this={mainEl}
    style:grid-template-columns={mainGridColumns}
  >
    <aside class="side">
      <FileTree {files} {selectedPath} onSelect={selectFile} />
    </aside>
    <section class="center">
      <EditorPane
        path={selectedPath}
        onDocumentChange={onEditorChange}
        onReady={(t, loadedPath) => {
          buffer = t;
          editorBufferPath = loadedPath;
        }}
      />
      <DiagnosticsPanel {diagnostics} />
    </section>
    <div
      class="splitter"
      role="separator"
      aria-orientation="vertical"
      aria-valuenow={previewWidthPx}
      aria-valuemin={MIN_PREVIEW_W}
      aria-valuemax={previewWidthMaxPx}
      aria-label="Breite der Vorschau anpassen"
      onpointerdown={onSplitPointerDown}
      onpointermove={onSplitPointerMove}
      onpointerup={onSplitPointerUp}
      onpointercancel={onSplitPointerUp}
    ></div>
    <aside class="preview-col" class:preview-col--split-drag={splitDragging}>
      <PreviewPane {previewUrl} error={previewError} />
    </aside>
  </div>
</div>

<style>
  .ide {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }

  .bar {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    padding: 0.45rem 0.65rem;
    border-bottom: 1px solid var(--pd-border);
    background: var(--pd-surface);
    flex-shrink: 0;
  }

  .ghost {
    border: none;
    background: transparent;
    color: var(--pd-muted);
    font-size: 0.85rem;
    padding: 0.35rem 0.5rem;
  }

  .ghost:hover {
    color: var(--pd-text);
  }

  .proj {
    font-size: 0.78rem;
    color: var(--pd-muted);
    max-width: 28vw;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .status {
    display: flex;
    gap: 0.35rem;
  }

  .pill {
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 0.2rem 0.45rem;
    border-radius: 4px;
    background: var(--pd-bg);
    color: var(--pd-muted);
  }

  .pill[data-state="dirty"] {
    color: var(--pd-warning);
  }

  .pill[data-state="saving"] {
    color: var(--pd-accent);
  }

  .pill[data-state="live"] {
    color: #69db7c;
  }

  .pill[data-state="starting"] {
    color: var(--pd-accent);
  }

  .pill[data-state="err"] {
    color: var(--pd-error);
  }

  .spacer {
    flex: 1;
  }

  .action {
    padding: 0.4rem 0.75rem;
    border-radius: 6px;
    border: 1px solid var(--pd-border);
    background: var(--pd-bg);
    color: var(--pd-text);
    font-size: 0.82rem;
  }

  .action:hover {
    border-color: var(--pd-muted);
  }

  .main {
    flex: 1;
    display: grid;
    grid-template-rows: minmax(0, 1fr);
    min-height: 0;
  }

  .splitter {
    grid-column: 3;
    grid-row: 1;
    width: 100%;
    min-height: 0;
    touch-action: none;
    user-select: none;
    cursor: col-resize;
    background: var(--pd-border);
    z-index: 2;
  }

  .splitter:hover {
    background: var(--pd-muted);
  }

  .side {
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .center {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }

  .preview-col {
    min-width: 0;
    min-height: 0;
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .preview-col.preview-col--split-drag :global(.preview-frame) {
    pointer-events: none;
  }
</style>
