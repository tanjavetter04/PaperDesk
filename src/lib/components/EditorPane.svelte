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
  import { locale, t } from "$lib/i18n/locale.svelte";
  import { paperDeskLightCm } from "$lib/editor/cmTheme";
  import type { Locale } from "$lib/i18n/messages";
  import type { SpellDiagPlain } from "$lib/editor/spellcheckTypst";
  import {
    expandToWordContext,
    mapSpellDiagThroughEdit,
    mergeSpellAfterRescan,
    trySingleEdit,
  } from "$lib/editor/spellIncremental";
  import { spellPlainToCmDiagnostics } from "$lib/editor/spellcheckTypst";
  import { requestSpellScanInWorker, requestSpellSliceScanInWorker } from "$lib/editor/spellWorkerClient";

  /** Stable object; parent may replace `save` / `compile` so CodeMirror always calls the latest logic. */
  type HostCommands = {
    save?: () => void | Promise<void>;
    compile?: () => void | Promise<void>;
  };
  import {
    clipboardPasteForTypstEditor,
    readTextFile,
    writeBinaryFile,
  } from "$lib/tauri/api";
  import type { AiEditorContextHost, CompileDiagnostic } from "$lib/tauri/api";
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
    onTypstPreviewSourceScroll,
    compileDiagnostics = [],
    focusDiagnosticRequest,
    previewScroll,
    hostCommands,
    aiEditorRef = undefined,
    onBinaryAssetCreated,
    onPasteImageError,
  }: {
    path: string | null;
    reloadTick?: number;
    reloadFromDiskTick?: number;
    hostCommands?: HostCommands;
    aiEditorRef?: AiEditorContextHost;
    onDocumentChange: (text: string) => void;
    /** Fires after `path` was read from disk and the editor instance is created. */
    onReady?: (text: string, loadedPath: string) => void;
    /** UTF-8 byte offset of the primary cursor (for PDF forward sync). */
    onCursorActivity?: (utf8ByteOffset: number) => void;
    /** Caret in source → tinymist `panelScrollTo` (UTF-16 column within line, same as CodeMirror). */
    onTypstPreviewSourceScroll?: (pos: {
      line0: number;
      character: number;
      reason: "cursor" | "edit";
    }) => void;
    compileDiagnostics?: CompileDiagnostic[];
    focusDiagnosticRequest?: { tick: number; target: CompileDiagnostic | null };
    /** Live-preview click → source (0-based line/column from tinymist). */
    previewScroll?: { tick: number; line0: number; column0: number };
    /** After an image was saved under the project (e.g. paste); refresh file tree. */
    onBinaryAssetCreated?: (relativePath: string) => void;
    /** User-visible error when clipboard image save/insert fails. */
    onPasteImageError?: (message: string) => void;
  } = $props();

  let host = $state<HTMLDivElement | null>(null);
  let view = $state<EditorView | null>(null);
  /** Bumps when the open document text changes (spellcheck debounce). */
  let docRevision = $state(0);
  /** Active spell-worker jobs (supports overlap while a stale run finishes). */
  let spellWorkerInflight = $state(0);

  /** Enables incremental worker scans after a full pass for this tab. */
  let spellEditCache: {
    path: string;
    reloadTick: number;
    reloadFromDiskTick: number;
    uiLocale: Locale;
    text: string;
    plain: SpellDiagPlain[];
    spellLang: "de" | "en";
  } | null = null;

  type SpellHeadState = { kind: "hidden" } | { kind: "checking" } | { kind: "done"; count: number };

  /** Header line: only show a result count when it matches the live document and cache metadata. */
  const spellHeadState = $derived.by((): SpellHeadState => {
    void docRevision;
    const p = path;
    const sl = appSettings.spellcheckLanguage;
    if (!p?.endsWith(".typ") || sl === "off") return { kind: "hidden" };
    const v = view;
    if (!v) return { kind: "hidden" };
    if (spellWorkerInflight > 0) return { kind: "checking" };
    const c = spellEditCache;
    if (
      !c ||
      c.path !== p ||
      c.spellLang !== sl ||
      c.uiLocale !== locale.value ||
      c.reloadTick !== reloadTick ||
      c.reloadFromDiskTick !== reloadFromDiskTick
    ) {
      return { kind: "hidden" };
    }
    if (c.text !== v.state.doc.toString()) return { kind: "hidden" };
    return { kind: "done", count: c.plain.length };
  });

  function spellErrorSummaryLabel(count: number): string {
    return count === 1 ? t("editor.spellErrorsOne") : t("editor.spellErrorsOther", { n: count });
  }

  const themeCompartment = new Compartment();

  function cmThemeBundle() {
    return appSettings.theme === "light" ? paperDeskLightCm : oneDark;
  }

  function utf8OffsetBefore(doc: Text, utf16Head: number): number {
    return new TextEncoder().encode(doc.sliceString(0, utf16Head)).length;
  }

  function mimeToImageExt(mime: string): string {
    const m = mime.toLowerCase().split(";")[0]?.trim() ?? "";
    if (m === "image/png") return "png";
    if (m === "image/jpeg" || m === "image/jpg") return "jpg";
    if (m === "image/webp") return "webp";
    if (m === "image/gif") return "gif";
    return "png";
  }

  /** Local wall-clock stamp; same pattern as Rust `clipboard_paste_for_typst` (`image-YYYYMMDD-HHMMSS-mmm`). */
  function pastedImageTimestampedName(ext: string): string {
    const d = new Date();
    const pad = (n: number, w = 2) => String(n).padStart(w, "0");
    const stamp = `${d.getFullYear()}${pad(d.getMonth() + 1)}${pad(d.getDate())}-${pad(d.getHours())}${pad(d.getMinutes())}${pad(d.getSeconds())}-${pad(d.getMilliseconds(), 3)}`;
    return `image-${stamp}.${ext}`;
  }

  /** Path from the current file’s directory to another project-relative POSIX path. */
  function posixRelativePath(fromFileRel: string, toProjectRel: string): string {
    const slash = fromFileRel.lastIndexOf("/");
    const dir = slash === -1 ? "" : fromFileRel.slice(0, slash);
    const a = dir.split("/").filter(Boolean);
    const b = toProjectRel.split("/").filter(Boolean);
    let i = 0;
    while (i < a.length && i < b.length && a[i] === b[i]) i += 1;
    const ups = a.length - i;
    const rest = b.slice(i);
    const prefix = ups > 0 ? "../".repeat(ups) : "";
    return prefix + rest.join("/");
  }

  function uint8ToBase64(bytes: Uint8Array): string {
    let binary = "";
    const len = bytes.byteLength;
    const chunk = 0x8000;
    for (let i = 0; i < len; i += chunk) {
      const sub = bytes.subarray(i, Math.min(i + chunk, len));
      binary += String.fromCharCode.apply(null, sub as unknown as number[]);
    }
    return btoa(binary);
  }

  function clipboardSuggestsImage(cd: DataTransfer): boolean {
    for (const t of cd.types) {
      if (t.toLowerCase().startsWith("image/")) return true;
    }
    for (const item of Array.from(cd.items)) {
      if (item.type.toLowerCase().startsWith("image/")) return true;
    }
    return false;
  }

  /** Screenshot / file item / drag-style `files` / rich HTML with inline data URL. */
  function syncImageFileFromClipboard(cd: DataTransfer): File | null {
    for (const item of Array.from(cd.items)) {
      if (!item.type.toLowerCase().startsWith("image/")) continue;
      if (item.kind === "file") {
        const file = item.getAsFile();
        if (file && file.size > 0) return file;
      }
    }
    const files = cd.files;
    if (files?.length) {
      for (let i = 0; i < files.length; i++) {
        const file = files.item(i);
        if (file && file.type.toLowerCase().startsWith("image/") && file.size > 0) {
          return file;
        }
      }
    }
    const html = cd.getData("text/html");
    if (html && html.length > 40) {
      const m = html.match(
        /<\s*img[^>]+src\s*=\s*["'](data:image\/(?:png|jpeg|jpg|gif|webp);base64,[^"']+)["']/i,
      );
      if (m?.[1]) {
        const dataUrl = m[1];
        try {
          const comma = dataUrl.indexOf(",");
          if (comma < 0) return null;
          const header = dataUrl.slice(0, comma).toLowerCase();
          const b64 = dataUrl.slice(comma + 1).replace(/\s/g, "");
          const mimeMatch = header.match(/data:(image\/[a-z0-9.+_-]+)/);
          const mime = mimeMatch?.[1] ?? "image/png";
          const binary = atob(b64);
          const bytes = new Uint8Array(binary.length);
          for (let j = 0; j < binary.length; j++) bytes[j] = binary.charCodeAt(j);
          const ext = mimeToImageExt(mime);
          return new File([bytes], `paste.${ext}`, { type: mime });
        } catch {
          return null;
        }
      }
    }
    return null;
  }

  async function imageFileFromClipboardApi(): Promise<File | null> {
    if (typeof navigator === "undefined" || !navigator.clipboard?.read) return null;
    try {
      const items = await navigator.clipboard.read();
      for (const ci of items) {
        for (const type of ci.types) {
          if (!type.toLowerCase().startsWith("image/")) continue;
          const blob = await ci.getType(type);
          if (blob.size < 1) continue;
          const ext = mimeToImageExt(type);
          return new File([blob], `paste.${ext}`, { type });
        }
      }
    } catch {
      return null;
    }
    return null;
  }

  function pastePlainFromClipboard(view: EditorView, cd: DataTransfer) {
    const raw = cd.getData("text/plain") || cd.getData("text/uri-list") || "";
    view.dispatch(view.state.replaceSelection(raw), {
      userEvent: "input.paste",
      scrollIntoView: true,
    });
  }

  function extensions(
    onChange: (s: string) => void,
    onCursor: ((utf8: number) => void) | undefined,
    onPreviewSrcScroll:
      | ((pos: { line0: number; character: number; reason: "cursor" | "edit" }) => void)
      | undefined,
    cmds: HostCommands | undefined,
    typstFile: boolean,
    pasteImageMode: "typst" | "markdown" | null,
    editorPathForPaste: string | null,
    onBinaryAssetCreatedCb: ((rel: string) => void) | undefined,
    onPasteImageErrorCb: ((msg: string) => void) | undefined,
  ) {
    async function savePastedImageAndInsert(view: EditorView, file: File) {
      const ep = editorPathForPaste;
      const mode = pasteImageMode;
      if (!ep || !mode) return;
      const mime = (file.type || "image/png").toLowerCase();
      const ext = mimeToImageExt(mime);
      const name = pastedImageTimestampedName(ext);
      const relProject = `assets/${name}`;
      let bytes: Uint8Array;
      try {
        bytes = new Uint8Array(await file.arrayBuffer());
      } catch (e) {
        const d = e instanceof Error ? e.message : String(e);
        onPasteImageErrorCb?.(t("editor.pasteImageFailed", { detail: d }));
        return;
      }
      const b64 = uint8ToBase64(bytes);
      try {
        await writeBinaryFile(relProject, b64);
      } catch (e) {
        const d =
          typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
        onPasteImageErrorCb?.(t("editor.pasteImageFailed", { detail: d }));
        return;
      }
      onBinaryAssetCreatedCb?.(relProject);
      const relDoc = posixRelativePath(ep, relProject);
      const insert =
        mode === "markdown" ? `![](${relDoc})` : `#image("${relDoc}")`;
      const { from, to } = view.state.selection.main;
      const full = `${insert}\n`;
      view.dispatch({
        changes: { from, to, insert: full },
        selection: { anchor: from + full.length },
      });
    }

    return [
      ...(pasteImageMode === "typst" && editorPathForPaste
        ? [
            Prec.highest(
              EditorView.domEventHandlers({
                paste(event, view) {
                  if (view.state.readOnly) return false;
                  event.preventDefault();
                  const ep = editorPathForPaste;
                  void (async () => {
                    try {
                      const r = await clipboardPasteForTypstEditor();
                      if (r.kind === "none" || !ep) return;
                      if (r.kind === "image") {
                        const img = r as {
                          relativePath?: string;
                          relative_path?: string;
                        };
                        const relProj =
                          (typeof img.relativePath === "string"
                            ? img.relativePath
                            : undefined) ??
                          (typeof img.relative_path === "string"
                            ? img.relative_path
                            : "") ??
                          "";
                        if (!relProj) {
                          onPasteImageErrorCb?.(
                            t("editor.pasteImageFailed", {
                              detail: "missing relativePath",
                            }),
                          );
                          return;
                        }
                        onBinaryAssetCreatedCb?.(relProj);
                        const relDoc = posixRelativePath(ep, relProj);
                        const full = `#image("${relDoc}")\n`;
                        const { from, to } = view.state.selection.main;
                        view.dispatch({
                          changes: { from, to, insert: full },
                          selection: { anchor: from + full.length },
                        });
                      } else if (r.kind === "text") {
                        view.dispatch(view.state.replaceSelection(r.content), {
                          userEvent: "input.paste",
                          scrollIntoView: true,
                        });
                      }
                    } catch (e) {
                      const d =
                        typeof e === "string"
                          ? e
                          : e instanceof Error
                            ? e.message
                            : String(e);
                      onPasteImageErrorCb?.(
                        t("editor.pasteImageFailed", { detail: d }),
                      );
                    }
                  })();
                  return true;
                },
              }),
            ),
          ]
        : []),
      ...(pasteImageMode === "markdown" && editorPathForPaste
        ? [
            Prec.highest(
              EditorView.domEventHandlers({
                paste(event, view) {
                  const cd = event.clipboardData;
                  if (!cd) return false;

                  const file = syncImageFileFromClipboard(cd);
                  if (file) {
                    event.preventDefault();
                    void savePastedImageAndInsert(view, file).catch((e) => {
                      const d = e instanceof Error ? e.message : String(e);
                      onPasteImageErrorCb?.(
                        t("editor.pasteImageFailed", { detail: d }),
                      );
                    });
                    return true;
                  }

                  if (clipboardSuggestsImage(cd)) {
                    event.preventDefault();
                    const plain = cd.getData("text/plain") || "";
                    void (async () => {
                      let detail = "Clipboard API";
                      try {
                        const fromApi = await imageFileFromClipboardApi();
                        if (fromApi && fromApi.size > 0) {
                          await savePastedImageAndInsert(view, fromApi);
                          return;
                        }
                      } catch (e) {
                        detail = e instanceof Error ? e.message : String(e);
                      }
                      if (plain) {
                        pastePlainFromClipboard(view, cd);
                        return;
                      }
                      onPasteImageErrorCb?.(
                        t("editor.pasteImageUnreadable", { detail }),
                      );
                    })();
                    return true;
                  }

                  return false;
                },
              }),
            ),
          ]
        : []),
      lineNumbers(),
      highlightActiveLineGutter(),
      highlightActiveLine(),
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
          docRevision = docRevision + 1;
        }
        if (onCursor && (u.selectionSet || u.docChanged)) {
          const head = u.state.selection.main.head;
          onCursor(utf8OffsetBefore(u.state.doc, head));
        }
        if (
          typstFile &&
          onPreviewSrcScroll &&
          (u.selectionSet || u.docChanged)
        ) {
          const head = u.state.selection.main.head;
          const ln = u.state.doc.lineAt(head);
          onPreviewSrcScroll({
            line0: ln.number - 1,
            character: head - ln.from,
            reason: u.docChanged ? "edit" : "cursor",
          });
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
      const pasteMode = p.endsWith(".typ")
        ? ("typst" as const)
        : p.endsWith(".md")
          ? ("markdown" as const)
          : null;
      const state = EditorState.create({
        doc: text,
        extensions: extensions(
          (s) => onDocumentChange(s),
          onCursorActivity,
          onTypstPreviewSourceScroll,
          hostCommands,
          p.endsWith(".typ"),
          pasteMode,
          pasteMode ? p : null,
          onBinaryAssetCreated,
          onPasteImageError,
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
    void view;
    void path;
    const ref = aiEditorRef;
    if (!ref) return;
    ref.read = () => {
      const v = view;
      const p = path;
      if (!v) return { path: p, selectedText: "" };
      const { from, to } = v.state.selection.main;
      return { path: p, selectedText: v.state.doc.sliceString(from, to) };
    };
    return () => {
      ref.read = () => ({ path: null, selectedText: "" });
    };
  });

  $effect(() => {
    const v = view;
    const p = path;
    const diags = compileDiagnostics;
    const spellLang = appSettings.spellcheckLanguage;
    void docRevision;
    const uiLoc = locale.value;
    const rk = reloadTick;
    const rfd = reloadFromDiskTick;

    if (!v) return;

    const compileCm =
      p?.endsWith(".typ") ? compileDiagnosticsToCm(v.state.doc, p, diags) : [];

    if (spellLang === "off" || !p?.endsWith(".typ")) {
      spellEditCache = null;
      v.dispatch(setDiagnostics(v.state, compileCm));
      return;
    }

    v.dispatch(setDiagnostics(v.state, compileCm));

    let cancelled = false;
    const docSnap = v.state.doc.toString();
    const spellUnknown = t("editor.spellUnknown");
    const spellSuggestions = t("editor.spellSuggestions");
    const langActive = spellLang;
    const pathActive = p;
    const handle = window.setTimeout(() => {
      void (async () => {
        try {
          let plain: SpellDiagPlain[];

          const cacheOk =
            spellEditCache &&
            spellEditCache.path === pathActive &&
            spellEditCache.spellLang === langActive &&
            spellEditCache.uiLocale === uiLoc &&
            spellEditCache.reloadTick === rk &&
            spellEditCache.reloadFromDiskTick === rfd;

          if (cacheOk && spellEditCache!.text === docSnap) {
            plain = spellEditCache!.plain;
          } else if (cacheOk && spellEditCache!.text !== docSnap) {
            const edit = trySingleEdit(spellEditCache!.text, docSnap);
            if (edit) {
              const mapped = spellEditCache!.plain
                .map((d) => mapSpellDiagThroughEdit(d, edit))
                .filter((x): x is SpellDiagPlain => x != null);
              const exp = expandToWordContext(docSnap, edit.newMidStart, edit.newMidEnd);
              spellWorkerInflight += 1;
              try {
                const slice = docSnap.slice(exp.from, exp.to);
                const fresh =
                  slice.length > 0
                    ? await requestSpellSliceScanInWorker({
                        lang: langActive,
                        slice,
                        baseOffset: exp.from,
                        unknownMessage: spellUnknown,
                        suggestionsLabel: spellSuggestions,
                      })
                    : [];
                plain = mergeSpellAfterRescan(mapped, fresh, exp.from, exp.to);
              } catch (e) {
                console.warn("PaperDesk spellcheck (incremental):", e);
                plain = await requestSpellScanInWorker({
                  lang: langActive,
                  text: docSnap,
                  unknownMessage: spellUnknown,
                  suggestionsLabel: spellSuggestions,
                });
              } finally {
                spellWorkerInflight -= 1;
              }
            } else {
              spellWorkerInflight += 1;
              try {
                plain = await requestSpellScanInWorker({
                  lang: langActive,
                  text: docSnap,
                  unknownMessage: spellUnknown,
                  suggestionsLabel: spellSuggestions,
                });
              } finally {
                spellWorkerInflight -= 1;
              }
            }
          } else {
            spellWorkerInflight += 1;
            try {
              plain = await requestSpellScanInWorker({
                lang: langActive,
                text: docSnap,
                unknownMessage: spellUnknown,
                suggestionsLabel: spellSuggestions,
              });
            } finally {
              spellWorkerInflight -= 1;
            }
          }

          if (cancelled || v.state.doc.toString() !== docSnap) return;
          const spellDiags = spellPlainToCmDiagnostics(plain);
          if (cancelled) return;
          const compileCm2 =
            p?.endsWith(".typ") ? compileDiagnosticsToCm(v.state.doc, p, diags) : [];
          v.dispatch(setDiagnostics(v.state, [...compileCm2, ...spellDiags]));
          spellEditCache = {
            path: pathActive,
            reloadTick: rk,
            reloadFromDiskTick: rfd,
            uiLocale: uiLoc,
            text: docSnap,
            plain,
            spellLang: langActive,
          };
        } catch (e) {
          console.warn("PaperDesk spellcheck:", e);
        }
      })();
    }, 480);

    return () => {
      cancelled = true;
      window.clearTimeout(handle);
    };
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
    {#if path?.endsWith(".typ") && appSettings.spellcheckLanguage !== "off"}
      {#if spellHeadState.kind === "checking"}
        <span class="spell-status" role="status" aria-live="polite"
          >{t("editor.spellChecking")}</span
        >
      {:else if spellHeadState.kind === "done"}
        <span class="spell-status" role="status" aria-live="polite"
          >{spellErrorSummaryLabel(spellHeadState.count)}</span
        >
      {/if}
    {/if}
  </div>
  <div class="cm-host" bind:this={host}></div>
</div>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    min-height: 0;
    background: var(--pd-editor-chrome);
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    min-width: 0;
    padding: 0.55rem 0.8rem;
    font-size: 1rem;
    border-bottom: 1px solid var(--pd-border);
    background: var(--pd-surface);
  }

  .path {
    font-family: var(--pd-mono);
    color: var(--pd-text);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .spell-status {
    flex-shrink: 0;
    font-family: var(--pd-font, var(--pd-sans, system-ui, sans-serif));
    font-size: 0.85rem;
    font-weight: 500;
    color: var(--pd-muted);
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
