/// <reference lib="webworker" />

import NSpell from "nspell";

import extraDeDic from "./spellExtraDe.dic?raw";
import deAffUrl from "../../../node_modules/dictionary-de/index.aff?url";
import deDicUrl from "../../../node_modules/dictionary-de/index.dic?url";
import enAffUrl from "../../../node_modules/dictionary-en/index.aff?url";
import enDicUrl from "../../../node_modules/dictionary-en/index.dic?url";

import { runTypstSpellScanPlain, type SpellChecker } from "./spellcheckTypstCore";

type SpellLang = "de" | "en";

type Inbound =
  | { type: "warm"; lang: SpellLang }
  | {
      type: "spellScan";
      id: number;
      lang: SpellLang;
      text: string;
      unknownMessage: string;
      suggestionsLabel: string;
    }
  | {
      type: "spellScanSlice";
      id: number;
      lang: SpellLang;
      slice: string;
      baseOffset: number;
      unknownMessage: string;
      suggestionsLabel: string;
    };

function utf8FromArrayBuffer(buf: ArrayBuffer): string {
  return new TextDecoder("utf-8").decode(buf);
}

async function fetchDict(affUrl: string, dicUrl: string) {
  const [affRes, dicRes] = await Promise.all([fetch(affUrl), fetch(dicUrl)]);
  if (!affRes.ok) throw new Error(`spell dict aff: ${affRes.status}`);
  if (!dicRes.ok) throw new Error(`spell dict dic: ${dicRes.status}`);
  const [affBuf, dicBuf] = await Promise.all([affRes.arrayBuffer(), dicRes.arrayBuffer()]);
  return {
    aff: utf8FromArrayBuffer(affBuf),
    dic: utf8FromArrayBuffer(dicBuf),
  };
}

const cache = new Map<SpellLang, Promise<SpellChecker>>();

async function buildSpellchecker(lang: SpellLang): Promise<SpellChecker> {
  if (lang === "de") {
    const { aff, dic } = await fetchDict(deAffUrl, deDicUrl);
    return new NSpell([{ aff, dic }, { dic: extraDeDic }]) as SpellChecker;
  }
  const { aff, dic } = await fetchDict(enAffUrl, enDicUrl);
  return new NSpell({ aff, dic }) as SpellChecker;
}

function ensureSpell(lang: SpellLang): Promise<SpellChecker> {
  let p = cache.get(lang);
  if (!p) {
    p = buildSpellchecker(lang);
    cache.set(lang, p);
  }
  return p;
}

/** One spell job at a time so rapid typing does not stack overlapping scans. */
let jobChain: Promise<void> = Promise.resolve();

self.onmessage = (ev: MessageEvent<Inbound>) => {
  const msg = ev.data;
  if (!msg || typeof msg !== "object") return;

  if (msg.type === "warm") {
    void ensureSpell(msg.lang).catch(() => {});
    return;
  }

  if (msg.type === "spellScan" || msg.type === "spellScanSlice") {
    const payload = msg;
    jobChain = jobChain
      .then(() =>
        (async () => {
          const { id, lang, unknownMessage, suggestionsLabel } = payload;
          try {
            const spell = await ensureSpell(lang);
            const text = payload.type === "spellScanSlice" ? payload.slice : payload.text;
            const base = payload.type === "spellScanSlice" ? payload.baseOffset : 0;
            let diags = runTypstSpellScanPlain(text, spell, unknownMessage, suggestionsLabel, lang);
            if (base !== 0) {
              diags = diags.map((d) => ({
                ...d,
                from: d.from + base,
                to: d.to + base,
              }));
            }
            self.postMessage({ type: "spellResult", id, diags });
          } catch (e) {
            const err = e instanceof Error ? e.message : String(e);
            self.postMessage({ type: "spellResult", id, diags: [], error: err });
          }
        })(),
      )
      .catch(() => {});
  }
};

export {};
