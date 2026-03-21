import type { Text } from "@codemirror/state";
import { utf16OffsetByCodePointSteps } from "$lib/editor/utf16CodePoints";

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
