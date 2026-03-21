# PaperDesk

Local Typst writing environment (Tauri + SvelteKit + TypeScript).

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
