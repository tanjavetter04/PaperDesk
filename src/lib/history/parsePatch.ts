/** Parse a unified patch string (e.g. from git2 DiffFormat::Patch) for structured UI. */

export type DiffLineKind = "add" | "del" | "ctx" | "meta" | "hunk";

export interface DiffViewLine {
  kind: DiffLineKind;
  raw: string;
}

export interface DiffViewHunk {
  header: string;
  lines: DiffViewLine[];
}

export interface DiffViewFile {
  /** Best-effort path from diff --git (b/ side). */
  path: string;
  headerLine: string;
  metaBeforeHunks: DiffViewLine[];
  hunks: DiffViewHunk[];
}

export interface ParsedPatch {
  files: DiffViewFile[];
  truncated: boolean;
  preamble: string;
}

export function patchLooksTruncated(text: string): boolean {
  return /\[(diff truncated|truncated)\]/i.test(text);
}

/** Strip libgit2-style stray origin chars if present (legacy strings before backend fix). */
export function normalizePatchText(text: string): string {
  return text
    .replace(/\r\n/g, "\n")
    .replace(/(^|\n)([FH])(diff --git )/g, "$1$3")
    .replace(/(^|\n)H(@@)/g, "$1$2");
}

function classifyLine(line: string): DiffLineKind {
  if (line.startsWith("@@")) return "hunk";
  if (
    line.startsWith("+++ ") ||
    line.startsWith("--- ") ||
    line.startsWith("diff --git") ||
    line.startsWith("new file mode") ||
    line.startsWith("deleted file mode") ||
    line.startsWith("similarity index") ||
    line.startsWith("dissimilarity index") ||
    line.startsWith("rename from") ||
    line.startsWith("rename to") ||
    line.startsWith("Binary files") ||
    line.startsWith("index ")
  ) {
    return "meta";
  }
  if (line.startsWith("\\")) return "meta";
  if (line.startsWith("+")) return "add";
  if (line.startsWith("-")) return "del";
  if (line.startsWith(" ") || line === "") return "ctx";
  return "meta";
}

function pathFromDiffGit(line: string): string {
  if (!line.startsWith("diff --git ")) return line;
  const rest = line.slice("diff --git ".length);
  const parts = rest.split(/\s+/).filter(Boolean);
  const bPart = [...parts].reverse().find((p) => p.startsWith("b/"));
  if (bPart) return bPart.slice(2);
  const aPart = parts.find((p) => p.startsWith("a/"));
  if (aPart) return aPart.slice(2);
  return rest.trim() || line;
}

function splitIntoFilePatches(text: string): string[] {
  const lines = text.split("\n");
  const chunks: string[][] = [];
  let cur: string[] = [];
  for (const line of lines) {
    if (line.startsWith("diff --git ") && cur.length > 0) {
      chunks.push(cur);
      cur = [];
    }
    cur.push(line);
  }
  if (cur.length) chunks.push(cur);
  return chunks.map((c) => c.join("\n"));
}

function parseFilePatch(chunk: string): DiffViewFile {
  const lines = chunk.split("\n");
  const headerLine = lines[0] ?? "";
  const path = pathFromDiffGit(headerLine);
  const metaBeforeHunks: DiffViewLine[] = [];
  const hunks: DiffViewHunk[] = [];
  let i = 1;
  while (i < lines.length && !lines[i].startsWith("@@")) {
    metaBeforeHunks.push({ kind: classifyLine(lines[i]), raw: lines[i] });
    i++;
  }
  while (i < lines.length) {
    if (lines[i].startsWith("@@")) {
      const hunkHeader = lines[i];
      const hunkLines: DiffViewLine[] = [];
      i++;
      while (
        i < lines.length &&
        !lines[i].startsWith("@@") &&
        !lines[i].startsWith("diff --git ")
      ) {
        hunkLines.push({ kind: classifyLine(lines[i]), raw: lines[i] });
        i++;
      }
      hunks.push({ header: hunkHeader, lines: hunkLines });
    } else {
      i++;
    }
  }
  return { path, headerLine, metaBeforeHunks, hunks };
}

/**
 * Strips the truncation marker line from the tail so it is not rendered as diff content.
 */
function stripTruncationMarker(text: string): string {
  return text.replace(/\n*\[diff truncated\]\s*$/i, "").replace(/\n*\[truncated\]\s*$/i, "");
}

export function parseGitPatch(patch: string): ParsedPatch {
  const truncated = patchLooksTruncated(patch);
  const normalized = normalizePatchText(patch);
  const withoutTrunc = stripTruncationMarker(normalized);

  const firstDiff = withoutTrunc.search(/^diff --git /m);
  let preamble = "";
  let body = withoutTrunc;
  if (firstDiff === -1) {
    preamble = withoutTrunc;
    body = "";
  } else if (firstDiff > 0) {
    preamble = withoutTrunc.slice(0, firstDiff).trimEnd();
    body = withoutTrunc.slice(firstDiff);
  }

  const rawFiles = body ? splitIntoFilePatches(body) : [];
  const files = rawFiles.map(parseFilePatch);
  return { files, truncated, preamble };
}

const REASON_LABELS: Record<string, string> = {
  idle: "Pause",
  manual: "Manuell",
  "manual-save": "Speichern",
  "new-file": "Neue Datei",
  "new folder": "Neuer Ordner",
  export: "Export",
  compile: "Kompilieren",
  hub: "Projekt verlassen",
  close: "Schließen",
  "move/rename": "Verschieben/Umbenennen",
};

/**
 * Human-readable label for `paperdesk: …` checkpoint messages (handles accidental double prefix).
 */
export function checkpointDisplayLabel(message: string): { label: string; raw: string } {
  const raw = message.trim();
  let rest = raw;
  while (/^paperdesk:\s*/i.test(rest)) {
    rest = rest.replace(/^paperdesk:\s*/i, "").trim();
  }
  const label = REASON_LABELS[rest] ?? (rest || "Checkpoint");
  return { label, raw };
}
