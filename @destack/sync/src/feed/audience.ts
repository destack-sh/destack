import { sql, type SQL, type Table } from "@destack/db";
import { type Condition } from "@destack/db/query";
import type { Change, LogPosition } from "@destack/db/log";
import type { Row } from "@destack/db";

/** An audience that holds and reads every row, for copies between trusted databases. */
export const EVERYONE: Audience = {
    watches: [],
    where: () => sql`true`,
    key: "everyone",
    admits: async (_table, rows) => new Set(rows.keys()),
    concealable: () => [],
    conceals: async (_table, rows) => rows.map(() => []),
    dependents: async () => [],
    until: async () => undefined,
    refresh: async () => {},
};

/**
 * Who a subscription serves: which rows it may hold, which columns it may read, and what changes that.
 *
 * It decides rows as of a page's position with access as of now, so a page never sends what a committed revocation hides.
 * A change of access reaches the subscriber through the rows it affects.
 */
export interface Audience {
    /** The tables whose changes decide visibility, followed without being held. */
    readonly watches: readonly Watch[];
    /** Match the rows of a table the subscriber may hold, as SQL, or "memory" when only `admits` decides them, as for tables held in another database than their access. */
    where(table: Table): SQL | "memory";
    /** The name of everything the audience decides, equal for audiences that decide alike, so that their subscribers share evaluations. */
    readonly key: string;
    /** Decide which rows of a table as of a position the subscriber may hold now, by index in the list, sharing reads per position. */
    admits(table: Table, rows: readonly Row[], position: LogPosition): Promise<ReadonlySet<number>>;
    /** List the columns of a table the subscriber may not read on some rows, which queries may not filter, order, join or measure by. */
    concealable(table: Table): readonly string[];
    /** List, for each row of a table as of a position, the columns the subscriber may not read now, sharing reads per position. */
    conceals(
        table: Table,
        rows: readonly Row[],
        position: LogPosition,
    ): Promise<readonly (readonly string[])[]>;
    /** Read when the audience's decisions next change by time alone, absent when only changes decide them. */
    until(): Promise<number | undefined>;
    /** Decide as of now from here on, once the moment `until` names passed. */
    refresh(): Promise<void>;
    /** List the rows a change decides visibility of beyond itself, or everything when it reaches too far to list. */
    dependents(change: Change): Promise<readonly RowKey[] | "everything">;
}

/** The tables and scopes whose changes decide what a subscriber may hold. */
export interface Watch {
    /** The logged table. */
    readonly table: Table;
    /** The scopes whose changes matter. */
    readonly scopes: readonly string[];
    /** The rows whose changes matter within the scopes, as they were or are, absent for every row. */
    readonly where?: Condition;
}

/** A row whose visibility a change decided beyond the row itself. */
export interface RowKey {
    /** The row's table. */
    readonly table: Table;
    /** The row's key, by column property. */
    readonly key: Row;
}
