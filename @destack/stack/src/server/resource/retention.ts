import type { ExternalStorage, ObjectStore } from "@destack/object/server";
import { Snapshot } from "@destack/db";
import type { ContentStore } from "@destack/resource";
import { sweepSnapshots } from "./snapshot.ts";
import { SettingValue } from "@destack/setting/object";
import type { ReconcileController } from "@destack/db";
import { buildRetention } from "../../setting/index.ts";
import { BuildStore } from "../installation/build.ts";
import { LEASE_LIFETIME_SECONDS } from "@destack/bucket/s3";

/** How often a machine sweeps its stores: once per write lease, which bounds a lost upload. */
const INTERVAL_MILLISECONDS = LEASE_LIFETIME_SECONDS * 1000;

/** How long stored files stay safe from a sweep: two write leases, to upload and to finish. */
const SWEEP_GRACE_MILLISECONDS = 2 * LEASE_LIFETIME_SECONDS * 1000;

/** The external storage of graph indexes, one per manifest in the build store. */
export interface ExternalGraphStorage extends ExternalStorage {
    /** Delete the indexes of the manifests the build store no longer has. */
    readonly sweep: () => Promise<void>;
}

/** Sweep a machine's stores down to what retention retains. */
export class RetentionController implements ReconcileController {
    /** The controller's name in reports. */
    readonly name = "retention";
    /** The object store. */
    readonly #objects: Pick<
        ObjectStore,
        "database" | "principal" | "as" | "mutate" | "clock" | "environment"
    >;
    /** The machine's store of every build. */
    readonly #builds: BuildStore;
    /** The graph indexes the sweep deletes once their builds go. */
    readonly #graphs: Pick<ExternalGraphStorage, "sweep">;
    /** The space's content store of its snapshots. */
    readonly #store: ContentStore;

    /** Sweep a machine's stores by the retention of the spaces in its object store's database. */
    constructor(
        objects: Pick<
            ObjectStore,
            "database" | "principal" | "as" | "mutate" | "clock" | "environment"
        >,
        builds: BuildStore,
        graphs: Pick<ExternalGraphStorage, "sweep">,
        store: ContentStore,
    ) {
        // store the objects, the build store, the graph sweep and the content store
        this.#objects = objects;
        this.#builds = builds;
        this.#graphs = graphs;
        this.#store = store;
    }

    /** List the one sweep of the machine's stores. */
    async list(): Promise<readonly string[]> {
        return ["stores"];
    }

    /** Retain the space's builds and snapshots each interval, and sweep the stores once due. */
    async reconcile(): Promise<number> {
        // retire the space's snapshots and root its builds an interval after it last rooted them
        const now = this.#objects.clock.now();
        const { database, principal } = this.#objects;
        const rooted = await this.#builds.packages.rootedAt(principal.id);
        const isDue = rooted === undefined || now - rooted >= INTERVAL_MILLISECONDS;
        if (isDue) {
            await sweepSnapshots(this.#objects, this.#store);
            await this.#builds.retain(
                this.#objects,
                (space) =>
                    SettingValue.resolve(Snapshot.live(database), buildRetention, { scope: space }),
                now,
            );
        }

        // sweep the builds no root retains once due beyond the files' grace, then their indexes
        const swept = await this.#builds.packages.sweepIfDue(
            new Date(now - SWEEP_GRACE_MILLISECONDS),
            new Date(now),
            INTERVAL_MILLISECONDS,
        );
        if (swept !== undefined) {
            await this.#graphs.sweep();
        }

        return isDue ? INTERVAL_MILLISECONDS : INTERVAL_MILLISECONDS - (now - rooted);
    }
}
