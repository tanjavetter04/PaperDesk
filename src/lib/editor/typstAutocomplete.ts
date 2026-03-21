import {
  autocompletion,
  type Completion,
  type CompletionContext,
  completionKeymap,
} from "@codemirror/autocomplete";
import type { Extension } from "@codemirror/state";

const S = {
  doc: { name: "Dokument", rank: 10 },
  layout: { name: "Layout & Boxen", rank: 20 },
  text: { name: "Text & Verweise", rank: 30 },
  lists: { name: "Listen & Tabellen", rank: 40 },
  visual: { name: "Grafiken", rank: 50 },
  math: { name: "Mathe", rank: 60 },
  util: { name: "Hilfen", rank: 70 },
  logic: { name: "Logik & Module", rank: 80 },
} as const;

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
