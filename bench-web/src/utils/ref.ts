import { customRef, type Ref, watch, isRef, onUnmounted } from "vue";

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

export function onUnmountedIfComponent(callback: () => void) {
  try {
    onUnmounted(callback);
  } catch (e) {
    /* not a vue component */
  }
}
