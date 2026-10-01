import type { DatabaseConnection, Table } from "@destack/db";
import { Condition, type Scalar } from "@destack/db/query";
import type { Controller, Reconciliation } from "@destack/service/control";
import { type Change, Snapshot } from "@destack/db/log";
import { canonicalize } from "@destack/schema/json";
import type { ObjectType } from "./object.ts";
import { type ObjectServer, SystemCall } from "../server/server.ts";

/** A row as a controller reads it. */
type Row = Readonly<Record<string, unknown>>;

/** A controller declared on an object type: the objects with work waiting, and how to reconcile them. */
export interface ObjectController<Selected extends Row = Row> {
    /** The objects with work waiting. */
    readonly pending: Condition;
    /** The fields grouping objects reconciled together, the identifier by default. */
    readonly key?: (row: Selected) => Row;
    /** The other tables whose changed rows select keys again. */
    readonly watches?: readonly ObjectWatch[];
    /** Whether a key converges and returns, or follows until its objects leave the pending ones. */
    readonly mode?: "reconcile" | "follow";
    /** The keys reconciled at once, one by default. */
    readonly concurrency?: number;
    /** Reconcile the pending objects of one key, returning the wait until the next look. */
    reconcile(reconciliation: ObjectReconciliation<Selected>): Promise<number | undefined>;
}

/** Another table whose changed rows select a controller's keys. */
export interface ObjectWatch {
    /** The watched table. */
    readonly table: Table;
    /** Read the keys a changed row selects. */
    keys(
        row: Readonly<Record<string, unknown>>,
        database: DatabaseConnection,
    ):
        | readonly Readonly<Record<string, unknown>>[]
        | Promise<readonly Readonly<Record<string, unknown>>[]>;
}

/** One key's reconciliation: its pending objects, and the system methods reconciling them. */
export interface ObjectReconciliation<Selected extends Row = Row> extends Reconciliation {
    /** The pending objects of the key. */
    readonly rows: readonly Selected[];
    /** The reconcile time, in UTC epoch milliseconds. */
    readonly now: number;
    /** The objects' database. */
    readonly database: DatabaseConnection;
    /** The object server running the controller. */
    readonly server: ObjectServer;
    /** Run one system method on some objects in one transaction. */
    execute(method: string, rows: readonly Row[]): Promise<unknown[]>;
}

/** Build the controllers object types declare. */
export const ObjectController = {
    /** Build the controller a type declares, reconciling or following its pending objects by key. */
    control(server: ObjectServer, object: ObjectType, declared: ObjectController): Controller {
        // key each pending object by its declared fields
        const table = object.table as Table;
        const match = Condition.compile(declared.pending, table);
        const keyOf = (fields: Readonly<Record<string, unknown>>) => canonicalize(fields);
        const pending = (key: string) => {
            const fields = Object.entries(JSON.parse(key) as Readonly<Record<string, Scalar>>);

            return Condition.all(
                declared.pending,
                ...fields.map(([name, value]) => Condition.eq(name, value)),
            );
        };
        const watches = new Map((declared.watches ?? []).map((watch) => [watch.table, watch]));
        const keys = async (change: Change) => {
            // select the keys of a watched row, or the key of a changed object left pending
            const row = (change.after ?? change.before) as Readonly<Record<string, unknown>>;
            const watch = watches.get(change.table);
            const selected =
                watch !== undefined
                    ? await watch.keys(row, server.database)
                    : change.after !== undefined && Condition.matches(match, row)
                      ? [declared.key?.(row) ?? { id: row.id }]
                      : [];

            return selected.map(keyOf);
        };

        return {
            name: object.name,
            watches: [table, ...watches.keys()],
            ...(declared.concurrency === undefined ? {} : { concurrency: declared.concurrency }),
            ...(declared.mode === undefined ? {} : { mode: declared.mode }),
            keys,
            list: async () => {
                // list the keys of every pending object
                const rows = await Snapshot.live(server.database).rows(table, declared.pending);

                return [
                    ...new Set(rows.map((row) => keyOf(declared.key?.(row) ?? { id: row.id }))),
                ];
            },
            reconcile: async (key, reconciliation) => {
                // reconcile the key's pending objects, if any are left
                const rows = await Snapshot.live(server.database).rows(table, pending(key));
                if (rows.length === 0) {
                    return undefined;
                }
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
            },
        };
    },
};
