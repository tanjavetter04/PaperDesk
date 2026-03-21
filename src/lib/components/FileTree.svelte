<script lang="ts">
  import type { ProjectEntry } from "$lib/tauri/api";
  import { buildFileTree, type FileTreeNode } from "$lib/editor/buildFileTree";

  let {
    entries,
    selectedFilePath,
    targetDirPath,
    onSelectFile,
    onTargetDirChange,
    onNewFile,
    onNewFolder,
    onMoveFile,
  }: {
    entries: ProjectEntry[];
    selectedFilePath: string | null;
    /** Default folder for new items (`""` = project root). */
    targetDirPath: string;
    onSelectFile: (path: string) => void;
    onTargetDirChange: (dirPath: string) => void;
    onNewFile: () => void;
    onNewFolder: () => void;
    onMoveFile: (destinationDir: string) => void;
  } = $props();

  let expanded = $state<Record<string, boolean>>({ "": true });
  let moveOpen = $state(false);
  let moveDestDir = $state("");

  const tree = $derived(buildFileTree(entries));

  const folderPaths = $derived(
    [...new Set(entries.filter((e) => e.isDir).map((e) => e.path))].sort(
      (a, b) => a.localeCompare(b, undefined, { sensitivity: "base" }),
    ),
  );

  function parentDirOfFile(path: string): string {
    const i = path.lastIndexOf("/");
    return i === -1 ? "" : path.slice(0, i);
  }

  function toggleDir(path: string) {
    expanded[path] = !expanded[path];
    expanded = expanded;
  }

  function rowClick(node: FileTreeNode) {
    if (node.isDir) {
      onTargetDirChange(node.path);
      toggleDir(node.path);
    } else {
      onSelectFile(node.path);
      onTargetDirChange(parentDirOfFile(node.path));
    }
  }

  function dirExpanded(path: string): boolean {
    return expanded[path] ?? false;
  }

  const canMoveSelection = $derived(
    selectedFilePath != null &&
      selectedFilePath !== "main.typ" &&
      !entries.find((e) => e.path === selectedFilePath && e.isDir),
  );

  const moveButtonTooltip = $derived.by(() => {
    if (!selectedFilePath) {
      return "Keine Datei ausgewählt.";
    }
    if (selectedFilePath === "main.typ") {
      return "main.typ kann nicht verschoben werden.";
    }
    if (entries.find((e) => e.path === selectedFilePath && e.isDir)) {
      return "Ordner können hier nicht verschoben werden.";
    }
    return "Ausgewählte Datei verschieben";
  });

  function openMoveDialog() {
    moveDestDir = targetDirPath;
    moveOpen = true;
  }

  function confirmMove() {
    moveOpen = false;
    onMoveFile(moveDestDir);
  }
</script>

