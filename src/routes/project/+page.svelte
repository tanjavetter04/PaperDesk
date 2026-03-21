<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { goto } from "$app/navigation";
  import FileTree from "$lib/components/FileTree.svelte";
  import EditorPane from "$lib/components/EditorPane.svelte";
  import PreviewPane from "$lib/components/PreviewPane.svelte";
  import DiagnosticsPanel from "$lib/components/DiagnosticsPanel.svelte";
  import InputModal from "$lib/components/InputModal.svelte";
  import MessageModal from "$lib/components/MessageModal.svelte";
  import {
    getOpenProject,
    listProjectEntries,
    writeTextFile,
    createProjectDir,
    moveProjectPath,
    compileProject,
    exportPdf,
    closeProject,
    startTinymistPreview,
    restartTinymistPreview,
  } from "$lib/tauri/api";
  import type {
    CompileDiagnostic,
    PreviewScrollToSource,
    PreviewSource,
    ProjectEntry,
  } from "$lib/tauri/api";

  let rootPath = $state<string | null>(null);
  let projectEntries = $state<ProjectEntry[]>([]);
  /** Sidebar: folder for “new file / new folder” (`""` = root). */
  let treeTargetDir = $state("");
  let selectedPath = $state<string | null>(null);
  let buffer = $state("");
  let saveLabel = $state<"saved" | "dirty" | "saving">("saved");
  let previewLabel = $state<"idle" | "starting" | "live" | "err">("idle");
  let diagnostics = $state<CompileDiagnostic[]>([]);
  let diagnosticFocus = $state<{ tick: number; target: CompileDiagnostic | null }>({
    tick: 0,
    target: null,
  });
  let previewUrl = $state<string | null>(null);
  let previewError = $state<string | null>(null);
  let previewScroll = $state({ tick: 0, line0: 0, column0: 0 });
  let pendingPreviewJump = $state<PreviewScrollToSource | null>(null);

  let newFileModalOpen = $state(false);
  let newFolderModalOpen = $state(false);
  let messageModalOpen = $state(false);
  let messageModalText = $state("");

  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  let diagnosticsTimer: ReturnType<typeof setTimeout> | null = null;
  /** `buffer` is only a safe preview overlay when it matches the open tab (see EditorPane onReady). */
  let editorBufferPath = $state<string | null>(null);
  const LIVE_SAVE_DEBOUNCE_MS = 140;
  /** Typst compile for the diagnostics panel (tinymist preview does not feed this list). */
  const DIAGNOSTICS_DEBOUNCE_MS = 420;

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

  function pathUnderProjectRoot(absPath: string, root: string): string | null {
    const a = absPath.replace(/\\/g, "/");
    const r = root.replace(/\\/g, "/").replace(/\/+$/, "");
    if (a === r) return "";
    const prefix = r + "/";
    if (!a.startsWith(prefix)) return null;
    return a.slice(prefix.length);
  }

  function firePreviewScroll(line0: number, column0: number) {
    previewScroll = {
      tick: previewScroll.tick + 1,
      line0,
      column0,
    };
  }

  async function handlePreviewScrollToSource(p: PreviewScrollToSource) {
    const root = rootPath;
    if (!root) return;
    const rel = pathUnderProjectRoot(p.filepath, root);
    if (rel == null || rel === "") return;

    if (rel !== selectedPath) {
      pendingPreviewJump = p;
      await selectFile(rel);
      return;
    }
    firePreviewScroll(p.line0, p.column0);
  }

  onMount(() => {
    let unlistenPreview: (() => void) | undefined;
    void listen<PreviewScrollToSource>("preview-scroll-to-source", (e) => {
      void handlePreviewScrollToSource(e.payload);
    }).then((fn) => {
      unlistenPreview = fn;
    });

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
    return () => {
      window.removeEventListener("resize", onResize);
      unlistenPreview?.();
    };
  });

  function projectFilePaths(): string[] {
    return projectEntries.filter((e) => !e.isDir).map((e) => e.path);
  }

  async function refreshFiles() {
    try {
      projectEntries = await listProjectEntries();
      const files = projectFilePaths();
      if (files.length && !selectedPath) {
        selectedPath = files.find((f) => f === "main.typ") ?? files[0] ?? null;
      }
      if (selectedPath && !files.includes(selectedPath)) {
        selectedPath = files.find((f) => f === "main.typ") ?? files[0] ?? null;
        editorBufferPath = null;
      }
    } catch {
      projectEntries = [];
    }
  }

  function safeTreeBasename(name: string): string | null {
    const t = name.trim().replace(/\\/g, "/");
    if (!t || t.includes("..") || t.includes("/")) return null;
    return t;
  }

  function parentDirOfRel(path: string): string {
    const i = path.lastIndexOf("/");
    return i === -1 ? "" : path.slice(0, i);
  }

  function newItemHint(): string {
    return treeTargetDir
      ? `Neu in: ${treeTargetDir}`
      : "Neu im Projektstamm";
  }

  function formatUserError(e: unknown): string {
    if (e instanceof Error) return e.message;
    if (typeof e === "string") return e;
    if (e && typeof e === "object" && "message" in e) {
      const m = (e as { message: unknown }).message;
      if (typeof m === "string") return m;
    }
    return "Unbekannter Fehler.";
  }

  function showMessage(text: string) {
    messageModalText = text;
    messageModalOpen = true;
  }

  function handleNewFile() {
    newFileModalOpen = true;
  }

  function handleNewFolder() {
    newFolderModalOpen = true;
  }

  async function confirmNewFile(raw: string) {
    newFileModalOpen = false;
    const base = safeTreeBasename(raw);
    if (!base) {
      showMessage("Ungültiger Name (keine Schrägstriche oder ..).");
      return;
    }
    const rel = treeTargetDir ? `${treeTargetDir}/${base}` : base;
    if (projectEntries.some((e) => e.path === rel)) {
      showMessage("Dieser Pfad existiert bereits.");
      return;
    }
    try {
      await writeTextFile(rel, "");
      await refreshFiles();
      await selectFile(rel);
    } catch (e) {
      showMessage(formatUserError(e));
    }
  }

  async function confirmNewFolder(raw: string) {
    newFolderModalOpen = false;
    const base = safeTreeBasename(raw);
    if (!base) {
      showMessage("Ungültiger Name (keine Schrägstriche oder ..).");
      return;
    }
    const rel = treeTargetDir ? `${treeTargetDir}/${base}` : base;
    if (projectEntries.some((e) => e.path === rel)) {
      showMessage("Dieser Pfad existiert bereits.");
      return;
    }
    try {
      await createProjectDir(rel);
      await refreshFiles();
      treeTargetDir = rel;
    } catch (e) {
      showMessage(formatUserError(e));
    }
  }

  async function handleMoveFile(destinationDir: string) {
    const from = selectedPath;
    if (!from || from === "main.typ") return;
    const slash = from.lastIndexOf("/");
    const base = slash === -1 ? from : from.slice(slash + 1);
    const dest = destinationDir.replace(/\/+$/, "");
    const to = dest === "" ? base : `${dest}/${base}`;
    if (to === from) return;
    if (projectEntries.some((e) => e.path === to)) {
      showMessage("Ziel existiert bereits.");
      return;
    }
    try {
      if (saveTimer) {
        clearTimeout(saveTimer);
        saveTimer = null;
      }
      await persistFile(from, buffer);
      await moveProjectPath(from, to);
      await refreshFiles();
      selectedPath = to;
      editorBufferPath = null;
      treeTargetDir = parentDirOfRel(to);
    } catch (e) {
      showMessage(formatUserError(e));
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
        ? await restartTinymistPreview()
        : await startTinymistPreview();
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
    if (diagnosticsTimer) clearTimeout(diagnosticsTimer);
  });

  async function persistFile(path: string, text: string) {
    saveLabel = "saving";
    await writeTextFile(path, text);
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

  function scheduleDiagnosticsRefresh() {
    if (!selectedPath?.endsWith(".typ")) return;
    if (diagnosticsTimer) clearTimeout(diagnosticsTimer);
    diagnosticsTimer = setTimeout(() => {
      diagnosticsTimer = null;
      void refreshDiagnostics();
    }, DIAGNOSTICS_DEBOUNCE_MS);
  }

  async function refreshDiagnostics() {
    if (!selectedPath?.endsWith(".typ")) return;
    const pathWhenStarted = selectedPath;
    const source = previewSourceForCompile();
    try {
      const r = await compileProject(source);
      if (selectedPath !== pathWhenStarted) return;
      diagnostics = r.diagnostics;
    } catch (e) {
      if (selectedPath !== pathWhenStarted) return;
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

  function onEditorChange(text: string) {
    buffer = text;
    scheduleSave();
    scheduleDiagnosticsRefresh();
  }

  async function selectFile(p: string) {
    if (p === selectedPath) return;
    if (saveTimer) clearTimeout(saveTimer);
    if (diagnosticsTimer) {
      clearTimeout(diagnosticsTimer);
      diagnosticsTimer = null;
    }
    if (selectedPath) {
      try {
        await persistFile(selectedPath, buffer);
      } catch {
        saveLabel = "dirty";
      }
    }
    selectedPath = p;
    treeTargetDir = parentDirOfRel(p);
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
      await exportPdf();
    } catch {
      /* dialog plugin surfaces errors */
    }
  }

  async function compileNow() {
    try {
      const r = await compileProject(previewSourceForCompile());
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

  function onEditorReady(text: string, loadedPath: string) {
    buffer = text;
    editorBufferPath = loadedPath;
    if (loadedPath.endsWith(".typ")) {
      void refreshDiagnostics();
    }
    const pend = pendingPreviewJump;
    const root = rootPath;
    if (pend && root) {
      const r = pathUnderProjectRoot(pend.filepath, root);
      if (r === loadedPath) {
        pendingPreviewJump = null;
        firePreviewScroll(pend.line0, pend.column0);
      }
    }
  }

  function jumpToDiagnosticInEditor(d: CompileDiagnostic) {
    diagnosticFocus = {
      tick: diagnosticFocus.tick + 1,
      target: d,
    };
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
      <FileTree
        entries={projectEntries}
        selectedFilePath={selectedPath}
        targetDirPath={treeTargetDir}
        onSelectFile={selectFile}
        onTargetDirChange={(d) => {
          treeTargetDir = d;
        }}
        onNewFile={handleNewFile}
        onNewFolder={handleNewFolder}
        onMoveFile={handleMoveFile}
      />
    </aside>
    <section class="center">
      <EditorPane
        path={selectedPath}
        onDocumentChange={onEditorChange}
        onReady={onEditorReady}
        compileDiagnostics={diagnostics}
        focusDiagnosticRequest={diagnosticFocus}
        {previewScroll}
      />
      <DiagnosticsPanel {diagnostics} onJumpTo={jumpToDiagnosticInEditor} />
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

  <InputModal
    open={newFileModalOpen}
    title="Neue Datei"
    hint={`${newItemHint()} · nur Dateiname, z. B. chapter.typ`}
    initialValue="chapter.typ"
    submitLabel="Anlegen"
    onClose={() => (newFileModalOpen = false)}
    onSubmit={(v) => void confirmNewFile(v)}
  />
  <InputModal
    open={newFolderModalOpen}
    title="Neuer Ordner"
    hint={newItemHint()}
    initialValue="sections"
    submitLabel="Anlegen"
    onClose={() => (newFolderModalOpen = false)}
    onSubmit={(v) => void confirmNewFolder(v)}
  />
  <MessageModal
    open={messageModalOpen}
    message={messageModalText}
    onClose={() => (messageModalOpen = false)}
  />
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
