export type ThemeMode = "dark" | "light";

const K_THEME = "paperdesk.theme";
const K_FONT = "paperdesk.fontSizePx";
const K_DIR = "paperdesk.defaultProjectDir";

function readTheme(): ThemeMode {
  if (typeof localStorage === "undefined") return "dark";
  const raw = localStorage.getItem(K_THEME);
  return raw === "light" || raw === "dark" ? raw : "dark";
}

function readFontSize(): number {
  if (typeof localStorage === "undefined") return 14;
  const raw = localStorage.getItem(K_FONT);
  if (!raw) return 14;
  const n = parseInt(raw, 10);
  return Number.isFinite(n) && n >= 12 && n <= 22 ? n : 14;
}

function readDefaultDir(): string {
  if (typeof localStorage === "undefined") return "";
  return localStorage.getItem(K_DIR) ?? "";
}

export const appSettings = $state({
  theme: readTheme(),
  fontSizePx: readFontSize(),
  defaultProjectDir: readDefaultDir(),
});

export function applyAppearance() {
  if (typeof document === "undefined") return;
  document.documentElement.dataset.theme = appSettings.theme;
  document.documentElement.style.setProperty(
    "--pd-editor-font-size",
    `${appSettings.fontSizePx}px`,
  );
}

applyAppearance();

export function setTheme(next: ThemeMode) {
  appSettings.theme = next;
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(K_THEME, next);
  }
  applyAppearance();
}

export function setFontSizePx(px: number) {
  const n = Math.min(22, Math.max(12, Math.round(px)));
  appSettings.fontSizePx = n;
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(K_FONT, String(n));
  }
  applyAppearance();
}

export function setDefaultProjectDir(path: string) {
  appSettings.defaultProjectDir = path;
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(K_DIR, path);
  }
}

export function clearDefaultProjectDir() {
  appSettings.defaultProjectDir = "";
  if (typeof localStorage !== "undefined") {
    localStorage.removeItem(K_DIR);
  }
}
