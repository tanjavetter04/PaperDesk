import type { Text } from "@codemirror/state";
import type { Diagnostic } from "@codemirror/lint";
import type { EditorView } from "@codemirror/view";

import type { SpellChecker } from "./spellDictionaries";

const WORD = /[\p{L}\p{M}]{2,}/gu;

const MAX_DIAGNOSTICS = 280;
const MAX_SUGGESTIONS_LIST = 6;
const MAX_SUGGESTION_ACTIONS = 5;
const SUGGEST_POOL = 24;

/** nspell has no Hunspell compound rules; long German compounds need recursive splits. */
const MIN_TOP_LEVEL_COMPOUND_LEN = 5;
const MIN_COMPOUND_PART = 3;
const MAX_COMPOUND_DEPTH = 8;

/** Typst / markup heuristics: reduce noise on directive lines and identifiers. */
function shouldSkipWord(fullText: string, from: number, to: number): boolean {
  const before = from > 0 ? fullText.charCodeAt(from - 1) : 0;
  if (before === 35 /* # */) return true;

  const lineStart = fullText.lastIndexOf("\n", from - 1) + 1;
  let lineEnd = fullText.indexOf("\n", to);
  if (lineEnd === -1) lineEnd = fullText.length;
  const line = fullText.slice(lineStart, lineEnd);
  const trimmed = line.trimStart();
  if (/^#(?:import|include|let|show|set|outline|figure|table|grid|stack)\b/.test(trimmed)) return true;

  const w = fullText.slice(from, to);
  if (w.length >= 2 && w === w.toUpperCase() && /^[\p{L}\p{M}]+$/u.test(w)) return true;

  return false;
}

/** German sentence case (igerman98 often marks lowercase lemmas ONLYINCOMPOUND; capitalized form is valid). */
function germanSentenceCase(w: string): string {
  if (w.length === 0) return w;
  return w.charAt(0).toLocaleUpperCase("de-DE") + w.slice(1).toLocaleLowerCase("de-DE");
}

function isKnownWord(spell: SpellChecker, w: string, spellLang: "de" | "en"): boolean {
  if (spell.correct(w) || spell.correct(w.toLowerCase())) return true;
  if (
    spellLang === "de" &&
    w.length >= 2 &&
    w === w.toLowerCase() &&
    /^[\p{L}\p{M}ßäöü]+$/u.test(w)
  ) {
    const sc = germanSentenceCase(w);
    if (sc !== w && spell.correct(sc)) return true;
  }
  return false;
}

function isGermanCompound(
  spell: SpellChecker,
  word: string,
  depth: number,
  spellLang: "de" | "en",
): boolean {
  if (isKnownWord(spell, word, spellLang)) return true;
  if (word.length < 4) return false;
  if (depth > MAX_COMPOUND_DEPTH) return false;
  if (depth === 0 && word.length < MIN_TOP_LEVEL_COMPOUND_LEN) return false;

  for (let i = MIN_COMPOUND_PART; i <= word.length - MIN_COMPOUND_PART; i++) {
    const left = word.slice(0, i);
    const right = word.slice(i);
    if (!isKnownWord(spell, left, spellLang)) continue;

    const rightCap = germanSentenceCase(right);
    if (isKnownWord(spell, rightCap, spellLang) || isKnownWord(spell, right, spellLang)) return true;
    if (isGermanCompound(spell, rightCap, depth + 1, spellLang)) return true;
    if (isGermanCompound(spell, right, depth + 1, spellLang)) return true;
  }

  // Fugen-s: …ung[s]beispiele… (linking "s" is not a separate morpheme in our binary split)
  for (let k = MIN_COMPOUND_PART; k < word.length - MIN_COMPOUND_PART - 1; k++) {
    if (word.charCodeAt(k) !== 115 /* s */) continue;
    const left = word.slice(0, k);
    const right = word.slice(k + 1);
    if (left.length < MIN_COMPOUND_PART - 1 || right.length < MIN_COMPOUND_PART) continue;

    const leftOk =
      isKnownWord(spell, left, spellLang) || isGermanCompound(spell, left, depth + 1, spellLang);
    if (!leftOk) continue;

    const rightCap = germanSentenceCase(right);
    if (isKnownWord(spell, rightCap, spellLang) || isKnownWord(spell, right, spellLang)) return true;
    if (isGermanCompound(spell, rightCap, depth + 1, spellLang)) return true;
    if (isGermanCompound(spell, right, depth + 1, spellLang)) return true;
  }

  return false;
}

function wordAccepted(spell: SpellChecker, raw: string, spellLang: "de" | "en"): boolean {
  if (isKnownWord(spell, raw, spellLang)) return true;
  if (spellLang === "de") return isGermanCompound(spell, raw, 0, spellLang);
  return false;
}

/** Drop Hunspell edge forms (leading/trailing hyphen) and duplicates; prefer sentence case for German. */
function refineSuggestions(
  spell: SpellChecker,
  raw: string,
  rawSuggestions: readonly string[],
  spellLang: "de" | "en",
): string[] {
  const out: string[] = [];
  const seen = new Set<string>();

  const push = (s: string) => {
    const t = s.trim();
    if (!t) return;
    if (t.startsWith("-") || t.endsWith("-")) return;
    const k = t.toLowerCase();
    if (k === raw.toLowerCase()) return;
    if (seen.has(k)) return;
    seen.add(k);
    out.push(t);
  };

  if (
    spellLang === "de" &&
    raw.length >= 2 &&
    raw === raw.toLowerCase() &&
    /^[\p{L}\p{M}ßäöü]+$/u.test(raw)
  ) {
    const sc = germanSentenceCase(raw);
    if (sc !== raw && spell.correct(sc)) push(sc);
  }

  for (const s of rawSuggestions) push(s);

  return out.slice(0, MAX_SUGGESTIONS_LIST);
}

export function typstSpellDiagnostics(
  doc: Text,
  spell: SpellChecker,
  unknownMessage: string,
  suggestionsLabel: string,
  spellLang: "de" | "en",
): Diagnostic[] {
  const full = doc.toString();
  const out: Diagnostic[] = [];
  WORD.lastIndex = 0;
  let m: RegExpExecArray | null;
  while ((m = WORD.exec(full)) !== null) {
    if (out.length >= MAX_DIAGNOSTICS) break;
    const from = m.index;
    const to = from + m[0].length;
    if (shouldSkipWord(full, from, to)) continue;

    const raw = m[0];
    if (wordAccepted(spell, raw, spellLang)) continue;

    const refined = refineSuggestions(spell, raw, spell.suggest(raw).slice(0, SUGGEST_POOL), spellLang);
    const listed = refined.slice(0, MAX_SUGGESTION_ACTIONS);
    const message =
      listed.length > 0
        ? `${unknownMessage} · ${suggestionsLabel}: ${listed.join(", ")}`
        : unknownMessage;

    const actions =
      listed.length > 0
        ? listed.map((replacement) => ({
            name: replacement,
            apply(view: EditorView, a: number, b: number) {
              view.dispatch({ changes: { from: a, to: b, insert: replacement } });
            },
          }))
        : undefined;

    out.push({
      from,
      to,
      severity: "error",
      source: "spell",
      message,
      actions,
    });
  }
  return out;
}
