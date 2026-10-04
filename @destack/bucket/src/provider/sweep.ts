import type { Controller } from "@destack/service/control";
import type { CatalogueBucketHost } from "../catalogue/index.ts";

/** How often a host sweeps its open buckets: daily, since only interrupted writes leave blobs behind. */
const SWEEP_INTERVAL_MILLISECONDS = 24 * 60 * 60_000;

/** How soon a host sweeps again after writers kept a sweep from finishing: a minute, about one large upload. */
const BUSY_INTERVAL_MILLISECONDS = 60_000;

/** The one key the sweep controller reconciles: the host's open buckets. */
const BUCKETS = "buckets";

/** Sweeps a host's open buckets daily, deleting what interrupted writes left behind. */
export class SweepController implements Controller {
    /** The controller's name in reports. */
    readonly name = "bucket-sweep";
    /** The host whose buckets it sweeps. */
    readonly #host: CatalogueBucketHost;

    /** Sweep a host's buckets. */
    constructor(host: CatalogueBucketHost) {
        this.#host = host;
    }

    /** List the host's buckets, swept on start. */
    async list(): Promise<readonly string[]> {
        return [BUCKETS];
    }

    /** Sweep the open buckets, then look again after the interval, or sooner when writers kept a sweep from finishing. */
    async reconcile(): Promise<number> {
        return (await this.#host.sweep())
            ? SWEEP_INTERVAL_MILLISECONDS
            : BUSY_INTERVAL_MILLISECONDS;
    }
}
