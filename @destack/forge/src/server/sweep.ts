import type { Controller } from "@destack/service/control";
import type { ForgeServer } from "./server.ts";

/** How often the forge sweeps its store: every hour, which leaves an unreleased build at most an hour past its window. */
export const SWEEP_INTERVAL_MILLISECONDS = 60 * 60 * 1000;

/** Sweeps the forge's store down to the builds and archives its releases name. */
export class SweepController implements Controller {
    /** The controller's name in reports. */
    readonly name = "forge-sweep";
    /** The forge whose store it sweeps. */
    readonly #forge: ForgeServer;
    /** The last sweep, in UTC epoch milliseconds. */
    #sweptAt = 0;

    /** Sweep a forge's store. */
    constructor(forge: ForgeServer) {
        this.#forge = forge;
    }

    /** List the one sweep of the store. */
    async list(): Promise<readonly string[]> {
        return ["store"];
    }

    /** Sweep at most once an interval. */
    async reconcile(): Promise<number> {
        // wait out the rest of the interval since the last sweep
        const now = Date.now();
        const since = now - this.#sweptAt;
        if (since < SWEEP_INTERVAL_MILLISECONDS) {
            return SWEEP_INTERVAL_MILLISECONDS - since;
        }
        this.#sweptAt = now;

        // drop the builds no release names
        await this.#forge.sweep(now);

        return SWEEP_INTERVAL_MILLISECONDS;
    }
}
