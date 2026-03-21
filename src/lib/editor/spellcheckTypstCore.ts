/** Shared between main thread and spell worker — no CodeMirror / DOM. */

export type SpellChecker = {
  correct(value: string): boolean;
  suggest(value: string): string[];
};

export type SpellDiagPlain = {
  from: number;
  to: number;
  message: string;
  replacements: string[];
};

const WORD = /[\p{L}\p{M}]{2,}/gu;

const MAX_DIAGNOSTICS = 280;
const MAX_SUGGESTIONS_LIST = 6;
const MAX_SUGGESTION_ACTIONS = 5;
const SUGGEST_POOL = 24;

const MIN_TOP_LEVEL_COMPOUND_LEN = 5;
const MIN_COMPOUND_PART = 3;
const MAX_COMPOUND_DEPTH = 8;

function skipWsBack(text: string, i: number): number {
  let j = i;
  while (j >= 0 && /\s/.test(text[j])) j--;
  return j;
}

/** Typst keywords that may precede `{` after an expression (e.g. `if x {`). */
const KEYWORD_BEFORE_BRACE = new Set([
  "if",
  "else",
  "for",
  "while",
  "in",
  "and",
  "or",
  "not",
]);

/** If `endInclusive` is the last char of an ASCII identifier, returns that word when it is `KEYWORD_BEFORE_BRACE`. */
function keywordBeforeBraceEndingAt(text: string, endInclusive: number): string | null {
  if (endInclusive < 0 || !/[\w.-]/u.test(text[endInclusive])) return null;
  let s = endInclusive;
  while (s >= 0 && /[\w.-]/u.test(text[s])) s--;
  const w = text.slice(s + 1, endInclusive + 1);
  return KEYWORD_BEFORE_BRACE.has(w) ? w : null;
}

/** After `(` or `[` at `openIdx`, true if this looks like Typst code, not prose "(note)". */
function beforeOpenIsCodeContext(text: string, openIdx: number): boolean {
  let i = skipWsBack(text, openIdx - 1);
  if (i < 0) return false;
  const c = text[i];
  if (c === "#") return true;
  if (c === ",") return true;
  if (c === "=" || c === "<" || c === ">" || c === "+" || c === "-" || c === "*" || c === "/") return true;
  if (c === ")" || c === "]" || c === "}") return true;
  if (c === ":") {
    let j = skipWsBack(text, i - 1);
    if (j < 0) return false;
    if (/[\w\]\).-]/u.test(text[j])) return true;
  }
  if (/[\w\])]/u.test(c)) {
    let j = i;
    while (j >= 0 && /[\w.-]/u.test(text[j])) j--;
    let k = skipWsBack(text, j);
    while (k >= 0) {
      if (text[k] === "#") return true;
      if (keywordBeforeBraceEndingAt(text, k)) return true;
      if (!/[\w.-]/u.test(text[k])) break;
      let j2 = k;
      while (j2 >= 0 && /[\w.-]/u.test(text[j2])) j2--;
      k = skipWsBack(text, j2);
    }
    if (k >= 0 && (text[k] === "}" || text[k] === ")" || text[k] === "]")) return true;
    if (k < 0) return true;
  }
  return false;
}

/**
 * True if `from` lies inside Typst-flavoured `(...)`, `[...]`, or `{...}` (not prose parens).
 */
function insideTypstCodeDelimiters(fullText: string, from: number): boolean {
  let depthParen = 0;
  let depthBracket = 0;
  let depthBrace = 0;
  for (let i = from - 1; i >= 0; i--) {
    const c = fullText[i];
    if (c === "}") depthBrace++;
    else if (c === "{") {
      if (depthBrace > 0) depthBrace--;
      else if (depthParen === 0 && depthBracket === 0 && beforeOpenIsCodeContext(fullText, i)) return true;
    } else if (c === ")") depthParen++;
    else if (c === "(") {
      if (depthParen > 0) depthParen--;
      else if (depthBracket === 0 && depthBrace === 0 && beforeOpenIsCodeContext(fullText, i)) return true;
    } else if (c === "]") depthBracket++;
    else if (c === "[") {
      if (depthBracket > 0) depthBracket--;
      else if (depthParen === 0 && depthBrace === 0 && beforeOpenIsCodeContext(fullText, i)) return true;
    }
  }
  return false;
}

