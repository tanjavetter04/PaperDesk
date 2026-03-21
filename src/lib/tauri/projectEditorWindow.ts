import { goto } from "$app/navigation";
import { isTauri } from "@tauri-apps/api/core";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

/** Must match `windows` entry in `src-tauri/capabilities/default.json`. */
export const PROJECT_EDITOR_WEBVIEW_LABEL = "paperdesk-project";

function projectEditorUrl(): string {
  return new URL("/project", window.location.origin).href;
}

/** Opens the IDE: separate OS window in Tauri, same-tab navigation in the browser. */
export async function openProjectEditorUi(): Promise<void> {
  if (!isTauri()) {
    await goto("/project");
    return;
  }
  const url = projectEditorUrl();
  const existing = await WebviewWindow.getByLabel(PROJECT_EDITOR_WEBVIEW_LABEL);
  if (existing) {
    try {
      await existing.close();
    } catch {
      /* ignore */
    }
  }
  const w = new WebviewWindow(PROJECT_EDITOR_WEBVIEW_LABEL, {
    url,
    title: "PaperDesk",
    width: 1280,
    height: 800,
  });
  await new Promise<void>((resolve, reject) => {
    const t = window.setTimeout(
      () => reject(new Error("window create timeout")),
      15_000,
    );
    void w.once("tauri://created", () => {
      window.clearTimeout(t);
      resolve();
    });
    void w.once("tauri://error", (e: unknown) => {
      window.clearTimeout(t);
      reject(e instanceof Error ? e : new Error(String(e)));
    });
  });
}

/** After closing the project: close dedicated editor window or navigate home. */
export async function leaveProjectEditorUi(): Promise<void> {
  if (!isTauri()) {
    await goto("/");
    return;
  }
  try {
    const cur = WebviewWindow.getCurrent();
    if (cur.label === PROJECT_EDITOR_WEBVIEW_LABEL) {
      await cur.close();
      return;
    }
  } catch {
    /* fall through */
  }
  await goto("/");
}
