<script lang="ts">
  import {
    appSettings,
    clearDefaultProjectDir,
    setDefaultProjectDir,
    setFontSizePx,
    setSpellcheckLanguage,
    setTheme,
    setZoteroBibRelativePath,
    type SpellcheckLanguage,
    type ThemeMode,
  } from "$lib/appSettings.svelte";
  import { locale, setLocale, t } from "$lib/i18n/locale.svelte";
  import type { Locale } from "$lib/i18n/messages";
  import { pickProjectFolder, restartBibWatcher } from "$lib/tauri/api";

  let {
    open,
    onClose,
  }: {
    open: boolean;
    onClose: () => void;
  } = $props();

  /** Native `<select>` popups stay dark on some WebKit/GTK builds; custom list uses app colors only. */
  let localeMenuOpen = $state(false);
  let themeMenuOpen = $state(false);
  let spellMenuOpen = $state(false);
  let langRoot = $state<HTMLDivElement | null>(null);
  let themeRoot = $state<HTMLDivElement | null>(null);
  let spellRoot = $state<HTMLDivElement | null>(null);

  $effect(() => {
    if (!open) {
      localeMenuOpen = false;
      themeMenuOpen = false;
      spellMenuOpen = false;
    }
  });

  function onDocPointerDown(e: PointerEvent) {
    if (!localeMenuOpen && !themeMenuOpen && !spellMenuOpen) return;
    const node = e.target as Node;
    if (localeMenuOpen && langRoot && !langRoot.contains(node)) {
      localeMenuOpen = false;
    }
    if (themeMenuOpen && themeRoot && !themeRoot.contains(node)) {
      themeMenuOpen = false;
    }
    if (spellMenuOpen && spellRoot && !spellRoot.contains(node)) {
      spellMenuOpen = false;
    }
  }

  function pickLocale(next: Locale) {
    setLocale(next);
    localeMenuOpen = false;
  }

  function pickTheme(next: ThemeMode) {
    setTheme(next);
    themeMenuOpen = false;
  }

  function spellLangLabel(lang: SpellcheckLanguage): string {
    if (lang === "off") return t("settings.spellLangOff");
    if (lang === "de") return t("settings.spellLangDe");
    return t("settings.spellLangEn");
  }

  function pickSpellLang(next: SpellcheckLanguage) {
    setSpellcheckLanguage(next);
    spellMenuOpen = false;
  }

  async function chooseDefaultProjectFolder() {
    const p = await pickProjectFolder(t("dialog.newProjectParentFolder"), {
      defaultPath: appSettings.defaultProjectDir.trim() || undefined,
    });
    if (p) setDefaultProjectDir(p);
  }

  let bibDraft = $state(appSettings.zoteroBibRelativePath);

  $effect(() => {
    if (open) bibDraft = appSettings.zoteroBibRelativePath;
  });

  async function applyBibPath() {
    const next = setZoteroBibRelativePath(bibDraft);
    bibDraft = next;
    try {
      await restartBibWatcher(next);
    } catch {
      /* no project open or watcher failed — ignore */
    }
  }
</script>

