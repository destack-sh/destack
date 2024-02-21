import type { AnyFn } from "@vueuse/core";
import { customRef, watch, type Ref, isRef, ref, onMounted, EffectScope, effectScope } from "vue";

export function reverseRecord<T extends PropertyKey, U extends PropertyKey>(input: Partial<Record<T, U>>) {
  return Object.fromEntries(Object.entries(input).map(([key, value]) => [value, key])) as Record<U, T>;
}

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

export function valueRef<T>(value: T) {
  return customRef<T>((track, trigger) => {
    return {
      get() {
        track();
        return value;
      },
      set(newValue: T) {
        // TODO :Performance: find better ways to implement value ref semantics
        const newValueType = typeof newValue;
        const valueType = typeof value;
        if (newValueType == valueType) {
          if (
            newValueType == "number" &&
            (newValue == value || (isNaN(newValue as unknown as number) && isNaN(value as unknown as number)))
          ) {
            return;
          } else if ((newValueType == "string" || newValueType == "boolean") && newValue == value) {
            return;
          } else if (JSON.stringify(value) == JSON.stringify(newValue)) {
            return;
          }
        }
        value = newValue;
        trigger();
      },
    };
  });
}

export function toValueRef<T>(value: Ref<T>) {
  const ref = valueRef(value.value);
  watch(value, (newValue) => {
    ref.value = newValue;
  });
  return ref;
}

type RefsToValueRefs<T> = {
  [K in keyof T]: T[K] extends Ref<infer U> ? Ref<U> : T[K];
};

export function wrapValueRefs<T extends Record<string, any>>(obj?: T): RefsToValueRefs<T> {
  if (obj == null) {
    return {} as RefsToValueRefs<T>;
  }
  const result: Partial<RefsToValueRefs<T>> = {};
  for (const key in obj) {
    const k = key as Extract<keyof T, string>;
    if (isRef(obj[k])) {
      result[k] = toValueRef(obj[k]) as RefsToValueRefs<T>[typeof k];
    } else {
      result[k] = obj[k];
    }
  }
  return result as RefsToValueRefs<T>;
}

export function startStopIf(
  predicate: Ref<boolean>,
  start: () => void,
  stop: () => void,
  options?: { immediate?: boolean }
) {
  watch(
    predicate,
    (value, oldValue) => {
      if (value) {
        start();
      } else if (!oldValue) {
        stop();
      }
    },
    options
  );
}

export function useDelayed(delay: number): Ref<boolean> {
  const ret = ref(false);
  onMounted(() => {
    setTimeout(() => {
      ret.value = true;
    }, delay);
  });
  return ret;
}

export function randomHexString(length = 6): string {
  return Math.random()
    .toString(16)
    .substring(2, length + 2);
}

export function getUUIDFromGlobalID(globalId: string): string {
  return atob(globalId).split(":")[1];
}

export function toGlobalId(type: string, id: string): string {
  return btoa(`${type}:${id}`);
}

export function getFieldNameFromTypeName(name: string): string | undefined {
  // turn something like MyType123 into my type 123, ignorning non alphanum characters
  return name
    .replace(/[^a-zA-Z0-9]/g, " ")
    .replace(/([a-z])([A-Z])/g, "$1 $2")
    .toLowerCase();
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

// :IdentifierStrings

export enum IdentifierType {
  METHOD = "method",
  TYPE = "type",
  CONSTANT = "constant",
  PATH = "path",
  VARIABLE = "variable",
  FIELD = "field",
}

export function toPyIdentifier(name: string, type: IdentifierType): string {
  if ([IdentifierType.METHOD, IdentifierType.VARIABLE, IdentifierType.FIELD, IdentifierType.PATH].includes(type)) {
    name = name.replace(/[^a-zA-Z0-9_]/g, "_");
    return stripAlphaNum(name).toLowerCase();
  } else if (type === IdentifierType.TYPE) {
    if (/^[A-Z][a-z0-9]+([A-Z]+[a-z0-9]+)+/.test(name)) return name;
    name = name.replace(/[^a-zA-Z0-9]/g, " ").replace(/([a-z])([A-Z0-9])/g, "$1 $2");
    name = name
      .split(" ")
      .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
      .join("");
    return stripAlphaNum(name);
  } else if (type === IdentifierType.CONSTANT) {
    name = name.replace(/[^a-zA-Z0-9]/g, " ").replace(/([a-z])([A-Z0-9])/g, "$1 $2");
    return stripAlphaNum(name).replace(/\s+/g, "_").toUpperCase();
  } else {
    throw new Error(`unexpected identifier type: ${type}`);
  }
}

function toAllCaps(name: string): string {
  name = name.replace(/[^a-zA-Z0-9]/g, " ").replace(/([a-z])([A-Z0-9])/g, "$1 $2");
  return stripAlphaNum(name).replace(/\s+/g, " ").trim().replace(/\s/g, "_").toUpperCase();
}

// def _strip_alpha_num(name: str) -> str:
//     # remove leading underscores
//     name = re.sub(r"^_+", "", name)
//     # remove trailing underscores
//     name = re.sub(r"_+$", "", name)
//     # remove double underscores
//     name = re.sub(r"__+", "_", name)
//     # remove leading digits
//     name = re.sub(r"^[0-9]+", "", name)
//     return name

function stripAlphaNum(name: string): string {
  // remove leading underscores
  name = name.replace(/^_+/, "");
  // remove trailing underscores
  name = name.replace(/_+$/, "");
  // remove double underscores
  name = name.replace(/__+/g, "_");
  // remove leading digits
  name = name.replace(/^[0-9]+/, "");
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
            matrix[i - 1][j] + 1
          )
        ); // deletion
      }
    }
  }

  return matrix[b.length][a.length];
}
