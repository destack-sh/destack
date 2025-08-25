// NOTE @Robustness: simple deterministic PRNG (Mulberry32)

export function rngSeedFromTime(): number {
  const n = Date.now() ^ Math.floor(Math.random() * 0xffffffff);
  return (n >>> 0) || 0x9e3779b9;
}

export function rngNext(state: number): [number, number] {
  // mulberry32
  let t = (state + 0x6d2b79f5) >>> 0;
  t = Math.imul(t ^ (t >>> 15), t | 1);
  t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
  const next = (t ^ (t >>> 14)) >>> 0;
  return [next, (next >>> 0) / 4294967296];
}


