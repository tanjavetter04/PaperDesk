/** Map N Unicode code points from the start of `lineText` to a UTF-16 offset (CodeMirror). */
export function utf16OffsetByCodePointSteps(
  lineText: string,
  codePoints: number,
): number {
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
