# PaperDesk

A local, privacy-focused writing environment for [Typst](https://typst.app/) documents, built with Tauri, SvelteKit, and TypeScript. Designed for academic writing, theses, and technical documents.

## Features

### Editor

- Full **Typst editor** powered by CodeMirror 6 with syntax highlighting, line numbers, line wrapping, and active-line highlighting
- **Typst autocomplete** for commands (`#set`, `#show`, `#import`, …), symbols (`sym.`), math mode (Greek letters, operators, functions), and more
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

- **Compile** Typst projects
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

### Settings

- **Language**: German or English (UI and autocomplete labels)
- **Theme**: light or dark
- **Font size**: adjustable editor font
- **Default project folder**: preset directory for dialogs
- **Zotero bibliography path**: relative path to the `.bib` file PaperDesk should watch (see **References & Zotero** below)
- **Spell check language**: off, German, or English
- **AI configuration**: enable/disable, API key, base URL, model

### References & Zotero (local `.bib`)

PaperDesk does **not** talk to Zotero over the network. The usual workflow: **Zotero** with **[Better BibTeX](https://retorque.re/zotero-better-bibtex/)** keeps a `.bib` file **inside your project** up to date; PaperDesk **watches** that path (set in **Settings**) and refreshes diagnostics, the open editor tab, and the live preview when the file changes on disk.

**What PaperDesk does**

- Watches the bibliography **relative path** from **Settings** (default: `literature.bib`).
- Refreshes diagnostics, the editor, and the live preview when that file changes on disk.
- Warns if the `.bib` changes while you still have **unsaved** editor changes.

**What you put in Typst**

- Use `#bibliography("…")` with the **same** path as in Zotero export and in Settings.
- Cite with `@citationkey` or `#cite(<citationkey>)` as in the [Typst bibliography documentation](https://typst.app/docs/reference/model/bibliography/).

**Zotero setup (step by step)**

1. **Install Better BibTeX** — Open the [Better BibTeX for Zotero](https://retorque.re/zotero-better-bibtex/) page, download the `.xpi` for your Zotero version (6 vs 7 differ). In Zotero: **Tools → Add-ons** (or **Edit → Settings → Advanced → Files and Folders**, depending on version) → install → restart Zotero when prompted.

2. **Export a collection or library into the project** — Auto-export (“Keep updated”) applies to a **library**, **collection** (folder), or **group**, not a single item. Select **My Library** or a collection → right-click → **Export Collection…** (or **File → Export Library…**). Choose **Better BibTeX** or **Better BibLaTeX** (good for Typst / BibLaTeX-style bibliographies). Save the file **inside your PaperDesk project**, e.g. `literature.bib` next to `main.typ`.

3. **Enable “Keep updated”** — In the export dialog, turn on **Keep updated** (sometimes labeled “Automatically update”). Zotero will rewrite that `.bib` whenever the exported set changes.

   **If that option is missing or fails:** use a **Better BibTeX** export format, not plain “BibTeX”. On **Linux**, Zotero **Flatpak/Snap** may not be allowed to write under `~/Documents` — prefer a build from [zotero.org](https://www.zotero.org/), adjust sandbox paths, or export somewhere writable. You can always **re-export manually**; PaperDesk still picks up file changes on disk.

4. **Match PaperDesk settings** — In **Settings**, set **Zotero bibliography (relative path in project)** to that file’s path from the project root (default `literature.bib`; use e.g. `refs/literature.bib` if you saved it there).

5. **Wire it in `main.typ`**

   ```typst
   #bibliography("literature.bib")
   ```

   Use the **same** path as in step 2 and in Settings.

**Note:** Sync is **one-way** (Zotero → file). Editing the `.bib` only in PaperDesk does not update Zotero.
