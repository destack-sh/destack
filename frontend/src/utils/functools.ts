import { customRef, watch, type Ref, isRef } from "vue";

export function reverseRecord<T extends PropertyKey, U extends PropertyKey>(input: Record<T, U>) {
  return Object.fromEntries(Object.entries(input).map(([key, value]) => [value, key])) as Record<U, T>;
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
        // there are definitely nicer ways to do this
        if (JSON.stringify(value) != JSON.stringify(newValue)) {
          value = newValue;
          trigger();
        }
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
