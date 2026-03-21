<script lang="ts">
  import { onDestroy } from "svelte";
  import { Compartment, EditorSelection, EditorState, Prec } from "@codemirror/state";
  import type { Text } from "@codemirror/state";
  import {
    EditorView,
    keymap,
    lineNumbers,
    highlightActiveLineGutter,
    highlightActiveLine,
    drawSelection,
    dropCursor,
    rectangularSelection,
    crosshairCursor,
  } from "@codemirror/view";
  import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
  import { markdown } from "@codemirror/lang-markdown";
  import { lintGutter, lintKeymap, setDiagnostics } from "@codemirror/lint";
  import { search, searchKeymap } from "@codemirror/search";
  import { oneDark } from "@codemirror/theme-one-dark";
  import { appSettings } from "$lib/appSettings.svelte";
  import { paperDeskLightCm } from "$lib/editor/cmTheme";

  /** Stable object; parent may replace `save` / `compile` so CodeMirror always calls the latest logic. */
  type HostCommands = {
    save?: () => void | Promise<void>;
    compile?: () => void | Promise<void>;
  };
  import { readTextFile } from "$lib/tauri/api";
  import { t } from "$lib/i18n/locale.svelte";
  import type { CompileDiagnostic } from "$lib/tauri/api";
  import {
    compileDiagnosticCursorPos,
    compileDiagnosticsToCm,
  } from "$lib/editor/compileDiagnosticsCm";
  import { cursorPosFromTinymistEditorScroll } from "$lib/editor/sourceScrollCm";
  import {
    completionKeymap as typstCompletionKeymap,
    typstAutocompleteBundle,
  } from "$lib/editor/typstAutocomplete";

  let {
    path,
    /** Bump to force a re-read from disk for the same `path` (e.g. after history restore). */
    reloadTick = 0,
    /** Increment to reload the open file from disk when the path is unchanged (e.g. external .bib sync). */
    reloadFromDiskTick = 0,
    onDocumentChange,
    onReady,
    onCursorActivity,
    compileDiagnostics = [],
    focusDiagnosticRequest,
    previewScroll,
    hostCommands,
  }: {
    path: string | null;
    reloadTick?: number;
    reloadFromDiskTick?: number;
    hostCommands?: HostCommands;
    onDocumentChange: (text: string) => void;
    /** Fires after `path` was read from disk and the editor instance is created. */
    onReady?: (text: string, loadedPath: string) => void;
    /** UTF-8 byte offset of the primary cursor (for PDF forward sync). */
    onCursorActivity?: (utf8ByteOffset: number) => void;
    compileDiagnostics?: CompileDiagnostic[];
    focusDiagnosticRequest?: { tick: number; target: CompileDiagnostic | null };
    /** Live-preview click → source (0-based line/column from tinymist). */
    previewScroll?: { tick: number; line0: number; column0: number };
  } = $props();

  let host = $state<HTMLDivElement | null>(null);
  let view = $state<EditorView | null>(null);

  const themeCompartment = new Compartment();

  function cmThemeBundle() {
    return appSettings.theme === "light" ? paperDeskLightCm : oneDark;
  }

  function utf8OffsetBefore(doc: Text, utf16Head: number): number {
    return new TextEncoder().encode(doc.sliceString(0, utf16Head)).length;
  }

  function extensions(
    onChange: (s: string) => void,
    onCursor: ((utf8: number) => void) | undefined,
    cmds: HostCommands | undefined,
    typstFile: boolean,
  ) {
    return [
      lineNumbers(),
      highlightActiveLineGutter(),
      highlightActiveLine(),
      drawSelection(),
      dropCursor(),
      rectangularSelection(),
      crosshairCursor(),
      history(),
      markdown(),
      ...(typstFile ? typstAutocompleteBundle() : []),
      search(),
      lintGutter(),
      Prec.high(
        keymap.of([
          {
            key: "Mod-s",
            preventDefault: true,
            run: () => {
              void cmds?.save?.();
              return true;
            },
          },
          {
            key: "Mod-Shift-b",
            preventDefault: true,
            run: () => {
              void cmds?.compile?.();
              return true;
            },
          },
        ]),
      ),
      keymap.of([
        ...searchKeymap,
        ...(typstFile ? typstCompletionKeymap : []),
        ...defaultKeymap,
        ...historyKeymap,
        indentWithTab,
        ...lintKeymap,
      ]),
      themeCompartment.of(cmThemeBundle()),
      EditorView.lineWrapping,
      EditorView.updateListener.of((u) => {
        if (u.docChanged) {
          onChange(u.state.doc.toString());
        }
        if (onCursor && (u.selectionSet || u.docChanged)) {
          const head = u.state.selection.main.head;
          onCursor(utf8OffsetBefore(u.state.doc, head));
        }
      }),
    ];
  }

  $effect(() => {
    const el = host;
    const p = path;
    void reloadTick;
    void reloadFromDiskTick;
    if (!el) return;

    if (!p) {
      view?.destroy();
      view = null;
      el.innerHTML = "";
      return;
    }

    let cancelled = false;

    (async () => {
      const text = await readTextFile(p);
      if (cancelled) return;
      view?.destroy();
      const state = EditorState.create({
        doc: text,
        extensions: extensions(
          (s) => onDocumentChange(s),
          onCursorActivity,
          hostCommands,
          p.endsWith(".typ"),
        ),
      });
      view = new EditorView({ state, parent: el });
      if (onCursorActivity) {
        onCursorActivity(utf8OffsetBefore(view.state.doc, view.state.selection.main.head));
      }
      onReady?.(text, p);
    })();

    return () => {
      cancelled = true;
    };
  });

  $effect(() => {
    const v = view;
    const p = path;
    const diags = compileDiagnostics;
    if (!v) return;
    if (!p?.endsWith(".typ")) {
      v.dispatch(setDiagnostics(v.state, []));
      return;
    }
    v.dispatch(setDiagnostics(v.state, compileDiagnosticsToCm(v.state.doc, p, diags)));
  });

  $effect(() => {
    const v = view;
    const p = path;
    const { tick, target } = focusDiagnosticRequest ?? {
      tick: 0,
      target: null,
    };
    if (!v || tick === 0 || !target || !p?.endsWith(".typ")) return;
    const pos = compileDiagnosticCursorPos(v.state.doc, p, target);
    if (pos == null) return;
    v.focus();
    v.dispatch({
      selection: EditorSelection.cursor(pos),
      effects: EditorView.scrollIntoView(pos, { y: "center" }),
    });
  });

  $effect(() => {
    const v = view;
    if (!v) return;
    void appSettings.theme;
    v.dispatch({
      effects: themeCompartment.reconfigure(cmThemeBundle()),
    });
  });

  $effect(() => {
    const v = view;
    const p = path;
    const ps = previewScroll;
    if (!v || !ps || ps.tick === 0 || !p?.endsWith(".typ")) return;
    const pos = cursorPosFromTinymistEditorScroll(v.state.doc, ps.line0, ps.column0);
    if (pos == null) return;
    v.focus();
    v.dispatch({
      selection: EditorSelection.cursor(pos),
      effects: EditorView.scrollIntoView(pos, { y: "center" }),
    });
  });

  onDestroy(() => {
    view?.destroy();
    view = null;
  });
