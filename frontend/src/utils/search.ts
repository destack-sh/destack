// default sort from https://github.com/leeoniya/uFuzzy/blob/main/src/uFuzzy.js
// we need access to the inner sort so we can modulate the scores with context per item
export const ufSort = (info: uFuzzy.Info, haystack: string[], needle: string) => {
  const { idx, chars, terms, interLft2, interLft1, start, intraIns, interIns } = info;
  const cmp = new Intl.Collator("en").compare;

  return (ia: number, ib: number) =>
    // most contig chars matched
    chars[ib] - chars[ia] ||
    // least char intra-fuzz (most contiguous)
    intraIns[ia] - intraIns[ib] ||
    // most prefix bounds, boosted by full term matches
    terms[ib] + interLft2[ib] + 0.5 * interLft1[ib] - (terms[ia] + interLft2[ia] + 0.5 * interLft1[ia]) ||
    // highest density of match (least span)
    //	span[ia] - span[ib] ||
    // highest density of match (least term inter-fuzz)
    interIns[ia] - interIns[ib] ||
    // earliest start of match
    start[ia] - start[ib] ||
    // alphabetic
    cmp(haystack[idx[ia]], haystack[idx[ib]]);
};
