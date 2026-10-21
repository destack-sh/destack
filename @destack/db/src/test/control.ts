import type { DatabaseConnection } from "../database/index.ts";
import type { Controller, Reconciliation } from "../control/index.ts";
import { Snapshot } from "../log/index.ts";

/** Build a reconciliation outside a control loop: stopped by a signal, its key never changing. */
export function reconciliation(signal: AbortSignal = new AbortController().signal): Reconciliation {
    return { signal, changed: () => new Promise<void>(() => {}) };
}

/** Reconcile each key a controller lists once, returning the delays. */
export async function reconcileOnce(
    controller: Controller,
    database: DatabaseConnection,
): Promise<(number | undefined)[]> {
    // list through a live view, then reconcile each key in turn
    const delays: (number | undefined)[] = [];
    for (const key of await controller.list(Snapshot.live(database))) {
        delays.push(await controller.reconcile(key, reconciliation()));
    }

    return delays;
}
