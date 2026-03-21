<script lang="ts">
  import { onDestroy } from "svelte";
  import { EditorSelection, EditorState } from "@codemirror/state";
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
  import { oneDark } from "@codemirror/theme-one-dark";
  import { readTextFile } from "$lib/tauri/api";
  import type { CompileDiagnostic } from "$lib/tauri/api";
  import {
    compileDiagnosticCursorPos,
    compileDiagnosticsToCm,
  } from "$lib/editor/compileDiagnosticsCm";

  let {
    path,
    onDocumentChange,
    onReady,
    onCursorActivity,
    compileDiagnostics = [],
    focusDiagnosticRequest,
  }: {
    path: string | null;
    onDocumentChange: (text: string) => void;
    /** Fires after `path` was read from disk and the editor instance is created. */
    onReady?: (text: string, loadedPath: string) => void;
    /** UTF-8 byte offset of the primary cursor (for PDF forward sync). */
    onCursorActivity?: (utf8ByteOffset: number) => void;
    compileDiagnostics?: CompileDiagnostic[];
    focusDiagnosticRequest?: { tick: number; target: CompileDiagnostic | null };
  } = $props();

  let host = $state<HTMLDivElement | null>(null);
  let view = $state<EditorView | null>(null);

  function utf8OffsetBefore(doc: Text, utf16Head: number): number {
    return new TextEncoder().encode(doc.sliceString(0, utf16Head)).length;
  }

  function extensions(
    onChange: (s: string) => void,
    onCursor?: (utf8: number) => void,
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
      lintGutter(),
      keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab, ...lintKeymap]),
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
        extensions: extensions((s) => onDocumentChange(s), onCursorActivity),
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
    padding: 0.5rem 0.75rem;
    font-size: 0.8rem;
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
    font-size: 13px;
    line-height: 1.45;
  }
</style>
