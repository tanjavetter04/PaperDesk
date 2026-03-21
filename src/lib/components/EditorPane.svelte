<script lang="ts">
  import { onDestroy } from "svelte";
  import { EditorSelection, EditorState, Prec } from "@codemirror/state";
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

  /** Stable object; parent may replace `save` / `compile` so CodeMirror always calls the latest logic. */
  type HostCommands = {
    save?: () => void | Promise<void>;
    compile?: () => void | Promise<void>;
  };
  import { readTextFile } from "$lib/tauri/api";
  import type { CompileDiagnostic } from "$lib/tauri/api";
  import {
    compileDiagnosticCursorPos,
    compileDiagnosticsToCm,
  } from "$lib/editor/compileDiagnosticsCm";
  import { cursorPosFromTinymistEditorScroll } from "$lib/editor/sourceScrollCm";

  let {
    path,
    onDocumentChange,
    onReady,
    onCursorActivity,
    compileDiagnostics = [],
    focusDiagnosticRequest,
    previewScroll,
    hostCommands,
  }: {
    path: string | null;
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

  function utf8OffsetBefore(doc: Text, utf16Head: number): number {
    return new TextEncoder().encode(doc.sliceString(0, utf16Head)).length;
  }

  function extensions(
    onChange: (s: string) => void,
    onCursor: ((utf8: number) => void) | undefined,
    cmds: HostCommands | undefined,
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
        ...defaultKeymap,
        ...historyKeymap,
        indentWithTab,
        ...lintKeymap,
      ]),
      oneDark,
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
      <span class="muted">Select a file</span>
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
    background: #282c34;
  }

  .head {
    padding: 0.55rem 0.8rem;
    font-size: 0.85rem;
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
    font-size: 14px;
    line-height: 1.48;
  }

  .cm-host :global(.cm-panel.cm-search) {
    color: var(--pd-text);
    background: var(--pd-surface);
    border-bottom: 1px solid var(--pd-border);
    font-family: var(--pd-font, var(--pd-sans, system-ui, sans-serif));
    font-size: 13px;
    padding: 8px 10px;
    gap: 8px 10px;
  }

  .cm-host :global(.cm-panel.cm-search label) {
    color: var(--pd-muted);
    font-size: 12px;
  }

  .cm-host :global(.cm-panel.cm-search .cm-textfield) {
    font-family: var(--pd-mono);
    font-size: 13px;
    line-height: 1.4;
    padding: 6px 10px;
    min-height: 2.1rem;
    background: #1e2127;
    color: #abb2bf;
    border: 1px solid var(--pd-border);
    border-radius: 5px;
  }

  .cm-host :global(.cm-panel.cm-search .cm-button) {
    font-family: var(--pd-font, inherit);
    font-size: 12px;
    padding: 6px 12px;
    min-height: 2rem;
    background: #3a3f4b;
    color: var(--pd-text);
    border: 1px solid var(--pd-border);
    border-radius: 5px;
  }
</style>
