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
  import ConfirmModal from "$lib/components/ConfirmModal.svelte";
  import HistoryPanel from "$lib/components/HistoryPanel.svelte";
  import AiAssistantPanel from "$lib/components/AiAssistantPanel.svelte";
  import {
    getOpenProject,
    renameProject,
    listProjectEntries,
    writeTextFile,
    createProjectDir,
    moveProjectPath,
    deleteProjectPath,
    compileProject,
    exportPdf,
    closeProject,
    startTinymistPreview,
    restartTinymistPreview,
    tinymistPanelScrollToSource,
    historyGetStatus,
    historyRespondEnable,
    historyRespondExistingGit,
    historyCheckpoint,
    historyListCommits,
    historyDiffWorkdir,
    historyRestore,
    restartBibWatcher,
  } from "$lib/tauri/api";
  import type {
    AiEditorContextHost,
    CompileDiagnostic,
    HistoryCommitSummary,
    HistoryStatus,
    PreviewScrollToSource,
    PreviewSource,
    ProjectEntry,
  } from "$lib/tauri/api";
  import { appSettings } from "$lib/appSettings.svelte";
  import { scheduleWarmAlternateSpellDicts } from "$lib/editor/spellDictionaries";
  import { t } from "$lib/i18n/locale.svelte";
  import { openSettingsModal } from "$lib/settingsModal.svelte";

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
  let projectRenameModalOpen = $state(false);
  let messageModalOpen = $state(false);
  let messageModalText = $state("");
  let bibConflictModalOpen = $state(false);
  let treeRenameModalOpen = $state(false);
  let treeRenameSourcePath = $state<string | null>(null);
  let treeDeleteModalOpen = $state(false);
  let treeDeletePath = $state<string | null>(null);
  /** Bump to force EditorPane to re-read the current `selectedPath` from disk. */
  let reloadFromDiskTick = $state(0);

  let historyStatus = $state<HistoryStatus | null>(null);
  let historyPromptEnableOpen = $state(false);
  let historyPromptExistingOpen = $state(false);
  let historyPanelOpen = $state(false);
  let historyCommits = $state<HistoryCommitSummary[]>([]);
  let historyBusy = $state(false);
  /** True while reloading commits but we already had a list (keep list visible). */
  let historyRefreshing = $state(false);
  let historyDiffText = $state("");
  let historyDiffOpen = $state(false);
  let historyRestoreCommitId = $state<string | null>(null);
  let editorReloadTick = $state(0);
  let aiPanelOpen = $state(false);
  const aiEditorRef: AiEditorContextHost = {
    read: () => ({ path: null, selectedText: "" }),
  };

  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  let diagnosticsTimer: ReturnType<typeof setTimeout> | null = null;
  let previewSourceScrollTimer: ReturnType<typeof setTimeout> | null = null;
  let historyIdleTimer: ReturnType<typeof setTimeout> | null = null;
  let lastPreviewSourceScroll:
    | { relativePath: string; line0: number; character: number }
    | null = null;

  const HISTORY_IDLE_MS = 10_000;
  let bibTinymistTimer: ReturnType<typeof setTimeout> | null = null;

  const editorHostCommands: {
    save: () => void | Promise<void>;
    compile: () => void | Promise<void>;
  } = {
    save() {},
    compile() {},
  };
  /** `buffer` is only a safe preview overlay when it matches the open tab (see EditorPane onReady). */
  let editorBufferPath = $state<string | null>(null);
  const LIVE_SAVE_DEBOUNCE_MS = 140;
  /** Typst compile for the diagnostics panel (tinymist preview does not feed this list). */
  const DIAGNOSTICS_DEBOUNCE_MS = 420;
  /** Source caret → tinymist live preview scroll (`panelScrollTo`). */
  const PREVIEW_SOURCE_SCROLL_DEBOUNCE_MS = 120;

  const historyActive = $derived.by(() => {
    const s = historyStatus;
    if (!s) return false;
    return (
      s.enabled &&
      s.hasGitDir &&
      s.useExistingGit === true &&
      !s.promptEnable &&
      !s.promptExistingGit
    );
  });

  const PREVIEW_WIDTH_STORAGE = "paperdesk.previewWidthPx";
  const SIDEBAR_WIDTH_STORAGE = "paperdesk.sidebarWidthPx";
  const SPLITTER_W = 6;
  const MIN_PREVIEW_W = 200;
  const MIN_EDITOR_W = 200;
  const MIN_SIDEBAR_W = 160;
  const DEFAULT_SIDEBAR_W = 220;

  let mainEl = $state<HTMLDivElement | null>(null);
  let sidebarWidthPx = $state(DEFAULT_SIDEBAR_W);
  let previewWidthPx = $state(360);
  /** Bumped on window resize so aria / max width stay in sync with the grid. */
  let layoutMeasure = $state(0);

  const previewWidthMaxPx = $derived.by(() => {
    void layoutMeasure;
    void sidebarWidthPx;
    if (!mainEl) return 800;
    const total = mainEl.getBoundingClientRect().width;
    return Math.max(
      MIN_PREVIEW_W,
      Math.floor(
        total - sidebarWidthPx - 2 * SPLITTER_W - MIN_EDITOR_W,
      ),
    );
  });

  const sidebarWidthMaxPx = $derived.by(() => {
    void layoutMeasure;
    void previewWidthPx;
    if (!mainEl) return 480;
    const total = mainEl.getBoundingClientRect().width;
    return Math.max(
      MIN_SIDEBAR_W,
      Math.floor(
        total - MIN_PREVIEW_W - 2 * SPLITTER_W - MIN_EDITOR_W,
      ),
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
            sidebarWidthPx -
            2 * SPLITTER_W -
            MIN_EDITOR_W,
        )
      : 560;
    return Math.round(
      Math.max(MIN_PREVIEW_W, Math.min(Math.max(MIN_PREVIEW_W, max), next)),
    );
  }

  function clampSidebarWidth(next: number): number {
    const max = mainEl
      ? Math.floor(
          mainEl.getBoundingClientRect().width -
            previewWidthPx -
            2 * SPLITTER_W -
            MIN_EDITOR_W,
        )
      : 480;
    return Math.round(
      Math.max(MIN_SIDEBAR_W, Math.min(Math.max(MIN_SIDEBAR_W, max), next)),
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

  function finishPreviewSplitDrag() {
    splitDragging = false;
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(PREVIEW_WIDTH_STORAGE, String(previewWidthPx));
    }
  }

  function onSplitPointerUp(e: PointerEvent) {
    const el = e.currentTarget as HTMLElement;
    if (el.hasPointerCapture(e.pointerId)) {
      el.releasePointerCapture(e.pointerId);
    }
    finishPreviewSplitDrag();
  }

  let sidebarSplitDragStartX = 0;
  let sidebarSplitDragStartW = 0;

  function onSidebarSplitPointerDown(e: PointerEvent) {
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    sidebarSplitDragStartX = e.clientX;
    sidebarSplitDragStartW = sidebarWidthPx;
    e.preventDefault();
  }

  function onSidebarSplitPointerMove(e: PointerEvent) {
    if (!(e.currentTarget as HTMLElement).hasPointerCapture(e.pointerId)) return;
    const dx = e.clientX - sidebarSplitDragStartX;
    sidebarWidthPx = clampSidebarWidth(sidebarSplitDragStartW + dx);
  }

  function finishSidebarSplitDrag() {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(SIDEBAR_WIDTH_STORAGE, String(sidebarWidthPx));
    }
  }

  function onSidebarSplitPointerUp(e: PointerEvent) {
    const el = e.currentTarget as HTMLElement;
    if (el.hasPointerCapture(e.pointerId)) {
      el.releasePointerCapture(e.pointerId);
    }
    finishSidebarSplitDrag();
  }

  function clearPreviewSplitIfStuck() {
    if (splitDragging) finishPreviewSplitDrag();
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

  function handlePreviewScrollToSource(p: PreviewScrollToSource) {
    const root = rootPath;
    if (!root) return;
    const rel = pathUnderProjectRoot(p.filepath, root);
    if (rel == null || rel === "") return;

    if (rel !== selectedPath) {
      pendingPreviewJump = p;
      selectFile(rel);
      return;
    }
    firePreviewScroll(p.line0, p.column0);
  }

  function scheduleBibTinymistRestart() {
    if (bibTinymistTimer) clearTimeout(bibTinymistTimer);
    bibTinymistTimer = setTimeout(() => {
      bibTinymistTimer = null;
      void ensurePreview(true);
    }, 450);
  }

  async function handleBibExternallyUpdated(relPath: string) {
    await refreshFiles();
    void refreshDiagnostics();
    scheduleBibTinymistRestart();
    if (selectedPath !== relPath) return;
    if (saveLabel !== "saved") {
      bibConflictModalOpen = true;
      return;
    }
    reloadFromDiskTick += 1;
  }

  $effect(() => {
    scheduleWarmAlternateSpellDicts(appSettings.spellcheckLanguage);
  });

  onMount(() => {
    let unlistenPreview: (() => void) | undefined;
    let unlistenBib: (() => void) | undefined;
    void listen<PreviewScrollToSource>("preview-scroll-to-source", (e) => {
      handlePreviewScrollToSource(e.payload);
    }).then((fn) => {
      unlistenPreview = fn;
    });
    void listen<{ relativePath: string }>("bib-externally-updated", (e) => {
      const p = e.payload?.relativePath?.trim().replace(/\\/g, "/");
      if (p) void handleBibExternallyUpdated(p);
    }).then((fn) => {
      unlistenBib = fn;
    });

    if (typeof localStorage !== "undefined") {
      const rawSb = localStorage.getItem(SIDEBAR_WIDTH_STORAGE);
      if (rawSb) {
        const n = Number(rawSb);
        if (Number.isFinite(n) && n >= MIN_SIDEBAR_W && n <= 2000) {
          sidebarWidthPx = n;
        }
      }
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
      sidebarWidthPx = clampSidebarWidth(sidebarWidthPx);
      previewWidthPx = clampPreviewWidth(previewWidthPx);
    };
    const onVisibilityForSplit = () => {
      if (document.visibilityState === "hidden") clearPreviewSplitIfStuck();
    };
    window.addEventListener("resize", onResize);
    window.addEventListener("blur", clearPreviewSplitIfStuck);
    document.addEventListener("visibilitychange", onVisibilityForSplit);
    void tick().then(() => {
      previewWidthPx = clampPreviewWidth(previewWidthPx);
      sidebarWidthPx = clampSidebarWidth(sidebarWidthPx);
      previewWidthPx = clampPreviewWidth(previewWidthPx);
    });

    editorHostCommands.save = async () => {
      if (!selectedPath) return;
      if (saveTimer) {
        clearTimeout(saveTimer);
        saveTimer = null;
      }
      try {
        await persistFile(selectedPath, buffer);
        try {
          await historyCheckpoint("manual-save", true);
        } catch {
          /* best effort */
        }
      } catch {
        saveLabel = "dirty";
      }
    };
    editorHostCommands.compile = () => {
      void compileNow();
    };

    return () => {
      window.removeEventListener("resize", onResize);
      window.removeEventListener("blur", clearPreviewSplitIfStuck);
      document.removeEventListener("visibilitychange", onVisibilityForSplit);
      unlistenPreview?.();
      unlistenBib?.();
      editorHostCommands.save = () => {};
      editorHostCommands.compile = () => {};
    };
  });

  function projectFilePaths(): string[] {
    return projectEntries.filter((e) => !e.isDir).map((e) => e.path);
  }

  function pickTypEditorPath(files: string[]): string | null {
    return (
      files.find((f) => f === "main.typ") ??
      files.find((f) => f.toLowerCase().endsWith(".typ")) ??
      null
    );
  }

  async function refreshFiles() {
    try {
      projectEntries = await listProjectEntries();
      const files = projectFilePaths();
      if (files.length && !selectedPath) {
        selectedPath = pickTypEditorPath(files);
      }
      const stale =
        selectedPath &&
        (!files.includes(selectedPath) ||
          !selectedPath.toLowerCase().endsWith(".typ"));
      if (stale) {
        selectedPath = pickTypEditorPath(files);
        editorBufferPath = null;
      }
    } catch {
      projectEntries = [];
    }
  }

  /** Merge project entries by path (for local tree updates without a full listing). */
  function upsertProjectEntries(
    entries: ProjectEntry[],
    ...added: ProjectEntry[]
  ): ProjectEntry[] {
    const m = new Map(entries.map((e) => [e.path, e]));
    for (const e of added) {
      m.set(e.path, e);
    }
    return [...m.values()].sort((a, b) =>
      a.path.localeCompare(b.path, undefined, { sensitivity: "base" }),
    );
  }

  function entriesForNewFile(rel: string): ProjectEntry[] {
    const parts = rel.split("/").filter(Boolean);
    const out: ProjectEntry[] = [];
    for (let i = 0; i < parts.length - 1; i++) {
      out.push({
        path: parts.slice(0, i + 1).join("/"),
        isDir: true,
      });
    }
    out.push({ path: rel, isDir: false });
    return out;
  }

  function entriesForNewFolder(rel: string): ProjectEntry[] {
    const parts = rel.split("/").filter(Boolean);
    const out: ProjectEntry[] = [];
    for (let i = 0; i < parts.length; i++) {
      out.push({
        path: parts.slice(0, i + 1).join("/"),
        isDir: true,
      });
    }
    return out;
  }

  function entriesAfterMove(from: string, to: string): ProjectEntry[] {
    const filtered = projectEntries.filter((e) => e.path !== from);
    return upsertProjectEntries(filtered, ...entriesForNewFile(to));
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

  function treeItemBasename(path: string): string {
    const i = path.lastIndexOf("/");
    return i === -1 ? path : path.slice(i + 1);
  }

  function adjustTreeTargetAfterRenamePrefix(from: string, to: string) {
    if (treeTargetDir === from) {
      treeTargetDir = to;
    } else if (treeTargetDir.startsWith(from + "/")) {
      treeTargetDir = to + treeTargetDir.slice(from.length);
    }
  }

  function openTreeRename(path: string, _isDir: boolean) {
    if (path === "main.typ") return;
    treeRenameSourcePath = path;
    treeRenameModalOpen = true;
  }

  function openTreeDelete(path: string, _isDir: boolean) {
    if (path === "main.typ") return;
    treeDeletePath = path;
    treeDeleteModalOpen = true;
  }

  async function confirmTreeRename(raw: string) {
    treeRenameModalOpen = false;
    const from = treeRenameSourcePath;
    treeRenameSourcePath = null;
    if (!from) return;
    const base = safeTreeBasename(raw);
    if (!base) {
      showMessage(t("project.invalidName"));
      return;
    }
    const parent = parentDirOfRel(from);
    const to = parent ? `${parent}/${base}` : base;
    if (to === from) return;
    if (projectEntries.some((e) => e.path === to)) {
      showMessage(t("project.pathExists"));
      return;
    }
    try {
      if (selectedPath) {
        const underRename =
          selectedPath === from || selectedPath.startsWith(from + "/");
        if (underRename) {
          if (saveTimer) {
            clearTimeout(saveTimer);
            saveTimer = null;
          }
          await flushPathToDisk(selectedPath, buffer);
        }
      }
      await moveProjectPath(from, to);
      if (selectedPath === from) {
        selectedPath = to;
        treeTargetDir = parentDirOfRel(to);
        editorBufferPath = null;
      } else if (selectedPath?.startsWith(from + "/")) {
        selectedPath = to + selectedPath.slice(from.length);
        treeTargetDir = parentDirOfRel(selectedPath);
        editorBufferPath = null;
      }
      adjustTreeTargetAfterRenamePrefix(from, to);
      await refreshFiles();
      try {
        await historyCheckpoint("move/rename", true);
      } catch {
        /* best effort */
      }
      const watch = appSettings.zoteroBibRelativePath?.trim();
      if (watch && selectedPath?.endsWith(".bib")) {
        void restartBibWatcher(selectedPath);
      }
      await ensurePreview(true);
      void refreshDiagnostics();
    } catch (e) {
      showMessage(formatUserError(e));
    }
  }

  async function confirmTreeDelete() {
    const path = treeDeletePath;
    treeDeleteModalOpen = false;
    treeDeletePath = null;
    if (!path) return;
    try {
      if (
        selectedPath &&
        (selectedPath === path || selectedPath.startsWith(path + "/"))
      ) {
        if (saveTimer) {
          clearTimeout(saveTimer);
          saveTimer = null;
        }
        await flushPathToDisk(selectedPath, buffer);
      }
      await deleteProjectPath(path);
      if (treeTargetDir === path || treeTargetDir.startsWith(path + "/")) {
        treeTargetDir = parentDirOfRel(path);
      }
      await refreshFiles();
      const files = projectFilePaths();
      if (
        selectedPath &&
        (selectedPath === path || selectedPath.startsWith(path + "/"))
      ) {
        selectedPath = pickTypEditorPath(files);
        editorBufferPath = null;
        diagnostics = [];
        saveLabel = "saved";
      }
      try {
        await historyCheckpoint("delete", true);
      } catch {
        /* best effort */
      }
      const watch = appSettings.zoteroBibRelativePath?.trim();
      if (watch && selectedPath?.endsWith(".bib")) {
        void restartBibWatcher(selectedPath);
      }
      await ensurePreview(true);
      void refreshDiagnostics();
    } catch (e) {
      showMessage(formatUserError(e));
    }
  }

  function projectFolderLabel(absolutePath: string): string {
    const n = absolutePath.trim().replace(/\\/g, "/").replace(/\/+$/, "");
    if (!n || n === "/") return absolutePath;
    const parts = n.split("/").filter(Boolean);
    return parts.length ? parts[parts.length - 1]! : absolutePath;
  }

  function newItemHint(): string {
    return treeTargetDir
      ? t("project.newInFolder", { folder: treeTargetDir })
      : t("project.newInRoot");
  }

  function formatUserError(e: unknown): string {
    if (e instanceof Error) return e.message;
    if (typeof e === "string") return e;
    if (e && typeof e === "object" && "message" in e) {
      const m = (e as { message: unknown }).message;
      if (typeof m === "string") return m;
    }
    return t("project.unknownError");
  }

  function showMessage(text: string) {
    messageModalText = text;
    messageModalOpen = true;
  }

  function clearHistoryIdleTimer() {
    if (historyIdleTimer) {
      clearTimeout(historyIdleTimer);
      historyIdleTimer = null;
    }
  }

  function bumpHistoryIdleTimer() {
    if (!historyActive) return;
    clearHistoryIdleTimer();
    historyIdleTimer = setTimeout(() => {
      historyIdleTimer = null;
      void historyCheckpoint("idle", false);
    }, HISTORY_IDLE_MS);
  }

  async function syncHistoryStatus(showPrompts = false) {
    try {
      const s = await historyGetStatus();
      historyStatus = s;
      if (showPrompts) {
        if (s.promptEnable) historyPromptEnableOpen = true;
        else if (s.promptExistingGit) historyPromptExistingOpen = true;
      }
    } catch {
      historyStatus = null;
    }
  }

  async function refreshHistoryCommits() {
    if (!historyActive) return;
    const keepListVisible = historyCommits.length > 0;
    historyBusy = true;
    historyRefreshing = keepListVisible;
    try {
      historyCommits = await historyListCommits(80);
    } catch (e) {
      showMessage(formatUserError(e));
    } finally {
      historyBusy = false;
      historyRefreshing = false;
    }
  }

  async function openHistoryPanel() {
    // Avoid a pending idle checkpoint firing right when opening the panel (feels like "History caused a commit").
    clearHistoryIdleTimer();
    await syncHistoryStatus(false);
    if (!historyActive) {
      const s = historyStatus;
      if (!s) {
        showMessage(t("history.inactiveToast"));
        return;
      }
      if (s.promptEnable) {
        historyPromptEnableOpen = true;
        return;
      }
      if (s.promptExistingGit) {
        historyPromptExistingOpen = true;
        return;
      }
      historyPromptEnableOpen = true;
      return;
    }
    aiPanelOpen = false;
    historyPanelOpen = true;
    await refreshHistoryCommits();
  }

  async function toggleHistoryPanel() {
    if (historyPanelOpen) {
      historyPanelOpen = false;
      historyDiffOpen = false;
      return;
    }
    await openHistoryPanel();
  }

  function toggleAiPanel() {
    if (aiPanelOpen) {
      aiPanelOpen = false;
      return;
    }
    historyPanelOpen = false;
    historyDiffOpen = false;
    aiPanelOpen = true;
  }

  async function handleHistorySnapshot() {
    try {
      await historyCheckpoint("manual", true);
      await refreshHistoryCommits();
    } catch (e) {
      showMessage(formatUserError(e));
    }
  }

  async function handleHistoryDiff(commitId: string) {
    try {
      historyDiffText = await historyDiffWorkdir(commitId);
      historyDiffOpen = true;
    } catch (e) {
      showMessage(formatUserError(e));
    }
  }

  async function confirmHistoryRestore() {
    const id = historyRestoreCommitId;
    historyRestoreCommitId = null;
    if (!id) return;
    try {
      if (saveTimer) {
        clearTimeout(saveTimer);
        saveTimer = null;
      }
      if (selectedPath) {
        await persistFile(selectedPath, buffer);
      }
      await historyRestore(id, null);
      historyDiffOpen = false;
      editorReloadTick += 1;
      await refreshFiles();
      await ensurePreview(true);
      if (selectedPath?.endsWith(".typ")) {
        void refreshDiagnostics();
      }
      await refreshHistoryCommits();
    } catch (e) {
      showMessage(formatUserError(e));
    }
  }

  async function onHistoryEnableYes() {
    historyPromptEnableOpen = false;
    try {
      await historyRespondEnable(true);
      await syncHistoryStatus(false);
      if (historyStatus?.promptExistingGit) {
        historyPromptExistingOpen = true;
      }
    } catch (e) {
      showMessage(formatUserError(e));
    }
  }

  async function onHistoryEnableNo() {
    historyPromptEnableOpen = false;
    try {
      await historyRespondEnable(false);
      await syncHistoryStatus(false);
    } catch (e) {
      showMessage(formatUserError(e));
    }
  }

  async function onHistoryExistingYes() {
    historyPromptExistingOpen = false;
    try {
      await historyRespondExistingGit(true);
      await syncHistoryStatus(false);
    } catch (e) {
      showMessage(formatUserError(e));
    }
  }

  async function onHistoryExistingNo() {
    historyPromptExistingOpen = false;
    try {
      await historyRespondExistingGit(false);
      await syncHistoryStatus(false);
    } catch (e) {
      showMessage(formatUserError(e));
    }
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
      showMessage(t("project.invalidName"));
      return;
    }
    const rel = treeTargetDir ? `${treeTargetDir}/${base}` : base;
    if (projectEntries.some((e) => e.path === rel)) {
      showMessage(t("project.pathExists"));
      return;
    }
    try {
      await writeTextFile(rel, "");
      try {
        await historyCheckpoint("new-file", true);
      } catch {
        /* best effort */
      }
      projectEntries = upsertProjectEntries(projectEntries, ...entriesForNewFile(rel));
      selectFile(rel);
    } catch (e) {
      showMessage(formatUserError(e));
    }
  }

  async function confirmNewFolder(raw: string) {
    newFolderModalOpen = false;
    const base = safeTreeBasename(raw);
    if (!base) {
      showMessage(t("project.invalidName"));
      return;
    }
    const rel = treeTargetDir ? `${treeTargetDir}/${base}` : base;
    if (projectEntries.some((e) => e.path === rel)) {
      showMessage(t("project.pathExists"));
      return;
    }
    try {
      await createProjectDir(rel);
      projectEntries = upsertProjectEntries(
        projectEntries,
        ...entriesForNewFolder(rel),
      );
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
      showMessage(t("project.destinationExists"));
      return;
    }
    try {
      if (saveTimer) {
        clearTimeout(saveTimer);
        saveTimer = null;
      }
      await persistFile(from, buffer);
      await moveProjectPath(from, to);
      projectEntries = entriesAfterMove(from, to);
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
      await Promise.all([refreshFiles(), ensurePreview()]);
      await syncHistoryStatus(true);
      if (gone) return;
      try {
        await restartBibWatcher(appSettings.zoteroBibRelativePath);
      } catch {
        /* ignore */
      }
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
      lastPreviewSourceScroll = null;
    }
    try {
      const url = restart
        ? await restartTinymistPreview()
        : await startTinymistPreview();
      if (previewUrl !== url) {
        previewUrl = url;
        lastPreviewSourceScroll = null;
      }
      previewLabel = "live";
    } catch (e) {
      previewUrl = null;
      lastPreviewSourceScroll = null;
      previewLabel = "err";
      previewError = String(e);
    }
  }

  onDestroy(() => {
    if (saveTimer) clearTimeout(saveTimer);
    if (diagnosticsTimer) clearTimeout(diagnosticsTimer);
    if (previewSourceScrollTimer) clearTimeout(previewSourceScrollTimer);
    clearHistoryIdleTimer();
    if (bibTinymistTimer) clearTimeout(bibTinymistTimer);
  });

  async function persistFile(path: string, text: string) {
    saveLabel = "saving";
    await writeTextFile(path, text);
    saveLabel = "saved";
    bumpHistoryIdleTimer();
  }

  /** Write without updating UI; used when switching files so the tab change is not blocked. */
  async function flushPathToDisk(path: string, text: string): Promise<void> {
    await writeTextFile(path, text);
    bumpHistoryIdleTimer();
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

  function schedulePreviewSourceScroll(pos: {
    line0: number;
    character: number;
    reason: "cursor" | "edit";
  }) {
    if (!selectedPath?.endsWith(".typ")) return;
    if (!previewUrl) return;
    if (splitDragging) return;
    const rel = selectedPath;
    const last = lastPreviewSourceScroll;
    const samePath = last?.relativePath === rel;
    const sameLine = samePath && last?.line0 === pos.line0;
    const samePos = sameLine && last?.character === pos.character;
    if (samePos) return;
    // Typing on the same source line re-triggers tinymist's marker animation without
    // meaningfully changing the visible target, so only resync on actual line changes.
    if (pos.reason === "edit" && sameLine) return;
    if (previewSourceScrollTimer) clearTimeout(previewSourceScrollTimer);
    const delay = pos.reason === "cursor" ? 0 : PREVIEW_SOURCE_SCROLL_DEBOUNCE_MS;
    previewSourceScrollTimer = setTimeout(() => {
      previewSourceScrollTimer = null;
      if (selectedPath !== rel) return;
      lastPreviewSourceScroll = {
        relativePath: rel,
        line0: pos.line0,
        character: pos.character,
      };
      void tinymistPanelScrollToSource(rel, pos.line0, pos.character).catch(() => {});
    }, delay);
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
    bumpHistoryIdleTimer();
  }

  function selectFile(p: string) {
    if (!p.toLowerCase().endsWith(".typ")) return;
    if (p === selectedPath) return;
    if (saveTimer) {
      clearTimeout(saveTimer);
      saveTimer = null;
    }
    if (diagnosticsTimer) {
      clearTimeout(diagnosticsTimer);
      diagnosticsTimer = null;
    }
    if (previewSourceScrollTimer) {
      clearTimeout(previewSourceScrollTimer);
      previewSourceScrollTimer = null;
    }
    lastPreviewSourceScroll = null;
    const prevPath = selectedPath;
    const prevBuffer = buffer;
    if (prevPath) {
      void flushPathToDisk(prevPath, prevBuffer).catch(() => {
        showMessage(t("project.saveFailed", { path: prevPath }));
      });
    }
    selectedPath = p;
    treeTargetDir = parentDirOfRel(p);
    editorBufferPath = null;
    diagnostics = [];
    saveLabel = "saved";
  }

  async function confirmProjectRename(name: string) {
    const root = rootPath;
    if (!root) return;
    if (saveTimer) {
      clearTimeout(saveTimer);
      saveTimer = null;
    }
    if (selectedPath) {
      try {
        await flushPathToDisk(selectedPath, buffer);
      } catch {
        /* best effort before rename */
      }
    }
    try {
      const next = await renameProject(root, name);
      rootPath = next;
      projectRenameModalOpen = false;
      await ensurePreview(true);
      await syncHistoryStatus(false);
    } catch (e) {
      showMessage(formatUserError(e));
    }
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
    clearHistoryIdleTimer();
    try {
      await historyCheckpoint("hub", false);
    } catch {
      /* best effort */
    }
    try {
      await closeProject();
    } catch {
      /* Still leave the IDE: a failed backend close must not trap the user here. */
    }
    await goto("/");
  }

  async function doExport() {
    try {
      await exportPdf(t("dialog.exportPdf"));
      try {
        await historyCheckpoint("export", true);
      } catch {
        /* best effort */
      }
    } catch {
      /* dialog plugin surfaces errors */
    }
  }

  async function compileNow() {
    try {
      const r = await compileProject(previewSourceForCompile());
      diagnostics = r.diagnostics;
      await refreshFiles();
      try {
        await historyCheckpoint("compile", true);
      } catch {
        /* best effort */
      }
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
      scheduleDiagnosticsRefresh();
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

  function saveStatusLabel(): string {
    if (saveLabel === "saved") return t("status.saved");
    if (saveLabel === "saving") return t("status.saving");
    return t("status.dirty");
  }

  function previewStatusLabel(): string {
    if (previewLabel === "starting") return t("preview.starting");
    if (previewLabel === "live") return t("preview.live");
    if (previewLabel === "err") return t("preview.error");
    return t("preview.idle");
  }
</script>

<div class="ide">
  <header class="bar">
    <button type="button" class="ghost bar-back" onclick={goHub}>
      {t("project.backToProjects")}
    </button>
    <button
      type="button"
      class="ghost proj-rename"
      onclick={() => (projectRenameModalOpen = true)}
      title={t("project.renameAria")}
      aria-label={t("project.renameAria")}
    >
      <svg
        width="18"
        height="18"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true"
      >
        <path d="M12 20h9" />
        <path d="M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4Z" />
      </svg>
    </button>
    <span class="proj" title={rootPath ?? ""}>{rootPath ?? ""}</span>
    <span class="status">
      <span class="pill" data-state={saveLabel}>{saveStatusLabel()}</span>
      <span class="pill" data-state={previewLabel}>{previewStatusLabel()}</span>
    </span>
    <span class="spacer"></span>
    <button
      type="button"
      class="bar-icon-action"
      onclick={toggleAiPanel}
      title={t("project.aiToolbar")}
      aria-label={t("project.aiToolbar")}
      aria-pressed={aiPanelOpen}
    >
      <svg
        width="18"
        height="18"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true"
      >
        <path
          d="m12 3-1.912 5.813a2 2 0 0 1-1.275 1.275L3 12l5.813 1.912a2 2 0 0 1 1.275 1.275L12 21l1.912-5.813a2 2 0 0 1 1.275-1.275L21 12l-5.813-1.912a2 2 0 0 1-1.275-1.275L12 3Z"
        />
        <path d="M5 3v4" />
        <path d="M19 17v4" />
        <path d="M3 5h4" />
        <path d="M17 19h4" />
      </svg>
    </button>
    <button
      type="button"
      class="bar-icon-action"
      onclick={() => void toggleHistoryPanel()}
      title={historyActive ? t("history.toolbar") : t("history.toolbarEnable")}
      aria-label={historyActive ? t("history.toolbar") : t("history.toolbarEnable")}
      aria-pressed={historyPanelOpen}
    >
      <svg
        width="18"
        height="18"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true"
      >
        <line x1="6" y1="3" x2="6" y2="15" />
        <circle cx="18" cy="6" r="3" />
        <circle cx="6" cy="18" r="3" />
        <path d="M18 9v9a3 3 0 0 1-3 3H9" />
      </svg>
    </button>
    <button
      type="button"
      class="bar-icon-action"
      onclick={doExport}
      title={t("project.exportPdf")}
      aria-label={t("project.exportPdf")}
    >
      <svg
        width="18"
        height="18"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true"
      >
        <path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" />
        <path d="M14 2v4a2 2 0 0 0 2 2h4" />
        <path d="M12 18v-6" />
        <path d="m9 15 3 3 3-3" />
      </svg>
    </button>
  </header>

  <div class="main" bind:this={mainEl}>
    <aside class="side" style:width={`${sidebarWidthPx}px`}>
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
        onRenameItem={openTreeRename}
        onDeleteItem={openTreeDelete}
      />
    </aside>
    <div
      class="splitter"
      role="separator"
      aria-orientation="vertical"
      aria-valuenow={sidebarWidthPx}
      aria-valuemin={MIN_SIDEBAR_W}
      aria-valuemax={sidebarWidthMaxPx}
      aria-label={t("aria.sidebarWidth")}
      onpointerdown={onSidebarSplitPointerDown}
      onpointermove={onSidebarSplitPointerMove}
      onpointerup={onSidebarSplitPointerUp}
      onpointercancel={onSidebarSplitPointerUp}
      onlostpointercapture={onSidebarSplitPointerUp}
    ></div>
    <section class="center">
      <EditorPane
        path={selectedPath}
        reloadTick={editorReloadTick}
        reloadFromDiskTick={reloadFromDiskTick}
        hostCommands={editorHostCommands}
        aiEditorRef={aiEditorRef}
        onDocumentChange={onEditorChange}
        onReady={onEditorReady}
        onTypstPreviewSourceScroll={schedulePreviewSourceScroll}
        compileDiagnostics={diagnostics}
        focusDiagnosticRequest={diagnosticFocus}
        {previewScroll}
        onBinaryAssetCreated={(rel) => {
          projectEntries = upsertProjectEntries(
            projectEntries,
            ...entriesForNewFile(rel),
          );
          void historyCheckpoint("paste-image", true).catch(() => {});
        }}
        onPasteImageError={showMessage}
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
      aria-label={t("aria.previewWidth")}
      onpointerdown={onSplitPointerDown}
      onpointermove={onSplitPointerMove}
      onpointerup={onSplitPointerUp}
      onpointercancel={onSplitPointerUp}
      onlostpointercapture={onSplitPointerUp}
    ></div>
    <aside
      class="preview-col"
      class:preview-col--split-drag={splitDragging}
      style:width={`${previewWidthPx}px`}
    >
      <PreviewPane {previewUrl} error={previewError} />
    </aside>
  </div>

  <button
    type="button"
    class="ide-settings-fab"
    onclick={openSettingsModal}
    title={t("settings.open")}
    aria-label={t("settings.open")}
  >
    <svg
      width="22"
      height="22"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"
    >
      <path d="M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6Z" />
      <path
        d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1Z"
      />
    </svg>
  </button>

  <InputModal
    open={newFileModalOpen}
    title={t("project.newFileTitle")}
    hint={t("project.newFileHint", { hint: newItemHint() })}
    initialValue="chapter.typ"
    submitLabel={t("project.create")}
    onClose={() => (newFileModalOpen = false)}
    onSubmit={(v) => void confirmNewFile(v)}
  />
  <InputModal
    open={newFolderModalOpen}
    title={t("project.newFolderTitle")}
    hint={t("project.newFolderHint", { hint: newItemHint() })}
    initialValue="sections"
    submitLabel={t("project.create")}
    onClose={() => (newFolderModalOpen = false)}
    onSubmit={(v) => void confirmNewFolder(v)}
  />
  <InputModal
    open={treeRenameModalOpen}
    title={t("fileTree.renameItemTitle")}
    hint={t("fileTree.renameItemHint")}
    initialValue={treeRenameSourcePath ? treeItemBasename(treeRenameSourcePath) : ""}
    submitLabel={t("project.renameSubmit")}
    onClose={() => {
      treeRenameModalOpen = false;
      treeRenameSourcePath = null;
    }}
    onSubmit={(v) => void confirmTreeRename(v)}
  />
  <InputModal
    open={projectRenameModalOpen}
    title={t("project.renameTitle")}
    hint={t("project.renameHint")}
    initialValue={rootPath ? projectFolderLabel(rootPath) : ""}
    submitLabel={t("project.renameSubmit")}
    onClose={() => (projectRenameModalOpen = false)}
    onSubmit={(v) => void confirmProjectRename(v)}
  />
  <MessageModal
    open={messageModalOpen}
    message={messageModalText}
    onClose={() => (messageModalOpen = false)}
  />

  <ConfirmModal
    open={treeDeleteModalOpen}
    title={t("fileTree.deleteItemTitle")}
    message={treeDeletePath
      ? t("fileTree.deleteItemMessage", { path: treeDeletePath })
      : ""}
    confirmLabel={t("fileTree.deleteConfirm")}
    cancelLabel={t("common.cancel")}
    onConfirm={() => void confirmTreeDelete()}
    onCancel={() => {
      treeDeleteModalOpen = false;
      treeDeletePath = null;
    }}
  />

  <ConfirmModal
    open={historyPromptEnableOpen}
    title={t("history.promptEnableTitle")}
    message={t("history.promptEnableMessage")}
    confirmLabel={t("history.promptEnableConfirm")}
    cancelLabel={t("history.promptEnableCancel")}
    onConfirm={() => void onHistoryEnableYes()}
    onCancel={() => void onHistoryEnableNo()}
  />

  <ConfirmModal
    open={historyPromptExistingOpen}
    title={t("history.promptExistingTitle")}
    message={t("history.promptExistingMessage")}
    confirmLabel={t("history.promptExistingConfirm")}
    cancelLabel={t("history.promptExistingCancel")}
    onConfirm={() => void onHistoryExistingYes()}
    onCancel={() => void onHistoryExistingNo()}
  />

  <ConfirmModal
    open={historyRestoreCommitId !== null}
    title={t("history.restoreTitle")}
    message={t("history.restoreMessage")}
    confirmLabel={t("history.restoreConfirm")}
    cancelLabel={t("common.cancel")}
    onConfirm={() => void confirmHistoryRestore()}
    onCancel={() => (historyRestoreCommitId = null)}
  />

  <AiAssistantPanel
    open={aiPanelOpen}
    onClose={() => (aiPanelOpen = false)}
    editorContext={aiEditorRef}
  />

  <HistoryPanel
    open={historyPanelOpen}
    commits={historyCommits}
    busy={historyBusy}
    refreshing={historyRefreshing}
    tipShort={historyStatus?.tipShort ?? null}
    historyRefExists={historyStatus?.historyRefExists ?? false}
    diffText={historyDiffText}
    diffOpen={historyDiffOpen}
    onClose={() => {
      historyPanelOpen = false;
      historyDiffOpen = false;
    }}
    onRefresh={() => void refreshHistoryCommits()}
    onSnapshot={() => void handleHistorySnapshot()}
    onRequestDiff={(id) => void handleHistoryDiff(id)}
    onCloseDiff={() => (historyDiffOpen = false)}
    onRestore={(id) => (historyRestoreCommitId = id)}
  />

  <MessageModal
    open={bibConflictModalOpen}
    message={t("project.bibExternalChange")}
    secondaryLabel={t("project.bibReloadFromDisk")}
    onSecondary={() => {
      if (saveTimer) {
        clearTimeout(saveTimer);
        saveTimer = null;
      }
      saveLabel = "saved";
      reloadFromDiskTick += 1;
    }}
    onClose={() => (bibConflictModalOpen = false)}
  />
</div>

<style>
  .ide {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }

  .ide-settings-fab {
    position: fixed;
    left: 0.85rem;
    bottom: 0.85rem;
    z-index: 40;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 2.75rem;
    height: 2.75rem;
    padding: 0;
    border-radius: 10px;
    border: 1px solid color-mix(in srgb, var(--pd-accent) 42%, var(--pd-border));
    background: color-mix(in srgb, var(--pd-accent) 22%, var(--pd-surface));
    color: color-mix(in srgb, var(--pd-accent) 55%, var(--pd-text));
    cursor: pointer;
    box-shadow:
      0 1px 0 color-mix(in srgb, var(--pd-accent) 22%, transparent),
      0 4px 14px rgb(0 0 0 / 0.28);
  }

  .ide-settings-fab:hover {
    color: var(--pd-text);
    border-color: color-mix(in srgb, var(--pd-accent) 58%, var(--pd-border));
    background: color-mix(in srgb, var(--pd-accent) 30%, var(--pd-surface));
  }

  .ide-settings-fab:focus-visible {
    outline: 2px solid color-mix(in srgb, var(--pd-accent) 55%, transparent);
    outline-offset: 2px;
  }

  .bar {
    position: relative;
    z-index: 5;
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
    font-size: 1rem;
    padding: 0.35rem 0.5rem;
  }

  .ghost:hover {
    color: var(--pd-text);
  }

  .bar-back {
    flex-shrink: 0;
  }

  .proj-rename {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0.3rem;
    flex-shrink: 0;
  }

  .proj {
    font-size: 1rem;
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
    font-size: 1rem;
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

  .bar-icon-action {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0.35rem;
    border-radius: 6px;
    border: 1px solid var(--pd-border);
    background: var(--pd-bg);
    color: var(--pd-text);
    flex-shrink: 0;
    cursor: pointer;
  }

  .bar-icon-action:hover {
    border-color: var(--pd-muted);
  }

  .bar-icon-action:focus-visible {
    outline: 2px solid color-mix(in srgb, var(--pd-accent) 55%, transparent);
    outline-offset: 2px;
  }

  .main {
    flex: 1;
    display: flex;
    flex-direction: row;
    align-items: stretch;
    min-height: 0;
    min-width: 0;
  }

  .splitter {
    flex: 0 0 6px;
    width: 6px;
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
    flex: 0 0 auto;
    min-height: 0;
    min-width: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .center {
    flex: 1 1 0;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }

  .preview-col {
    flex: 0 0 auto;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .preview-col.preview-col--split-drag :global(.preview-frame) {
    pointer-events: none;
  }
</style>
