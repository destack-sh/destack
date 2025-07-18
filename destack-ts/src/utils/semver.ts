/** Convert a semantic version string to a number (4 digits per segment). */
export function semverToInt(v: string): number {
  const base = 10_000;
  const parts = v.split(".").map((p) => Number(p || "0"));
  while (parts.length < 4) parts.push(0);
  return parts.reduce((acc, seg) => {
    if (seg >= base) throw new RangeError(`segment ${seg} ≥ ${base}`);
    return acc * base + seg;
  }, 0);
}

/** Convert a number to a semantic version string. */
export function intToSemver(v: number): string {
  const parts = [];
  for (let i = 0; i < 4; i++) {
    parts.push(v % 10_000);
    v /= 10_000;
  }
  return parts.reverse().join(".");
}
