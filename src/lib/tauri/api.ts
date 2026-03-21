import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";

export type CompileDiagnostic = {
  severity: string;
  message: string;
  path: string | null;
  line: number | null;
  column: number | null;
};

export type CompileOutcome = {
  ok: boolean;
  pdf_base64: string | null;
  diagnostics: CompileDiagnostic[];
  preview_page: number | null;
};

export async function getRecentProjects(): Promise<string[]> {
  return invoke("get_recent_projects");
}

export async function addRecentProject(path: string): Promise<void> {
  return invoke("add_recent_project", { path });
}

export async function openProject(path: string): Promise<void> {
  return invoke("open_project", { path });
}

export async function getOpenProject(): Promise<string | null> {
  return invoke("get_open_project");
}

export async function closeProject(): Promise<void> {
  return invoke("close_project");
}

/** Folder picker via JS plugin (avoids GTK/WebView deadlocks from Rust `blocking_pick_*`). */
export async function pickProjectFolder(): Promise<string | null> {
  return open({
    directory: true,
    multiple: false,
    title: "Project folder",
  });
}

export async function listProjectFiles(): Promise<string[]> {
  return invoke("list_project_files");
}

export async function readTextFile(relativePath: string): Promise<string> {
  return invoke("read_text_file", { relativePath });
}

export async function writeTextFile(
  relativePath: string,
  content: string,
): Promise<void> {
  return invoke("write_text_file", { relativePath, content });
}

export type PreviewSource = {
  path: string;
  text: string;
  cursor_byte_offset?: number;
};

export async function compileProject(
  entry?: string | null,
  previewSource?: PreviewSource | null,
): Promise<CompileOutcome> {
  return invoke("compile_project", {
    entry: entry ?? null,
    previewSource: previewSource ?? null,
  });
}

export async function exportPdf(entry?: string | null): Promise<void> {
  const path = await save({
    title: "Export PDF",
    defaultPath: "export.pdf",
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  });
  if (path === null) return;
  return invoke("export_pdf_to_path", {
    path,
    entry: entry ?? null,
  });
}

export async function createFromTemplate(
  templateId: string,
  targetDir: string,
): Promise<string> {
  return invoke("create_from_template", {
    templateId,
    targetDir,
  });
}
