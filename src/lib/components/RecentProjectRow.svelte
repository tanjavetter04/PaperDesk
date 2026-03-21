<script lang="ts">
  import { compileProjectAtPath } from "$lib/tauri/api";
  import { renderFirstPageThumbFromBase64 } from "$lib/pdf/renderFirstPageThumb";

  let {
    path,
    displayName,
    busy,
    onclick,
    onRename,
    onDuplicate,
    onDelete,
    renameAria,
    duplicateAria,
    deleteAria,
  }: {
    path: string;
    displayName: string;
    busy: boolean;
    onclick: () => void;
    onRename: () => void;
    onDuplicate: () => void;
    onDelete: () => void;
    renameAria: string;
    duplicateAria: string;
    deleteAria: string;
  } = $props();

  let thumbUrl = $state<string | null>(null);
  let thumbPhase = $state<"loading" | "ready" | "miss">("loading");

  $effect(() => {
    const projectPath = path;
    thumbPhase = "loading";
    thumbUrl = null;
    const ac = { cancelled: false };

    void (async () => {
      try {
        const r = await compileProjectAtPath(projectPath);
        if (ac.cancelled) return;
        if (!r.ok || !r.pdf_base64) {
          thumbPhase = "miss";
          return;
        }
        const dataUrl = await renderFirstPageThumbFromBase64(r.pdf_base64, 132);
        if (ac.cancelled) return;
        if (dataUrl) {
          thumbUrl = dataUrl;
          thumbPhase = "ready";
        } else {
          thumbPhase = "miss";
        }
      } catch {
        if (!ac.cancelled) thumbPhase = "miss";
      }
    })();

    return () => {
      ac.cancelled = true;
    };
  });
</script>

<li class="recent-item">
  <div class="recent-row">
    <button type="button" class="recent-card" disabled={busy} {onclick}>
      <div class="recent-thumb" aria-hidden="true">
        {#if thumbPhase === "loading"}
          <span class="thumb-skel"></span>
        {:else if thumbUrl}
          <img src={thumbUrl} alt="" class="thumb-img" />
        {:else}
          <span class="thumb-fallback">PDF</span>
        {/if}
      </div>
      <span class="recent-title">{displayName}</span>
      <span class="recent-path">{path}</span>
    </button>
    <div class="recent-actions">
      <button
        type="button"
        class="recent-action"
        disabled={busy}
        onclick={onDuplicate}
        title={duplicateAria}
        aria-label={duplicateAria}
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
          <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
          <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
        </svg>
      </button>
      <button
        type="button"
        class="recent-action"
        disabled={busy}
        onclick={onRename}
        title={renameAria}
        aria-label={renameAria}
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
      <button
        type="button"
        class="recent-action recent-action-danger"
        disabled={busy}
        onclick={onDelete}
        title={deleteAria}
        aria-label={deleteAria}
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
          <path d="M3 6h18" />
          <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
          <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
          <line x1="10" x2="10" y1="11" y2="17" />
          <line x1="14" x2="14" y1="11" y2="17" />
        </svg>
      </button>
    </div>
  </div>
</li>

<style>
  .recent-item {
    list-style: none;
    margin: 0;
    min-width: 0;
  }

  .recent-row {
    position: relative;
    display: inline-block;
    min-width: 0;
    vertical-align: top;
  }

  .recent-actions {
    position: absolute;
    top: 0.35rem;
    right: 0.45rem;
    z-index: 3;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.28rem;
    transition:
      opacity 0.12s ease,
      visibility 0.12s ease;
  }

  .recent-action {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 2.25rem;
    height: 2.25rem;
    padding: 0;
    border-radius: 8px;
    border: 1px solid var(--pd-border);
    background: color-mix(in srgb, var(--pd-surface) 92%, transparent);
    backdrop-filter: blur(6px);
    color: var(--pd-muted);
    cursor: pointer;
    box-shadow: 0 2px 10px rgb(0 0 0 / 0.2);
  }

  .recent-action-danger:hover:not(:disabled) {
    border-color: color-mix(in srgb, var(--pd-error) 45%, var(--pd-border));
    color: var(--pd-error);
  }

  @media (hover: hover) {
    .recent-actions {
      opacity: 0;
      visibility: hidden;
      pointer-events: none;
    }

    .recent-row:hover .recent-actions,
    .recent-row:focus-within .recent-actions {
      opacity: 1;
      visibility: visible;
      pointer-events: auto;
    }

    .recent-row:hover .recent-actions .recent-action:disabled,
    .recent-row:focus-within .recent-actions .recent-action:disabled {
      opacity: 0.45;
    }
  }

  .recent-action:hover:not(:disabled) {
    border-color: var(--pd-muted);
    color: var(--pd-text);
    background: var(--pd-bg);
  }

  .recent-action:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .recent-card {
    --recent-pad-x: 0.65rem;
    /* Inner row width matches fixed thumb (132px); + button border (1px each side). */
    box-sizing: border-box;
    width: calc(134px + 2 * var(--recent-pad-x));
    max-width: 100%;
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 0.45rem;
    padding: 0.55rem var(--recent-pad-x) 0.6rem;
    overflow: visible;
    position: relative;
    z-index: 0;
    border-radius: 8px;
    border: 1px solid var(--pd-border);
    background: var(--pd-surface);
    color: var(--pd-text);
    text-align: left;
    cursor: pointer;
  }

  .recent-card:hover:not(:disabled) {
    border-color: var(--pd-muted);
    background: var(--pd-bg);
    z-index: 1;
  }

  .recent-card:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .recent-thumb {
    box-sizing: border-box;
    width: 132px;
    max-width: 100%;
    aspect-ratio: 3 / 4;
    max-height: 7.5rem;
    border-radius: 5px;
    background: color-mix(in srgb, var(--pd-muted) 12%, var(--pd-bg));
    border: 1px solid var(--pd-border);
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    align-self: center;
    flex-shrink: 0;
  }

  .thumb-skel {
    width: 100%;
    height: 100%;
    min-height: 4rem;
    border-radius: 6px;
    background: linear-gradient(
      110deg,
      color-mix(in srgb, var(--pd-muted) 18%, transparent) 0%,
      color-mix(in srgb, var(--pd-muted) 32%, transparent) 45%,
      color-mix(in srgb, var(--pd-muted) 18%, transparent) 90%
    );
  }

  .thumb-img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: contain;
    object-position: center top;
  }

  .thumb-fallback {
    font-size: 1rem;
    font-weight: 600;
    letter-spacing: 0.06em;
    color: var(--pd-muted);
    text-transform: uppercase;
  }

  .recent-title {
    font-weight: 500;
    font-size: 1rem;
    line-height: 1.25;
    display: block;
    width: 100%;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .recent-path {
    font-size: 1rem;
    color: var(--pd-muted);
    line-height: 1.3;
    width: 100%;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
