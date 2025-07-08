export function dedent(input: string, levels: number): string {
  const spacesToRemove = levels * 2;
  const inputLines = input.split("\n");
  const dedentedLines: string[] = [];

  for (const line of inputLines) {
    if (line.startsWith(" ".repeat(spacesToRemove))) {
      dedentedLines.push(line.slice(spacesToRemove));
    } else if (line.startsWith("\t".repeat(levels))) {
      dedentedLines.push(line.slice(levels));
    } else {
      dedentedLines.push(line);
    }
  }

  return dedentedLines.join("\n");
}

export enum Casing {
  SNAKE = 1,
  CAMEL = 2,
  LOWER_CAMEL = 3,
  ALL_CAPS = 4,
}

function stripAlphaNum(name: string): string {
  // remove leading underscores
  name = name.replace(/^_+/, "");
  // remove trailing underscores
  name = name.replace(/_+$/, "");
  // remove double underscores
  name = name.replace(/__+/, "_");
  // remove leading digits
  name = name.replace(/^[0-9]+/, "");
  return name;
}

const _CASING_CACHE: Record<string, string> = {};

export function toCasing(name: string, casing: Casing, allowWhitespace: boolean = false): string {
  const cacheKey = `${name}-${casing}-${allowWhitespace}`;
  const cached = _CASING_CACHE[cacheKey];
  if (cached) {
    return cached;
  }

  if (casing === Casing.SNAKE) {
    // first transform lowerUpper transitions into lower_upper
    name = name.replace(/(?<=[a-z])(?=[A-Z])/g, "_");
    // turn non-alphanumeric characters into underscores
    name = name.replace(/[^a-zA-Z0-9_]/g, "_");
    name = stripAlphaNum(name);
    name = name.toLowerCase();
    if (allowWhitespace) {
      name = name.replace(/_/g, " ").trim();
    }
  } else if (casing === Casing.CAMEL || casing === Casing.LOWER_CAMEL) {
    // if it's already a mix of uppercase and lowercase starting with uppercase, leave it alone
    if (/^[A-Z][a-z0-9]+([A-Z]+[a-z0-9]+)+/.test(name)) {
      if (casing === Casing.LOWER_CAMEL) {
        name = name[0].toLowerCase() + name.slice(1);
      }
      _CASING_CACHE[cacheKey] = name;
      return name;
    }
    // ignore non-alphanumeric characters and capitalize the next character
    name = name.replace(/[^a-zA-Z0-9]/g, " ");
    // split on existing uppercase characters and spaces
    name = name.split(/(?<=[a-z])(?=[A-Z0-9])/g).join(" ");
    name = stripAlphaNum(name);
    // title case each word
    name = name.replace(/\b\w/g, (char) => char.toUpperCase());
    if (allowWhitespace) {
      name = name.replace(/_/g, " ").trim();
    } else {
      name = name.replace(/ /g, "");
    }
    if (casing === Casing.LOWER_CAMEL) {
      name = name[0].toLowerCase() + name.slice(1);
    }
  } else if (casing === Casing.ALL_CAPS) {
    // ignore non-alphanumeric characters and capitalize the next character
    name = name.replace(/[^a-zA-Z0-9]/g, " ");
    // split on existing uppercase characters and spaces
    name = name.split(/(?<=[a-z])(?=[A-Z0-9])/g).join(" ");
    name = stripAlphaNum(name).toUpperCase().replace(/ /g, "_");
    if (allowWhitespace) {
      name = name.replace(/_/g, " ").trim();
    }
  } else {
    throw new Error(`unexpected casing for ${name}: ${casing}`);
  }

  _CASING_CACHE[cacheKey] = name;
  return name;
}

export function levenshteinDistance(a: string, b: string): number {
  // https://stackoverflow.com/a/11958496/125433
  if (a.length == 0) return b.length;
  if (b.length == 0) return a.length;

  const matrix = [];

  // increment along the first column of each row
  for (let i = 0; i <= b.length; i++) {
    matrix[i] = [i];
  }

  // increment each column in the first row
  for (let j = 0; j <= a.length; j++) {
    matrix[0][j] = j;
  }

  // Fill in the rest of the matrix
  for (let i = 1; i <= b.length; i++) {
    for (let j = 1; j <= a.length; j++) {
      if (b.charAt(i - 1) == a.charAt(j - 1)) {
        matrix[i][j] = matrix[i - 1][j - 1];
      } else {
        matrix[i][j] = Math.min(
          matrix[i - 1][j - 1] + 1, // substitution
          Math.min(
            matrix[i][j - 1] + 1, // insertion
            matrix[i - 1][j] + 1,
          ),
        ); // deletion
      }
    }
  }

  return matrix[b.length][a.length];
}

/**
 * Formats numbers into their highest 3-exponent of 10 (k, m, b)
 *  (like 57 -> 57, 7207 -> 7.2k, 2000000 -> 2m)
 */
export function humanizeNumber(num: number): string {
  if (num < 1000) {
    return num.toString();
  } else if (num < 1000000) {
    return `${Math.round(num / 100) / 10}k`;
  } else if (num < 1000000000) {
    return `${Math.round(num / 100000) / 10}m`;
  } else {
    return `${Math.round(num / 100000000) / 10}b`;
  }
}

/** Shorten bytes into nearest (KB, MB, GB, etc.), keep up to 3 significant digits */
export function humanizeBytes(bytes: number, options?: { cutoff?: number; round?: boolean }) {
  const cutoff = options?.cutoff ?? 100;
  const units = ["B", "KB", "MB", "GB", "TB", "PB"];
  let unit = 0;
  while (bytes >= cutoff && unit < units.length - 1) {
    bytes /= 1024;
    unit++;
  }
  if (options?.round) {
    return `${Math.round(bytes)}${units[unit]}`;
  }
  if (unit == 0) {
    return `${bytes.toFixed(0)}${units[unit]}`;
  } else if (bytes < 10) {
    return `${bytes.toFixed(1)}${units[unit]}`;
  } else {
    return `${bytes.toFixed(0)}${units[unit]}`;
  }
}

/** XOR encode a 'data' string with a key. */
export function xorString(data: string, key: number): string {
  // to bytes
  const keyBytes: number[] = [];
  while (key > 0) {
    keyBytes.push(key & 0xff);
    key = key >> 8;
  }
  keyBytes.reverse();

  const result: number[] = [];
  for (let i = 0; i < data.length; i++) {
    // convert character to ASCII code, perform XOR with key, and convert back to character
    const charCode = data.charCodeAt(i) ^ keyBytes[i % keyBytes.length];
    result.push(charCode);
  }

  return String.fromCharCode(...result);
}

/** LCG number generator. Very not secure but fast. */
export function pseudoRandomNumber(seed: number): number {
  const a = 1664525;
  const c = 1013904223;
  const m = 2 ** 32;
  return (a * seed + c) % m;
}
