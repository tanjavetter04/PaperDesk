import {
  autocompletion,
  type Completion,
  type CompletionContext,
  completionKeymap,
} from "@codemirror/autocomplete";
import type { Extension, Text } from "@codemirror/state";

const S = {
  doc: { name: "Dokument", rank: 10 },
  layout: { name: "Layout & Boxen", rank: 20 },
  text: { name: "Text & Verweise", rank: 30 },
  lists: { name: "Listen & Tabellen", rank: 40 },
  visual: { name: "Grafiken", rank: 50 },
  math: { name: "Mathe", rank: 60 },
  mathGreek: { name: "Mathe · Griechisch", rank: 61 },
  mathFn: { name: "Mathe · Funktionen", rank: 62 },
  mathStruct: { name: "Mathe · Struktur", rank: 63 },
  mathOp: { name: "Mathe · Operatoren & Symbole", rank: 64 },
  util: { name: "Hilfen", rank: 70 },
  logic: { name: "Logik & Module", rank: 80 },
} as const;

/** True wenn der Cursor zwischen `$ … $` liegt (`\$` zählt nicht). */
function isInTypstMath(doc: Text, pos: number): boolean {
  let inMath = false;
  for (let i = 0; i < pos; i++) {
    const ch = doc.sliceString(i, i + 1);
    if (ch === "\\") {
      i += 1;
      continue;
    }
    if (ch === "$") inMath = !inMath;
  }
  return inMath;
}

/** Aktuelles Mathe-Token (Buchstaben, Ziffern, `_`, `.`) für Filter/Replace. */
function mathTokenBounds(doc: Text, pos: number): { from: number; to: number } {
  let from = pos;
  while (from > 0) {
    const c = doc.sliceString(from - 1, from);
    if (/[\w.]/.test(c)) from -= 1;
    else break;
  }
  return { from, to: pos };
}

