<script lang="ts">
  import { compileProjectAtPath } from "$lib/tauri/api";
  import { renderFirstPageThumbFromBase64 } from "$lib/pdf/renderFirstPageThumb";

  let {
    path,
    displayName,
    busy,
    onclick,
  }: {
    path: string;
    displayName: string;
    busy: boolean;
    onclick: () => void;
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
</li>

<style>
  .recent-item {
    list-style: none;
    margin: 0;
    min-width: 0;
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
    overflow: hidden;
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
    animation: thumb-pulse 1.1s ease-in-out infinite;
  }

  @keyframes thumb-pulse {
    0%,
    100% {
      opacity: 0.55;
    }
    50% {
      opacity: 1;
    }
  }

  .thumb-img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: contain;
    object-position: center top;
  }

  .thumb-fallback {
    font-size: 0.65rem;
    font-weight: 600;
    letter-spacing: 0.06em;
    color: var(--pd-muted);
    text-transform: uppercase;
  }

  .recent-title {
    font-weight: 500;
    font-size: 0.9rem;
    line-height: 1.25;
    width: 100%;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .recent-path {
    font-size: 0.72rem;
    color: var(--pd-muted);
    line-height: 1.3;
    width: 100%;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
