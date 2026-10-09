import { earliest, Caller } from "@destack/access";
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
import type { Controller, Reconciliation } from "@destack/service/control";
import type { Table } from "@destack/db";
import type { ObjectStore } from "./store.ts";

import { recoverableType, type RecoverableDefinition } from "../trait/recoverable.ts";
import type { TraitTable } from "../object/table.ts";
import { RequestId } from "@destack/service/request";
import { Call } from "../method/call.ts";
/** The component name the purge runs and records its calls under. */
const PURGE = "purge";

/** Purge deleted objects as their recovery windows end, scheduling the next window's end. */
export class RecoverableController implements Controller {
    /** The controller's name. */
    readonly name = PURGE;
    /** The tables of the recoverable objects. */
    readonly watches: readonly Table[];
    /** The store the purge runs through. */
    readonly #store: Omit<ObjectStore, "router">;
    /** The recoverable object types with their recovery options. */
    readonly #types: ReturnType<typeof recoverables>;

    /** Watch the recoverable objects a store serves. */
    constructor(store: Omit<ObjectStore, "router">) {
        this.#store = store;
        this.#types = recoverables(store.objects);
        this.watches = this.#types.map(({ object }) => object.table);
    }

    /** Look again when a row enters the trash. */
    keys(change: Change): string[] {
        // select the trash once a row's deletion request appears
        const deleted = Change.after(change)?.["deletionRequestedAt"] ?? null;

        return deleted !== null && Change.before(change)?.["deletionRequestedAt"] !== deleted
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
        const now = this.#store.clock();
        await RecoverableController.purge(this.#store, now);
        const ends = await Promise.all(
            this.#types.map(({ object, options }) => ending(this.#store.database, object, options)),
        );
        const next = earliest(ends);

        return next === undefined ? undefined : Math.max(0, next - now);
    }

    /** Purge expired deleted objects as the store's principal, returning the count. */
    static async purge(store: Omit<ObjectStore, "router">, now: number): Promise<number> {
        // purge each type in batches until one comes back short
        let purged = 0;
        for (const { object, options } of recoverables(store.objects)) {
            for (let batch = PURGE_ROWS; batch === PURGE_ROWS;) {
                // purge one batch
                const rows = await expired(store.database, object, options, now);
                await store.mutate(
                    Caller.of(store.principal, now),
                    {
                        id: RequestId.create(now),
                        calls: rows.map((row) =>
                            Call.record(object, purgeMethod(object), object.target(row)),
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
}

/** The most rows one purge removes per transaction: about a millisecond of deletes. */
const PURGE_ROWS = 100;

/** Read a batch of a type's expired, unpurged deleted rows. */
async function expired(
    database: DatabaseConnection,
    object: ObjectOf<{ table: TraitTable<{ readonly recoverable: RecoverableDefinition }> }>,
    options: RecoverableDefinition,
    now: number,
): Promise<Select<TraitTable<{ readonly recoverable: RecoverableDefinition }>>[]> {
    // read expired rows, unpurged where the type keeps purged records
    const table = object.table;
    const window = Duration.milliseconds(options.within);
    const unpurged =
        options.keep === "record" ? isNull(table[TABLE].column("purgedAt")) : undefined;

    return database
        .select()
        .from(table)
        .where(and(lte(table.deletionRequestedAt, now - window), unpurged))
        .limit(PURGE_ROWS);
}

/** Read when a type's earliest pending deletion leaves its window. */
async function ending(
    database: DatabaseConnection,
    object: ObjectOf<{ table: TraitTable<{ readonly recoverable: RecoverableDefinition }> }>,
    options: RecoverableDefinition,
): Promise<number | undefined> {
    // read the earliest unpurged deletion request
    const table = object.table;
    const unpurged =
        options.keep === "record" ? isNull(table[TABLE].column("purgedAt")) : undefined;
    const [row] = await database
        .select({ requestedAt: min(table.deletionRequestedAt) })
        .from(table)
        .where(and(isNotNull(table.deletionRequestedAt), unpurged));
    if (row === undefined) {
        throw new TypeError(`the earliest deletion of ${object.name} read no row`);
    }
    const requestedAt = row.requestedAt;

    return requestedAt === null ? undefined : requestedAt + Duration.milliseconds(options.within);
}

/** Select the recoverable object types among the served ones, with their recovery options. */
function recoverables(objects: readonly ObjectType[]): {
    readonly object: ObjectOf<{
        table: TraitTable<{ readonly recoverable: RecoverableDefinition }>;
    }>;
    readonly options: RecoverableDefinition;
}[] {
    return objects.flatMap((object) => {
        const typed = recoverableType(object);

        return typed === undefined || object.lifecycle.recoverable === undefined
            ? []
            : [{ object: typed, options: object.lifecycle.recoverable }];
    });
}

/** Find the purge method a recoverable type takes. */
function purgeMethod(object: ObjectType): string {
    const [name] = present(
        Object.entries(object.methods).find(([, declared]) => declared.kind === "purge"),
        `the purge method of ${object.name}`,
    );

    return name;
}
