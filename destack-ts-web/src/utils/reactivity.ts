import type { SignalOptions } from "@preact/signals-core";
import { effect } from "@preact/signals-core";
import { Signal, useComputed } from "@preact/signals-react";
import { useEffect, useRef, useSyncExternalStore } from "react";

/** Reactively read the value of a signal. */
export function useSignalValue<T>(s: Signal<T>): T {
  return useSyncExternalStore(
    (cb) => s.subscribe(cb),
    () => s.value,
  );
}

/** Reactively read the computed value of a signal. */
export function useComputedValue<T>(compute: () => T, options?: SignalOptions<T>): T {
  const sig = useComputed(compute, options);
  return useSyncExternalStore(
    (cb) => sig.subscribe(cb),
    () => sig.value,
  );
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
