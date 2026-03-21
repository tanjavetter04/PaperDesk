<script lang="ts">
  import { tick } from "svelte";

  let {
    open,
    title,
    hint = "",
    initialValue = "",
    submitLabel = "OK",
    onSubmit,
    onClose,
  }: {
    open: boolean;
    title: string;
    hint?: string;
    initialValue?: string;
    submitLabel?: string;
    onSubmit: (value: string) => void;
    onClose: () => void;
  } = $props();

  let field = $state("");
  let inputEl = $state<HTMLInputElement | null>(null);

  $effect(() => {
    if (open) {
      field = initialValue;
      void tick().then(() => {
        inputEl?.focus();
        inputEl?.select();
      });
    }
  });

  function submit() {
    onSubmit(field);
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
    if (e.key === "Enter") {
      e.preventDefault();
      submit();
    }
  }
</script>

{#if open}
  <div
    class="backdrop"
    role="presentation"
    onclick={(e) => e.target === e.currentTarget && onClose()}
  ></div>
  <div class="modal" role="dialog" aria-modal="true" aria-labelledby="input-modal-title">
    <h2 id="input-modal-title">{title}</h2>
    {#if hint}
      <p class="hint">{hint}</p>
    {/if}
    <input
      bind:this={inputEl}
      class="field"
      type="text"
      spellcheck="false"
      autocomplete="off"
      bind:value={field}
      onkeydown={onKeydown}
    />
    <div class="btns">
      <button type="button" class="ghost" onclick={onClose}>Abbrechen</button>
      <button type="button" class="primary" onclick={submit}>{submitLabel}</button>
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
    min-width: min(340px, calc(100vw - 2rem));
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

  .hint {
    margin: 0 0 0.65rem;
    font-size: 1rem;
    color: var(--pd-muted);
    line-height: 1.35;
  }

  .field {
    box-sizing: border-box;
    width: 100%;
    margin-bottom: 0.85rem;
    padding: 0.45rem 0.55rem;
    border-radius: 6px;
    border: 1px solid var(--pd-border);
    background: var(--pd-bg);
    color: var(--pd-text);
    font-size: 1rem;
    font-family: var(--pd-mono);
  }

  .field:focus {
    outline: 2px solid color-mix(in srgb, var(--pd-accent) 45%, transparent);
    outline-offset: 1px;
  }

  .btns {
    display: flex;
    justify-content: flex-end;
    gap: 0.45rem;
  }

  .ghost {
    padding: 0.4rem 0.65rem;
    border: none;
    background: transparent;
    color: var(--pd-muted);
    cursor: pointer;
    font-size: 1rem;
  }

  .ghost:hover {
    color: var(--pd-text);
  }

  .primary {
    padding: 0.4rem 0.75rem;
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
