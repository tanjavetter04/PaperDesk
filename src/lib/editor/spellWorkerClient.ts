import type { SpellDiagPlain } from "./spellcheckTypstCore";

type SpellLang = "de" | "en";

type SpellResultMsg = {
  type: "spellResult";
  id: number;
  diags: SpellDiagPlain[];
  error?: string;
};

let worker: Worker | null = null;
let seq = 0;
let listenerAttached = false;

const pending = new Map<
  number,
  { resolve: (d: SpellDiagPlain[]) => void; reject: (e: Error) => void }
>();

function attachListener(w: Worker) {
  if (listenerAttached) return;
  listenerAttached = true;
  w.addEventListener("message", (ev: MessageEvent<SpellResultMsg>) => {
    const d = ev.data;
    if (!d || d.type !== "spellResult" || typeof d.id !== "number") return;
    const entry = pending.get(d.id);
    if (!entry) return;
    pending.delete(d.id);
    if (d.error) entry.reject(new Error(d.error));
    else entry.resolve(d.diags);
  });
}

function getWorker(): Worker {
  if (typeof Worker === "undefined") {
    throw new Error("Web Workers are not available");
  }
  if (!worker) {
    worker = new Worker(new URL("./spellWorker.ts", import.meta.url), { type: "module" });
    attachListener(worker);
  }
  return worker;
}

/** Pre-parse a dictionary in the worker (idle warm). */
export function warmSpellLangInWorker(lang: SpellLang): void {
  try {
    getWorker().postMessage({ type: "warm", lang });
  } catch {
    /* ignore */
  }
}

export function requestSpellScanInWorker(args: {
  lang: SpellLang;
  text: string;
  unknownMessage: string;
  suggestionsLabel: string;
}): Promise<SpellDiagPlain[]> {
  const id = ++seq;
  const w = getWorker();
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    w.postMessage({
      type: "spellScan",
      id,
      lang: args.lang,
      text: args.text,
      unknownMessage: args.unknownMessage,
      suggestionsLabel: args.suggestionsLabel,
    });
  });
}
