import type { DatabaseConnection } from "@destack/db";
import type { Condition } from "@destack/db/query";

/** A controller declared on an object type: the objects with work waiting, and how to reconcile them. */
export interface ObjectController {
    /** The objects with work waiting. */
    readonly pending: Condition;
    /** The fields grouping objects reconciled together, the identifier by default. */
    readonly key?: (row: Readonly<Record<string, unknown>>) => Readonly<Record<string, unknown>>;
    /** The keys reconciled at once, one by default. */
    readonly concurrency?: number;
    /** Reconcile the pending objects of one key, returning the wait until the next look. */
    reconcile(control: ObjectControl): Promise<number | undefined>;
}

/** The pending objects of one key, and the system methods reconciling them. */
export interface ObjectControl {
    /** The pending objects of the key. */
    readonly rows: readonly Readonly<Record<string, unknown>>[];
    /** The reconcile time, in UTC epoch milliseconds. */
    readonly now: number;
    /** The objects' database. */
    readonly database: DatabaseConnection;
    /** Run one system method on some objects in one transaction. */
    execute(method: string, rows: readonly Readonly<Record<string, unknown>>[]): Promise<unknown[]>;
}
