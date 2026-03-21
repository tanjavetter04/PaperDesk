# PaperDesk

A local, privacy-focused writing environment for [Typst](https://typst.app/) documents, built with Tauri, SvelteKit, and TypeScript. Designed for academic writing, theses, and technical documents.

## Features

### Editor

- Full **Typst editor** powered by CodeMirror 6 with syntax highlighting, line numbers, line wrapping, and active-line highlighting
- **Typst autocomplete** for commands (`#set`, `#show`, `#import`, …), symbols (`sym.`), math mode (Greek letters, operators, functions), and more
- **Markdown** editing support for `.md` files
- Configurable **font size** (12–22 px)
- **Light and dark themes**
- **Find and replace** via CodeMirror search
- **Undo/redo**, rectangular selection, and standard keyboard shortcuts
- **Auto-save** with debounced writes (~140 ms)

### Live Preview

- Real-time **PDF preview** powered by [tinymist](https://github.com/Myriad-Dreamin/tinymist)
- **Source-to-preview sync** — cursor position scrolls the preview to the corresponding location
- **Preview-to-source sync** — clicking in the preview jumps to the matching source line
- Fallback PDF rendering via pdf.js when tinymist is unavailable

### Compilation and Export

- **Compile** Typst projects with inline diagnostics (errors and warnings with file, line, and column)
- **Export to PDF** via a save dialog
- **Diagnostics panel** listing all errors and warnings, with click-to-jump navigation

### Project Management

- **Open** existing project folders or **create new projects** from a thesis template or as an empty project
- **Recent projects** hub with up to 6 tiles showing PDF thumbnails
- **Rename**, **duplicate**, and **delete** projects from the hub
- Configurable **default project directory** for open/create dialogs

### File Management

- **Hierarchical file tree** with collapsible folders
- **Create**, **rename**, **move**, and **delete** files and folders
- Context menu for quick actions
- Protection against deleting or renaming `main.typ`

### Clipboard and Image Paste

- **Paste images** from the system clipboard directly into the editor — images are saved as PNG to `assets/` and an `#image(…)` reference is inserted automatically
- Supports pasting from clipboard API, HTML data URLs, and plain text fallback
- Relative paths are computed from the current file to the asset

### Spell Checking

- Built-in **spell checker** using nspell (Hunspell-compatible)
- Supports **German** and **English** dictionaries
- Incremental scanning for performance on large documents
- Inline error count and quick-fix suggestions

### AI Assistant

- Optional **AI writing assistant** using any OpenAI-compatible API (e.g. Featherless)
- **Chat panel** with conversation history
- **Quick actions**: improve selected text or get help with Typst code
- Sends editor context (file path, selection) for relevant suggestions
- Configurable API key, base URL, and model (with suggested presets)

### Version History

- **Git-based version history** using a dedicated `refs/paperdesk/history` ref (does not interfere with your own Git workflow)
- **Manual snapshots** via a "Create snapshot" button
- **Automatic checkpoints** on save, compile, export, file operations, and after idle periods
- **Commit list** showing up to 80 history entries
- **Diff view** showing changes between any snapshot and the current state
- **Restore** the project to any previous snapshot

### Zotero and Bibliography Integration

- Watches a `.bib` file in the project for external changes (e.g. from Zotero with Better BibTeX auto-export)
- Automatically refreshes diagnostics, editor, and preview when the bibliography changes
- Conflict detection when the `.bib` file changes while the editor has unsaved modifications

### Settings

- **Language**: German or English (UI and autocomplete labels)
- **Theme**: light or dark
- **Font size**: adjustable editor font
- **Default project folder**: preset directory for dialogs
- **Zotero bibliography path**: relative path to the `.bib` file
- **Spell check language**: off, German, or English
- **AI configuration**: enable/disable, API key, base URL, model

### Internationalization

- Full **German** and **English** UI translations (~150+ message keys)
- Dynamic document title and autocomplete section labels adapt to the selected language

---

## Zotero and bibliography (local setup)

PaperDesk does **not** talk to Zotero directly. The usual workflow is: **Zotero** (with **Better BibTeX**) keeps a `.bib` file in your project folder up to date; PaperDesk **watches** that file and refreshes diagnostics, the open editor tab, and the live preview when it changes.

### 1. Install Better BibTeX

1. Open [Better BibTeX for Zotero](https://retorque.re/zotero-better-bibtex/).
2. Download the `.xpi` for your Zotero version (Zotero 6 vs 7 builds differ).
3. In Zotero: **Tools → Add-ons** (or **Edit → Settings → Advanced → Files and Folders** depending on version) → install the extension → restart Zotero when prompted.

### 2. Export a collection or library to your project

Auto-export (“Keep updated”) only applies to a **library**, **collection** (folder), or **group** — not to a single item.

1. In the left pane, select **My Library** or a **collection** that contains the references you want in this project.
2. Right-click the selection → **Export Collection…** (or use **File → Export Library…** for the whole library).
3. Choose format **Better BibTeX** or **Better BibLaTeX** (recommended for Typst / BibLaTeX-style bibliographies).
4. Save the file **inside your PaperDesk project folder**, e.g. `literatur.bib` next to `main.typ`.

### 3. Enable “Keep updated”

In the export dialog, enable **Keep updated** (wording may be “Automatically update” depending on version). After that, Zotero rewrites the same `.bib` whenever the exported collection changes.

**If “Keep updated” is unavailable or fails:**

- You must use a **Better BibTeX** export format, not plain “BibTeX”.
- On **Linux**, Zotero **Flatpak/Snap** may be unable to write to arbitrary folders (e.g. under `~/Documents`). Use a native Zotero build from [zotero.org](https://www.zotero.org/), widen sandbox permissions, or export to a path the sandbox allows.
- Without auto-export, you can still **re-export manually** whenever you change references; PaperDesk will pick up file changes on disk.

### 4. Match the path in PaperDesk

In **Settings**, set **Zotero bibliography (relative path in project)** to the path of that file relative to the project root (default: `literatur.bib`). If you use e.g. `refs/literatur.bib`, enter exactly that.

### 5. Use the file in Typst

In `main.typ`:

```typst
#bibliography("literatur.bib")
```

Use the same filename/path as in step 2 and in PaperDesk settings. Then cite with `@citationkey` or `#cite(<citationkey>)` as in the [Typst bibliography documentation](https://typst.app/docs/reference/model/bibliography/).

---

**Note:** Sync is **one-way** (Zotero → file). Editing the `.bib` in PaperDesk does not update Zotero.

## Live preview (tinymist)

The Typst live preview uses **tinymist**. On `cargo build` / `cargo tauri build`, `src-tauri/build.rs` downloads a matching [tinymist release](https://github.com/Myriad-Dreamin/tinymist/releases) for the current Rust `TARGET` into `src-tauri/resources/bin/` (gitignored). That folder is bundled with the app via `tauri.conf.json` → `bundle.resources`.

- **Override binary**: set `TINYMIST_PATH` to an absolute path.
- **Skip download** (e.g. sandboxed CI): set `TINYMIST_SKIP_BUNDLE=1` and place `tinymist` / `tinymist.exe` in `src-tauri/resources/bin/` yourself, or rely on `tinymist` on `PATH`.
- **Release tag** is pinned in `src-tauri/build.rs` (`TINYMIST_RELEASE_TAG`); bump it when you change Typst/tinymist expectations.

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).
