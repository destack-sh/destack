import { customRef, watch, type Ref, isRef } from "vue";

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
        // TODO @Performance: find better ways to implement value ref semantics
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

export function randomHexString(length = 6): string {
  return Math.random()
    .toString(16)
    .substring(2, length + 2);
}

export function getUUIDFromGlobalID(globalId: string): string {
  return atob(globalId).split(":")[1];
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