/** Häufige Typst-Syntax nach `#` (Stand Typst 0.14). */
const TYPST_AFTER_HASH: readonly Completion[] = [
  // Dokument
  { label: "set", type: "keyword", detail: "Stil / Optionen", section: S.doc },
  { label: "show", type: "keyword", detail: "Show-Regel", section: S.doc },
  { label: "import", type: "keyword", detail: 'Modul importieren (#import "pfad")', section: S.doc },
  { label: "include", type: "keyword", detail: 'Datei einbinden (#include "pfad")', section: S.doc },
  { label: "metadata", type: "keyword", detail: "Dokument-Metadaten", section: S.doc },

  // Layout
  { label: "page", type: "keyword", detail: "Seitenlayout", section: S.layout },
  { label: "par", type: "keyword", detail: "Absatz", section: S.layout },
  { label: "linebreak", type: "keyword", detail: "Zeilenumbruch", section: S.layout },
  { label: "pagebreak", type: "keyword", detail: "Seitenumbruch", section: S.layout },
  { label: "colbreak", type: "keyword", detail: "Spaltenumbruch", section: S.layout },
  { label: "align", type: "function", detail: "Ausrichtung", section: S.layout },
  { label: "block", type: "keyword", detail: "Block", section: S.layout },
  { label: "box", type: "function", detail: "Inline-Box", section: S.layout },
  { label: "stack", type: "function", detail: "Vertikal/horizontal stapeln", section: S.layout },
  { label: "grid", type: "function", detail: "Raster", section: S.layout },
  { label: "columns", type: "keyword", detail: "Mehrspaltig", section: S.layout },
  { label: "place", type: "function", detail: "Absolut platzieren", section: S.layout },
  { label: "pad", type: "function", detail: "Innenabstand", section: S.layout },
  { label: "move", type: "function", detail: "Verschieben", section: S.layout },
  { label: "scale", type: "function", detail: "Skalieren", section: S.layout },
  { label: "rotate", type: "function", detail: "Drehen", section: S.layout },
  { label: "repeat", type: "function", detail: "Wiederholen", section: S.layout },
  { label: "hide", type: "keyword", detail: "Ausblenden", section: S.layout },

  // Text & Verweise
  { label: "text", type: "function", detail: "Text mit Font/Größe", section: S.text },
  { label: "strong", type: "keyword", detail: "Fett", section: S.text },
  { label: "emph", type: "keyword", detail: "Kursiv", section: S.text },
  { label: "underline", type: "keyword", detail: "Unterstrichen", section: S.text },
  { label: "strike", type: "keyword", detail: "Durchgestrichen", section: S.text },
  { label: "highlight", type: "keyword", detail: "Hervorheben", section: S.text },
  { label: "sub", type: "keyword", detail: "Tiefgestellt", section: S.text },
  { label: "super", type: "keyword", detail: "Hochgestellt", section: S.text },
  { label: "smallcaps", type: "keyword", detail: "Kapitälchen", section: S.text },
  { label: "link", type: "function", detail: "Hyperlink", section: S.text },
  { label: "ref", type: "function", detail: "Referenz auf Label", section: S.text },
  { label: "label", type: "function", detail: "Label setzen", section: S.text },
  { label: "footnote", type: "keyword", detail: "Fußnote", section: S.text },
  { label: "quote", type: "keyword", detail: "Zitat", section: S.text },
  { label: "cite", type: "function", detail: "Literaturzitat", section: S.text },
  { label: "bibliography", type: "function", detail: "Bibliographie", section: S.text },
  { label: "outline", type: "keyword", detail: "Inhaltsverzeichnis", section: S.text },
  { label: "heading", type: "function", detail: "Überschrift (Markup: = …)", section: S.text },

  // Listen & Tabellen
  { label: "list", type: "keyword", detail: "Aufzählung", section: S.lists },
  { label: "enum", type: "keyword", detail: "Nummerierte Liste", section: S.lists },
  { label: "terms", type: "keyword", detail: "Definitionsliste", section: S.lists },
  { label: "table", type: "function", detail: "Tabelle", section: S.lists },
  { label: "figure", type: "keyword", detail: "Abbildung mit Beschriftung", section: S.lists },

  // Grafik
  { label: "image", type: "function", detail: "Bild einbinden", section: S.visual },
  { label: "rect", type: "function", detail: "Rechteck", section: S.visual },
  { label: "circle", type: "function", detail: "Kreis", section: S.visual },
  { label: "ellipse", type: "function", detail: "Ellipse", section: S.visual },
  { label: "line", type: "function", detail: "Linie", section: S.visual },
  { label: "polygon", type: "function", detail: "Polygon", section: S.visual },
  { label: "path", type: "function", detail: "Pfad", section: S.visual },

  // Mathe
  { label: "math", type: "keyword", detail: "Mathe-Modus ($ … $)", section: S.math },
  { label: "sym", type: "function", detail: "Symbol (z. B. sym.arrow)", section: S.math },
  { label: "eval", type: "keyword", detail: "Code in Mathe auswerten", section: S.math },

  // Hilfen
  { label: "lorem", type: "function", detail: "Blindtext", section: S.util, boost: 4 },
  { label: "raw", type: "keyword", detail: "Rohtext / Code", section: S.util },
  { label: "counter", type: "function", detail: "Zähler", section: S.util },
  { label: "state", type: "keyword", detail: "Zustand (context)", section: S.util },
  { label: "context", type: "keyword", detail: "Kontext-Callback", section: S.util },
  { label: "locate", type: "keyword", detail: "Position auf Seite", section: S.util },
  { label: "query", type: "function", detail: "Selektor-Abfrage", section: S.util },
  { label: "selector", type: "keyword", detail: "Selektor", section: S.util },
  { label: "numbering", type: "function", detail: "Nummerierungsformat", section: S.util },

  // Logik & Typen
  { label: "let", type: "keyword", detail: "Variable / Funktion", section: S.logic },
  { label: "if", type: "keyword", detail: "Bedingung", section: S.logic },
  { label: "else", type: "keyword", detail: "Alternative", section: S.logic },
  { label: "for", type: "keyword", detail: "Schleife", section: S.logic },
  { label: "while", type: "keyword", detail: "While-Schleife", section: S.logic },
  { label: "break", type: "keyword", detail: "Schleife verlassen", section: S.logic },
  { label: "continue", type: "keyword", detail: "Schleifendurchlauf überspringen", section: S.logic },
  { label: "return", type: "keyword", detail: "Rückgabe", section: S.logic },
];

