<script lang="ts">
  import { locale, setLocale, t } from "$lib/i18n/locale.svelte";
  import type { Locale } from "$lib/i18n/messages";

  let {
    open,
    onClose,
  }: {
    open: boolean;
    onClose: () => void;
  } = $props();

  function pick(next: Locale) {
    setLocale(next);
  }
</script>

{#if open}
  <div
    class="backdrop"
    role="presentation"
    onclick={(e) => e.target === e.currentTarget && onClose()}
  ></div>
  <div
    class="modal"
    role="dialog"
    aria-modal="true"
    aria-labelledby="settings-title"
  >
    <h2 id="settings-title">{t("settings.title")}</h2>
    <label class="field">
      {t("settings.language")}
      <select
        class="select"
        value={locale.value}
        onchange={(e) => pick(e.currentTarget.value as Locale)}
      >
        <option value="de">{t("settings.languageDe")}</option>
        <option value="en">{t("settings.languageEn")}</option>
      </select>
    </label>
    <div class="btns">
      <button type="button" class="primary" onclick={onClose}>
        {t("settings.close")}
      </button>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.45);
    z-index: 200;
  }

  .modal {
    position: fixed;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    z-index: 210;
    min-width: min(340px, calc(100vw - 2rem));
    padding: 1rem 1.1rem;
    border-radius: 8px;
    border: 1px solid var(--pd-border);
    background: var(--pd-surface);
    color: var(--pd-text);
    box-shadow: 0 12px 40px rgb(0 0 0 / 0.35);
  }

  .modal h2 {
    margin: 0 0 0.75rem;
    font-size: 1rem;
    font-weight: 600;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    font-size: 1rem;
    color: var(--pd-muted);
    margin-bottom: 1rem;
  }

  .select {
    padding: 0.45rem 0.55rem;
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
