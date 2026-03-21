import type { Text } from "@codemirror/state";
import type { Diagnostic } from "@codemirror/lint";
import type { CompileDiagnostic } from "$lib/tauri/api";

/** Backend may send absolute paths; open tab uses project-relative paths. */
export function diagnosticTargetsOpenFile(
  diagPath: string | null,
  openRel: string,
): boolean {
  if (diagPath == null || diagPath === "") return true;
  const open = openRel.replace(/\\/g, "/");
  const d = diagPath.replace(/\\/g, "/");
  return d === open || d.endsWith("/" + open);
}

/**
 * Typst reports 1-based line and column. Column is treated as a 1-based Unicode
 * code point index within the line (common for Rust tooling); mapped to UTF-16 for CodeMirror.
 */
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

function diagnosticToRange(
  doc: Text,
  d: CompileDiagnostic,
): { from: number; to: number } | null {
  if (d.line == null || d.line < 1 || d.line > doc.lines) return null;
  const line = doc.line(d.line);
  const col1 = d.column ?? 1;
  const col0 = Math.max(0, col1 - 1);
  const offsetInLine = utf16OffsetByCodePointSteps(line.text, col0);
  const from = line.from + Math.min(offsetInLine, line.length);
  let to = from;
  if (from < line.to) {
    to = Math.min(from + 1, line.to);
  } else if (line.length === 0) {
    to = from;
  } else {
    to = line.to;
  }
  return { from, to };
}

function severityForCm(s: string): Diagnostic["severity"] {
  if (s === "error") return "error";
  if (s === "warning") return "warning";
  return "info";
}

export function compileDiagnosticsToCm(
  doc: Text,
  openRel: string,
  diags: readonly CompileDiagnostic[],
): Diagnostic[] {
  const out: Diagnostic[] = [];
  for (const d of diags) {
    if (!diagnosticTargetsOpenFile(d.path, openRel)) continue;
    const range = diagnosticToRange(doc, d);
    if (!range) continue;
    let { from, to } = range;
    if (from === to) {
      const ln = doc.lineAt(from);
      if (from < ln.to) {
        to = from + 1;
      }
    }
    out.push({
      from,
      to,
      severity: severityForCm(d.severity),
      message: d.message,
      source: "typst",
    });
  }
  return out;
}

/** Cursor position for scrolling / selection; null if not mapped to this file. */
export function compileDiagnosticCursorPos(
  doc: Text,
  openRel: string,
  d: CompileDiagnostic,
): number | null {
  if (!diagnosticTargetsOpenFile(d.path, openRel)) return null;
  const range = diagnosticToRange(doc, d);
  return range?.from ?? null;
}
