export type ThemeMode = "dark" | "light";

/** Hunspell-based editor spellcheck; `off` disables it. */
export type SpellcheckLanguage = "off" | "de" | "en";

const K_THEME = "paperdesk.theme";
const K_FONT = "paperdesk.fontSizePx";
const K_DIR = "paperdesk.defaultProjectDir";
const K_BIB = "paperdesk.zoteroBibRelativePath";
const K_SPELL_LANG = "paperdesk.spellcheckLang";
const K_SPELL_LEGACY = "paperdesk.spellcheck";
const DEFAULT_BIB_REL = "literatur.bib";

function readSpellcheckLanguage(): SpellcheckLanguage {
  if (typeof localStorage === "undefined") return "de";
  const raw = localStorage.getItem(K_SPELL_LANG);
  if (raw === "off" || raw === "de" || raw === "en") return raw;
  const legacy = localStorage.getItem(K_SPELL_LEGACY);
  if (legacy === "0") return "off";
  return "de";
}

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

function readZoteroBibRelativePath(): string {
  if (typeof localStorage === "undefined") return DEFAULT_BIB_REL;
  const raw = localStorage.getItem(K_BIB)?.trim();
  if (!raw) return DEFAULT_BIB_REL;
  const n = raw.replace(/\\/g, "/");
  if (n.startsWith("/") || n.includes("..")) return DEFAULT_BIB_REL;
  return n;
}

export const appSettings = $state({
  theme: readTheme(),
  fontSizePx: readFontSize(),
  defaultProjectDir: readDefaultDir(),
  zoteroBibRelativePath: readZoteroBibRelativePath(),
  spellcheckLanguage: readSpellcheckLanguage(),
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

export function setSpellcheckLanguage(next: SpellcheckLanguage) {
  appSettings.spellcheckLanguage = next;
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(K_SPELL_LANG, next);
  }
}

export function setZoteroBibRelativePath(path: string): string {
  const t = path.trim().replace(/\\/g, "/");
  const next =
    !t || t.startsWith("/") || t.includes("..") ? DEFAULT_BIB_REL : t;
  appSettings.zoteroBibRelativePath = next;
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(K_BIB, next);
  }
  return next;
}
