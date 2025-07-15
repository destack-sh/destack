const _crypto: Crypto = globalThis.crypto ?? require("crypto").webcrypto;
const _byteToHex: string[] = Array.from({ length: 256 }, (_, i) =>
  (i + 0x100).toString(16).slice(1),
);

/**
 * Generate a UUIDv4.
 */
export function uuid4(): string {
  return _crypto.randomUUID();
}

/**
 * Generate a UUIDv7.
 * See https://www.rfc-editor.org/rfc/rfc9562.html#name-uuid-version-7.
 */
export function uuid7(): string {
  // 48-bit ms timestamp
  const t = Date.now();
  const hi = (t / 0x1_0000_0000) | 0;
  const lo = t >>> 0;

  // grab 80 random bits from a CSPRNG (10 bytes)
  const rnd = new Uint8Array(10);
  _crypto.getRandomValues(rnd);

  const r1 = ((rnd[0] << 24) | (rnd[1] << 16) | (rnd[2] << 8) | rnd[3]) >>> 0; // 32 bits
  const r2 = ((rnd[4] << 24) | (rnd[5] << 16) | (rnd[6] << 8) | rnd[7]) >>> 0; // 32 bits
  const r3 = (rnd[8] << 8) | rnd[9]; // 16 bits

  const b = new Uint8Array(16);
  b[0] = hi >>> 8;
  b[1] = hi & 0xff;
  b[2] = lo >>> 24;
  b[3] = (lo >>> 16) & 0xff;
  b[4] = (lo >>> 8) & 0xff;
  b[5] = lo & 0xff;

  // version + rand_a
  b[6] = 0x70 | ((r1 >>> 28) & 0x0f);
  b[7] = (r1 >>> 20) & 0xff;

  // variant + rest of randomness
  b[8] = 0x80 | ((r1 >>> 12) & 0x3f);
  b[9] = (r1 >>> 4) & 0xff;
  b[10] = ((r1 & 0x0f) << 4) | (r2 >>> 28);
  b[11] = (r2 >>> 20) & 0xff;
  b[12] = (r2 >>> 12) & 0xff;
  b[13] = (r2 >>> 4) & 0xff;
  b[14] = ((r2 & 0x0f) << 4) | (r3 >>> 12);
  b[15] = r3 & 0xff;

  return (
    _byteToHex[b[0]] +
    _byteToHex[b[1]] +
    _byteToHex[b[2]] +
    _byteToHex[b[3]] +
    "-" +
    _byteToHex[b[4]] +
    _byteToHex[b[5]] +
    "-" +
    _byteToHex[b[6]] +
    _byteToHex[b[7]] +
    "-" +
    _byteToHex[b[8]] +
    _byteToHex[b[9]] +
    "-" +
    _byteToHex[b[10]] +
    _byteToHex[b[11]] +
    _byteToHex[b[12]] +
    _byteToHex[b[13]] +
    _byteToHex[b[14]] +
    _byteToHex[b[15]]
  );
}

export const NANO_ID_LENGTH = 5;
export const NANO_ID_ALPHABET = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/** Convert a UUID to a nano ID. */
export function toNanoId(uuid: string): string {
  let value = parseInt(uuid.slice(-4), 16);
  if (value === 0) {
    return NANO_ID_ALPHABET[0].repeat(NANO_ID_LENGTH);
  }
  const alphabetLen = NANO_ID_ALPHABET.length;
  const result = [];
  for (let i = 0; i < NANO_ID_LENGTH; i++) {
    result.push(NANO_ID_ALPHABET[value % alphabetLen]);
    value = Math.floor(value / alphabetLen);
  }
  return result.reverse().join("");
}
