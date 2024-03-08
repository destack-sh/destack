export function reverseRecord<T extends PropertyKey, U extends PropertyKey>(input: Partial<Record<T, U>>) {
  return Object.fromEntries(Object.entries(input).map(([key, value]) => [value, key])) as Record<U, T>;
}

export function cyrb53a(str: string, seed = 0): number {
  // 53-bit cyrb53a hash
  // see https://github.com/bryc/code/blob/master/jshash/experimental/cyrb53.js
  let h1 = 0xdeadbeef ^ seed,
    h2 = 0x41c6ce57 ^ seed;
  for (let i = 0, ch; i < str.length; i++) {
    ch = str.charCodeAt(i);
    h1 = Math.imul(h1 ^ ch, 0x85ebca77);
    h2 = Math.imul(h2 ^ ch, 0xc2b2ae3d);
  }
  h1 ^= Math.imul(h1 ^ (h2 >>> 15), 0x735a2d97);
  h2 ^= Math.imul(h2 ^ (h1 >>> 15), 0xcaf649a9);
  h1 ^= h2 >>> 16;
  h2 ^= h1 >>> 16;
  return 2097152 * (h2 >>> 0) + (h1 >>> 11);
}

export function roundToDigits(value: number, digits: number): number {
  const factor = 10 ** digits;
  return Math.round(value * factor) / factor;
}
