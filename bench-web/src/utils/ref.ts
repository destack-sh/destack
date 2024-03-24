import type { Fn } from "@vueuse/core";
import {
  computed,
  customRef,
  getCurrentInstance,
  isRef,
  onUnmounted,
  ref,
  watch,
  type ComputedGetter,
  type Ref,
  type WatchOptions
} from "vue";

/** A ref that pretends to be read-only but really isn't */
export function pretendReadonly<T extends object>(value: T): Readonly<T> {
  return value as Readonly<T>;
}

/** Get/set a property of an object behind a Ref */
export function pickRef<T, K extends keyof T>(obj: Ref<T | null |undefined>, key: K, valueFallback: T[K], objFallback: T | {} = {}): Ref<T[K]> {
  return computed({
    get: () => obj.value?.[key] ?? valueFallback,
    set: (value) => (obj.value = { ...obj.value ?? objFallback, [key]: value } as T),
  });
}

/**
 * Checks whether two arbitrary JavaScript values are deeply equal.
 * Traverses objects and arrays recursively.
 */
export function deepValueEquals(a: any, b: any): boolean {
  if (a === b) return true;
  if (a == null || b == null) return false;
  if (typeof a != "object" || typeof b != "object") return false;
  if (Array.isArray(a) != Array.isArray(b)) return false;
  if (Array.isArray(a)) {
    if (a.length != b.length) return false;
    for (let i = 0; i < a.length; i++) {
      if (!deepValueEquals(a[i], b[i])) return false;
    }
  } else {
    const aKeys = Object.keys(a);
    const bKeys = Object.keys(b);
    if (aKeys.length != bKeys.length) return false;
    for (const key of aKeys) {
      if (!deepValueEquals(a[key], b[key])) return false;
    }
  }
  return true;
}

/** A ref that only trigger if the value deeply changes */
export function valueRef<T>(value: T) {
  return customRef<T>((track, trigger) => {
    return {
      get() {
        track();
        return value;
      },
      set(newValue: T) {
        if (!deepValueEquals(value, newValue)) {
          value = newValue;
          trigger();
        }
      },
    };
  });
}

/** A ref that only triggers if the value deeply changes */
export function toValueRef<T>(value: Ref<T>, options?: WatchOptions) {
  const ref = valueRef(value.value);
  watch(
    value,
    (newValue) => {
      ref.value = newValue;
    },
    options,
  );
  return ref;
}

type RefsToValueRefs<T> = {
  [K in keyof T]: T[K] extends Ref<infer U> ? Ref<U> : T[K];
};

/** Wrap all refs in an object to value refs */
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

/** A computed value ref that only triggers when the value deeply changes */
export function valueComputed<T>(get: ComputedGetter<T>) {
  throw new Error("not implemented");
}

/** A computed ref with a manual trigger. */
export type ManualComputedRef<T> = Ref<T> & {
  trigger: () => void;
};

/** A computed ref that is only triggered manually. */
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
 * Run a callback when the component is unmounted, error if not in a component.
 */
export function onUnmountedStrict(callback: () => void) {
  if (!getCurrentInstance()) {
    throw new Error("onUnmountedStrict can only be used in a component");
  }
  onUnmounted(callback);
}

/** A read-only reference that can stop its reactivity subscription (permanently) */
export type SubRef<T> = Ref<T> & {
  /** Stops tracking */
  stop(): void;
};

/**
 * A manually triggered stoppable reference.
 * @param get - the computed getter, should update its own dependencies
 * @param stop - the function to stop tracking any dependencies
 * @returns the ref and a trigger to trigger its update (via Vue's reactivity system for batching)
 */
export function manualSubRef<T>(get: () => T, stop: () => void): { ref: SubRef<T>; trigger: () => void } {
  const manualRef = manualComputed(get);
  const ref = manualRef as unknown as SubRef<T>;
  ref.stop = stop;
  return { ref, trigger: manualRef.trigger };
}

/**
 * An automatically triggered stoppable reference.
 * @param get - a computed getter, should update its own dependencies
 * @param stop - the function to stop tracking any dependencies
 */
export function computedSubRef<T>(get: () => T, stop: () => void): SubRef<T> {
  const computedRef = computed(get);
  const ref = computedRef as unknown as SubRef<T>;
  ref.stop = stop;
  return ref;
}