/** Vorschläge innerhalb von `$ … $` (Typst-Mathe, Stand 0.14). */
const TYPST_IN_MATH: readonly Completion[] = [
  // Griechisch
  { label: "alpha", type: "constant", detail: "α", section: S.mathGreek },
  { label: "beta", type: "constant", detail: "β", section: S.mathGreek },
  { label: "gamma", type: "constant", detail: "γ", section: S.mathGreek },
  { label: "delta", type: "constant", detail: "δ", section: S.mathGreek },
  { label: "epsilon", type: "constant", detail: "ε", section: S.mathGreek },
  { label: "epsilon.alt", type: "constant", detail: "ϵ", section: S.mathGreek },
  { label: "zeta", type: "constant", detail: "ζ", section: S.mathGreek },
  { label: "eta", type: "constant", detail: "η", section: S.mathGreek },
  { label: "theta", type: "constant", detail: "θ", section: S.mathGreek },
  { label: "theta.alt", type: "constant", detail: "ϑ", section: S.mathGreek },
  { label: "iota", type: "constant", detail: "ι", section: S.mathGreek },
  { label: "kappa", type: "constant", detail: "κ", section: S.mathGreek },
  { label: "lambda", type: "constant", detail: "λ", section: S.mathGreek },
  { label: "mu", type: "constant", detail: "μ", section: S.mathGreek },
  { label: "nu", type: "constant", detail: "ν", section: S.mathGreek },
  { label: "xi", type: "constant", detail: "ξ", section: S.mathGreek },
  { label: "omicron", type: "constant", detail: "ο", section: S.mathGreek },
  { label: "pi", type: "constant", detail: "π", section: S.mathGreek },
  { label: "rho", type: "constant", detail: "ρ", section: S.mathGreek },
  { label: "rho.alt", type: "constant", detail: "ϱ", section: S.mathGreek },
  { label: "sigma", type: "constant", detail: "σ", section: S.mathGreek },
  { label: "tau", type: "constant", detail: "τ", section: S.mathGreek },
  { label: "upsilon", type: "constant", detail: "υ", section: S.mathGreek },
  { label: "phi", type: "constant", detail: "φ", section: S.mathGreek },
  { label: "phi.alt", type: "constant", detail: "ϕ", section: S.mathGreek },
  { label: "chi", type: "constant", detail: "χ", section: S.mathGreek },
  { label: "psi", type: "constant", detail: "ψ", section: S.mathGreek },
  { label: "omega", type: "constant", detail: "ω", section: S.mathGreek },
  { label: "Alpha", type: "constant", detail: "Α", section: S.mathGreek },
  { label: "Beta", type: "constant", detail: "Β", section: S.mathGreek },
  { label: "Gamma", type: "constant", detail: "Γ", section: S.mathGreek },
  { label: "Delta", type: "constant", detail: "Δ", section: S.mathGreek },
  { label: "Epsilon", type: "constant", detail: "Ε", section: S.mathGreek },
  { label: "Zeta", type: "constant", detail: "Ζ", section: S.mathGreek },
  { label: "Eta", type: "constant", detail: "Η", section: S.mathGreek },
  { label: "Theta", type: "constant", detail: "Θ", section: S.mathGreek },
  { label: "Iota", type: "constant", detail: "Ι", section: S.mathGreek },
  { label: "Kappa", type: "constant", detail: "Κ", section: S.mathGreek },
  { label: "Lambda", type: "constant", detail: "Λ", section: S.mathGreek },
  { label: "Mu", type: "constant", detail: "Μ", section: S.mathGreek },
  { label: "Nu", type: "constant", detail: "Ν", section: S.mathGreek },
  { label: "Xi", type: "constant", detail: "Ξ", section: S.mathGreek },
  { label: "Omicron", type: "constant", detail: "Ο", section: S.mathGreek },
  { label: "Pi", type: "constant", detail: "Π", section: S.mathGreek },
  { label: "Rho", type: "constant", detail: "Ρ", section: S.mathGreek },
  { label: "Sigma", type: "constant", detail: "Σ", section: S.mathGreek },
  { label: "Tau", type: "constant", detail: "Τ", section: S.mathGreek },
  { label: "Upsilon", type: "constant", detail: "Υ", section: S.mathGreek },
  { label: "Phi", type: "constant", detail: "Φ", section: S.mathGreek },
  { label: "Chi", type: "constant", detail: "Χ", section: S.mathGreek },
  { label: "Psi", type: "constant", detail: "Ψ", section: S.mathGreek },
  { label: "Omega", type: "constant", detail: "Ω", section: S.mathGreek },

  // Funktionen & Standard-Symbole
  { label: "sqrt", type: "function", detail: "Wurzel √(…)", section: S.mathFn },
  { label: "root", type: "function", detail: "n-te Wurzel", section: S.mathFn },
  { label: "sum", type: "keyword", detail: "Summe ∑", section: S.mathFn, boost: 2 },
  { label: "product", type: "keyword", detail: "Produkt ∏", section: S.mathFn },
  { label: "integral", type: "keyword", detail: "Integral ∫", section: S.mathFn },
  { label: "int", type: "keyword", detail: "∫ (kurz)", section: S.mathFn },
  { label: "iint", type: "keyword", detail: "Doppelintegral", section: S.mathFn },
  { label: "iiint", type: "keyword", detail: "Dreifachintegral", section: S.mathFn },
  { label: "oint", type: "keyword", detail: "Konturintegral ∮", section: S.mathFn },
  { label: "lim", type: "keyword", detail: "Grenzwert", section: S.mathFn },
  { label: "max", type: "keyword", detail: "Maximum", section: S.mathFn },
  { label: "min", type: "keyword", detail: "Minimum", section: S.mathFn },
  { label: "sup", type: "keyword", detail: "Supremum", section: S.mathFn },
  { label: "inf", type: "keyword", detail: "Infimum", section: S.mathFn },
  { label: "gcd", type: "function", detail: "ggT", section: S.mathFn },
  { label: "lcm", type: "function", detail: "kgV", section: S.mathFn },
  { label: "log", type: "function", detail: "Logarithmus", section: S.mathFn },
  { label: "ln", type: "function", detail: "Natürlicher Log", section: S.mathFn },
  { label: "lg", type: "function", detail: "Log₁₀", section: S.mathFn },
  { label: "exp", type: "function", detail: "e^…", section: S.mathFn },
  { label: "sin", type: "function", detail: "Sinus", section: S.mathFn },
  { label: "cos", type: "function", detail: "Kosinus", section: S.mathFn },
  { label: "tan", type: "function", detail: "Tangens", section: S.mathFn },
  { label: "sec", type: "function", detail: "Sekans", section: S.mathFn },
  { label: "csc", type: "function", detail: "Kosekans", section: S.mathFn },
  { label: "cot", type: "function", detail: "Kotangens", section: S.mathFn },
  { label: "sinh", type: "function", detail: "Sinh", section: S.mathFn },
  { label: "cosh", type: "function", detail: "Cosh", section: S.mathFn },
  { label: "tanh", type: "function", detail: "Tanh", section: S.mathFn },
  { label: "arcsin", type: "function", detail: "Arcsin", section: S.mathFn },
  { label: "arccos", type: "function", detail: "Arccos", section: S.mathFn },
  { label: "arctan", type: "function", detail: "Arctan", section: S.mathFn },
  { label: "norm", type: "function", detail: "‖·‖", section: S.mathFn },
  { label: "abs", type: "function", detail: "|·|", section: S.mathFn },
  { label: "floor", type: "function", detail: "Abrunden ⌊·⌋", section: S.mathFn },
  { label: "ceil", type: "function", detail: "Aufrunden ⌈·⌉", section: S.mathFn },
  { label: "round", type: "function", detail: "Runden", section: S.mathFn },
  { label: "re", type: "function", detail: "Realteil", section: S.mathFn },
  { label: "im", type: "function", detail: "Imaginärteil", section: S.mathFn },
  { label: "conj", type: "function", detail: "Konjugiert", section: S.mathFn },
  { label: "arg", type: "function", detail: "Argument (Winkel)", section: S.mathFn },
  { label: "det", type: "function", detail: "Determinante", section: S.mathFn },
  { label: "tr", type: "function", detail: "Spur (trace)", section: S.mathFn },
  { label: "rank", type: "function", detail: "Rang", section: S.mathFn },
  { label: "dim", type: "function", detail: "Dimension", section: S.mathFn },
  { label: "ker", type: "function", detail: "Kern", section: S.mathFn },

  // Aufbau & Schrift
  { label: "frac", type: "function", detail: "Bruch a/b", section: S.mathStruct, boost: 3 },
  { label: "binom", type: "function", detail: "Binomialkoeffizient", section: S.mathStruct },
  { label: "vec", type: "function", detail: "Vektor (Pfeil)", section: S.mathStruct },
  { label: "mat", type: "function", detail: "Matrix", section: S.mathStruct },
  { label: "cases", type: "function", detail: "Fallunterscheidung {", section: S.mathStruct },
  { label: "text", type: "function", detail: 'Text in Formel: text("…")', section: S.mathStruct },
  { label: "upright", type: "function", detail: "Aufrechte Schrift", section: S.mathStruct },
  { label: "italic", type: "function", detail: "Kursiv", section: S.mathStruct },
  { label: "bold", type: "function", detail: "Fett", section: S.mathStruct },
  { label: "bb", type: "function", detail: "Blackboard (z. B. bb(R))", section: S.mathStruct },
  { label: "cal", type: "function", detail: "Kalligrafie", section: S.mathStruct },
  { label: "frak", type: "function", detail: "Fraktur", section: S.mathStruct },
  { label: "scr", type: "function", detail: "Script", section: S.mathStruct },
  { label: "sans", type: "function", detail: "Sans-Serif", section: S.mathStruct },
  { label: "mono", type: "function", detail: "Monospace", section: S.mathStruct },
  { label: "op", type: "function", detail: "Großer Operator", section: S.mathStruct },
  { label: "limits", type: "keyword", detail: "Grenzen unter/oben", section: S.mathStruct },

  // Operatoren, Relationen, Pfeile
  { label: "plus", type: "keyword", detail: "+", section: S.mathOp },
  { label: "minus", type: "keyword", detail: "−", section: S.mathOp },
  { label: "plus.minus", type: "keyword", detail: "±", section: S.mathOp },
  { label: "minus.plus", type: "keyword", detail: "∓", section: S.mathOp },
  { label: "times", type: "keyword", detail: "×", section: S.mathOp },
  { label: "dot", type: "keyword", detail: "·", section: S.mathOp },
  { label: "dot.op", type: "keyword", detail: "· (Operator)", section: S.mathOp },
  { label: "div", type: "keyword", detail: "÷", section: S.mathOp },
  { label: "ast", type: "keyword", detail: "∗", section: S.mathOp },
  { label: "star", type: "keyword", detail: "⋆", section: S.mathOp },
  { label: "bullet", type: "keyword", detail: "•", section: S.mathOp },
  { label: "circ", type: "keyword", detail: "∘", section: S.mathOp },
  { label: "without", type: "keyword", detail: "∖ (Mengendifferenz)", section: S.mathOp },
  { label: "union", type: "keyword", detail: "∪", section: S.mathOp },
  { label: "sect", type: "keyword", detail: "∩", section: S.mathOp },
  { label: "and", type: "keyword", detail: "∧", section: S.mathOp },
  { label: "or", type: "keyword", detail: "∨", section: S.mathOp },
  { label: "not", type: "keyword", detail: "¬", section: S.mathOp },
  { label: "xor", type: "keyword", detail: "⊕", section: S.mathOp },
  { label: "equiv", type: "keyword", detail: "≡", section: S.mathOp },
  { label: "approx", type: "keyword", detail: "≈", section: S.mathOp },
  { label: "propto", type: "keyword", detail: "∝", section: S.mathOp },
  { label: "sim", type: "keyword", detail: "∼", section: S.mathOp },
  { label: "eq", type: "keyword", detail: "=", section: S.mathOp },
  { label: "eq.not", type: "keyword", detail: "≠", section: S.mathOp },
  { label: "lt", type: "keyword", detail: "<", section: S.mathOp },
  { label: "gt", type: "keyword", detail: ">", section: S.mathOp },
  { label: "lt.eq", type: "keyword", detail: "≤", section: S.mathOp },
  { label: "gt.eq", type: "keyword", detail: "≥", section: S.mathOp },
  { label: "prec", type: "keyword", detail: "≺", section: S.mathOp },
  { label: "succ", type: "keyword", detail: "≻", section: S.mathOp },
  { label: "prec.eq", type: "keyword", detail: "≼", section: S.mathOp },
  { label: "succ.eq", type: "keyword", detail: "≽", section: S.mathOp },
  { label: "subset", type: "keyword", detail: "⊂", section: S.mathOp },
  { label: "supset", type: "keyword", detail: "⊃", section: S.mathOp },
  { label: "subset.eq", type: "keyword", detail: "⊆", section: S.mathOp },
  { label: "supset.eq", type: "keyword", detail: "⊇", section: S.mathOp },
  { label: "subset.neq", type: "keyword", detail: "⊊", section: S.mathOp },
  { label: "supset.neq", type: "keyword", detail: "⊋", section: S.mathOp },
  { label: "in", type: "keyword", detail: "∈", section: S.mathOp },
  { label: "in.not", type: "keyword", detail: "∉", section: S.mathOp },
  { label: "in.rev", type: "keyword", detail: "∋ (umgekehrtes ∈)", section: S.mathOp },
  { label: "arrow.r", type: "keyword", detail: "→", section: S.mathOp, boost: 2 },
  { label: "arrow.l", type: "keyword", detail: "←", section: S.mathOp },
  { label: "arrow.t", type: "keyword", detail: "↑", section: S.mathOp },
  { label: "arrow.b", type: "keyword", detail: "↓", section: S.mathOp },
  { label: "arrow.r.long", type: "keyword", detail: "⟶", section: S.mathOp },
  { label: "arrow.l.long", type: "keyword", detail: "⟵", section: S.mathOp },
  { label: "arrow.l.r", type: "keyword", detail: "↔", section: S.mathOp },
  { label: "arrow.r.double", type: "keyword", detail: "⇒", section: S.mathOp },
  { label: "arrow.l.double", type: "keyword", detail: "⇐", section: S.mathOp },
  { label: "arrow.l.r.double", type: "keyword", detail: "⇔", section: S.mathOp },
  { label: "mapsto", type: "keyword", detail: "↦", section: S.mathOp },
  { label: "harpoon.rt", type: "keyword", detail: "⇀ Harpune rechts oben", section: S.mathOp },
  { label: "harpoon.rb", type: "keyword", detail: "⇁ Harpune rechts unten", section: S.mathOp },
  { label: "harpoon.lt", type: "keyword", detail: "↼ Harpune links oben", section: S.mathOp },
  { label: "harpoon.lb", type: "keyword", detail: "↽ Harpune links unten", section: S.mathOp },
  { label: "infinity", type: "keyword", detail: "∞", section: S.mathOp, boost: 2 },
  { label: "emptyset", type: "keyword", detail: "∅", section: S.mathOp },
  { label: "forall", type: "keyword", detail: "∀", section: S.mathOp },
  { label: "exists", type: "keyword", detail: "∃", section: S.mathOp },
  { label: "exists.not", type: "keyword", detail: "∄", section: S.mathOp },
  { label: "top", type: "keyword", detail: "⊤", section: S.mathOp },
  { label: "bot", type: "keyword", detail: "⊥", section: S.mathOp },
  { label: "therefore", type: "keyword", detail: "∴", section: S.mathOp },
  { label: "because", type: "keyword", detail: "∵", section: S.mathOp },
  { label: "nabla", type: "keyword", detail: "∇", section: S.mathOp },
  { label: "partial", type: "keyword", detail: "∂", section: S.mathOp },
  { label: "prime", type: "keyword", detail: "′", section: S.mathOp },
  { label: "dots.h", type: "keyword", detail: "… horizontal", section: S.mathOp },
  { label: "dots.v", type: "keyword", detail: "⋮ vertikal", section: S.mathOp },
  { label: "dots.c", type: "keyword", detail: "⋯ zentriert", section: S.mathOp },
  { label: "parallel", type: "keyword", detail: "∥", section: S.mathOp },
  { label: "perp", type: "keyword", detail: "⊥ (senkrecht)", section: S.mathOp },
  { label: "angle", type: "keyword", detail: "∠", section: S.mathOp },
  { label: "degree", type: "keyword", detail: "°", section: S.mathOp },
];

