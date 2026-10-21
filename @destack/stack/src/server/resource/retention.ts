import type { ObjectStore } from "@destack/object/server";
import { Snapshot } from "@destack/db";
import type { ContentStore } from "@destack/resource";
import { sweepSnapshots } from "./snapshot.ts";
import { SettingValue } from "@destack/setting/object";
import type { Controller } from "@destack/runtime/control";
import { buildRetention } from "../../setting/index.ts";
import { BuildStore } from "../installation/build.ts";

/** How often a machine sweeps its stores: every ten minutes. */
const INTERVAL_MILLISECONDS = 10 * 60 * 1000;

/** How long stored builds stay safe from a sweep after their last use: an hour. */
const SWEEP_GRACE_MILLISECONDS = 60 * 60 * 1000;

/** Delete what build retention drops from a machine's pull cache, sparing what was stored since. */
export type SweepCache = (retained: ReadonlySet<string>, before: Date) => Promise<void>;

/** Sweep a machine's stores down to what retention keeps. */
export class RetentionController implements Controller {
    /** The controller's name in reports. */
    readonly name = "retention";
    /** The object store. */
    readonly #objects: Pick<ObjectStore, "database" | "principal" | "serving" | "mutate" | "clock">;
    /** The machine's store of every build. */
    readonly #builds: BuildStore;
    /** Delete what retention drops from the machine's pull cache, null on a machine without one. */
    readonly #sweepCache: SweepCache | null;
    /** The space's content store of its snapshots. */
    readonly #store: ContentStore;

    /** Sweep a machine's stores by the retention of the spaces in its object store's database. */
    constructor(
        objects: Pick<ObjectStore, "database" | "principal" | "serving" | "mutate" | "clock">,
        builds: BuildStore,
        sweepCache: SweepCache | null,
        store: ContentStore,
    ) {
        // keep the objects, the build store, the cache sweep and the content store
        this.#objects = objects;
        this.#builds = builds;
        this.#sweepCache = sweepCache;
        this.#store = store;
    }

    /** List the one sweep of the machine's stores. */
    async list(): Promise<readonly string[]> {
        return ["stores"];
    }

    /** Sweep the stores, and again once an interval passed. */
    async reconcile(): Promise<number> {
        // expire the builds retention drops, and sweep their files once due beyond the grace
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
        if (this.#sweepCache !== null && retained !== undefined) {
            await this.#sweepCache(retained, new Date(now - SWEEP_GRACE_MILLISECONDS));
        }

        // retire the snapshots past their retention and sweep the blobs none reads
        await sweepSnapshots(this.#objects, this.#store);

        return INTERVAL_MILLISECONDS;
    }
}
