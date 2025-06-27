import { xxHash32 } from "js-xxhash";

const _SEED = 0;

/** Little-endian 64-bit integer bytes. */
function leInt64Bytes(value: bigint): Uint8Array {
  const buf = new ArrayBuffer(8);
  const view = new DataView(buf);
  view.setBigInt64(0, value, true); // little‑endian
  return new Uint8Array(buf);
}

/** Big-endian 64-bit float bytes. */
function beFloat64Bytes(value: number): Uint8Array {
  const buf = new ArrayBuffer(8);
  const view = new DataView(buf);

  // canonicalise NaN so Python ⇆ TS agree on the hash
  if (Number.isNaN(value)) {
    view.setUint32(0, 0x7ff80000, false); // quiet NaN
    view.setUint32(4, 0x00000000, false);
  } else {
    view.setFloat64(0, value, false); // big‑endian
  }
  return new Uint8Array(buf);
}

/**
 * Hash a unicode string (decomposed/composed forms are collapsed, UTF-8 bytes).
 */
export function hashString(value: string): number {
  return xxHash32(value.normalize("NFC"), _SEED);
}

/**
 * Hash a byte sequence.
 */
export function hashBytes(value: Uint8Array): number {
  return xxHash32(value, _SEED);
}

/**
 * Hash a 64-bit integer.
 */
export function hashInt(value: number | bigint): number {
  return xxHash32(leInt64Bytes(BigInt(value)), _SEED);
}

/**
 * Hash a 64-bit float.
 */
export function hashFloat(value: number): number {
  return xxHash32(beFloat64Bytes(value), _SEED);
}

/**
 * Hash a boolean.
 */
export function hashBool(value: boolean): number {
  return xxHash32(Uint8Array.of(value ? 1 : 0), _SEED);
}
