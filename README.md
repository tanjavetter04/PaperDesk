# Tauri + SvelteKit + TypeScript

This template should help get you started developing with Tauri, SvelteKit and TypeScript in Vite.

## Live preview (tinymist)

The Typst live preview uses **tinymist**. On `cargo build` / `cargo tauri build`, `src-tauri/build.rs` downloads a matching [tinymist release](https://github.com/Myriad-Dreamin/tinymist/releases) for the current Rust `TARGET` into `src-tauri/resources/bin/` (gitignored). That folder is bundled with the app via `tauri.conf.json` → `bundle.resources`.

- **Override binary**: set `TINYMIST_PATH` to an absolute path.
- **Skip download** (e.g. sandboxed CI): set `TINYMIST_SKIP_BUNDLE=1` and place `tinymist` / `tinymist.exe` in `src-tauri/resources/bin/` yourself, or rely on `tinymist` on `PATH`.
- **Release tag** is pinned in `src-tauri/build.rs` (`TINYMIST_RELEASE_TAG`); bump it when you change Typst/tinymist expectations.

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).
