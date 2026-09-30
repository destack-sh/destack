import type { DatabaseConnection, Table } from "@destack/db";
import type { Condition } from "@destack/db/query";
import type { Reconciliation } from "@destack/service/control";
import type { ObjectServer } from "../server/server.ts";

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
