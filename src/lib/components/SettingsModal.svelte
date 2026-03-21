<script lang="ts">
  import {
    appSettings,
    clearDefaultProjectDir,
    setDefaultProjectDir,
    setFontSizePx,
    setTheme,
    type ThemeMode,
  } from "$lib/appSettings.svelte";
  import { locale, setLocale, t } from "$lib/i18n/locale.svelte";
  import type { Locale } from "$lib/i18n/messages";
  import { pickProjectFolder } from "$lib/tauri/api";

  let {
    open,
    onClose,
  }: {
    open: boolean;
    onClose: () => void;
  } = $props();

  function pickLocale(next: Locale) {
    setLocale(next);
  }

  function pickTheme(next: ThemeMode) {
    setTheme(next);
  }

  async function chooseDefaultProjectFolder() {
    const p = await pickProjectFolder(t("dialog.projectFolder"), {
      defaultPath: appSettings.defaultProjectDir.trim() || undefined,
    });
    if (p) setDefaultProjectDir(p);
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
        onchange={(e) => pickLocale(e.currentTarget.value as Locale)}
      >
        <option value="de">{t("settings.languageDe")}</option>
        <option value="en">{t("settings.languageEn")}</option>
      </select>
    </label>
    <label class="field">
      {t("settings.theme")}
      <select
        class="select"
        value={appSettings.theme}
        onchange={(e) => pickTheme(e.currentTarget.value as ThemeMode)}
      >
        <option value="dark">{t("settings.themeDark")}</option>
        <option value="light">{t("settings.themeLight")}</option>
      </select>
    </label>
    <label class="field">
      {t("settings.fontSize")}
      <div class="range-row">
        <input
          type="range"
          class="range"
          min="12"
          max="22"
          step="1"
          value={appSettings.fontSizePx}
          aria-valuemin={12}
          aria-valuemax={22}
          aria-valuenow={appSettings.fontSizePx}
          oninput={(e) => setFontSizePx(+e.currentTarget.value)}
        />
        <span class="range-value">{appSettings.fontSizePx}px</span>
      </div>
    </label>
    <div class="field">
      <span class="field-label">{t("settings.defaultProjectFolder")}</span>
      <p
        class="path-preview"
        title={appSettings.defaultProjectDir || undefined}
      >
        {appSettings.defaultProjectDir.trim()
          ? appSettings.defaultProjectDir
          : t("settings.defaultFolderNone")}
      </p>
      <div class="folder-btns">
        <button
          type="button"
          class="secondary"
          onclick={() => void chooseDefaultProjectFolder()}
        >
          {t("settings.chooseDefaultFolder")}
        </button>
        {#if appSettings.defaultProjectDir.trim()}
          <button type="button" class="ghost" onclick={clearDefaultProjectDir}>
            {t("settings.clearDefaultFolder")}
          </button>
        {/if}
      </div>
    </div>
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
    min-width: min(380px, calc(100vw - 2rem));
    max-height: min(90vh, 640px);
    overflow: auto;
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

  .field-label {
    display: block;
  }

  .path-preview {
    margin: 0;
    padding: 0.45rem 0.55rem;
    border-radius: 6px;
    border: 1px solid var(--pd-border);
    background: var(--pd-bg);
    color: var(--pd-text);
    font-size: 0.85rem;
    font-family: var(--pd-mono), monospace;
    word-break: break-all;
    line-height: 1.35;
  }

  .folder-btns {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-top: 0.35rem;
  }

  .range-row {
    display: flex;
    align-items: center;
    gap: 0.65rem;
  }

  .range {
    flex: 1;
    min-width: 0;
    accent-color: var(--pd-accent);
    height: 1.25rem;
  }

  .range-value {
    flex-shrink: 0;
    font-variant-numeric: tabular-nums;
    color: var(--pd-text);
    font-size: 0.95rem;
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
    accent-color: var(--pd-accent);
    appearance: none;
    -webkit-appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 12 12' fill='none'%3E%3Cpath d='M3 4.5L6 7.5L9 4.5' stroke='%23868e96' stroke-width='1.25' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 0.45rem center;
    cursor: pointer;
  }

  :global(:root[data-theme="light"]) .select {
    color-scheme: light;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 12 12' fill='none'%3E%3Cpath d='M3 4.5L6 7.5L9 4.5' stroke='%236c757d' stroke-width='1.25' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E");
  }

  :global(:root[data-theme="dark"]) .select {
    color-scheme: dark;
  }

  .select:focus {
    outline: 2px solid color-mix(in srgb, var(--pd-accent) 45%, transparent);
    outline-offset: 1px;
  }

  .btns {
    display: flex;
    justify-content: flex-end;
  }

  .primary,
  .secondary,
  .ghost {
    padding: 0.4rem 0.85rem;
    border-radius: 6px;
    border: 1px solid var(--pd-border);
    font-size: 1rem;
    cursor: pointer;
  }

  .primary {
    background: color-mix(in srgb, var(--pd-accent) 18%, var(--pd-bg));
    color: var(--pd-text);
  }

  .primary:hover {
    border-color: var(--pd-muted);
  }

  .secondary {
    background: var(--pd-bg);
    color: var(--pd-text);
  }

  .secondary:hover {
    border-color: var(--pd-muted);
  }

  .ghost {
    background: transparent;
    color: var(--pd-muted);
    border-color: transparent;
  }

  .ghost:hover {
    color: var(--pd-text);
    border-color: var(--pd-border);
  }
</style>
