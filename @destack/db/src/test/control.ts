import type { Following } from "../control/index.ts";

/** Build a reconciliation outside a control loop: stopped by a signal, its key never changing. */
export function reconciliation(signal: AbortSignal = new AbortController().signal): Following {
    return { signal, changed: () => new Promise<void>(() => {}), synced: () => {} };
}
