import {
    Change,
    Condition,
    type DatabaseConnection,
    type RowImage,
    type Row,
    Namespace,
    Predicate,
    Scalar,
    Snapshot,
    type Table,
} from "@destack/db";
import type { Controller, Reconciliation } from "@destack/service/control";
import { canonicalize, schema, type JsonValue } from "@destack/schema";
import type { ObjectType } from "./object.ts";
import type { ResultOf } from "../method/call.ts";
import type { MethodName } from "../method/procedure.ts";
import { type ObjectServer, SystemCall } from "../server/server.ts";

/** The fields of a pending object's key, as its key name keeps them. */
const KeyFields = schema.record(schema.string(), Scalar);

/** Read an object row's identifier, which every object table keeps. */
function identifierOf(row: Row): string {
    return schema.string().parse(row["id"]);
}

/** The fields grouping objects a controller reconciles together, in JSON form. */
export type KeyFields = Readonly<Record<string, JsonValue>>;

/** A controller declared on an object type: the objects with work waiting, and how to reconcile them. */
export interface ObjectController<Object extends ObjectType = ObjectType> {
    /** The objects with work waiting. */
    readonly pending: Condition;
    /** The fields grouping objects reconciled together, the identifier by default. */
    key?(row: RowImage<Object["table"]>): KeyFields;
    /** The tables whose changed rows select keys beside a changed pending object's own. */
    readonly watches?: readonly ObjectWatch[];
    /** Whether a key converges and returns, or follows until its objects leave the pending ones. */
    readonly mode?: "reconcile" | "follow";
    /** The keys reconciled at once, one by default. */
    readonly concurrency?: number;
    /** Reconcile the pending objects of one key, returning the wait until the next look. */
    reconcile(reconciliation: ObjectReconciliation<Object>): Promise<number | undefined>;
}

/** Another table whose changed rows select a controller's keys. */
export interface ObjectWatch<Watched extends Table = Table> {
    /** The watched table. */
    readonly table: Watched;
    /** Read the keys a changed row selects. */
    keys(
        row: RowImage<Watched>,
        database: DatabaseConnection,
    ): readonly KeyFields[] | Promise<readonly KeyFields[]>;
}

/** Watch other tables for a controller. */
export const ObjectWatch = {
    /** Watch a table's changed rows, typing them by the table. */
    of<Watched extends Table>(
        table: Watched,
        keys: ObjectWatch<Watched>["keys"],
    ): ObjectWatch<Watched> {
        return { table, keys };
    },
};

/** One key's reconciliation: its pending objects, and the system methods reconciling them. */
export interface ObjectReconciliation<
    Object extends ObjectType = ObjectType,
> extends Reconciliation {
    /** The pending objects of the key. */
    readonly rows: readonly RowImage<Object["table"]>[];
    /** The reconcile time, in UTC epoch milliseconds. */
    readonly now: number;
    /** The objects' database. */
    readonly database: DatabaseConnection;
    /** The object server running the controller, without the router its served types type. */
    readonly server: Omit<ObjectServer, "router">;
    /** Run one system method of the object type on some objects in one transaction, returning each result. */
    execute<Name extends MethodName<Object>>(
        method: Name,
        rows: readonly RowImage<Object["table"]>[],
    ): Promise<ResultOf<Object, Name>[]>;
}

/** Build the controllers object types declare. */
export const ObjectController = {
    /** Build the controller a type declares, reconciling or following its pending objects by key. */
    control(
        server: Omit<ObjectServer, "router">,
        object: ObjectType,
        declared: ObjectController,
    ): Controller {
        // key each pending object by its declared fields
        const table = object.table;
        const match = Predicate.compile(
            Condition.resolve(declared.pending, Namespace.fields(table)),
            table,
        );
        const watches = new Map((declared.watches ?? []).map((watch) => [watch.table, watch]));
        const keyOf = (row: Row): KeyFields => declared.key?.(row) ?? { id: identifierOf(row) };
        const keys = async (change: Change) => {
            // select the keys a watch of the changed table reads
            const watch = watches.get(change.table);
            const watched =
                watch === undefined ? [] : await watch.keys(Change.image(change), server.database);

            // add the key of a changed object left pending
            const row = Change.image(change);
            const isPending =
                Change.of(change, table) &&
                change.operation !== "delete" &&
                Predicate.matches(match, row);
            const selected = isPending ? [...watched, keyOf(row)] : watched;

            return [...new Set(selected.map((fields) => canonicalize(fields)))];
        };

        return {
            name: object.name,
            watches: [...new Set([table, ...watches.keys()])],
            ...(declared.concurrency === undefined ? {} : { concurrency: declared.concurrency }),
            ...(declared.mode === undefined ? {} : { mode: declared.mode }),
            keys,
            list: async () => {
                // list the keys of every pending object
                const rows = await Snapshot.live(server.database).rows(table, declared.pending);

                return [...new Set(rows.map((row) => canonicalize(keyOf(row))))];
            },
            reconcile: (key, reconciliation) =>
                reconcilePending(server, object, declared, key, reconciliation),
        };
    },
};

/** Reconcile one key's pending objects, if any are left. */
async function reconcilePending(
    server: Omit<ObjectServer, "router">,
    object: ObjectType,
    declared: ObjectController,
    key: string,
    reconciliation: Reconciliation,
): Promise<number | undefined> {
    // read the key's pending objects
    const pending: Condition = {
        AND: [declared.pending, Condition.equal(KeyFields.parse(JSON.parse(key)))],
    };
    const rows = await Snapshot.live(server.database).rows(object.table, pending);
    if (rows.length === 0) {
        return undefined;
    }

    // reconcile them, running system methods at one time
    const now = server.clock();

    return declared.reconcile({
        rows,
        now,
        database: server.database,
        server,
        signal: reconciliation.signal,
        ...(reconciliation.epoch === undefined ? {} : { epoch: reconciliation.epoch }),
        changed: () => reconciliation.changed(),
        execute: (method, targets) =>
            server.executeAsSystem(
                object,
                method,
                targets.map((row) => SystemCall.of(row)),
                now,
            ),
    });
}
