import type { ObjectStore } from "@destack/object/server";
import { type ReconcileController, Snapshot } from "@destack/db";
import { SettingValue } from "@destack/setting/object";
import { build, installationRevision, restore, snapshot } from "../../object/index.ts";
import { buildRetention } from "../../setting/index.ts";
import type { BuildStore } from "../installation/build.ts";
import { expireSnapshots } from "./snapshot.ts";

/** The key of the one retention the controller reconciles: the space's. */
const RETENTION = "retention";

/** Retires a space's expired snapshots and expires the builds its retention drops. */
export class RetentionController implements ReconcileController {
    /** The controller's name in reports. */
    readonly name = "retention";
    /** The tables whose changes move what the space retains. */
    readonly watches = [build.table, installationRevision.table, snapshot.table, restore.table];
    /** The object store. */
    readonly #objects: Pick<ObjectStore, "database" | "as" | "mutate" | "clock" | "environment">;
    /** The space's builds. */
    readonly #builds: BuildStore;

    /** Retain a space's snapshots and builds. */
    constructor(
        objects: Pick<ObjectStore, "database" | "as" | "mutate" | "clock" | "environment">,
        builds: BuildStore,
    ) {
        // store the objects and the build store
        this.#objects = objects;
        this.#builds = builds;
    }

    /** List the space's one retention. */
    async list(): Promise<readonly string[]> {
        return [RETENTION];
    }

    /** Reconcile the retention again on each change to what the space retains. */
    keys(): readonly string[] {
        return [RETENTION];
    }

    /** Retire expired snapshots and expire builds past retention, until the next expiry. */
    async reconcile(): Promise<number | undefined> {
        // retire the expired snapshots, then expire the space's builds past their retention
        const now = this.#objects.clock.now();
        const { database } = this.#objects;
        const expiry = await expireSnapshots(this.#objects);
        await this.#builds.retain(
            this.#objects,
            (space) =>
                SettingValue.resolve(Snapshot.live(database), buildRetention, { scope: space }),
            now,
        );

        // look again once the next snapshot expires, else on a change
        return expiry === undefined ? undefined : Math.max(0, expiry - now);
    }
}