function typstCompletionSource(context: CompletionContext) {
  const beforeHash = context.matchBefore(/#[\w]*$/);
  if (beforeHash) {
    const from = beforeHash.from + 1;
    const to = context.pos;
    return {
      from,
      to,
      options: TYPST_AFTER_HASH,
      validFor: /^[\w]*$/,
    };
  }

  const doc = context.state.doc;
  if (isInTypstMath(doc, context.pos)) {
    const { from, to } = mathTokenBounds(doc, context.pos);
    if (from === to && !context.explicit) return null;
    return {
      from,
      to,
      options: TYPST_IN_MATH,
      validFor: /^[\w.]*$/,
    };
  }

  return null;
}

function completionKindBadge(completion: Completion): HTMLSpanElement {
  const el = document.createElement("span");
  el.className = "pd-cm-completion-kind";
  const raw = (completion.type ?? "text").split(/\s+/)[0] ?? "text";
  el.setAttribute("data-kind", raw);
  if (raw === "function") el.textContent = "fn";
  else if (raw === "keyword") el.textContent = "kw";
  else el.textContent = raw.slice(0, 2);
  return el;
}

/** Autocomplete für `.typ`: Vorschläge nach `#` und per Strg+Leertaste. */
export function typstAutocompleteBundle(): Extension[] {
  return [
    autocompletion({
      override: [typstCompletionSource],
      activateOnTyping: true,
      tooltipClass: () => "pd-cm-autocomplete",
      maxRenderedOptions: 72,
      icons: false,
      addToOptions: [
        {
          position: 18,
          render: completionKindBadge,
        },
      ],
    }),
  ];
}

export { completionKeymap };
