import type { ObjectStore } from "@destack/object/server";
import type { Controller } from "@destack/db";
import type { Forge } from "./forge.ts";

/** How often the forge sweeps its store: hourly, under a second for 10k builds. */
export const SWEEP_INTERVAL_MILLISECONDS = 60 * 60 * 1000;

/** Sweeps the forge's store down to the builds and archives of its releases. */
export class SweepController implements Controller {
    /** The controller's name in reports. */
    readonly name = "forge-sweep";
    /** The forge whose store it sweeps. */
    readonly #forge: Forge;
    /** The object store with the releases the sweep spares. */
    readonly #objects: ObjectStore;

    /** Sweep a forge's store down to the releases of an object store. */
    constructor(forge: Forge, objects: ObjectStore) {
        this.#forge = forge;
        this.#objects = objects;
    }

    /** List the one sweep of the store. */
    async list(): Promise<readonly string[]> {
        return ["store"];
    }

    /** Sweep the store, and again an interval later. */
    async reconcile(): Promise<number> {
        // drop the builds and archives of no release
        await this.#forge.sweep(this.#objects.clock.now(), this.#objects.database);

        return SWEEP_INTERVAL_MILLISECONDS;
    }
}
