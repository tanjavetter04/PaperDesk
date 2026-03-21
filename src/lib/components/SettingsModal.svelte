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
  import { FEATHERLESS_SUGGESTED_MODELS } from "$lib/ai/featherlessSuggestedModels";
  import {
    aiGetStatus,
    aiSetConfig,
    pickProjectFolder,
    restartBibWatcher,
    type AiStatus,
  } from "$lib/tauri/api";

  /** Dropdown sentinel; never sent to the API. */
  const AI_MODEL_OTHER = "__paperdesk_ai_other__";

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
  let aiModelPresetRoot = $state<HTMLDivElement | null>(null);
  let aiModelPresetMenuOpen = $state(false);

  $effect(() => {
    if (!open) {
      localeMenuOpen = false;
      themeMenuOpen = false;
      spellMenuOpen = false;
      aiModelPresetMenuOpen = false;
    }
  });

  function onDocPointerDown(e: PointerEvent) {
    if (
      !localeMenuOpen &&
      !themeMenuOpen &&
      !spellMenuOpen &&
      !aiModelPresetMenuOpen
    ) {
      return;
    }
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
    if (
      aiModelPresetMenuOpen &&
      aiModelPresetRoot &&
      !aiModelPresetRoot.contains(node)
    ) {
      aiModelPresetMenuOpen = false;
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

  let aiStatus = $state<AiStatus | null>(null);
  let aiEnabled = $state(false);
  let aiApiKeyDraft = $state("");
  let aiApiKeyTouched = $state(false);
  let aiBaseUrl = $state("");
  let aiModelPick = $state("");
  let aiModelOtherDraft = $state("");
  let aiReady = $state(false);
  let aiSaveFlash = $state<"ok" | "err" | null>(null);
  let aiKeyRemoveBusy = $state(false);

  function featherlessSuggestedLabel(
    m: (typeof FEATHERLESS_SUGGESTED_MODELS)[number],
  ): string {
    return locale.value === "de" ? m.labelDe : m.labelEn;
  }

  function effectiveAiModel(): string {
    if (aiModelPick === AI_MODEL_OTHER) return aiModelOtherDraft.trim();
    return aiModelPick.trim();
  }

  function hydrateAiModelFromStored(stored: string) {
    const hit = FEATHERLESS_SUGGESTED_MODELS.find((m) => m.id === stored);
    if (hit) {
      aiModelPick = stored;
      aiModelOtherDraft = "";
    } else {
      aiModelPick = AI_MODEL_OTHER;
      aiModelOtherDraft = stored;
    }
  }

  function pickAiModelChoice(choice: string) {
    aiModelPick = choice;
    aiModelPresetMenuOpen = false;
    if (choice !== AI_MODEL_OTHER) {
      aiModelOtherDraft = "";
    }
  }

  function aiModelTriggerText(): string {
    if (!aiReady || aiModelPick === "") return t("settings.aiModelPresets");
    if (aiModelPick === AI_MODEL_OTHER) return t("settings.aiModelOther");
    const m = FEATHERLESS_SUGGESTED_MODELS.find((x) => x.id === aiModelPick);
    return m ? featherlessSuggestedLabel(m) : t("settings.aiModelOther");
  }

  $effect(() => {
    if (open) bibDraft = appSettings.zoteroBibRelativePath;
  });

  function aiNeedsPersist(): boolean {
    if (!aiStatus) return false;
    if (aiEnabled !== aiStatus.enabled) return true;
    if (aiBaseUrl.trim() !== aiStatus.baseUrl.trim()) return true;
    if (effectiveAiModel() !== aiStatus.model.trim()) return true;
    if (aiApiKeyTouched) return true;
    return false;
  }

  async function syncAiFromUi(context: "normal" | "close") {
    if (!aiStatus) return;
    if (context === "normal" && (!open || !aiReady)) return;
    if (!aiNeedsPersist()) return;
    if (context === "normal") aiSaveFlash = null;
    try {
      await aiSetConfig({
        enabled: aiEnabled,
        baseUrl: aiBaseUrl.trim(),
        model: effectiveAiModel(),
        ...(aiApiKeyTouched ? { apiKey: aiApiKeyDraft } : {}),
      });
      aiStatus = await aiGetStatus();
      aiApiKeyDraft = "";
      aiApiKeyTouched = false;
      if (context === "normal") {
        aiSaveFlash = "ok";
        setTimeout(() => {
          aiSaveFlash = null;
        }, 2200);
      }
    } catch {
      if (context === "normal") {
        aiSaveFlash = "err";
      }
    }
  }

  $effect(() => {
    if (!open) {
      aiReady = false;
      return;
    }
    aiReady = false;
    void (async () => {
      try {
        const s = await aiGetStatus();
        aiStatus = s;
        aiEnabled = s.enabled;
        aiApiKeyDraft = "";
        aiApiKeyTouched = false;
        aiBaseUrl = s.baseUrl;
        hydrateAiModelFromStored(s.model.trim());
        aiSaveFlash = null;
        aiReady = true;
      } catch {
        aiStatus = null;
        aiSaveFlash = null;
        aiReady = false;
      }
    })();
    return () => {
      void syncAiFromUi("close");
    };
  });

  $effect(() => {
    if (!open || !aiReady) return;
    aiBaseUrl;
    aiModelPick;
    aiModelOtherDraft;
    aiApiKeyDraft;
    aiApiKeyTouched;
    const id = setTimeout(() => {
      void syncAiFromUi("normal");
    }, 450);
    return () => clearTimeout(id);
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

  async function removeStoredAiKey() {
    if (aiKeyRemoveBusy) return;
    aiSaveFlash = null;
    aiKeyRemoveBusy = true;
    try {
      await aiSetConfig({
        enabled: aiEnabled,
        baseUrl: aiBaseUrl.trim(),
        model: effectiveAiModel(),
        apiKey: "",
      });
      aiStatus = await aiGetStatus();
      aiApiKeyDraft = "";
      aiApiKeyTouched = false;
      aiSaveFlash = "ok";
      setTimeout(() => {
        aiSaveFlash = null;
      }, 2200);
    } catch {
      aiSaveFlash = "err";
    } finally {
      aiKeyRemoveBusy = false;
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
            aiModelPresetMenuOpen = false;
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
            aiModelPresetMenuOpen = false;
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
            aiModelPresetMenuOpen = false;
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
    <div class="ai-block">
      <h3 class="ai-heading">{t("settings.aiHeading")}</h3>
      <label class="check-row">
        <input
          type="checkbox"
          checked={aiEnabled}
          onchange={(e) => {
            aiEnabled = e.currentTarget.checked;
            void syncAiFromUi("normal");
          }}
        />
        <span>{t("settings.aiEnable")}</span>
      </label>
      <label class="field">
        {t("settings.aiApiKey")}
        <input
          type="password"
          class="text-input"
          spellcheck="false"
          autocomplete="off"
          placeholder={t("settings.aiApiKeyPlaceholder")}
          bind:value={aiApiKeyDraft}
          oninput={() => {
            aiApiKeyTouched = true;
          }}
        />
        {#if aiStatus?.hasApiKey}
          <button
            type="button"
            class="ai-remove-key"
            disabled={aiKeyRemoveBusy}
            aria-busy={aiKeyRemoveBusy}
            onclick={() => void removeStoredAiKey()}
          >
            {t("settings.aiRemoveKey")}
          </button>
        {/if}
      </label>
      <label class="field">
        {t("settings.aiBaseUrl")}
        <input
          type="text"
          class="text-input"
          spellcheck="false"
          autocomplete="off"
          bind:value={aiBaseUrl}
        />
      </label>
      <div class="field ai-model-settings">
        <span class="field-label">{t("settings.aiModel")}</span>
        <p id="settings-ai-model-flow" class="hint">{t("settings.aiModelFlowExplain")}</p>
        <div class="custom-select ai-model-presets" bind:this={aiModelPresetRoot}>
          <button
            type="button"
            class="custom-select-trigger"
            aria-haspopup="listbox"
            aria-expanded={aiModelPresetMenuOpen}
            aria-label={t("settings.aiModel")}
            aria-describedby="settings-ai-model-flow"
            onclick={() => {
              localeMenuOpen = false;
              themeMenuOpen = false;
              spellMenuOpen = false;
              aiModelPresetMenuOpen = !aiModelPresetMenuOpen;
            }}
          >
            <span class="ai-model-preset-trigger-text">{aiModelTriggerText()}</span>
          </button>
          {#if aiModelPresetMenuOpen}
            <div
              class="custom-select-list ai-model-preset-list"
              role="listbox"
              aria-label={t("settings.aiModelPresets")}
            >
              {#each FEATHERLESS_SUGGESTED_MODELS as m (m.id)}
                <button
                  type="button"
                  role="option"
                  class="custom-select-option ai-model-preset-option"
                  aria-selected={aiModelPick === m.id}
                  onclick={() => pickAiModelChoice(m.id)}
                >
                  <span class="ai-model-preset-title">{featherlessSuggestedLabel(m)}</span>
                  <span class="ai-model-preset-id">{m.id}</span>
                </button>
              {/each}
              <button
                type="button"
                role="option"
                class="custom-select-option ai-model-preset-option ai-model-other-option"
                aria-selected={aiModelPick === AI_MODEL_OTHER}
                onclick={() => pickAiModelChoice(AI_MODEL_OTHER)}
              >
                {t("settings.aiModelOther")}
              </button>
            </div>
          {/if}
        </div>
        {#if aiModelPick === AI_MODEL_OTHER}
          <label class="ai-model-other-block" for="settings-ai-model-other-input">
            <span class="ai-model-sublabel">{t("settings.aiModelOther")}</span>
            <input
              id="settings-ai-model-other-input"
              type="text"
              class="text-input"
              spellcheck="false"
              autocomplete="off"
              placeholder={t("settings.aiModelOtherPlaceholder")}
              bind:value={aiModelOtherDraft}
              aria-describedby="settings-ai-model-other-hint"
            />
            <p id="settings-ai-model-other-hint" class="hint">{t("settings.aiModelOtherHint")}</p>
          </label>
        {/if}
      </div>
      <p class="hint">{t("settings.aiHint")}</p>
      {#if aiSaveFlash === "ok"}
        <p class="ai-flash ok">{t("settings.aiSaved")}</p>
      {:else if aiSaveFlash === "err"}
        <p class="ai-flash err">{t("settings.aiSaveError")}</p>
      {/if}
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

  .ai-model-settings .field-label {
    color: var(--pd-text);
    font-weight: 600;
  }

  .ai-model-sublabel {
    display: block;
    font-size: 0.88rem;
    font-weight: 600;
    color: var(--pd-text);
    line-height: 1.35;
  }

  .ai-model-other-block {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    margin-top: 0.5rem;
  }

  .ai-model-other-block .hint {
    margin: 0;
  }

  .ai-model-other-option {
    font-weight: 500;
  }

  .ai-model-presets {
    margin-top: 0.35rem;
  }

  .ai-model-preset-list {
    max-height: 16rem;
  }

  .ai-model-preset-option {
    white-space: normal;
    line-height: 1.35;
  }

  .ai-model-preset-title {
    display: block;
    font-weight: 500;
    color: var(--pd-text);
  }

  .ai-model-preset-id {
    display: block;
    margin-top: 0.2rem;
    font-size: 0.78rem;
    font-family: var(--pd-mono), monospace;
    color: var(--pd-muted);
    word-break: break-all;
  }

  .ai-model-preset-trigger-text {
    min-width: 0;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
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

  .ai-block {
    margin: 0.5rem 0 1rem;
    padding-top: 0.75rem;
    border-top: 1px solid var(--pd-border);
  }

  .ai-heading {
    margin: 0 0 0.65rem;
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--pd-text);
  }

  .check-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.85rem;
    font-size: 1rem;
    color: var(--pd-text);
    cursor: pointer;
  }

  .check-row input {
    accent-color: var(--pd-accent);
    width: 1rem;
    height: 1rem;
  }

  .ai-remove-key {
    box-sizing: border-box;
    width: fit-content;
    max-width: 100%;
    align-self: flex-start;
    margin-top: 0.5rem;
    padding: 0.55rem 0.75rem;
    border-radius: 6px;
    border: 1px solid color-mix(in srgb, #f87171 55%, var(--pd-border));
    background: color-mix(in srgb, #f87171 16%, var(--pd-bg));
    color: color-mix(in srgb, #f87171 88%, var(--pd-text));
    font-size: 0.95rem;
    font-weight: 600;
    font-family: inherit;
    cursor: pointer;
    text-align: left;
  }

  .ai-remove-key:hover:not(:disabled) {
    background: color-mix(in srgb, #f87171 24%, var(--pd-bg));
    border-color: color-mix(in srgb, #f87171 72%, var(--pd-border));
  }

  .ai-remove-key:focus-visible {
    outline: 2px solid color-mix(in srgb, #f87171 55%, transparent);
    outline-offset: 2px;
  }

  .ai-remove-key:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .ai-flash {
    margin: 0 0 0.5rem;
    font-size: 0.88rem;
  }

  .ai-flash.ok {
    color: color-mix(in srgb, #4ade80 80%, var(--pd-text));
  }

  .ai-flash.err {
    color: color-mix(in srgb, #f87171 85%, var(--pd-text));
  }
</style>
