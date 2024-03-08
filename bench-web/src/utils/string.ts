export function toCamelCase(input: string): string {
  /* TEXT to Text */
  return input.charAt(0).toUpperCase() + input.slice(1).toLowerCase();
}

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

// :IdentifierStrings

export enum IdentifierType {
  FILE = 1,
  TYPE = 2,
  CONSTANT = 3,
  FUNCTION = 4,
  VARIABLE = 5,
  PROPERTY = 6,
}

export enum Casing {
  SNAKE = 1,
  CAMEL = 2,
  ALL_CAPS = 3,
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
  } else if (casing === Casing.SNAKE) {
    name = name.replace(/(?<=[a-z])(?=[A-Z])/g, "_");
    name = name.replace(/[^a-zA-Z0-9_]/g, "_");
    name = stripAlphaNum(name);
    name = name.toLowerCase();
    if (allowWhitespace) {
      name = name.replace(/_/g, " ").trim();
    }
  } else if (casing === Casing.CAMEL) {
    if (/^[A-Z][a-z0-9]+([A-Z]+[a-z0-9]+)+/.test(name)) {
      return name;
    }
    name = name.replace(/[^a-zA-Z0-9]/g, " ");
    name = name.split(/(?<=[a-z])(?=[A-Z0-9])/g).join(" ");
    name = stripAlphaNum(name).replace(/\b\w/g, (char) => char.toUpperCase());
    if (allowWhitespace) {
      name = name.replace(/_/g, " ").trim();
    } else {
      name = name.replace(/ /g, "");
    }
  } else if (casing === Casing.ALL_CAPS) {
    name = name.replace(/[^a-zA-Z0-9]/g, " ");
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