</script>

<div class="editor">
  <div class="head">
    {#if path}
      <span class="path">{path}</span>
    {:else}
      <span class="muted">{t("editor.selectFile")}</span>
    {/if}
  </div>
  <div class="cm-host" bind:this={host}></div>
</div>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    background: var(--pd-editor-chrome);
  }

  .head {
    padding: 0.55rem 0.8rem;
    font-size: 1rem;
    border-bottom: 1px solid var(--pd-border);
    background: var(--pd-surface);
  }

  .path {
    font-family: var(--pd-mono);
    color: var(--pd-text);
  }

  .muted {
    color: var(--pd-muted);
  }

  .cm-host {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .cm-host :global(.cm-editor) {
    height: 100%;
  }

  .cm-host :global(.cm-scroller) {
    font-family: var(--pd-mono);
    font-size: 1rem;
    line-height: 1.48;
  }

  .cm-host :global(.cm-panel.cm-search) {
    color: var(--pd-text);
    background: var(--pd-surface);
    border-bottom: 1px solid var(--pd-border);
    font-family: var(--pd-font, var(--pd-sans, system-ui, sans-serif));
    font-size: 1rem;
    padding: 8px 10px;
    gap: 8px 10px;
  }

  .cm-host :global(.cm-panel.cm-search label) {
    color: var(--pd-muted);
    font-size: 1rem;
  }

  .cm-host :global(.cm-panel.cm-search .cm-textfield) {
    font-family: var(--pd-mono);
    font-size: 1rem;
    line-height: 1.4;
    padding: 6px 10px;
    min-height: 2.1rem;
    background: var(--pd-bg);
    color: var(--pd-text);
    border: 1px solid var(--pd-border);
    border-radius: 5px;
  }

  .cm-host :global(.cm-panel.cm-search .cm-button) {
    font-family: var(--pd-font, inherit);
    font-size: 1rem;
    padding: 6px 12px;
    min-height: 2rem;
    background: color-mix(in srgb, var(--pd-accent) 14%, var(--pd-bg));
    color: var(--pd-text);
    border: 1px solid var(--pd-border);
    border-radius: 5px;
  }

  /* Typst autocomplete (CodeMirror tooltip lives under .cm-editor) */
  .cm-host :global(.cm-tooltip.cm-tooltip-autocomplete.pd-cm-autocomplete) {
    filter: drop-shadow(0 10px 28px rgb(0 0 0 / 0.28));
    border: none;
    background: transparent;
    padding: 0;
  }

  .cm-host :global(.cm-tooltip.cm-tooltip-autocomplete.pd-cm-autocomplete > ul) {
    font-family: var(--pd-mono);
    font-size: 1.0625rem;
    line-height: 1.42;
    margin: 0;
    padding: 6px 0;
    min-width: min(20rem, 92vw);
    max-width: min(28rem, 94vw);
    max-height: min(18rem, 46vh);
    overflow: hidden auto;
    list-style: none;
    background: var(--pd-surface);
    color: var(--pd-text);
    border: 1px solid var(--pd-border);
    border-radius: 10px;
    scrollbar-width: thin;
    scrollbar-color: var(--pd-border) transparent;
  }

  .cm-host :global(.cm-tooltip.cm-tooltip-autocomplete.pd-cm-autocomplete > ul > li[role="option"]) {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 0 6px;
    padding: 0.42rem 0.6rem;
    border-radius: 7px;
    cursor: pointer;
  }

  .cm-host :global(.cm-tooltip.cm-tooltip-autocomplete.pd-cm-autocomplete > ul > li[role="option"]:hover) {
    background: color-mix(in srgb, var(--pd-muted) 12%, transparent);
  }

  .cm-host :global(.cm-tooltip.cm-tooltip-autocomplete.pd-cm-autocomplete ul > li[aria-selected]) {
    background: color-mix(in srgb, var(--pd-accent) 26%, var(--pd-surface)) !important;
    color: var(--pd-text) !important;
  }

  .cm-host :global(.pd-cm-completion-kind) {
    flex-shrink: 0;
    min-width: 1.45rem;
    padding: 0.12rem 0.32rem;
    border-radius: 4px;
    font-family: var(--pd-font);
    font-size: 0.6875rem;
    font-weight: 700;
    line-height: 1.2;
    letter-spacing: 0.03em;
    text-align: center;
    text-transform: uppercase;
    color: var(--pd-muted);
    background: color-mix(in srgb, var(--pd-muted) 16%, var(--pd-bg));
    border: 1px solid color-mix(in srgb, var(--pd-border) 70%, transparent);
  }

  .cm-host :global(.pd-cm-completion-kind[data-kind="keyword"]) {
    color: color-mix(in srgb, var(--pd-accent) 88%, var(--pd-text));
    background: color-mix(in srgb, var(--pd-accent) 18%, var(--pd-bg));
    border-color: color-mix(in srgb, var(--pd-accent) 35%, var(--pd-border));
  }

  .cm-host :global(.pd-cm-completion-kind[data-kind="function"]) {
    color: var(--pd-muted);
    background: color-mix(in srgb, var(--pd-muted) 12%, var(--pd-bg));
  }

  .cm-host :global(.cm-tooltip.cm-tooltip-autocomplete.pd-cm-autocomplete .cm-completionLabel) {
    flex: 1 1 auto;
    min-width: 0;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cm-host :global(.cm-tooltip.cm-tooltip-autocomplete.pd-cm-autocomplete .cm-completionLabel::before) {
    content: "#";
    font-weight: 500;
    color: var(--pd-muted);
    margin-right: 1px;
  }

  .cm-host :global(.cm-tooltip.cm-tooltip-autocomplete.pd-cm-autocomplete .cm-completionDetail) {
    flex: 0 1 42%;
    margin: 0;
    min-width: 0;
    font-family: var(--pd-font);
    font-size: 0.9375rem;
    font-style: normal;
    font-weight: 400;
    color: var(--pd-muted);
    text-align: right;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cm-host :global(.cm-tooltip.cm-tooltip-autocomplete.pd-cm-autocomplete .cm-completionMatchedText) {
    text-decoration: none;
    color: var(--pd-accent);
    font-weight: 700;
  }

  .cm-host :global(.cm-tooltip.cm-tooltip-autocomplete.pd-cm-autocomplete completion-section) {
    display: list-item;
    list-style: none;
    margin: 0.35rem 0 0.15rem;
    padding: 0.38rem 0.9rem 0.22rem;
    font-family: var(--pd-font);
    font-size: 0.75rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--pd-muted);
    border-bottom: 1px solid var(--pd-border);
    background: linear-gradient(
      to bottom,
      color-mix(in srgb, var(--pd-bg) 40%, var(--pd-surface)),
      var(--pd-surface)
    );
    cursor: default;
    pointer-events: none;
  }

  .cm-host :global(.cm-tooltip.cm-tooltip-autocomplete.pd-cm-autocomplete completion-section:first-child) {
    margin-top: 0;
    border-radius: 10px 10px 0 0;
  }
</style>
