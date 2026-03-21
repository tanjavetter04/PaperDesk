<script lang="ts">
  import type { HistoryCommitSummary } from "$lib/tauri/api";
  import {
    checkpointDisplayLabel,
    parseGitPatch,
    type DiffViewLine,
  } from "$lib/history/parsePatch";
  import { locale, t } from "$lib/i18n/locale.svelte";

  const HISTORY_PANEL_W_KEY = "paperdesk.historyPanelWidthPx";
  const HISTORY_PANEL_MIN = 280;
  const HISTORY_PANEL_MAX = 960;
  const HISTORY_PANEL_DEFAULT = 440;

  function readStoredPanelWidth(): number {
    if (typeof localStorage === "undefined") return HISTORY_PANEL_DEFAULT;
    const n = Number(localStorage.getItem(HISTORY_PANEL_W_KEY));
    return Number.isFinite(n) && n >= HISTORY_PANEL_MIN && n <= HISTORY_PANEL_MAX
      ? n
      : HISTORY_PANEL_DEFAULT;
  }

  function clampPanelW(w: number): number {
    const maxV =
      typeof window !== "undefined"
        ? Math.max(HISTORY_PANEL_MIN, window.innerWidth - 32)
        : HISTORY_PANEL_MAX;
    const cap = Math.min(HISTORY_PANEL_MAX, maxV);
    return Math.min(cap, Math.max(HISTORY_PANEL_MIN, Math.round(w)));
  }

  let {
    open,
    commits,
    busy,
    refreshing = false,
    tipShort = null,
    historyRefExists = false,
    diffText,
    diffOpen,
    onClose,
    onRefresh,
    onSnapshot,
    onRequestDiff,
    onCloseDiff,
    onRestore,
  }: {
    open: boolean;
    commits: HistoryCommitSummary[];
    busy: boolean;
    refreshing?: boolean;
    tipShort?: string | null;
    historyRefExists?: boolean;
    diffText: string;
    diffOpen: boolean;
    onClose: () => void;
    onRefresh: () => void;
    onSnapshot: () => void;
    onRequestDiff: (commitId: string) => void;
    onCloseDiff: () => void;
    onRestore: (commitId: string) => void;
  } = $props();

  let panelWidthPx = $state(
    typeof window !== "undefined" ? clampPanelW(readStoredPanelWidth()) : HISTORY_PANEL_DEFAULT,
  );
  let resizing = $state(false);

  $effect(() => {
    if (typeof window === "undefined") return;
    function onWinResize() {
      panelWidthPx = clampPanelW(panelWidthPx);
    }
    window.addEventListener("resize", onWinResize);
    return () => window.removeEventListener("resize", onWinResize);
  });

  function onResizePointerDown(e: PointerEvent) {
    if (e.button !== 0) return;
    e.preventDefault();
    const el = e.currentTarget as HTMLElement;
    const startX = e.clientX;
    const startW = panelWidthPx;
    resizing = true;
    el.setPointerCapture(e.pointerId);

    function onMove(ev: PointerEvent) {
      const dx = ev.clientX - startX;
      panelWidthPx = clampPanelW(startW - dx);
    }

    function onUp() {
      resizing = false;
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("pointercancel", onUp);
      if (typeof localStorage !== "undefined") {
        localStorage.setItem(HISTORY_PANEL_W_KEY, String(panelWidthPx));
      }
      try {
        el.releasePointerCapture(e.pointerId);
      } catch {
        /* released */
      }
    }

    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", onUp);
  }

  function formatTime(unix: number): string {
    try {
      return new Date(unix * 1000).toLocaleString();
    } catch {
      return String(unix);
    }
  }

  const parsedDiff = $derived(parseGitPatch(diffText || ""));
  const diffFileCount = $derived(parsedDiff.files.length);

  const diffSubtitleLine = $derived.by(() => {
    void locale.value;
    const n = diffFileCount;
    const scope =
      n <= 0
        ? t("history.diffFilesNone")
        : n === 1
          ? t("history.diffFilesOne")
          : t("history.diffFilesOther", { n });
    return t("history.diffSubtitle", { scope });
  });

  function gutterChar(line: DiffViewLine): string {
    switch (line.kind) {
      case "add":
        return "+";
      case "del":
        return "−";
      case "ctx":
        return " ";
      case "hunk":
        return "@";
      default:
        return " ";
    }
  }

  function lineClass(line: DiffViewLine): string {
    switch (line.kind) {
      case "add":
        return "diff-line add";
      case "del":
        return "diff-line del";
      case "ctx":
        return "diff-line ctx";
      case "hunk":
        return "diff-line hunk";
      default:
        return "diff-line meta";
    }
  }

  function formatDiffLine(line: DiffViewLine): string {
    const raw = line.raw;
    if (line.kind === "add" && raw.startsWith("+")) return raw.slice(1);
    if (line.kind === "del" && raw.startsWith("-")) return raw.slice(1);
    if (line.kind === "ctx" && raw.startsWith(" ")) return raw.slice(1);
    return raw;
  }
