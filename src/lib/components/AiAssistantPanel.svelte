<script lang="ts">
  import { aiChat, type AiChatMessage, type AiEditorContextHost } from "$lib/tauri/api";
  import { locale, t } from "$lib/i18n/locale.svelte";

  const SYSTEM = `You are a concise assistant for Typst and academic writing. Prefer short answers; use fenced code blocks for Typst when helpful. Match the user's language when it is clear from their message.`;

  let {
    open,
    onClose,
    editorContext = undefined,
  }: {
    open: boolean;
    onClose: () => void;
    editorContext?: AiEditorContextHost;
  } = $props();

  type Row = { role: "user" | "assistant"; content: string };

  let rows = $state<Row[]>([]);
  let input = $state("");
  let busy = $state(false);
  let errorLine = $state<string | null>(null);

  $effect(() => {
    if (!open) errorLine = null;
  });

  function buildApiMessages(forSend: Row[]): AiChatMessage[] {
    return [
      { role: "system", content: SYSTEM },
      ...forSend.map((r) => ({ role: r.role, content: r.content })),
    ];
  }

  function mapInvokeError(msg: string): string {
    const m = msg.toLowerCase();
    if (m.includes("disabled")) return t("ai.errorDisabled");
    if (m.includes("no api key")) return t("ai.errorNoKey");
    return `${t("ai.errorGeneric")} ${msg}`;
  }

  async function submitUserMessage(content: string) {
    const trimmed = content.trim();
    if (!trimmed || busy) return;
    errorLine = null;
    const nextRows = [...rows, { role: "user" as const, content: trimmed }];
    rows = nextRows;
    busy = true;
    try {
      const reply = await aiChat(buildApiMessages(nextRows), 0.5);
      rows = [...nextRows, { role: "assistant", content: reply.trim() || "…" }];
    } catch (e) {
      const msg =
        typeof e === "string"
          ? e
          : e instanceof Error
            ? e.message
            : String(e);
      rows = nextRows.slice(0, -1);
      errorLine = mapInvokeError(msg);
    } finally {
      busy = false;
    }
  }

  function sendFromInput() {
    const text = input.trim();
    if (!text) return;
    input = "";
    void submitUserMessage(text);
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      sendFromInput();
    }
  }

  function quickImprove() {
    const ctx = editorContext?.read();
    const sel = ctx?.selectedText?.trim() ?? "";
    if (!sel) {
      errorLine = t("ai.noSelection");
      return;
    }
    const path = ctx?.path ?? "";
    const prompt = `Improve the wording and clarity of the following text. Keep the same language as the source. If the file is Typst (.typ), preserve markup commands.\n\nFile: ${path || "(unknown)"}\n\n---\n${sel}\n---`;
    void submitUserMessage(prompt);
  }

  function quickTypst() {
    const ctx = editorContext?.read();
    const sel = ctx?.selectedText?.trim() ?? "";
    if (!sel) {
      errorLine = t("ai.noSelection");
      return;
    }
    const path = ctx?.path ?? "";
    const prompt = `You are a Typst expert. Explain constructs, suggest fixes, or show idiomatic Typst for this snippet. Use fenced code for new Typst.\n\nFile: ${path || "(unknown)"}\n\n---\n${sel}\n---`;
    void submitUserMessage(prompt);
  }

  function clearChat() {
    if (busy) return;
    rows = [];
    errorLine = null;
  }
</script>

