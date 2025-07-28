import { type Signal, useComputed } from "@preact/signals";
import type { SignalOptions } from "@preact/signals-core";
import { effect } from "@preact/signals-core";
import { useEffect, useRef, useState } from "preact/hooks";

/** Reactively read the value of a signal. */
export function useSignalValue<T>(s: Signal<T>): T {
  const [value, setValue] = useState(s.value);

  useEffect(() => {
    return s.subscribe((val) => setValue(val));
  }, [s]);

  return value;
}

/** Reactively read the computed value of a signal. */
export function useComputedValue<T>(compute: () => T, options?: SignalOptions<T>): T {
  const sig = useComputed(compute, options);
  const [value, setValue] = useState(sig.value);

  useEffect(() => {
    return sig.subscribe((val) => setValue(val));
  }, [sig]);

  return value;
}

/** Run fn whenever any signal it reads changes (like useEffect for signals). */
export function useSignalEffect(fn: () => void | (() => void)) {
  const cleanupRef = useRef<ReturnType<typeof fn>>(undefined);

  useEffect(() => {
    const eff = effect(() => {
      // previous cleanup
      cleanupRef.current?.();
      cleanupRef.current = fn();
    });

    return () => {
      eff();
      cleanupRef.current?.();
    };
  }, []);
}
