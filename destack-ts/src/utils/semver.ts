/** Convert a semantic version string to a bigint (4 digits per segment). */
export function semverToInt(v: string): bigint {
  const base = 1000n;
  const parts = v.split(".").map((p) => BigInt(p || "0"));
  while (parts.length < 4) parts.push(0n);
  return parts.reduce((acc, seg) => {
    if (seg >= base) throw new RangeError(`segment ${seg} ≥ ${base}`);
    return acc * base + seg;
  }, 0n);
}

/** Convert a bigint to a semantic version string. */
export function intToSemver(v: bigint): string {
  const parts = [];
  for (let i = 0; i < 4; i++) {
    parts.push(v % 1000n);
    v /= 1000n;
  }
  return parts.reverse().join(".");
}
