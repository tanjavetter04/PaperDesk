<script lang="ts">
  import { onDestroy } from "svelte";
  import { EditorState } from "@codemirror/state";
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
  import { oneDark } from "@codemirror/theme-one-dark";
  import { readTextFile } from "$lib/tauri/api";

  let {
    path,
    onDocumentChange,
    onReady,
  }: {
    path: string | null;
    onDocumentChange: (text: string) => void;
    onReady?: (text: string) => void;
  } = $props();

  let host = $state<HTMLDivElement | null>(null);
  let view = $state<EditorView | null>(null);

  function extensions(onChange: (s: string) => void) {
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
      keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
      oneDark,
      EditorView.lineWrapping,
      EditorView.updateListener.of((u) => {
        if (u.docChanged) {
          onChange(u.state.doc.toString());
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
      onReady?.(text);
      const state = EditorState.create({
        doc: text,
        extensions: extensions((s) => onDocumentChange(s)),
      });
      view = new EditorView({ state, parent: el });
    })();

    return () => {
      cancelled = true;
    };
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
