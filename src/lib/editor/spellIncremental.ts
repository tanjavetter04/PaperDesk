import type { SpellDiagPlain } from "./spellcheckTypstCore";

/** One contiguous replace: `before` → `after` with shared prefix/suffix. */
export type SingleEditInfo = {
  prefixLen: number;
  suffixLen: number;
  oldMidStart: number;
  oldMidEnd: number;
  newMidStart: number;
  newMidEnd: number;
  delta: number;
};

const MAX_INCREMENTAL_MID_SUM = 12_000;
const MAX_INCREMENTAL_MID_EACH = 6_000;

/**
 * Describes a single edit as prefix + replaced middle + suffix (Myers-style).
 * Fails for some multi-patch cases; then caller should full-scan.
 */
export function trySingleEdit(before: string, after: string): SingleEditInfo | null {
  if (before === after) return null;
  const bl = before.length;
  const al = after.length;
  let p = 0;
  const minLen = Math.min(bl, al);
  while (p < minLen && before.charCodeAt(p) === after.charCodeAt(p)) p++;
  let s = 0;
  while (s < bl - p && s < al - p && before.charCodeAt(bl - 1 - s) === after.charCodeAt(al - 1 - s)) {
    s++;
  }
  const oldMidStart = p;
  const oldMidEnd = bl - s;
  const newMidStart = p;
  const newMidEnd = al - s;
  const oldMidLen = oldMidEnd - oldMidStart;
  const newMidLen = newMidEnd - newMidStart;
  if (oldMidLen + newMidLen > MAX_INCREMENTAL_MID_SUM) return null;
  if (oldMidLen > MAX_INCREMENTAL_MID_EACH || newMidLen > MAX_INCREMENTAL_MID_EACH) return null;
  return {
    prefixLen: p,
    suffixLen: s,
    oldMidStart,
    oldMidEnd,
    newMidStart,
    newMidEnd,
    delta: newMidLen - oldMidLen,
  };
}

function overlapsRange(d: SpellDiagPlain, rangeFrom: number, rangeTo: number): boolean {
  return d.from < rangeTo && rangeFrom < d.to;
}

/** Map diagnostic indices from `before` to `after` for a single edit. */
export function mapSpellDiagThroughEdit(
  d: SpellDiagPlain,
  e: SingleEditInfo,
): SpellDiagPlain | null {
  const { from: f, to: t } = d;
  const p = e.prefixLen;
  const oldMidEnd = e.oldMidEnd;
  const delta = e.delta;
  if (t <= p) return d;
  if (f >= oldMidEnd) return { ...d, from: f + delta, to: t + delta };
  if (f >= e.oldMidStart && t <= oldMidEnd) return null;
  return null;
}

const LETTER = /[\p{L}\p{M}]/u;

/** Grow [from,to) so spell tokens touching the edit are fully inside the slice. */
export function expandToWordContext(s: string, from: number, to: number): { from: number; to: number } {
  let a = Math.max(0, Math.min(from, s.length));
  let b = Math.max(a, Math.min(to, s.length));
  while (a > 0 && LETTER.test(s[a - 1])) a--;
  while (b < s.length && LETTER.test(s[b])) b++;
  return { from: a, to: b };
}

/** Drop old diags overlapping the rescanned span; append fresh results; sort by position. */
export function mergeSpellAfterRescan(
  mappedPrevious: SpellDiagPlain[],
  freshInRegion: SpellDiagPlain[],
  rescanFrom: number,
  rescanTo: number,
): SpellDiagPlain[] {
  const kept = mappedPrevious.filter((d) => !overlapsRange(d, rescanFrom, rescanTo));
  const merged = [...kept, ...freshInRegion];
  merged.sort((x, y) => x.from - y.from || x.to - y.to);
  return merged;
}
