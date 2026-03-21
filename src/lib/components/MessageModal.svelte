<script lang="ts">
  let {
    open,
    message,
    title = "Hinweis",
    onClose,
  }: {
    open: boolean;
    message: string;
    title?: string;
    onClose: () => void;
  } = $props();
</script>

{#if open}
  <div
    class="backdrop"
    role="presentation"
    onclick={(e) => e.target === e.currentTarget && onClose()}
  ></div>
  <div class="modal" role="alertdialog" aria-modal="true" aria-labelledby="msg-modal-title">
    <h2 id="msg-modal-title">{title}</h2>
    <p class="body">{message}</p>
    <div class="btns">
      <button type="button" class="primary" onclick={onClose}>OK</button>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.45);
    z-index: 120;
  }

  .modal {
    position: fixed;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    z-index: 130;
    min-width: min(320px, calc(100vw - 2rem));
    max-width: min(420px, calc(100vw - 2rem));
    padding: 1rem 1.1rem;
    border-radius: 8px;
    border: 1px solid var(--pd-border);
    background: var(--pd-surface);
    color: var(--pd-text);
    box-shadow: 0 12px 40px rgb(0 0 0 / 0.35);
  }

  .modal h2 {
    margin: 0 0 0.5rem;
    font-size: 1rem;
    font-weight: 600;
  }

  .body {
    margin: 0 0 0.9rem;
    font-size: 1rem;
    line-height: 1.45;
    color: var(--pd-text);
    white-space: pre-wrap;
    word-break: break-word;
  }

  .btns {
    display: flex;
    justify-content: flex-end;
  }

  .primary {
    padding: 0.4rem 0.85rem;
    border-radius: 6px;
    border: 1px solid var(--pd-border);
    background: color-mix(in srgb, var(--pd-accent) 18%, var(--pd-bg));
    color: var(--pd-text);
    cursor: pointer;
    font-size: 1rem;
  }

  .primary:hover {
    border-color: var(--pd-muted);
  }
</style>
