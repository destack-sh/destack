import { sql, type SQL, type Table } from "@destack/db";
import { type Condition } from "@destack/db/query";
import type { Change, LogPosition } from "@destack/db/log";
import type { Row } from "@destack/db";
import type { Query } from "../query/query.ts";

/** An audience that holds and reads every row. */
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

/** Who a subscription serves: the rows it may hold and the columns it may read. */
export interface Audience {
    /** The tables whose changes decide visibility. */
    readonly watches: readonly Watch[];
    /** Match the rows of a table the subscriber may hold, as SQL, or "memory" when only `admits` decides. */
    where(table: Table): SQL | "memory";
    /** The name of everything the audience decides. */
    readonly key: string;
    /** Decide which rows of a table the subscriber may hold, by index. */
    admits(table: Table, rows: readonly Row[], position: LogPosition): Promise<ReadonlySet<number>>;
    /** List the columns of a table the subscriber may not read on some rows. */
    concealable(table: Table): readonly string[];
    /** List the columns the subscriber may not read on each row. */
    conceals(
        table: Table,
        rows: readonly Row[],
        position: LogPosition,
    ): Promise<readonly (readonly string[])[]>;
    /** Read when the decisions next change by time alone. */
    until(): Promise<number | undefined>;
    /** Decide as of now once the `until` moment passed. */
    refresh(): Promise<void>;
    /** List the rows a change decides visibility of beyond itself. */
    dependents(change: Change): Promise<readonly RowKey[] | "everything">;
}

/** The tables and scopes whose changes decide what a subscriber holds. */
export interface Watch {
    /** The logged table. */
    readonly table: Table;
    /** The scopes whose changes matter, or every scope. */
    readonly scopes: Query["scopes"];
    /** The rows whose changes matter, absent for every row. */
    readonly where?: Condition;
}

/** Join the scopes some watches read for one log read, absent when one reads every scope. */
export function watchedScopes(watches: readonly Watch[]): string[] | undefined {
    const scopes = new Set<string>();
    for (const { scopes: watched } of watches) {
        if (watched === "every") {
            return undefined;
        }
        for (const scope of watched) {
            scopes.add(scope);
        }
    }

    return [...scopes];
}

/** A row whose visibility a change decided. */
export interface RowKey {
    /** The row's table. */
    readonly table: Table;
    /** The row's key, by column property. */
    readonly key: Row;
}