/**
 * Lines like `align: horizon,` or `inset: 8pt,` (only idents / dotted nums, commas).
 * Avoids skipping `Hinweis: langer Fließtext`.
 */
function lineLooksLikeTypstNamedArgList(line: string): boolean {
  const t = line.trim();
  const m = /^([\w.-]+)\s*:\s*(.+)$/u.exec(t);
  if (!m) return false;
  const rest = m[2].trim();
  if (rest.length === 0) return false;
  if (/\s\s/.test(rest)) return false;

  if (rest.startsWith("(") && /:\s*\d/u.test(rest)) return true;

  const collapsed = rest
    .replace(/"(?:[^"\\]|\\.)*"/gu, "x")
    .replace(/'(?:[^'\\]|\\.)*'/gu, "x");
  if (/\s\s/.test(collapsed)) return false;
  if (!/^[\w.+-x]+(\s*,\s*[\w.+-x]+)*\s*,?\s*$/u.test(collapsed)) return false;
  if (!/^\d/u.test(collapsed) && !collapsed.includes(",") && !t.endsWith(",")) return false;
  return true;
}

/** `15th` → only `th` matches WORD; skip when it follows a digit (English ordinals). */
function looksLikeEnglishOrdinalSuffix(fullText: string, from: number, to: number): boolean {
  if (from < 1) return false;
  const w = fullText.slice(from, to);
  if (!/^(st|nd|rd|th)$/iu.test(w)) return false;
  return /\d/u.test(fullText[from - 1]);
}

function shouldSkipWord(
  fullText: string,
  from: number,
  to: number,
  spellLang: "de" | "en",
): boolean {
  const before = from > 0 ? fullText.charCodeAt(from - 1) : 0;
  if (before === 35 /* # */) return true;
  if (spellLang === "en" && looksLikeEnglishOrdinalSuffix(fullText, from, to)) return true;

  const lineStart = fullText.lastIndexOf("\n", from - 1) + 1;
  let lineEnd = fullText.indexOf("\n", to);
  if (lineEnd === -1) lineEnd = fullText.length;
  const line = fullText.slice(lineStart, lineEnd);
  const trimmed = line.trimStart();
  if (/^#\w/u.test(trimmed)) return true;
  if (/^\s*\)+,?\s*$/.test(line)) return true;
  if (lineLooksLikeTypstNamedArgList(line)) return true;
  if (
    /^#(?:import|include|let|show|set|outline|figure|table|grid|stack|box|block|page|par|text|align|pad|move|scale|rotate|circle|ellipse|rect|line|polygon|path|enum|list|terms|cite|ref|link|label|footnote|strong|emph|raw|sym|calc|metadata|repr|eval)\b/u.test(
      trimmed,
    )
  ) {
    return true;
  }

  if (insideTypstCodeDelimiters(fullText, from)) return true;

  const w = fullText.slice(from, to);
  if (w.length >= 2 && w === w.toUpperCase() && /^[\p{L}\p{M}]+$/u.test(w)) return true;

  return false;
}

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

export function runTypstSpellScanPlain(
  full: string,
  spell: SpellChecker,
  unknownMessage: string,
  suggestionsLabel: string,
  spellLang: "de" | "en",
): SpellDiagPlain[] {
  const out: SpellDiagPlain[] = [];
  WORD.lastIndex = 0;
  let m: RegExpExecArray | null;
  while ((m = WORD.exec(full)) !== null) {
    if (out.length >= MAX_DIAGNOSTICS) break;
    const from = m.index;
    const to = from + m[0].length;
    if (shouldSkipWord(full, from, to, spellLang)) continue;

    const raw = m[0];
    if (wordAccepted(spell, raw, spellLang)) continue;

    const refined = refineSuggestions(spell, raw, spell.suggest(raw).slice(0, SUGGEST_POOL), spellLang);
    const listed = refined.slice(0, MAX_SUGGESTION_ACTIONS);
    const message =
      structuredMessage(unknownMessage, suggestionsLabel, listed);

    out.push({
      from,
      to,
      message,
      replacements: listed,
    });
  }
  return out;
}

function structuredMessage(
  unknownMessage: string,
  suggestionsLabel: string,
  listed: string[],
): string {
  return listed.length > 0
    ? `${unknownMessage} · ${suggestionsLabel}: ${listed.join(", ")}`
    : unknownMessage;
}
