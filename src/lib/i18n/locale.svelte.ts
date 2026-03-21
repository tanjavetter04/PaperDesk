import type { Locale, MessageKey } from "./messages";
import { messages } from "./messages";

const STORAGE_KEY = "paperdesk.locale";

function guessFromBrowser(): Locale {
  if (typeof navigator === "undefined") return "en";
  return navigator.language.toLowerCase().startsWith("de") ? "de" : "en";
}

function readInitial(): Locale {
  if (typeof localStorage === "undefined") return guessFromBrowser();
  const raw = localStorage.getItem(STORAGE_KEY);
  if (raw === "de" || raw === "en") return raw;
  return guessFromBrowser();
}

/** Mutate `locale.value` only — Svelte disallows reassigning exported `$state` bindings. */
export const locale = $state<{ value: Locale }>({ value: readInitial() });

function applyDomLang() {
  if (typeof document === "undefined") return;
  document.documentElement.lang = locale.value === "de" ? "de" : "en";
}

applyDomLang();

export function setLocale(next: Locale) {
  locale.value = next;
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(STORAGE_KEY, next);
  }
  applyDomLang();
}

export function t(key: MessageKey, vars?: Record<string, string | number>): string {
  let s = messages[locale.value][key];
  if (vars) {
    for (const [k, v] of Object.entries(vars)) {
      s = s.replaceAll(`{${k}}`, String(v));
    }
  }
  return s;
}
