<script lang="ts">
  import type { HistoryCommitSummary } from "$lib/tauri/api";

  let {
    open,
    commits,
    busy,
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
    diffText: string;
    diffOpen: boolean;
    onClose: () => void;
    onRefresh: () => void;
    onSnapshot: () => void;
    onRequestDiff: (commitId: string) => void;
    onCloseDiff: () => void;
    onRestore: (commitId: string) => void;
  } = $props();

  function formatTime(unix: number): string {
    try {
      return new Date(unix * 1000).toLocaleString();
    } catch {
      return String(unix);
    }
  }
</script>

{#if open}
  <div
    class="backdrop"
    role="presentation"
    onclick={(e) => e.target === e.currentTarget && onClose()}
  ></div>
  <div class="panel" role="dialog" aria-modal="true" aria-labelledby="hist-title">
    <div class="head">
      <h2 id="hist-title">Verlauf</h2>
      <div class="head-actions">
        <button type="button" class="ghost" disabled={busy} onclick={onSnapshot}>Checkpoint</button>
        <button type="button" class="ghost" disabled={busy} onclick={onRefresh}>Aktualisieren</button>
        <button type="button" class="ghost" onclick={onClose}>Schließen</button>
      </div>
    </div>
    <p class="hint">
      Checkpoints werden unter <code>refs/paperdesk/history</code> gespeichert und ändern deinen
      normalen Git-<code>HEAD</code> nicht.
    </p>
    {#if busy}
      <p class="muted">Laden…</p>
    {:else if commits.length === 0}
      <p class="muted">Noch keine Checkpoints.</p>
    {:else}
      <ul class="list">
        {#each commits as c (c.id)}
          <li class="row">
            <div class="meta">
              <span class="hash">{c.shortId}</span>
              <span class="time">{formatTime(c.timeUnix)}</span>
            </div>
            <div class="msg">{c.message}</div>
            <div class="actions">
              <button type="button" class="mini" onclick={() => onRequestDiff(c.id)}>Diff</button>
              <button type="button" class="mini danger" onclick={() => onRestore(c.id)}>Wiederherstellen</button>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}

{#if diffOpen}
  <div
    class="backdrop diff-backdrop"
    role="presentation"
    onclick={(e) => e.target === e.currentTarget && onCloseDiff()}
  ></div>
  <div class="diff-modal" role="dialog" aria-modal="true" aria-labelledby="diff-title">
    <div class="diff-head">
      <h2 id="diff-title">Diff (Checkpoint → Arbeitsverzeichnis)</h2>
      <button type="button" class="ghost" onclick={onCloseDiff}>Schließen</button>
    </div>
    <pre class="diff-pre">{diffText || "(leer)"}</pre>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.45);
    z-index: 110;
  }

  .diff-backdrop {
    z-index: 125;
  }

  .panel {
    position: fixed;
    right: 1rem;
    top: 4rem;
    bottom: 1rem;
    width: min(420px, calc(100vw - 2rem));
    z-index: 115;
    display: flex;
    flex-direction: column;
    border-radius: 8px;
    border: 1px solid var(--pd-border);
    background: var(--pd-surface);
    color: var(--pd-text);
    box-shadow: 0 12px 40px rgb(0 0 0 / 0.35);
    min-height: 0;
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

  .head h2 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
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

  .hint {
    margin: 0;
    padding: 0.5rem 1rem;
    font-size: 1rem;
    color: var(--pd-muted);
    line-height: 1.35;
    border-bottom: 1px solid color-mix(in srgb, var(--pd-border) 55%, transparent);
  }

  .hint code {
    font-family: var(--pd-mono);
    font-size: 0.95em;
    color: var(--pd-text);
  }

  .muted {
    margin: 0;
    padding: 1rem;
    color: var(--pd-muted);
    font-size: 1rem;
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0.5rem;
    overflow: auto;
    flex: 1;
    min-height: 0;
  }

  .row {
    padding: 0.55rem 0.5rem;
    border-radius: 6px;
    border: 1px solid var(--pd-border);
    margin-bottom: 0.45rem;
    background: var(--pd-bg);
  }

  .meta {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    margin-bottom: 0.25rem;
  }

  .hash {
    font-family: var(--pd-mono);
    font-size: 1rem;
    color: var(--pd-accent);
  }

  .time {
    font-size: 1rem;
    color: var(--pd-muted);
  }

  .msg {
    font-size: 1rem;
    color: var(--pd-text);
    word-break: break-word;
    margin-bottom: 0.45rem;
  }

  .actions {
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
    font-size: 1rem;
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
    width: min(900px, calc(100vw - 2rem));
    max-height: min(80vh, 720px);
    display: flex;
    flex-direction: column;
    border-radius: 8px;
    border: 1px solid var(--pd-border);
    background: var(--pd-surface);
    color: var(--pd-text);
    box-shadow: 0 12px 40px rgb(0 0 0 / 0.35);
  }

  .diff-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.65rem 0.85rem;
    border-bottom: 1px solid var(--pd-border);
    flex-shrink: 0;
  }

  .diff-head h2 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
  }

  .diff-pre {
    margin: 0;
    padding: 0.75rem;
    overflow: auto;
    flex: 1;
    min-height: 0;
    font-family: var(--pd-mono);
    font-size: 1rem;
    line-height: 1.35;
    white-space: pre-wrap;
    word-break: break-word;
    background: var(--pd-bg);
  }
</style>
