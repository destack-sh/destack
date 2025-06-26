// lookup table from base64 character to byte
const encTable = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/".split("");

// lookup table from base64 character *code* to byte because lookup by number is fast
const decTable: number[] = [];
for (let i = 0; i < encTable.length; i++) {
  decTable[encTable[i].charCodeAt(0)] = i;
}

// support base64url variants
decTable["-".charCodeAt(0)] = encTable.indexOf("+");
decTable["_".charCodeAt(0)] = encTable.indexOf("/");

/**
 * Decode a base64 string to a byte array.
 *
 * - ignores white-space, including line breaks and tabs
 * - allows inner padding (can decode concatenated base64 strings)
 * - does not require padding
 * - understands base64url encoding:
 *   "-" instead of "+",
 *   "_" instead of "/",
 *   no padding
 */
export function base64Decode(base64Str: string): Uint8Array {
  // estimate byte size, not accounting for inner padding and whitespace
  let estimatedSize = (base64Str.length * 3) / 4;
  if (base64Str[base64Str.length - 2] === "=") {
    estimatedSize -= 2;
  } else if (base64Str[base64Str.length - 1] === "=") {
    estimatedSize -= 1;
  }

  const bytes = new Uint8Array(estimatedSize);
  let bytePos = 0;
  let groupPos = 0;
  let currentByte: number;
  let previousByte = 0;

  for (let i = 0; i < base64Str.length; i++) {
    currentByte = decTable[base64Str.charCodeAt(i)];

    if (currentByte === undefined) {
      const char = base64Str[i];
      // skip whitespace and padding
      if (char === "=" || char === "\n" || char === "\r" || char === "\t" || char === " ") {
        if (char === "=") {
          groupPos = 0; // reset state when padding found
        }
        continue;
      }
      throw new Error(`invalid base64 string.`);
    }

    if (groupPos === 0) {
      previousByte = currentByte;
      groupPos = 1;
    } else if (groupPos === 1) {
      bytes[bytePos++] = (previousByte << 2) | ((currentByte & 48) >> 4);
      previousByte = currentByte;
      groupPos = 2;
    } else if (groupPos === 2) {
      bytes[bytePos++] = ((previousByte & 15) << 4) | ((currentByte & 60) >> 2);
      previousByte = currentByte;
      groupPos = 3;
    } else if (groupPos === 3) {
      bytes[bytePos++] = ((previousByte & 3) << 6) | currentByte;
      groupPos = 0;
    }
  }

  if (groupPos === 1) {
    throw new Error(`invalid base64 string.`);
  }

  return bytes.subarray(0, bytePos);
}

/**
 * Encode a byte array to a base64 string.
 * Adds padding at the end.
 * Does not insert newlines.
 */
export function base64Encode(bytes: Uint8Array): string {
  let base64 = "";
  let groupPos = 0;
  let currentByte: number;
  let carryOver = 0;

  for (let i = 0; i < bytes.length; i++) {
    currentByte = bytes[i];

    if (groupPos === 0) {
      base64 += encTable[currentByte >> 2];
      carryOver = (currentByte & 3) << 4;
      groupPos = 1;
    } else if (groupPos === 1) {
      base64 += encTable[carryOver | (currentByte >> 4)];
      carryOver = (currentByte & 15) << 2;
      groupPos = 2;
    } else if (groupPos === 2) {
      base64 += encTable[carryOver | (currentByte >> 6)];
      base64 += encTable[currentByte & 63];
      groupPos = 0;
    }
  }

  // add padding if needed
  if (groupPos > 0) {
    base64 += encTable[carryOver];
    base64 += "=";
    if (groupPos === 1) {
      base64 += "=";
    }
  }

  return base64;
}
