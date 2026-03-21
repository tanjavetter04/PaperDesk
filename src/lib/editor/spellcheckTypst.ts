import type { Diagnostic } from "@codemirror/lint";
import { EditorView } from "@codemirror/view";

import type { SpellDiagPlain } from "./spellcheckTypstCore";

export type { SpellDiagPlain } from "./spellcheckTypstCore";

/** Turn worker JSON into CodeMirror diagnostics (fix-it actions). */
export function spellPlainToCmDiagnostics(plain: SpellDiagPlain[]): Diagnostic[] {
  return plain.map((d) => ({
    from: d.from,
    to: d.to,
    severity: "error" as const,
    source: "spell",
    message: d.message,
    actions:
      d.replacements.length > 0
        ? d.replacements.map((replacement) => ({
            name: replacement,
            apply(view: EditorView, a: number, b: number) {
              view.dispatch({ changes: { from: a, to: b, insert: replacement } });
            },
          }))
        : undefined,
  }));
}
