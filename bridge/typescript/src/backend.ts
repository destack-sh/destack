/** Supported language backend kinds. */
export type LanguageBackend = "napi" | "wasm";

/** Detect the default backend for the current runtime. */
export function detectBackend(): LanguageBackend {
    const processValue = (globalThis as { process?: { versions?: { node?: string } } }).process;

    return processValue?.versions?.node == null ? "wasm" : "napi";
}
