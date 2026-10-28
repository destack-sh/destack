import type { ReconcileController } from "../control/control.ts";
import { blobGarbage, blobLease } from "./table.ts";
import type { BlobStore } from "./store.ts";

/** The key of the one collection the controller reconciles: the partition's blobs. */
const BLOBS = "blobs";

/** Collect a partition's blobs as rows let them go and as leases end. */
export class BlobController implements ReconcileController {
    /** The controller's name in reports. */
    readonly name = "blobs";
    /** The tables whose writes retire blobs or lease them. */
    readonly watches = [blobGarbage, blobLease];
    /** The partition's blob store. */
    readonly #blobs: BlobStore;

    /** Collect a partition's blobs. */
    constructor(blobs: BlobStore) {
        this.#blobs = blobs;
    }

    /** List the one collection. */
    async list(): Promise<readonly string[]> {
        return [BLOBS];
    }

    /** Collect again on each retirement or lease. */
    keys(): readonly string[] {
        return [BLOBS];
    }

    /** Collect, looking again once the next lease ends, else on the next retirement. */
    async reconcile(): Promise<number | undefined> {
        const now = this.#blobs.environment.clock.now();
        const next = await this.#blobs.collect();

        return next === undefined ? undefined : Math.max(0, next - now);
    }
}
