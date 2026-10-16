import {
    sql,
    type SQL,
    type Table,
    Condition,
    Change,
    type LogPosition,
    type Match,
    Namespace,
    Predicate,
    type Row,
} from "@destack/db";
import type { Query } from "../query/query.ts";

/** An audience that shows and reads every row. */
export const EVERYONE: Audience = {
    watches: [],
    where: () => sql`true`,
    key: "everyone",
    visible: async (_table, rows) => new Set(rows.keys()),
    concealable: () => [],
    conceals: async (_table, rows) => rows.map(() => []),
    decides: async () => false,
    until: async () => undefined,
    refresh: async () => {},
};

/** Who a subscription serves: the rows it may have and the columns it may read. */
export interface Audience {
    /** The tables whose changes decide visibility. */
    readonly watches: readonly Watch[];
    /** Match the rows the subscriber may have, under a query's own permission too, or "memory". */
    readonly where: (table: Table, permission?: string) => SQL | "memory";
    /** The name of what the audience decides by, alike for audiences deciding alike. */
    readonly key: string;
    /** Decide which rows the subscriber may have by index, under a query's own permission too. */
    readonly visible: (
        table: Table,
        rows: readonly Row[],
        position: LogPosition,
        permission?: string,
    ) => Promise<ReadonlySet<number>>;
    /** List the columns of a table the subscriber may not read on some rows. */
    readonly concealable: (table: Table) => readonly string[];
    /** List the columns the subscriber may not read on each row. */
    readonly conceals: (
        table: Table,
        rows: readonly Row[],
        position: LogPosition,
    ) => Promise<readonly (readonly string[])[]>;
    /** Read when the decisions next change on their own. */
    readonly until: () => Promise<number | undefined>;
    /** Decide as of now once the `until` moment passed. */
    readonly refresh: () => Promise<void>;
    /** Report whether a change may decide the visibility of rows beyond its own. */
    readonly decides: (change: Change) => Promise<boolean>;
}

/** The tables and scopes whose changes decide what a subscriber has. */
export interface Watch {
    /** The logged table. */
    readonly table: Table;
    /** The scopes whose changes matter, or every scope. */
    readonly scopes: Query["scopes"];
    /** The compiled condition of the rows whose changes matter, absent for every row. */
    readonly match?: Match;
}

/** Watches of logged tables. */
export const Watch = {
    /** Watch a table's changes in some scopes, of the rows a condition matches, compiled once. */
    of(table: Table, scopes: Query["scopes"], where?: Condition): Watch {
        // watch every row without a condition
        if (where === undefined) {
            return { table, scopes };
        }

        // compile the condition once for every change the watch sees
        const predicate = Condition.resolve(where, Namespace.fields(table));

        return { table, scopes, match: Predicate.compile(predicate, table) };
    },

    /** Decide whether a change is of a watched table, scope and row. */
    matches(watches: readonly Watch[], change: Change): boolean {
        return watches.some((entry) => {
            // require the table and scope
            if (
                entry.table !== change.table ||
                (entry.scopes !== "every" && !entry.scopes.includes(change.scope))
            ) {
                return false;
            }

            // require the row to match before or after the change
            const { match } = entry;

            return (
                match === undefined ||
                [Change.before(change), Change.after(change)].some(
                    (image) => image !== null && Predicate.matches(match, image),
                )
            );
        });
    },

    /** Join the scopes some watches read for one log read, absent when one reads every scope. */
    scopes(watches: readonly Watch[]): string[] | undefined {
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
    },
};