<svelte:window onpointerdown={onDocPointerDown} />

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
      <div class="custom-select" bind:this={langRoot}>
        <button
          type="button"
          class="custom-select-trigger"
          aria-haspopup="listbox"
          aria-expanded={localeMenuOpen}
          aria-label={t("settings.language")}
          onclick={() => {
            themeMenuOpen = false;
            spellMenuOpen = false;
            localeMenuOpen = !localeMenuOpen;
          }}
        >
          <span
            >{locale.value === "de"
              ? t("settings.languageDe")
              : t("settings.languageEn")}</span
          >
        </button>
        {#if localeMenuOpen}
          <div class="custom-select-list" role="listbox">
            <button
              type="button"
              role="option"
              class="custom-select-option"
              aria-selected={locale.value === "de"}
              onclick={() => pickLocale("de")}
            >
              {t("settings.languageDe")}
            </button>
            <button
              type="button"
              role="option"
              class="custom-select-option"
              aria-selected={locale.value === "en"}
              onclick={() => pickLocale("en")}
            >
              {t("settings.languageEn")}
            </button>
          </div>
        {/if}
      </div>
    </label>
    <label class="field">
      {t("settings.theme")}
      <div class="custom-select" bind:this={themeRoot}>
        <button
          type="button"
          class="custom-select-trigger"
          aria-haspopup="listbox"
          aria-expanded={themeMenuOpen}
          aria-label={t("settings.theme")}
          onclick={() => {
            localeMenuOpen = false;
            spellMenuOpen = false;
            themeMenuOpen = !themeMenuOpen;
          }}
        >
          <span
            >{appSettings.theme === "dark"
              ? t("settings.themeDark")
              : t("settings.themeLight")}</span
          >
        </button>
        {#if themeMenuOpen}
          <div class="custom-select-list" role="listbox">
            <button
              type="button"
              role="option"
              class="custom-select-option"
              aria-selected={appSettings.theme === "dark"}
              onclick={() => pickTheme("dark")}
            >
              {t("settings.themeDark")}
            </button>
            <button
              type="button"
              role="option"
              class="custom-select-option"
              aria-selected={appSettings.theme === "light"}
              onclick={() => pickTheme("light")}
            >
              {t("settings.themeLight")}
            </button>
          </div>
        {/if}
      </div>
    </label>
    <label class="field">
      {t("settings.spellcheckLanguage")}
      <div class="custom-select" bind:this={spellRoot}>
        <button
          type="button"
          class="custom-select-trigger"
          aria-haspopup="listbox"
          aria-expanded={spellMenuOpen}
          aria-label={t("settings.spellcheckLanguage")}
          onclick={() => {
            localeMenuOpen = false;
            themeMenuOpen = false;
            spellMenuOpen = !spellMenuOpen;
          }}
        >
          <span>{spellLangLabel(appSettings.spellcheckLanguage)}</span>
        </button>
        {#if spellMenuOpen}
          <div class="custom-select-list" role="listbox">
            <button
              type="button"
              role="option"
              class="custom-select-option"
              aria-selected={appSettings.spellcheckLanguage === "off"}
              onclick={() => pickSpellLang("off")}
            >
              {t("settings.spellLangOff")}
            </button>
            <button
              type="button"
              role="option"
              class="custom-select-option"
              aria-selected={appSettings.spellcheckLanguage === "de"}
              onclick={() => pickSpellLang("de")}
            >
              {t("settings.spellLangDe")}
            </button>
            <button
              type="button"
              role="option"
              class="custom-select-option"
              aria-selected={appSettings.spellcheckLanguage === "en"}
              onclick={() => pickSpellLang("en")}
            >
              {t("settings.spellLangEn")}
            </button>
          </div>
        {/if}
      </div>
      <p class="hint">{t("settings.spellcheckHint")}</p>
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
    <label class="field">
      <span class="field-label">{t("settings.zoteroBibPath")}</span>
      <input
        type="text"
        class="text-input"
        spellcheck="false"
        autocomplete="off"
        bind:value={bibDraft}
        onblur={() => void applyBibPath()}
        onkeydown={(e) => {
          if (e.key === "Enter") (e.currentTarget as HTMLInputElement).blur();
        }}
      />
      <p class="hint">{t("settings.zoteroBibHint")}</p>
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

  .text-input {
    box-sizing: border-box;
    width: 100%;
    padding: 0.45rem 0.55rem;
    border-radius: 6px;
    border: 1px solid var(--pd-border);
    background: var(--pd-bg);
    color: var(--pd-text);
    font-size: 0.95rem;
    font-family: var(--pd-mono), monospace;
  }

  .text-input:focus {
    outline: 2px solid color-mix(in srgb, var(--pd-accent) 45%, transparent);
    outline-offset: 1px;
  }

  .hint {
    margin: 0;
    font-size: 0.82rem;
    line-height: 1.4;
    color: var(--pd-muted);
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

  .custom-select {
    position: relative;
  }

  .custom-select-trigger {
    box-sizing: border-box;
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.45rem 0.55rem;
    padding-right: 1.85rem;
    border-radius: 6px;
    border: 1px solid var(--pd-border);
    background-color: var(--pd-bg);
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 12 12' fill='none'%3E%3Cpath d='M3 4.5L6 7.5L9 4.5' stroke='%23868e96' stroke-width='1.25' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 0.45rem center;
    color: var(--pd-text);
    font-size: 1rem;
    font-family: var(--pd-font), system-ui, sans-serif;
    text-align: left;
    cursor: pointer;
    accent-color: var(--pd-accent);
  }

  :global(:root[data-theme="light"]) .custom-select-trigger {
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 12 12' fill='none'%3E%3Cpath d='M3 4.5L6 7.5L9 4.5' stroke='%236c757d' stroke-width='1.25' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E");
  }

  .custom-select-trigger:focus {
    outline: 2px solid color-mix(in srgb, var(--pd-accent) 45%, transparent);
    outline-offset: 1px;
  }

  .custom-select-list {
    position: absolute;
    left: 0;
    right: 0;
    top: calc(100% + 4px);
    z-index: 320;
    margin: 0;
    padding: 0.3rem 0;
    border-radius: 6px;
    border: 1px solid var(--pd-border);
    background: var(--pd-surface);
    color: var(--pd-text);
    box-shadow: 0 10px 28px rgb(0 0 0 / 0.18);
    max-height: 14rem;
    overflow: auto;
  }

  .custom-select-option {
    display: block;
    width: 100%;
    box-sizing: border-box;
    margin: 0;
    padding: 0.5rem 0.65rem;
    border: none;
    border-radius: 0;
    background: transparent;
    font: inherit;
    font-size: 1rem;
    color: var(--pd-text);
    text-align: left;
    cursor: pointer;
  }

  .custom-select-option:hover,
  .custom-select-option[aria-selected="true"] {
    background: color-mix(in srgb, var(--pd-accent) 14%, var(--pd-bg));
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