</script>

{#if open}
  {#key locale.value}
    <div
      class="panel"
      class:resizing
      style:--panel-w="{panelWidthPx}px"
      role="dialog"
      aria-modal="false"
      aria-labelledby="hist-title"
    >
      <div
        class="resize-edge"
        role="separator"
        aria-orientation="vertical"
        aria-label={t("history.resizePanel")}
        onpointerdown={onResizePointerDown}
      ></div>
      <div class="head">
        <div class="head-titles">
          <h2 id="hist-title">{t("history.title")}</h2>
          {#if historyRefExists && tipShort}
            <p class="tip-line">
              {t("history.currentTip")}{" "}
              <code class="tip-hash">{tipShort}</code>
            </p>
          {:else if !historyRefExists}
            <p class="tip-line muted-tip">{t("history.noTipYet")}</p>
          {/if}
        </div>
        <div class="head-actions">
          <button type="button" class="ghost" disabled={busy} onclick={onSnapshot}
            >{t("history.checkpoint")}</button
          >
          <button type="button" class="ghost" disabled={busy} onclick={onRefresh}
            >{t("history.refresh")}</button
          >
          <button type="button" class="ghost" onclick={onClose}>{t("settings.close")}</button>
        </div>
      </div>
      {#if refreshing}
        <div class="refresh-bar" role="status" aria-live="polite">{t("history.refreshing")}</div>
      {/if}
      <p class="hint">
        {t("history.hintIntro")}
        <code>refs/paperdesk/history</code>
        {t("history.hintMid")}<code>HEAD</code>{t("history.hintOutro")}
      </p>
      <div class="list-wrap">
        {#if busy && commits.length === 0}
          <p class="muted">{t("history.loading")}</p>
        {:else if commits.length === 0}
          <p class="muted">{t("history.empty")}</p>
        {:else}
          <ul class="timeline" aria-busy={busy}>
            {#each commits as c, i (c.id)}
              {@const display = checkpointDisplayLabel(c.message, t)}
            <li class="tl-item">
              <div class="tl-rail" aria-hidden="true">
                <span class="tl-dot"></span>
                {#if i < commits.length - 1}
                  <span class="tl-stem"></span>
                {/if}
              </div>
              <article class="tl-card">
                <div class="tl-card-head">
                  <span class="hash" title={c.id}>{c.shortId}</span>
                  <span class="badge" title={display.raw}>{display.label}</span>
                  <time class="time" datetime={new Date(c.timeUnix * 1000).toISOString()}
                    >{formatTime(c.timeUnix)}</time
                  >
                </div>
                {#if !c.message.trim().toLowerCase().startsWith("paperdesk:")}
                  <p class="tl-msg">{c.message}</p>
                {/if}
                <div class="tl-actions">
                  <button type="button" class="mini" onclick={() => onRequestDiff(c.id)}
                    >{t("history.diff")}</button
                  >
                  <button type="button" class="mini danger" onclick={() => onRestore(c.id)}
                    >{t("history.restore")}</button
                  >
                </div>
              </article>
            </li>
          {/each}
        </ul>
        {/if}
      </div>
    </div>
  {/key}
{/if}

{#if diffOpen}
  <div
    class="diff-backdrop"
    role="presentation"
    onclick={(e) => e.target === e.currentTarget && onCloseDiff()}
  ></div>
  {#key locale.value}
    <div class="diff-modal" role="dialog" aria-modal="true" aria-labelledby="diff-title">
      <div class="diff-head">
        <div class="diff-head-main">
          <h2 id="diff-title">{t("history.changes")}</h2>
          <p class="diff-subtitle">
            {diffSubtitleLine}
          </p>
        </div>
        <div class="diff-head-actions">
          <button
            type="button"
            class="diff-close"
            onclick={onCloseDiff}
            aria-label={t("history.closeDiffAria")}
          >
            {t("settings.close")}
          </button>
        </div>
      </div>
      {#if parsedDiff.truncated}
        <div class="diff-truncation" role="status">
          {t("history.diffTruncation")}
        </div>
      {/if}
      <div class="diff-body">
        {#if parsedDiff.preamble && !parsedDiff.files.length}
          <pre class="diff-fallback">{diffText || t("history.diffEmpty")}</pre>
        {:else if parsedDiff.preamble}
          <pre class="diff-preamble">{parsedDiff.preamble}</pre>
        {/if}
        {#if parsedDiff.files.length === 0 && !parsedDiff.preamble}
          <p class="diff-empty">{t("history.diffNoChanges")}</p>
        {:else}
          {#each parsedDiff.files as file (file.headerLine)}
            <section class="diff-file">
              <header class="diff-file-head">
                <span class="diff-file-path" title={file.path}>{file.path}</span>
              </header>
              {#if file.metaBeforeHunks.length > 0}
                <details class="diff-meta">
                  <summary>{t("history.diffMetaSummary")}</summary>
                <div class="diff-meta-inner">
                  {#each file.metaBeforeHunks as line (line.raw)}
                    <div class={lineClass(line)}>
                      <span class="diff-gutter diff-gutter-meta" aria-hidden="true"> </span>
                      <code>{line.raw}</code>
                    </div>
                  {/each}
                </div>
              </details>
            {/if}
            {#each file.hunks as hunk (hunk.header)}
              <div class="diff-hunk">
                <div class="diff-line hunk">
                  <span class="diff-gutter" aria-hidden="true">@</span>
                  <code>{hunk.header}</code>
                </div>
                {#each hunk.lines as line, li (`${hunk.header}-${li}-${line.raw.slice(0, 40)}`)}
                  <div class={lineClass(line)}>
                    <span class="diff-gutter" aria-hidden="true">{gutterChar(line)}</span>
                    <code>{formatDiffLine(line)}</code>
                  </div>
                {/each}
              </div>
            {/each}
          </section>
        {/each}
        {/if}
      </div>
    </div>
  {/key}
{/if}

<style>
  /* Fullscreen dimming only for the diff overlay; main history pane floats like the AI chat. */
  .diff-backdrop {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.45);
    z-index: 125;
  }

  .panel {
    position: fixed;
    right: 1rem;
    top: 4rem;
    bottom: 1rem;
    left: auto;
    width: min(var(--panel-w, 440px), calc(100vw - 2rem));
    min-width: min(240px, calc(100vw - 2rem));
    z-index: 118;
    display: flex;
    flex-direction: column;
    border-radius: 10px;
    border: 1px solid var(--pd-border);
    background: var(--pd-surface);
    color: var(--pd-text);
    box-shadow:
      0 0 0 1px rgb(0 0 0 / 0.06),
      0 16px 48px rgb(0 0 0 / 0.28);
    min-height: 0;
  }

  .panel.resizing {
    user-select: none;
  }

  .resize-edge {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 8px;
    z-index: 3;
    cursor: ew-resize;
    touch-action: none;
    border-radius: 10px 0 0 10px;
  }

  .resize-edge:hover,
  .panel.resizing .resize-edge {
    background: color-mix(in srgb, var(--pd-accent, var(--pd-text)) 14%, transparent);
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--pd-border);
    flex-shrink: 0;
  }

  .head-titles {
    min-width: 0;
  }

  .head h2 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
  }

  .tip-line {
    margin: 0.2rem 0 0;
    font-size: 0.85rem;
    color: var(--pd-muted);
  }

  .muted-tip {
    opacity: 0.9;
  }

  .tip-hash {
    font-family: var(--pd-mono);
    font-size: 0.9em;
    color: var(--pd-accent);
  }

  .head-actions {
    display: flex;
    gap: 0.25rem;
    flex-shrink: 0;
  }

  .ghost {
    padding: 0.25rem 0.45rem;
    border: none;
    background: transparent;
    color: var(--pd-muted);
    cursor: pointer;
    font-size: 1rem;
  }

  .ghost:hover:not(:disabled) {
    color: var(--pd-text);
  }

  .ghost:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .refresh-bar {
    flex-shrink: 0;
    padding: 0.35rem 1rem;
    font-size: 0.85rem;
    color: var(--pd-muted);
    background: color-mix(in srgb, var(--pd-accent) 12%, transparent);
    border-bottom: 1px solid var(--pd-border);
  }

  .hint {
    margin: 0;
    padding: 0.5rem 1rem;
    font-size: 0.9rem;
    color: var(--pd-muted);
    line-height: 1.35;
    border-bottom: 1px solid color-mix(in srgb, var(--pd-border) 55%, transparent);
    flex-shrink: 0;
  }

  .hint code {
    font-family: var(--pd-mono);
    font-size: 0.95em;
    color: var(--pd-text);
  }

  .list-wrap {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .muted {
    margin: 0;
    padding: 1rem;
    color: var(--pd-muted);
    font-size: 1rem;
  }

  .timeline {
    list-style: none;
    margin: 0;
    padding: 0.5rem 0.5rem 0.75rem;
    overflow: auto;
    flex: 1;
    min-height: 0;
  }

  .tl-item {
    display: flex;
    gap: 0.5rem;
    align-items: stretch;
  }

  .tl-rail {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 14px;
    flex-shrink: 0;
    padding-top: 0.5rem;
  }

  .tl-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--pd-accent);
    flex-shrink: 0;
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--pd-accent) 35%, transparent);
  }

  .tl-stem {
    flex: 1;
    width: 2px;
    min-height: 0.5rem;
    margin-top: 2px;
    background: color-mix(in srgb, var(--pd-border) 80%, var(--pd-muted));
    border-radius: 1px;
  }

  .tl-card {
    flex: 1;
    min-width: 0;
    margin-bottom: 0.5rem;
    padding: 0.55rem 0.65rem;
    border-radius: 8px;
    border: 1px solid var(--pd-border);
    background: var(--pd-bg);
  }

  .tl-card-head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.35rem 0.5rem;
    margin-bottom: 0.35rem;
  }

  .hash {
    font-family: var(--pd-mono);
    font-size: 0.9rem;
    color: var(--pd-accent);
    font-weight: 500;
  }

  .badge {
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    padding: 0.12rem 0.4rem;
    border-radius: 4px;
    background: color-mix(in srgb, var(--pd-accent) 18%, transparent);
    color: var(--pd-text);
    border: 1px solid color-mix(in srgb, var(--pd-accent) 40%, var(--pd-border));
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .time {
    font-size: 0.8rem;
    color: var(--pd-muted);
    margin-left: auto;
  }

  .tl-msg {
    margin: 0 0 0.4rem;
    font-size: 0.85rem;
    color: var(--pd-muted);
    word-break: break-word;
    line-height: 1.3;
  }

  .tl-actions {
    display: flex;
    gap: 0.35rem;
    flex-wrap: wrap;
  }

  .mini {
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    border: 1px solid var(--pd-border);
    background: var(--pd-surface);
    color: var(--pd-text);
    font-size: 0.9rem;
    cursor: pointer;
  }

  .mini:hover {
    border-color: var(--pd-muted);
  }

  .mini.danger {
    border-color: color-mix(in srgb, var(--pd-error) 45%, var(--pd-border));
    color: var(--pd-error);
  }

  .diff-modal {
    position: fixed;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    z-index: 135;
    width: min(960px, calc(100vw - 1.5rem));
    max-height: min(85vh, 800px);
    display: flex;
    flex-direction: column;
    border-radius: 12px;
    border: 1px solid var(--pd-border);
    background: var(--pd-surface);
    color: var(--pd-text);
    box-shadow:
      0 8px 32px rgb(0 0 0 / 0.4),
      0 0 0 1px color-mix(in srgb, var(--pd-text) 6%, transparent);
  }

  .diff-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.85rem 1rem;
    border-bottom: 1px solid var(--pd-border);
    flex-shrink: 0;
    background: color-mix(in srgb, var(--pd-bg) 40%, var(--pd-surface));
  }

  .diff-head-main {
    min-width: 0;
  }

  .diff-head h2 {
    margin: 0;
    font-size: 1.05rem;
    font-weight: 650;
    letter-spacing: -0.02em;
  }

  .diff-subtitle {
    margin: 0.2rem 0 0;
    font-size: 0.8rem;
    color: var(--pd-muted);
    line-height: 1.35;
  }

  .diff-head-actions {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }

  .diff-close {
    padding: 0.35rem 0.75rem;
    border-radius: 8px;
    border: 1px solid var(--pd-border);
    background: var(--pd-surface);
    color: var(--pd-text);
    font-size: 0.85rem;
    font-weight: 500;
    cursor: pointer;
    transition:
      border-color 0.12s ease,
      background 0.12s ease;
  }

  .diff-close:hover {
    border-color: var(--pd-muted);
    background: color-mix(in srgb, var(--pd-bg) 55%, var(--pd-surface));
  }

  .diff-close:focus-visible {
    outline: 2px solid var(--pd-accent);
    outline-offset: 2px;
  }

  .diff-truncation {
    flex-shrink: 0;
    padding: 0.45rem 0.85rem;
    font-size: 0.85rem;
    color: var(--pd-text);
    background: color-mix(in srgb, var(--pd-error) 12%, transparent);
    border-bottom: 1px solid var(--pd-border);
  }

  .diff-body {
    flex: 1;
    min-height: 0;
    overflow: auto;
    background: var(--pd-bg);
  }

  .diff-empty {
    margin: 0;
    padding: 1rem 0.85rem;
    color: var(--pd-muted);
    font-size: 0.95rem;
  }

  .diff-fallback,
  .diff-preamble {
    margin: 0;
    padding: 0.75rem;
    font-family: var(--pd-mono);
    font-size: 0.85rem;
    line-height: 1.4;
    white-space: pre;
    overflow-x: auto;
    word-break: normal;
  }

  .diff-preamble {
    border-bottom: 1px solid var(--pd-border);
    color: var(--pd-muted);
  }

  .diff-file {
    border-bottom: 1px solid var(--pd-border);
  }

  .diff-file:last-child {
    border-bottom: none;
  }

  .diff-file-head {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem 0.5rem 0.65rem;
    background: color-mix(in srgb, var(--pd-surface) 88%, var(--pd-bg));
    border-bottom: 1px solid var(--pd-border);
    font-family: var(--pd-mono);
    font-size: 0.82rem;
    font-weight: 600;
    box-shadow: 0 1px 0 color-mix(in srgb, var(--pd-border) 50%, transparent);
  }

  .diff-file-head::before {
    content: "";
    width: 3px;
    align-self: stretch;
    min-height: 1.1rem;
    border-radius: 2px;
    background: var(--pd-accent);
    flex-shrink: 0;
  }

  .diff-file-path {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  .diff-meta {
    margin: 0;
    border-bottom: 1px solid color-mix(in srgb, var(--pd-border) 70%, transparent);
    font-size: 0.8rem;
  }

  .diff-meta > summary {
    cursor: pointer;
    list-style: none;
    padding: 0.4rem 0.75rem;
    color: var(--pd-muted);
    user-select: none;
  }

  .diff-meta > summary::-webkit-details-marker {
    display: none;
  }

  .diff-meta > summary::before {
    content: "▸ ";
    display: inline-block;
    width: 1em;
    transition: transform 0.12s ease;
  }

  .diff-meta[open] > summary::before {
    transform: rotate(90deg);
  }

  .diff-meta > summary:hover {
    color: var(--pd-text);
    background: color-mix(in srgb, var(--pd-muted) 6%, transparent);
  }

  .diff-meta-inner {
    padding: 0 0 0.35rem;
    border-top: 1px solid color-mix(in srgb, var(--pd-border) 55%, transparent);
  }

  .diff-hunk {
    margin: 0;
  }

  .diff-line {
    display: grid;
    grid-template-columns: 1.35rem minmax(0, 1fr);
    align-items: stretch;
    font-family: var(--pd-mono);
    font-size: 0.8rem;
    line-height: 1.5;
    margin: 0;
    border-left: 3px solid transparent;
  }

  .diff-gutter {
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 0.06rem;
    font-weight: 700;
    font-size: 0.78rem;
    color: var(--pd-muted);
    user-select: none;
    border-right: 1px solid color-mix(in srgb, var(--pd-border) 65%, transparent);
    background: color-mix(in srgb, var(--pd-bg) 70%, var(--pd-surface));
  }

  .diff-gutter-meta {
    opacity: 0.35;
  }

  .diff-line.add .diff-gutter {
    color: color-mix(in srgb, #27ae60 85%, var(--pd-text));
  }

  .diff-line.del .diff-gutter {
    color: color-mix(in srgb, var(--pd-error) 90%, var(--pd-text));
  }

  .diff-line.hunk .diff-gutter {
    color: var(--pd-accent);
    font-size: 0.7rem;
  }

  .diff-line code {
    padding: 0.06rem 0.5rem 0.06rem 0.4rem;
    white-space: pre;
    overflow-x: auto;
    display: block;
    min-width: 0;
  }

  .diff-line.add {
    border-left-color: color-mix(in srgb, #2ecc71 75%, var(--pd-border));
    background: color-mix(in srgb, #2ecc71 16%, transparent);
  }

  .diff-line.del {
    border-left-color: color-mix(in srgb, var(--pd-error) 80%, var(--pd-border));
    background: color-mix(in srgb, var(--pd-error) 14%, transparent);
  }

  .diff-line.ctx {
    border-left-color: transparent;
    background: color-mix(in srgb, var(--pd-bg) 35%, transparent);
  }

  .diff-line.hunk {
    border-left-color: var(--pd-accent);
    background: color-mix(in srgb, var(--pd-accent) 12%, transparent);
    font-weight: 600;
  }

  .diff-line.meta {
    border-left-color: transparent;
    color: var(--pd-muted);
    background: color-mix(in srgb, var(--pd-muted) 6%, transparent);
  }
</style>