<div class="tree">
  <div class="tree-head">
    <span class="tree-title">Files</span>
    <span class="tree-actions">
      <button type="button" class="icon-btn" title="Neue Datei" onclick={() => onNewFile()}>+</button>
      <button type="button" class="icon-btn" title="Neuer Ordner" onclick={() => onNewFolder()}>
        <svg
          class="folder-add-icon"
          viewBox="0 0 24 24"
          width="15"
          height="15"
          aria-hidden="true"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M12 10v6" />
          <path d="M9 13h6" />
          <path
            d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"
          />
        </svg>
      </button>
      <span class="move-wrap" title={moveButtonTooltip}>
        <button
          type="button"
          class="icon-btn"
          title={moveButtonTooltip}
          disabled={!canMoveSelection}
          onclick={openMoveDialog}
        >↗</button>
      </span>
    </span>
  </div>
  {#if targetDirPath}
    <div class="target-hint" title="Ziel für neue Dateien/Ordner">
      Neu in: <code>{targetDirPath}</code>
    </div>
  {:else}
    <div class="target-hint" title="Ziel für neue Dateien/Ordner">Neu im Projektstamm</div>
  {/if}

  <ul class="tree-root">
    {#each tree as node (node.path)}
      <li>
        {#if node.isDir}
          {@render dirRow(node, 0)}
        {:else}
          {@render fileRow(node, 0)}
        {/if}
      </li>
    {/each}
  </ul>
</div>

{#snippet caret(node: FileTreeNode)}
  {#if node.children.length}
    <span class="caret" class:open={dirExpanded(node.path)} aria-hidden="true">▸</span>
  {:else}
    <span class="caret spacer"></span>
  {/if}
{/snippet}

{#snippet folderGlyph()}
  <!-- Gefüllt: Tab + breiter Block — eindeutig „Ordner“, nicht Blatt -->
  <svg
    class="tree-svg tree-svg--folder"
    viewBox="0 0 24 24"
    width="18"
    height="15"
    aria-hidden="true"
  >
    <g fill="currentColor">
      <rect x="2" y="9" width="20" height="13" rx="2.25" ry="2.25" />
      <rect x="2" y="4" width="11" height="7" rx="1.75" ry="1.75" />
    </g>
  </svg>
{/snippet}

{#snippet fileGlyph()}
  <!-- Schmale Seite, umgeklappter Rand + horizontale Zeilen = Textdatei -->
  <svg
    class="tree-svg tree-svg--file"
    viewBox="0 0 24 24"
    width="13"
    height="13"
    aria-hidden="true"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
  >
    <path d="M14.5 2H7a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V7.5L14.5 2Z" />
    <path d="M14 2v6h6" />
    <path d="M8 10h6M8 14h8M8 18h5" />
  </svg>
{/snippet}

{#snippet fileRow(node: FileTreeNode, depth: number)}
  <button
    type="button"
    class="row file"
    class:sel={node.path === selectedFilePath}
    style:padding-left={`${0.5 + depth * 0.75}rem`}
    onclick={() => rowClick(node)}
  >
    <span class="caret spacer"></span>
    <span class="row-icon">{@render fileGlyph()}</span>
    <span class="label">{node.name}</span>
  </button>
{/snippet}

{#snippet dirRow(node: FileTreeNode, depth: number)}
  <button
    type="button"
    class="row dir"
    class:expanded={dirExpanded(node.path)}
    class:sel={node.path === targetDirPath}
    style:padding-left={`${0.5 + depth * 0.75}rem`}
    onclick={() => rowClick(node)}
  >
    {@render caret(node)}
    <span class="row-icon">{@render folderGlyph()}</span>
    <span class="label folder-name">{node.name}</span>
  </button>
  {#if dirExpanded(node.path) && node.children.length}
    <ul class="nested">
      {#each node.children as ch (ch.path)}
        <li>
          {#if ch.isDir}
            {@render dirRow(ch, depth + 1)}
          {:else}
            {@render fileRow(ch, depth + 1)}
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
{/snippet}

{#if moveOpen}
  <div
    class="modal-backdrop"
    role="presentation"
    onclick={(e) => e.target === e.currentTarget && (moveOpen = false)}
  ></div>
  <div class="modal" role="dialog" aria-labelledby="move-dlg-title">
    <h2 id="move-dlg-title">Datei verschieben</h2>
    <p class="modal-sub">
      {#if selectedFilePath}
        <code>{selectedFilePath}</code>
      {/if}
    </p>
    <label class="field">
      Zielordner
      <select bind:value={moveDestDir} class="select">
        <option value="">(Projektstamm)</option>
        {#each folderPaths as fp (fp)}
          <option value={fp}>{fp}</option>
        {/each}
      </select>
    </label>
    <div class="modal-btns">
      <button type="button" class="ghost" onclick={() => (moveOpen = false)}>Abbrechen</button>
      <button type="button" class="primary" onclick={confirmMove}>Verschieben</button>
    </div>
  </div>
{/if}

<style>
  .tree {
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-right: 1px solid var(--pd-border);
    background: var(--pd-surface);
    position: relative;
  }

  .tree-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.35rem;
    padding: 0.5rem 0.65rem;
    font-size: 1rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--pd-muted);
    border-bottom: 1px solid var(--pd-border);
  }

  .tree-title {
    flex-shrink: 0;
  }

  .tree-actions {
    display: flex;
    gap: 0.2rem;
  }

  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 1.65rem;
    height: 1.65rem;
    padding: 0 0.35rem;
    border-radius: 4px;
    border: 1px solid var(--pd-border);
    background: var(--pd-bg);
    color: var(--pd-text);
    font-size: 1rem;
    line-height: 1;
    cursor: pointer;
  }

  .folder-add-icon {
    flex-shrink: 0;
    opacity: 0.92;
  }

  .icon-btn:hover:not(:disabled) .folder-add-icon {
    opacity: 1;
  }

  .icon-btn:hover:not(:disabled) {
    border-color: var(--pd-muted);
  }

  .icon-btn:disabled {
    opacity: 0.35;
    cursor: default;
  }

  /* Damit das title-Tooltip auch bei disabled zuverlässig erscheint (Hover trifft den Wrapper). */
  .move-wrap {
    display: inline-flex;
    vertical-align: top;
  }

  .move-wrap:has(.icon-btn:disabled) {
    cursor: default;
  }

  .move-wrap .icon-btn:disabled {
    pointer-events: none;
  }

  .target-hint {
    font-size: 1rem;
    color: var(--pd-muted);
    padding: 0.35rem 0.75rem 0.25rem;
    border-bottom: 1px solid color-mix(in srgb, var(--pd-border) 55%, transparent);
  }

  .target-hint code {
    font-family: var(--pd-mono);
    font-size: 1em;
    color: var(--pd-text);
  }

  .tree-root,
  .nested {
    list-style: none;
    margin: 0;
    padding: 0.35rem 0;
    overflow: visible;
  }

  .nested {
    margin: 0.05rem 0 0.1rem 0.2rem;
    padding: 0.08rem 0 0.12rem 0.55rem;
    border-left: 1px solid color-mix(in srgb, var(--pd-accent) 18%, var(--pd-border));
  }

  .tree-root {
    flex: 1;
    overflow: auto;
    min-height: 0;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 0.15rem;
    width: 100%;
    text-align: left;
    padding: 0.3rem 0.65rem 0.3rem 0.5rem;
    border: none;
    border-radius: 5px;
    background: transparent;
    color: var(--pd-text);
    font-size: 1rem;
    font-family: var(--pd-mono);
    cursor: pointer;
  }

  .row:hover {
    background: color-mix(in srgb, var(--pd-text) 6%, transparent);
  }

  .row.sel {
    background: color-mix(in srgb, var(--pd-accent) 22%, transparent);
    color: var(--pd-text);
  }

  .row.dir:hover {
    background: color-mix(in srgb, var(--pd-accent) 7%, var(--pd-surface));
  }

  .row.dir.sel {
    background: color-mix(in srgb, var(--pd-accent) 16%, var(--pd-surface));
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--pd-accent) 28%, transparent);
  }

  .row-icon {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 1.15rem;
  }

  .tree-svg--folder {
    color: color-mix(in srgb, var(--pd-accent) 38%, var(--pd-muted));
    opacity: 0.95;
  }

  .row.dir.expanded .tree-svg--folder {
    color: color-mix(in srgb, var(--pd-accent) 58%, var(--pd-muted));
  }

  .row.dir.sel .tree-svg--folder {
    color: color-mix(in srgb, var(--pd-accent) 72%, var(--pd-text));
  }

  .tree-svg--file {
    color: var(--pd-muted);
    opacity: 0.88;
  }

  .row.file.sel .tree-svg--file {
    color: color-mix(in srgb, var(--pd-text) 70%, var(--pd-accent));
    opacity: 1;
  }

  .folder-name {
    font-weight: 600;
    font-family: var(--pd-font), system-ui, sans-serif;
    letter-spacing: 0.02em;
  }

  .row.file .label {
    font-weight: 400;
  }

  .caret {
    flex-shrink: 0;
    width: 0.85rem;
    display: inline-block;
    font-size: 1rem;
    color: var(--pd-muted);
    transition: transform 0.12s ease;
  }

  .caret.open {
    transform: rotate(90deg);
  }

  .caret.spacer {
    visibility: hidden;
  }

  .label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.45);
    z-index: 80;
  }

  .modal {
    position: fixed;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    z-index: 90;
    min-width: min(320px, calc(100vw - 2rem));
    padding: 1rem 1.1rem;
    border-radius: 8px;
    border: 1px solid var(--pd-border);
    background: var(--pd-surface);
    color: var(--pd-text);
    box-shadow: 0 12px 40px rgb(0 0 0 / 0.35);
  }

  .modal h2 {
    margin: 0 0 0.35rem;
    font-size: 1rem;
    font-weight: 600;
  }

  .modal-sub {
    margin: 0 0 0.75rem;
    font-size: 1rem;
    color: var(--pd-muted);
    word-break: break-all;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    font-size: 1rem;
    color: var(--pd-muted);
    margin-bottom: 0.85rem;
  }

  .select {
    padding: 0.4rem 0.5rem;
    padding-right: 1.85rem;
    border-radius: 6px;
    border: 1px solid var(--pd-border);
    background-color: var(--pd-bg);
    color: var(--pd-text);
    font-size: 1rem;
    font-family: var(--pd-font), system-ui, sans-serif;
    color-scheme: dark;
    accent-color: var(--pd-accent);
    appearance: none;
    -webkit-appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 12 12' fill='none'%3E%3Cpath d='M3 4.5L6 7.5L9 4.5' stroke='%23868e96' stroke-width='1.25' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 0.45rem center;
    cursor: pointer;
  }

  .select:focus {
    outline: 2px solid color-mix(in srgb, var(--pd-accent) 45%, transparent);
    outline-offset: 1px;
  }

  .select option {
    background-color: var(--pd-surface);
    color: var(--pd-text);
  }

  .modal-btns {
    display: flex;
    justify-content: flex-end;
    gap: 0.45rem;
  }

  .modal-btns .ghost {
    padding: 0.4rem 0.65rem;
    border: none;
    background: transparent;
    color: var(--pd-muted);
    cursor: pointer;
    font-size: 1rem;
  }

  .modal-btns .ghost:hover {
    color: var(--pd-text);
  }

  .modal-btns .primary {
    padding: 0.4rem 0.75rem;
    border-radius: 6px;
    border: 1px solid var(--pd-border);
    background: color-mix(in srgb, var(--pd-accent) 18%, var(--pd-bg));
    color: var(--pd-text);
    cursor: pointer;
    font-size: 1rem;
  }

  .modal-btns .primary:hover {
    border-color: var(--pd-muted);
  }
</style>
