import type { Controller } from "@destack/service/control";
import { monitorSegment } from "../segment/table.ts";
import { Monitor } from "./monitor.ts";

/** How often each installation's segments are compacted, pruned and swept: every ten minutes. */
const INTERVAL_MILLISECONDS = 10 * 60 * 1000;

/** Compacts, prunes and sweeps the segments of each installation and host scope the catalog names. */
export class SegmentController implements Controller {
    /** The controller's name in reports. */
    readonly name = "segments";
    /** The catalog with commits that list the keys again. */
    readonly watches = [monitorSegment];
    /** The monitor keeping the segments. */
    readonly #monitor: Monitor;
    /** The last reconciliation of each key, in Unix milliseconds. */
    readonly #reconciledAt = new Map<string, number>();

    /** Keep a monitor's segments. */
    constructor(monitor: Monitor) {
        this.#monitor = monitor;
    }

    /** List each installation and scope host the catalog has segments of. */
    list(): Promise<readonly string[]> {
        return this.#monitor.emitters();
    }

    /** Compact, prune and sweep a key's segments at most once an interval. */
    async reconcile(key: string): Promise<number> {
        // wait out the rest of the interval since the last pass
        const now = Date.now();
        const since = now - (this.#reconciledAt.get(key) ?? 0);
        if (since < INTERVAL_MILLISECONDS) {
            return INTERVAL_MILLISECONDS - since;
        }
        this.#reconciledAt.set(key, now);

        // merge finished hours, drop expired segments, then delete unnamed files
        const { scope, installation } = Monitor.emitter(key);
        await this.#monitor.compact(scope, installation, now);
        await this.#monitor.prune(scope, installation, now);
        await this.#monitor.sweep(scope, installation, now);

        return INTERVAL_MILLISECONDS;
    }
}
