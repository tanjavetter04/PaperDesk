import type { Text } from "@codemirror/state";

/** Map 0-based Unicode code point column within line to UTF-16 offset (CodeMirror). */
function utf16OffsetByCodePointSteps(lineText: string, codePoints: number): number {
  if (codePoints <= 0) return 0;
  let u16 = 0;
  let taken = 0;
  for (let i = 0; i < lineText.length && taken < codePoints; ) {
    const cp = lineText.codePointAt(i)!;
    const charLen = cp > 0xffff ? 2 : 1;
    u16 += charLen;
    i += charLen;
    taken++;
  }
  return u16;
}

/**
 * Tinymist `editorScrollTo` uses 0-based line index and column (per typst source lines).
 */
export function cursorPosFromTinymistEditorScroll(
  doc: Text,
  line0: number,
  column0: number,
): number | null {
  const line1 = line0 + 1;
  if (line1 < 1 || line1 > doc.lines) return null;
  const line = doc.line(line1);
  const offsetInLine = utf16OffsetByCodePointSteps(line.text, column0);
  const from = line.from + Math.min(offsetInLine, line.length);
  return from;
}
