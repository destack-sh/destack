import { earliest } from "@destack/access";
import { Duration, present } from "@destack/schema";
import {
    Change,
    and,
    type DatabaseConnection,
    isNotNull,
    isNull,
    lte,
    min,
    type Select,
    TABLE,
} from "@destack/db";
import type { ObjectOf, ObjectType } from "../object/object.ts";
import type { ReconcileController, Reconciliation, Table } from "@destack/db";
import type { ObjectStore } from "./store.ts";

import type { RecoverableDefinition } from "../trait/recoverable.ts";
import type { TraitTable } from "../object/table.ts";
import { RequestId } from "@destack/sync";
/** The component name the purge runs and records its calls under. */
const PURGE = "purge";

/** A recoverable object type with its recovery options. */
interface RecoverableObject {
    /** The object type, typed by its deletion columns. */
    readonly object: ObjectOf<{
        table: TraitTable<{ readonly recoverable: RecoverableDefinition }>;
    }>;
    /** Its recovery options. */
    readonly options: RecoverableDefinition;
}

/** Purge deleted objects as their recovery windows end, scheduling the next window's end. */
export class RecoverableController implements ReconcileController {
    /** The controller's name. */
    readonly name = PURGE;
    /** The tables of the recoverable objects. */
    readonly watches: readonly Table[];
    /** The store the purge runs through. */
    readonly #store: ObjectStore;
    /** The recoverable object types with their recovery options. */
    readonly #types: readonly RecoverableObject[];

    /** Watch the recoverable objects a store serves. */
    constructor(store: ObjectStore) {
        this.#store = store;
        this.#types = RecoverableController.#recoverables(store.objects);
        this.watches = this.#types.map(({ object }) => object.table);
    }

    /** Look again when a row enters the trash. */
    keys(change: Change): string[] {
        // select the trash once a row is deleted
        const deleted = Change.after(change)?.["deletedAt"] ?? null;

        return deleted !== null && Change.before(change)?.["deletedAt"] !== deleted
            ? ["trash"]
            : [];
    }

    /** List the one key the purge runs under. */
    async list(): Promise<string[]> {
        return ["trash"];
    }

    /** Purge what passed its window and schedule the earliest window end. */
    async reconcile(_key: string, _reconciliation: Reconciliation): Promise<number | undefined> {
        // purge and read each type's earliest pending deletion
        const now = this.#store.clock.now();
        await RecoverableController.purge(this.#store, now);
        const ends = await Promise.all(
            this.#types.map(({ object, options }) =>
                RecoverableController.#ending(this.#store.database, object, options),
            ),
        );
        const next = earliest(ends);

        return next === undefined ? undefined : Math.max(0, next - now);
    }

    /** Purge expired deleted objects as the code serving their type, returning the count. */
    static async purge(store: ObjectStore, now: number): Promise<number> {
        // purge each type in batches until one comes back short
        let purged = 0;
        for (const { object, options } of RecoverableController.#recoverables(store.objects)) {
            for (let batch = PURGE_ROWS; batch === PURGE_ROWS;) {
                // purge one batch
                const rows = await RecoverableController.#expired(
                    store.database,
                    object,
                    options,
                    now,
                );
                await store.mutate(
                    store.as(object, now),
                    {
                        id: RequestId.create(store.environment),
                        calls: rows.map((row) =>
                            object.call(
                                RecoverableController.#purgeMethod(object),
                                object.target(row),
                            ),
                        ),
                    },
                    { component: PURGE },
                );
                batch = rows.length;
                purged += batch;
            }
        }

        return purged;
    }

    /** Read a batch of a type's expired, unpurged deleted rows. */
    static async #expired(
        database: DatabaseConnection,
        object: ObjectOf<{ table: TraitTable<{ readonly recoverable: RecoverableDefinition }> }>,
        options: RecoverableDefinition,
        now: number,
    ): Promise<Select<TraitTable<{ readonly recoverable: RecoverableDefinition }>>[]> {
        // read expired rows, unpurged where the type retains purged records
        const table = object.table;
        const window = Duration.milliseconds(options.within);
        const unpurged =
            options.retain === "record" ? isNull(table[TABLE].column("purgedAt")) : undefined;

        return database
            .select()
            .from(table)
            .where(and(lte(table.deletedAt, now - window), unpurged))
            .limit(PURGE_ROWS);
    }

    /** Read when a type's earliest pending deletion leaves its window. */
    static async #ending(
        database: DatabaseConnection,
        object: ObjectOf<{ table: TraitTable<{ readonly recoverable: RecoverableDefinition }> }>,
        options: RecoverableDefinition,
    ): Promise<number | undefined> {
        // read the earliest unpurged deletion
        const table = object.table;
        const unpurged =
            options.retain === "record" ? isNull(table[TABLE].column("purgedAt")) : undefined;
        const [row] = await database
            .select({ requestedAt: min(table.deletedAt) })
            .from(table)
            .where(and(isNotNull(table.deletedAt), unpurged));
        if (row === undefined) {
            throw new TypeError(`reading the earliest deletion of ${object.name} returned no row`);
        }
        const requestedAt = row.requestedAt;

        return requestedAt === null
            ? undefined
            : requestedAt + Duration.milliseconds(options.within);
    }

    /** Select the recoverable object types among the served ones, with their recovery options. */
    static #recoverables(objects: readonly ObjectType[]): RecoverableObject[] {
        return objects.flatMap((object): RecoverableObject[] => {
            // take the types deleting to a trash with their recovery options
            const options = object.lifecycle.recoverable;
            if (!object.isTrashable() || options === undefined) {
                return [];
            }

            return [{ object, options }];
        });
    }

    /** Find the purge method a recoverable type takes. */
    static #purgeMethod(object: ObjectType): string {
        const [name] = present(
            Object.entries(object.methods).find(([, declared]) => declared.kind === "purge"),
            `the purge method of ${object.name}`,
        );

        return name;
    }
}

/** The most rows one purge removes per transaction: about a millisecond of deletes. */
const PURGE_ROWS = 100;
