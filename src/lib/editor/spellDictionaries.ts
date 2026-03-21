import NSpell from "nspell";

import extraDeDic from "./spellExtraDe.dic?raw";
import deAffUrl from "../../../node_modules/dictionary-de/index.aff?url";
import deDicUrl from "../../../node_modules/dictionary-de/index.dic?url";
import enAffUrl from "../../../node_modules/dictionary-en/index.aff?url";
import enDicUrl from "../../../node_modules/dictionary-en/index.dic?url";

export type SpellLang = "de" | "en";

/** Minimal surface used by our spell pass (nspell instance). */
export type SpellChecker = {
  correct(value: string): boolean;
  suggest(value: string): string[];
};

function utf8FromArrayBuffer(buf: ArrayBuffer): string {
  return new TextDecoder("utf-8").decode(buf);
}

async function fetchDict(affUrl: string, dicUrl: string) {
  const [affRes, dicRes] = await Promise.all([fetch(affUrl), fetch(dicUrl)]);
  if (!affRes.ok) throw new Error(`spell dict aff: ${affRes.status}`);
  if (!dicRes.ok) throw new Error(`spell dict dic: ${dicRes.status}`);
  const [affBuf, dicBuf] = await Promise.all([affRes.arrayBuffer(), dicRes.arrayBuffer()]);
  // nspell calls `.toString('utf8')` like Node's Buffer; Uint8Array does not decode that way in browsers.
  return {
    aff: utf8FromArrayBuffer(affBuf),
    dic: utf8FromArrayBuffer(dicBuf),
  };
}

const cache = new Map<SpellLang, Promise<SpellChecker>>();

export function getSpellchecker(lang: SpellLang): Promise<SpellChecker> {
  let p = cache.get(lang);
  if (!p) {
    p = (async () => {
      if (lang === "de") {
        const { aff, dic } = await fetchDict(deAffUrl, deDicUrl);
        return new NSpell([{ aff, dic }, { dic: extraDeDic }]);
      }
      const { aff, dic } = await fetchDict(enAffUrl, enDicUrl);
      return new NSpell({ aff, dic });
    })();
    cache.set(lang, p);
  }
  return p;
}
