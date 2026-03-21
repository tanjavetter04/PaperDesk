import type { SpellcheckLanguage } from "$lib/appSettings.svelte";

import { warmSpellLangInWorker } from "./spellWorkerClient";

export type SpellLang = "de" | "en";

/**
 * Preload the other language in the spell worker so DE↔EN switches are faster.
 */
export function scheduleWarmAlternateSpellDicts(current: SpellcheckLanguage): void {
  if (typeof window === "undefined" || current === "off") return;
  const other: SpellLang = current === "de" ? "en" : "de";
  const run = () => warmSpellLangInWorker(other);
  if (typeof requestIdleCallback !== "undefined") {
    requestIdleCallback(run, { timeout: 12_000 });
  } else {
    setTimeout(run, 900);
  }
}
