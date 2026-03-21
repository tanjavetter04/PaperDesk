import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { tags } from "@lezer/highlight";
import type { Extension } from "@codemirror/state";
import { EditorView } from "@codemirror/view";

/** Light editor chrome + highlighting (pairs with `oneDark` from `@codemirror/theme-one-dark`). */
const background = "#f6f8fa";
const gutterBackground = "#eaeef2";
const foreground = "#24292f";
const muted = "#57606a";

const paperDeskLightTheme = EditorView.theme(
  {
    "&": {
      color: foreground,
      backgroundColor: background,
    },
    ".cm-content": { caretColor: "#0969da" },
    ".cm-cursor, .cm-dropCursor": { borderLeftColor: "#0969da" },
    "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection":
      { backgroundColor: "#add6ff" },
    ".cm-panels": { backgroundColor: gutterBackground, color: foreground },
    ".cm-panels.cm-panels-top": { borderBottom: "1px solid #d0d7de" },
    ".cm-panels.cm-panels-bottom": { borderTop: "1px solid #d0d7de" },
    ".cm-searchMatch": {
      backgroundColor: "#fff8c5",
      outline: "1px solid #d4a72c",
    },
    ".cm-searchMatch.cm-searchMatch-selected": {
      backgroundColor: "#ffea7f80",
    },
    ".cm-activeLine": { backgroundColor: "#818b981f" },
    ".cm-selectionMatch": { backgroundColor: "#bf87004d" },
    "&.cm-focused .cm-matchingBracket, &.cm-focused .cm-nonmatchingBracket": {
      backgroundColor: "#0969da33",
    },
    ".cm-gutters": {
      backgroundColor: gutterBackground,
      color: muted,
      border: "none",
    },
    ".cm-activeLineGutter": { backgroundColor: "#d8dee4" },
    ".cm-foldPlaceholder": {
      backgroundColor: "transparent",
      border: "none",
      color: muted,
    },
    ".cm-tooltip": {
      border: "1px solid #d0d7de",
      backgroundColor: "#ffffff",
    },
    ".cm-tooltip .cm-tooltip-arrow:before": {
      borderTopColor: "transparent",
      borderBottomColor: "transparent",
    },
    ".cm-tooltip .cm-tooltip-arrow:after": {
      borderTopColor: "#ffffff",
      borderBottomColor: "#ffffff",
    },
    ".cm-tooltip-autocomplete": {
      "& > ul > li[aria-selected]": {
        backgroundColor: "#0969da",
        color: "#ffffff",
      },
    },
  },
  { dark: false },
);

const paperDeskLightHighlight = HighlightStyle.define([
  { tag: tags.keyword, color: "#cf222e" },
  {
    tag: [tags.name, tags.deleted, tags.character, tags.propertyName, tags.macroName],
    color: "#8250df",
  },
  {
    tag: [tags.function(tags.variableName), tags.labelName],
    color: "#0969da",
  },
  {
    tag: [
      tags.color,
      tags.constant(tags.name),
      tags.standard(tags.name),
    ],
    color: "#0550ae",
  },
  { tag: [tags.definition(tags.name), tags.separator], color: foreground },
  {
    tag: [
      tags.typeName,
      tags.className,
      tags.number,
      tags.changed,
      tags.annotation,
      tags.modifier,
      tags.self,
      tags.namespace,
    ],
    color: "#953800",
  },
  {
    tag: [
      tags.operator,
      tags.operatorKeyword,
      tags.url,
      tags.escape,
      tags.regexp,
      tags.link,
      tags.special(tags.string),
    ],
    color: "#0550ae",
  },
  { tag: [tags.meta, tags.comment], color: muted, fontStyle: "italic" },
  { tag: tags.strong, fontWeight: "bold" },
  { tag: tags.emphasis, fontStyle: "italic" },
  { tag: tags.strikethrough, textDecoration: "line-through" },
  { tag: tags.link, color: "#0969da", textDecoration: "underline" },
  { tag: tags.heading, fontWeight: "bold", color: "#cf222e" },
  {
    tag: [tags.atom, tags.bool, tags.special(tags.variableName)],
    color: "#0550ae",
  },
  { tag: [tags.processingInstruction, tags.string, tags.inserted], color: "#0a3069" },
  { tag: tags.invalid, color: "#f85149" },
]);

export const paperDeskLightCm: Extension = [
  paperDeskLightTheme,
  syntaxHighlighting(paperDeskLightHighlight),
];
