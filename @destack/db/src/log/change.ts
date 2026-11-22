import { TABLE, type RowImage, type Table } from "../table/table.ts";
import type { Key } from "../query/key.ts";

/** One logged write: an insert with the row after, an update with both, a delete with before. */
export type Write<Definition extends Table = Table> = {
    /** The writing transaction, absent for SQLite scripts outside a transaction. */
    readonly transaction: string | null;
    /** The changed table. */
    readonly table: Definition;
    /** The primary key of the changed row. */
    readonly key: Key<Definition>;
    /** The scope the changed row lives in. */
    readonly scope: string;
    /** The change time, in UTC epoch milliseconds. */
    readonly changedAt: number;
} & (
    | {
          /** Insert the row. */
          readonly operation: "insert";
          /** The row after the insert. */
          readonly after: RowImage<Definition>;
      }
    | {
          /** Update the row. */
          readonly operation: "update";
          /** The row before the update. */
          readonly before: RowImage<Definition>;
          /** The row after the update. */
          readonly after: RowImage<Definition>;
      }
    | {
          /** Delete the row. */
          readonly operation: "delete";
          /** The row before the delete. */
          readonly before: RowImage<Definition>;
      }
);

/** One committed write, at its position in commit order. */
export type Change<Definition extends Table = Table> = Write<Definition> & {
    /** The change's position in commit order. */
    readonly sequence: number;
};

/** Read changes. */
export const Change = {
    /** Read a write's latest row image: the row after an insert or update, before a delete. */
    image<Definition extends Table>(change: Write<Definition>): RowImage<Definition> {
        return change.operation === "delete" ? change.before : change.after;
    },

    /** Read the row before a write, null for an insert. */
    before<Definition extends Table>(change: Write<Definition>): RowImage<Definition> | null {
        return change.operation === "insert" ? null : change.before;
    },

    /** Read the row after a write, null for a delete. */
    after<Definition extends Table>(change: Write<Definition>): RowImage<Definition> | null {
        return change.operation === "delete" ? null : change.after;
    },

    /** Report whether an update leaves its table's generation, recording an observation alone. */
    isObservation(change: Write): boolean {
        const generation = change.table[TABLE].generation;

        return (
            generation !== undefined &&
            change.operation === "update" &&
            change.before[generation] === change.after[generation]
        );
    },

    /** Report whether a change is to one table, typing its rows by it. */
    of<Definition extends Table>(change: Change, table: Definition): change is Change<Definition> {
        return change.table === table;
    },
};

/** The positions and times of a transaction's changes. */
export interface TransactionBounds {
    /** The position before its first change. */
    readonly before: number;
    /** The position after its last change. */
    readonly after: number;
    /** The time of its first change, in UTC epoch milliseconds. */
    readonly startedAt: number;
    /** The time of its last change, in UTC epoch milliseconds. */
    readonly committedAt: number;
}

/** Changes read after a sequence. */
export interface ChangePage<Definition extends Table = Table> {
    /** The changes in commit order. */
    readonly changes: readonly Change<Definition>[];
    /** The sequence the next read continues after. */
    readonly sequence: number;
    /** Whether the page reached the log's end as read, leaving nothing past it for now. */
    readonly isLast: boolean;
}

/** The write-order id of a log entry: the row id on SQLite, the identity column on PostgreSQL. */
export type LogId = number;

/** The tables and log id of a read of the open transaction's own changes. */
export interface TransactionSelection<Definition extends Table> {
    /** The logged tables to read. */
    readonly tables: readonly Definition[];
    /** The log id read last, the transaction's start when absent. */
    readonly after?: LogId;
}

/** The open transaction's own changes after a log id. */
export interface TransactionPage<Definition extends Table = Table> {
    /** The changes in write order. */
    readonly changes: readonly Write<Definition>[];
    /** The log id the next read continues after. */
    readonly after: LogId;
}

/** The tables and position of a change read. */
export interface ChangeSelection<Definition extends Table> {
    /** The logged tables to read. */
    readonly tables: readonly Definition[];
    /** The consumed sequence, zero for the start. */
    readonly after: number;
    /** The most changes a page has before completing its last transaction. */
    readonly limit?: number;
    /** The scopes to read. */
    readonly scopes?: readonly string[];
}
