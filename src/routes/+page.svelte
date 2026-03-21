<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import {
    getRecentProjects,
    openProject,
    pickProjectFolder,
    createFromTemplate,
  } from "$lib/tauri/api";

  let recent = $state<string[]>([]);
  let busy = $state(false);
  let error = $state<string | null>(null);

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
      const p = await pickProjectFolder();
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

  async function newFromTemplate(id: "article" | "thesis") {
    error = null;
    busy = true;
    try {
      const p = await pickProjectFolder();
      if (!p) return;
      await createFromTemplate(id, p);
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
    <h1>PaperDesk</h1>
    <p class="tagline">Local Typst writing environment</p>
  </header>

  {#if error}
    <p class="err" role="alert">{error}</p>
  {/if}

  <section class="actions">
    <button type="button" class="primary" disabled={busy} onclick={openFolder}>
      Open project folder
    </button>
    <div class="templates">
      <span class="label">New from template</span>
      <button type="button" disabled={busy} onclick={() => newFromTemplate("article")}>
        Article
      </button>
      <button type="button" disabled={busy} onclick={() => newFromTemplate("thesis")}>
        Thesis
      </button>
    </div>
  </section>

  <section class="recent">
    <h2>Recent</h2>
    {#if recent.length === 0}
      <p class="muted">No recent projects yet.</p>
    {:else}
      <ul>
        {#each recent as p (p)}
          <li>
            <button type="button" class="linkish" disabled={busy} onclick={() => openRecent(p)}>
              {p}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</main>

<style>
  .hub {
    max-width: 42rem;
    margin: 0 auto;
    padding: 3rem 1.5rem;
  }

  .hub-header h1 {
    margin: 0 0 0.35rem;
    font-weight: 600;
    letter-spacing: -0.02em;
    font-size: 2rem;
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

  .actions {
    margin-top: 2rem;
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
    font-size: 0.8rem;
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

  .recent {
    margin-top: 2.5rem;
  }

  .recent h2 {
    font-size: 0.85rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--pd-muted);
    margin: 0 0 0.75rem;
  }

  .muted {
    color: var(--pd-muted);
    margin: 0;
  }

  .recent ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .recent li {
    margin-bottom: 0.35rem;
  }

  .linkish {
    background: none;
    border: none;
    color: var(--pd-accent);
    padding: 0.2rem 0;
    text-align: left;
    font-size: 0.95rem;
  }

  .linkish:hover:not(:disabled) {
    text-decoration: underline;
  }
</style>
