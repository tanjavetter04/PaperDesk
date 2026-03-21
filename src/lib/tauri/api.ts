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

/** Emitted by the backend when tinymist preview reports `editorScrollTo` (click in live preview). */
export type PreviewScrollToSource = {
  filepath: string;
  line0: number;
  column0: number;
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
export async function pickProjectFolder(
  title = "Project folder",
): Promise<string | null> {
  return open({
    directory: true,
    multiple: false,
    title,
  });
}

export type ProjectEntry = {
  path: string;
  isDir: boolean;
};

export async function listProjectEntries(): Promise<ProjectEntry[]> {
  return invoke("list_project_files");
}

export async function createProjectDir(relativePath: string): Promise<void> {
  return invoke("create_project_dir", { relativePath });
}

export async function moveProjectPath(from: string, to: string): Promise<void> {
  return invoke("move_project_path", { from, to });
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
  previewSource?: PreviewSource | null,
): Promise<CompileOutcome> {
  return invoke("compile_project", {
    previewSource: previewSource ?? null,
  });
}

export async function startTinymistPreview(): Promise<string> {
  return invoke("start_tinymist_preview");
}

export async function restartTinymistPreview(): Promise<string> {
  return invoke("restart_tinymist_preview");
}

/** Compile project at `projectPath` without switching the open project (e.g. hub previews). */
export async function compileProjectAtPath(
  projectPath: string,
): Promise<CompileOutcome> {
  return invoke("compile_project_at_path", {
    projectPath,
  });
}

export async function exportPdf(saveDialogTitle = "Export PDF"): Promise<void> {
  const path = await save({
    title: saveDialogTitle,
    defaultPath: "export.pdf",
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  });
  if (path === null) return;
  return invoke("export_pdf_to_path", {
    path,
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

export async function createEmptyProject(targetDir: string): Promise<string> {
  return invoke("create_empty_project", { targetDir });
}
