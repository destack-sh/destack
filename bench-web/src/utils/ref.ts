import type { Fn } from "@vueuse/core";
import { customRef, type Ref, watch, isRef, onUnmounted, ref, type ComputedGetter, getCurrentInstance } from "vue";

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

export type ManualComputedRef<T> = Ref<T> & {
  trigger: () => void;
};

/**
 * A computed ref that is only triggered manually.
 */
export function manualComputed<T>(get: ComputedGetter<T>): ManualComputedRef<T> {
  let value: T = undefined!;
  let track: Fn;
  let trigger: Fn;
  const dirty = ref(true);

  const update = () => {
    dirty.value = true;
    trigger();
  };

  const result = customRef<T>((_track, _trigger) => {
    track = _track;
    trigger = _trigger;

    return {
      get() {
        if (dirty.value) {
          value = get();
          dirty.value = false;
        }
        track();
        return value;
      },
      set(v) {
        throw new Error("manualComputed is readonly");
      },
    };
  }) as ManualComputedRef<T>;

  if (Object.isExtensible(result)) result.trigger = update;

  return result;
}

/**
 * Run a callback when the component is unmounted (only if we're in a component).
 */
export function onUnmountedIfComponent(callback: () => void) {
  if (getCurrentInstance()) {
    onUnmounted(callback);
  }
}
