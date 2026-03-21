<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import {
    getRecentProjects,
    openProject,
    pickProjectFolder,
    createFromTemplate,
    createEmptyProject,
  } from "$lib/tauri/api";
  import RecentProjectRow from "$lib/components/RecentProjectRow.svelte";
  import { t } from "$lib/i18n/locale.svelte";
  import { openSettingsModal } from "$lib/settingsModal.svelte";

  let recent = $state<string[]>([]);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let selectedFolder = $state<string | null>(null);

  const RECENT_HUB_LIMIT = 6;

  function normalizePath(p: string): string {
    return p.trim().replace(/\\/g, "/").replace(/\/+$/, "") || "/";
  }

  /** Parent directory using / as separator (works with Rust paths on any OS). */
  function parentDir(path: string): string {
    const norm = path.trim().replace(/\\/g, "/").replace(/\/+$/, "");
    const idx = norm.lastIndexOf("/");
    if (idx < 0) return norm;
    if (idx === 0) return "/";
    return norm.slice(0, idx);
  }

  function isProjectInsideFolder(folder: string, projectPath: string): boolean {
    const f = normalizePath(folder);
    const p = normalizePath(projectPath);
    if (p === f) return true;
    return p.startsWith(f + "/");
  }

  function folderDisplayName(folderPath: string): string {
    const n = normalizePath(folderPath);
    if (n === "/") return "/";
    const parts = n.split("/").filter(Boolean);
    return parts.length ? parts[parts.length - 1]! : n;
  }

  const recentOnHub = $derived(recent.slice(0, RECENT_HUB_LIMIT));

  const allFolders = $derived.by(() => {
    const seen = new Set<string>();
    const out: string[] = [];
    for (const p of recent) {
      const par = parentDir(p);
      if (!par || seen.has(par)) continue;
      seen.add(par);
      out.push(par);
    }
    out.sort((a, b) =>
      a.localeCompare(b, undefined, { sensitivity: "base" }),
    );
    return out;
  });

  const projectsInSelectedFolder = $derived.by(() => {
    const folder = selectedFolder;
    if (!folder) return [];
    return recent
      .filter((p) => isProjectInsideFolder(folder, p))
      .sort((a, b) =>
        a.localeCompare(b, undefined, { sensitivity: "base" }),
      );
  });

  function selectFolder(path: string) {
    selectedFolder = path;
  }

  function clearFolderSelection() {
    selectedFolder = null;
  }

  async function refresh() {
    try {
      recent = await getRecentProjects();
    } catch (e) {
      error = String(e);
    }
  }

  onMount(() => {
    void refresh();
  });

  async function openFolder() {
    error = null;
    busy = true;
    try {
      const p = await pickProjectFolder(t("dialog.projectFolder"));
      if (!p) return;
      await openProject(p);
      await goto("/project");
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function openRecent(p: string) {
    error = null;
    busy = true;
    try {
      await openProject(p);
      await goto("/project");
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function newFromThesisTemplate() {
    error = null;
    busy = true;
    try {
      const p = await pickProjectFolder(t("dialog.projectFolder"));
      if (!p) return;
      await createFromTemplate("thesis", p);
      await goto("/project");
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function newEmptyProject() {
    error = null;
    busy = true;
    try {
      const p = await pickProjectFolder(t("dialog.projectFolder"));
      if (!p) return;
      await createEmptyProject(p);
      await goto("/project");
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<main class="hub">
  <header class="hub-header">
    <div class="hub-header-top">
      <h1>{t("app.title")}</h1>
      <button
        type="button"
        class="hub-settings-btn"
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
    </div>
    <p class="tagline">{t("app.tagline")}</p>
  </header>

  {#if error}
    <p class="err" role="alert">{error}</p>
  {/if}

  <div class="hub-actions-wrap">
    <section class="actions">
      <button type="button" class="primary" disabled={busy} onclick={openFolder}>
        {t("hub.openProjectFolder")}
      </button>
      <div class="templates">
        <span class="label">{t("hub.newProject")}</span>
        <button type="button" disabled={busy} onclick={newFromThesisTemplate}>
          {t("hub.thesisTemplate")}
        </button>
        <button type="button" disabled={busy} onclick={newEmptyProject}>
          {t("hub.emptyProject")}
        </button>
      </div>
    </section>
  </div>

  <div class="hub-stack">
    <section class="recent">
      <h2>{t("hub.recentlyOpened")}</h2>
      <p class="section-hint">
        {t("hub.lastNProjects", { n: RECENT_HUB_LIMIT })}
      </p>
      {#if recent.length === 0}
        <p class="muted">{t("hub.noRecentYet")}</p>
      {:else}
        <ul class="recent-card-list">
          {#each recentOnHub as p (p)}
            <RecentProjectRow
              path={p}
              displayName={folderDisplayName(p)}
              {busy}
              onclick={() => openRecent(p)}
            />
          {/each}
        </ul>
      {/if}
    </section>

    <section class="folders">
      <h2>{t("hub.folders")}</h2>
      <p class="section-hint">{t("hub.foldersHint")}</p>

      {#if selectedFolder}
        <div class="folder-detail">
          <button
            type="button"
            class="back"
            disabled={busy}
            onclick={clearFolderSelection}
          >
            {t("hub.allFolders")}
          </button>
          <header class="folder-detail-header">
            <span class="folder-detail-kicker">{t("hub.currentFolder")}</span>
            <h3 class="folder-detail-title" title={selectedFolder}>
              {folderDisplayName(selectedFolder)}
            </h3>
            <p class="folder-detail-path">{selectedFolder}</p>
          </header>
          {#if projectsInSelectedFolder.length === 0}
            <p class="folder-projects-empty muted">
              {t("hub.noProjectsInFolder")}
            </p>
          {:else}
            <div class="folder-projects-block">
              <h4 class="folder-projects-label">
                {t("hub.projectsInFolder")}
              </h4>
              <ul class="folder-project-list">
                {#each projectsInSelectedFolder as p (p)}
                  <li class="folder-project-item">
                    <button
                      type="button"
                      class="folder-project-btn"
                      disabled={busy}
                      onclick={() => openRecent(p)}
                    >
                      <span class="folder-project-name">{folderDisplayName(p)}</span>
                      <span class="folder-project-path">{p}</span>
                    </button>
                  </li>
                {/each}
              </ul>
            </div>
          {/if}
        </div>
      {:else if recent.length === 0}
        <p class="muted">{t("hub.openProjectToSeeFolders")}</p>
      {:else if allFolders.length === 0}
        <p class="muted">{t("hub.noParentFolders")}</p>
      {:else}
        <ul class="folder-list">
          {#each allFolders as f (f)}
            <li>
              <button
                type="button"
                class="folder-row"
                disabled={busy}
                title={f}
                onclick={() => selectFolder(f)}
              >
                <span class="folder-row-name">{folderDisplayName(f)}</span>
                <span class="folder-row-path">{f}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  </div>
</main>

<style>
  .hub {
    /* 6 recent tiles (134px + 2×0.65rem) + 5×1rem gap + horizontal padding; small buffer */
    max-width: calc(6 * (134px + 1.3rem) + 5 * 1rem + 2 * 1.5rem + 0.5rem);
    margin: 0 auto;
    padding: 3rem 1.5rem;
  }

  .hub-stack {
    margin-top: 2.5rem;
    display: flex;
    flex-direction: column;
    gap: 2.25rem;
  }

  .recent-card-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: 0.85rem 1rem;
  }

  .section-hint {
    margin: -0.35rem 0 0.65rem;
    font-size: 1rem;
    color: var(--pd-muted);
    line-height: 1.35;
  }

  .hub-header-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 0.35rem;
  }

  .hub-header-top h1 {
    margin: 0;
    flex: 1;
    min-width: 0;
    font-weight: 600;
    letter-spacing: -0.02em;
    font-size: 2rem;
    line-height: 1.15;
  }

  .tagline {
    margin: 0;
    color: var(--pd-muted);
    font-size: 1rem;
  }

  .err {
    color: var(--pd-error);
    margin-top: 1.25rem;
  }

  .hub-actions-wrap {
    margin-top: 2rem;
  }

  .hub-settings-btn {
    flex-shrink: 0;
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

  .hub-settings-btn:hover {
    color: var(--pd-text);
    border-color: color-mix(in srgb, var(--pd-accent) 58%, var(--pd-border));
    background: color-mix(in srgb, var(--pd-accent) 30%, var(--pd-surface));
  }

  .hub-settings-btn:focus-visible {
    outline: 2px solid color-mix(in srgb, var(--pd-accent) 55%, transparent);
    outline-offset: 2px;
  }

  .actions {
    margin-top: 0;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .primary {
    padding: 0.65rem 1.1rem;
    border-radius: 8px;
    border: 1px solid var(--pd-accent);
    background: color-mix(in srgb, var(--pd-accent) 18%, transparent);
    color: var(--pd-text);
    font-weight: 500;
  }

  .primary:hover:not(:disabled) {
    background: color-mix(in srgb, var(--pd-accent) 28%, transparent);
  }

  .primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .templates {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem 0.75rem;
    padding: 1rem;
    background: var(--pd-surface);
    border: 1px solid var(--pd-border);
    border-radius: 10px;
  }

  .templates .label {
    width: 100%;
    font-size: 1rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--pd-muted);
  }

  .templates button {
    padding: 0.45rem 0.85rem;
    border-radius: 6px;
    border: 1px solid var(--pd-border);
    background: var(--pd-bg);
    color: var(--pd-text);
  }

  .templates button:hover:not(:disabled) {
    border-color: var(--pd-muted);
  }

  .recent h2 {
    font-size: 1rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--pd-muted);
    margin: 0 0 0.75rem;
  }

  .muted {
    color: var(--pd-muted);
    margin: 0;
  }

  .folder-list,
  .folder-project-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .folder-list li {
    margin-bottom: 0.35rem;
  }

  .folder-project-list .folder-project-item {
    margin-bottom: 0.3rem;
  }

  .folder-project-list .folder-project-item:last-child {
    margin-bottom: 0;
  }

  .folders h2 {
    font-size: 1rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--pd-muted);
    margin: 0 0 0.75rem;
  }

  .folder-row {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.2rem;
    padding: 0.55rem 0.65rem;
    border-radius: 8px;
    border: 1px solid var(--pd-border);
    background: var(--pd-surface);
    color: var(--pd-text);
    text-align: left;
    cursor: pointer;
  }

  .folder-row:hover:not(:disabled) {
    border-color: var(--pd-muted);
  }

  .folder-row:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .folder-row-name {
    font-weight: 500;
    font-size: 1rem;
  }

  .folder-row-path {
    font-size: 1rem;
    color: var(--pd-muted);
    word-break: break-all;
    line-height: 1.3;
  }

  .folder-detail {
    padding: 0.35rem 0 0;
  }

  .folder-detail-header {
    padding: 0.5rem 0.65rem 0.55rem;
    border-radius: 6px;
    border: 1px solid var(--pd-border);
    border-left: 2px solid color-mix(in srgb, var(--pd-accent) 55%, var(--pd-border));
    background: var(--pd-surface);
  }

  .folder-detail-kicker {
    display: block;
    margin: 0 0 0.2rem;
    font-size: 1rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--pd-muted);
  }

  .back {
    background: none;
    border: none;
    color: var(--pd-accent);
    padding: 0 0 0.5rem;
    font-size: 1rem;
    cursor: pointer;
  }

  .back:hover:not(:disabled) {
    text-decoration: underline;
  }

  .back:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .folder-detail-title {
    margin: 0 0 0.25rem;
    font-size: 1rem;
    font-weight: 600;
    letter-spacing: -0.015em;
    line-height: 1.25;
    color: var(--pd-text);
  }

  .folder-detail-path {
    margin: 0;
    font-size: 1rem;
    color: var(--pd-muted);
    word-break: break-all;
    line-height: 1.35;
  }

  .folder-projects-block {
    margin-top: 0.85rem;
    padding-top: 0.75rem;
    border-top: 1px solid var(--pd-border);
  }

  .folder-projects-label {
    margin: 0 0 0.45rem;
    font-size: 1rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--pd-muted);
  }

  .folder-projects-empty {
    margin-top: 0.75rem;
    padding-top: 0.75rem;
    border-top: 1px solid var(--pd-border);
    font-size: 1rem;
    line-height: 1.45;
  }

  .folder-project-btn {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.12rem;
    padding: 0.38rem 0.5rem;
    border-radius: 6px;
    border: 1px solid var(--pd-border);
    background: var(--pd-surface);
    color: var(--pd-text);
    text-align: left;
    font-size: 1rem;
    cursor: pointer;
  }

  .folder-project-btn:hover:not(:disabled) {
    border-color: var(--pd-muted);
    background: var(--pd-bg);
  }

  .folder-project-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .folder-project-name {
    font-weight: 500;
    color: var(--pd-text);
    font-size: 1rem;
  }

  .folder-project-path {
    font-size: 1rem;
    color: var(--pd-muted);
    word-break: break-all;
    line-height: 1.28;
  }
</style>
