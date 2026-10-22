import type { ExternalStorage, ObjectStore } from "@destack/object/server";
import { Snapshot } from "@destack/db";
import type { ContentStore } from "@destack/resource";
import { sweepSnapshots } from "./snapshot.ts";
import { SettingValue } from "@destack/setting/object";
import type { Controller } from "@destack/db";
import { buildRetention } from "../../setting/index.ts";
import { BuildStore } from "../installation/build.ts";
import { LEASE_LIFETIME_SECONDS } from "@destack/bucket/s3";

/** How often a machine sweeps its stores: once per write lease, which bounds a lost upload. */
const INTERVAL_MILLISECONDS = LEASE_LIFETIME_SECONDS * 1000;

/** How long stored files stay safe from a sweep: two write leases, to upload and to finish. */
const SWEEP_GRACE_MILLISECONDS = 2 * LEASE_LIFETIME_SECONDS * 1000;

/** The external storage of graph indexes, one per manifest the build store keeps. */
export interface ExternalGraphStorage extends ExternalStorage {
    /** Delete the indexes of the manifests the build store no longer keeps. */
    readonly sweep: () => Promise<void>;
}

/** Sweep a machine's stores down to what retention keeps. */
export class RetentionController implements Controller {
    /** The controller's name in reports. */
    readonly name = "retention";
    /** The object store. */
    readonly #objects: Pick<ObjectStore, "database" | "principal" | "serving" | "mutate" | "clock">;
    /** The machine's store of every build. */
    readonly #builds: BuildStore;
    /** The graph indexes the sweep deletes once their builds go. */
    readonly #graphs: Pick<ExternalGraphStorage, "sweep">;
    /** The space's content store of its snapshots. */
    readonly #store: ContentStore;

    /** Sweep a machine's stores by the retention of the spaces in its object store's database. */
    constructor(
        objects: Pick<ObjectStore, "database" | "principal" | "serving" | "mutate" | "clock">,
        builds: BuildStore,
        graphs: Pick<ExternalGraphStorage, "sweep">,
        store: ContentStore,
    ) {
        // keep the objects, the build store, the graph sweep and the content store
        this.#objects = objects;
        this.#builds = builds;
        this.#graphs = graphs;
        this.#store = store;
    }

    /** List the one sweep of the machine's stores. */
    async list(): Promise<readonly string[]> {
        return ["stores"];
    }

    /** Sweep the stores, and again once an interval passed. */
    async reconcile(): Promise<number> {
        // expire the builds retention drops, sweep their files once due beyond the grace, then their indexes
        const now = this.#objects.clock.now();
        const { database } = this.#objects;
        const retained = await this.#builds.sweepIfDue(
            this.#objects,
            (space) =>
                SettingValue.resolve(Snapshot.live(database), buildRetention, { scope: space }),
            new Date(now - SWEEP_GRACE_MILLISECONDS),
            now,
            INTERVAL_MILLISECONDS,
        );
        if (retained !== undefined) {
            await this.#graphs.sweep();
        }

        // retire the snapshots past their retention and sweep the blobs none reads
        await sweepSnapshots(this.#objects, this.#store);

        return INTERVAL_MILLISECONDS;
    }
}