{#if open}
  <div
    class="backdrop"
    role="presentation"
    onclick={(e) => e.target === e.currentTarget && onClose()}
  ></div>
  {#key locale.value}
    <div class="panel" role="dialog" aria-modal="true" aria-labelledby="ai-title">
      <div class="head">
        <h2 id="ai-title">{t("ai.title")}</h2>
        <div class="head-actions">
          <button type="button" class="ghost" disabled={busy || rows.length === 0} onclick={clearChat}>
            {t("ai.clear")}
          </button>
          <button type="button" class="ghost" onclick={onClose}>{t("ai.close")}</button>
        </div>
      </div>
      <p class="hint">{t("settings.aiHint")}</p>
      <div class="quick">
        <button type="button" class="mini" disabled={busy} onclick={quickImprove}>
          {t("ai.improveSelection")}
        </button>
        <button type="button" class="mini" disabled={busy} onclick={quickTypst}>
          {t("ai.typstHelp")}
        </button>
      </div>
      {#if errorLine}
        <div class="err" role="alert">{errorLine}</div>
      {/if}
      <div class="thread" aria-busy={busy}>
        {#each rows as r, i (i)}
          <article class="bubble" class:user={r.role === "user"} class:assistant={r.role === "assistant"}>
            <span class="role">{r.role === "user" ? t("ai.roleUser") : t("ai.roleAssistant")}</span>
            <pre class="body">{r.content}</pre>
          </article>
        {/each}
        {#if busy}
          <p class="thinking">{t("ai.thinking")}</p>
        {/if}
      </div>
      <div class="composer">
        <textarea
          class="ta"
          rows="3"
          bind:value={input}
          placeholder={t("ai.inputPlaceholder")}
          disabled={busy}
          onkeydown={onKeydown}
        ></textarea>
        <button type="button" class="send" disabled={busy || !input.trim()} onclick={sendFromInput}>
          {t("ai.send")}
        </button>
      </div>
    </div>
  {/key}
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.45);
    z-index: 116;
  }

  .panel {
    position: fixed;
    left: 1rem;
    top: 4rem;
    bottom: 1rem;
    width: min(440px, calc(100vw - 2rem));
    z-index: 118;
    display: flex;
    flex-direction: column;
    border-radius: 10px;
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
    font-size: 0.85rem;
    color: var(--pd-muted);
    line-height: 1.35;
    border-bottom: 1px solid color-mix(in srgb, var(--pd-border) 55%, transparent);
    flex-shrink: 0;
  }

  .quick {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    padding: 0.5rem 1rem;
    border-bottom: 1px solid var(--pd-border);
    flex-shrink: 0;
  }

  .mini {
    padding: 0.3rem 0.55rem;
    border-radius: 6px;
    border: 1px solid var(--pd-border);
    background: var(--pd-bg);
    color: var(--pd-text);
    font-size: 0.85rem;
    cursor: pointer;
  }

  .mini:hover:not(:disabled) {
    border-color: var(--pd-muted);
  }

  .mini:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .err {
    margin: 0;
    padding: 0.45rem 1rem;
    font-size: 0.88rem;
    color: color-mix(in srgb, #f87171 85%, var(--pd-text));
    background: color-mix(in srgb, #f87171 12%, transparent);
    border-bottom: 1px solid var(--pd-border);
    flex-shrink: 0;
  }

  .thread {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 0.5rem 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .bubble {
    border-radius: 8px;
    border: 1px solid var(--pd-border);
    padding: 0.45rem 0.55rem;
    background: var(--pd-bg);
  }

  .bubble.user {
    border-color: color-mix(in srgb, var(--pd-accent) 35%, var(--pd-border));
  }

  .bubble.assistant {
    border-color: color-mix(in srgb, var(--pd-muted) 40%, var(--pd-border));
  }

  .role {
    display: block;
    font-size: 0.72rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--pd-muted);
    margin-bottom: 0.25rem;
  }

  .body {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    font-family: var(--pd-font, system-ui, sans-serif);
    font-size: 0.9rem;
    line-height: 1.45;
    color: var(--pd-text);
  }

  .thinking {
    margin: 0;
    font-size: 0.88rem;
    color: var(--pd-muted);
    font-style: italic;
  }

  .composer {
    flex-shrink: 0;
    padding: 0.6rem 1rem 0.85rem;
    border-top: 1px solid var(--pd-border);
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }

  .ta {
    box-sizing: border-box;
    width: 100%;
    resize: vertical;
    min-height: 3.2rem;
    padding: 0.45rem 0.55rem;
    border-radius: 8px;
    border: 1px solid var(--pd-border);
    background: var(--pd-bg);
    color: var(--pd-text);
    font-family: var(--pd-font), system-ui, sans-serif;
    font-size: 0.95rem;
    line-height: 1.35;
  }

  .ta:focus {
    outline: 2px solid color-mix(in srgb, var(--pd-accent) 45%, transparent);
    outline-offset: 1px;
  }

  .ta:disabled {
    opacity: 0.65;
  }

  .send {
    align-self: flex-end;
    padding: 0.4rem 1rem;
    border-radius: 8px;
    border: 1px solid var(--pd-border);
    background: color-mix(in srgb, var(--pd-accent) 18%, var(--pd-bg));
    color: var(--pd-text);
    font-size: 0.95rem;
    cursor: pointer;
  }

  .send:hover:not(:disabled) {
    border-color: var(--pd-muted);
  }

  .send:disabled {
    opacity: 0.45;
    cursor: default;
  }
</style>
