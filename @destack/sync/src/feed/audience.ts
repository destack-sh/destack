import {
    sql,
    type SQL,
    type Table,
    Condition,
    type Match,
    Namespace,
    Predicate,
    type Row,
    Change,
    type LogPosition,
} from "@destack/db";
import type { Query } from "../query/query.ts";

/** No rows beyond the changed row. */
export const NO_ROWS: Affected = { kind: "none" };

/** Every row, as a change of the scope's own access decides them all. */
export const ALL_ROWS: Affected = { kind: "all" };

/** An audience that shows and reads every row. */
export const EVERYONE: Audience = {
    watches: [],
    candidates: async () => sql`true`,
    readable: async () => sql`true`,
    key: "everyone",
    visible: async (_table, rows) => new Set(rows.keys()),
    concealable: () => [],
    conceals: async (_table, rows) => rows.map(() => []),
    affects: async () => NO_ROWS,
    until: async () => undefined,
    refresh: async () => {},
};

/** Who a subscription serves: the rows it may have and the columns it may read. */
export interface Audience {
    /** The tables whose changes decide visibility. */
    readonly watches: readonly Watch[];
    /** Match a superset of the rows the subscriber may have, which `visible` decides. */
    readonly candidates: (table: Table, permission?: string) => Promise<SQL>;
    /** Match exactly the rows the subscriber may have, where SQL reads related rows whole. */
    readonly readable: (table: Table, permission?: string) => Promise<SQL>;
    /** The name of what the audience decides by, alike for audiences deciding alike. */
    readonly key: string;
    /** Decide which rows the subscriber may have by index, under a query's permission too. */
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
    /** Read when the decisions next change by themselves. */
    readonly until: () => Promise<number | undefined>;
    /** Decide as of now once the `until` moment passed. */
    readonly refresh: () => Promise<void>;
    /** Read the rows beyond the changed row whose visibility a change may change. */
    readonly affects: (change: Change) => Promise<Affected>;
}

/** The rows beyond a changed row whose visibility the change may change. */
export type Affected =
    | { readonly kind: "none" }
    | {
          readonly kind: "rows";
          /** Read the affected rows' conditions by table, at the positions they are decided at. */
          readonly read: (
              positions: readonly LogPosition[],
          ) => Promise<ReadonlyMap<Table, Condition>>;
      }
    | { readonly kind: "all" };

/** The tables and scopes whose changes decide what a subscriber has. */
export interface Watch {
    /** The logged table. */
    readonly table: Table;
    /** The scopes whose changes matter, or every scope. */
    readonly scopes: Query["scopes"];
    /** The compiled condition of the rows whose changes matter, absent for every row. */
    readonly match?: Match;
}

/** The route of changes in any scope, beside the route of each scope. */
export const EVERY_SCOPE = "*";

/** Watches of logged tables. */
export const Watch = {
    /** List the routes some watches and other scopes wait on: each scope, any scope as one. */
    routes(watches: readonly Watch[], scopes: readonly string[]): string[] {
        return [
            ...new Set([
                ...watches.flatMap((watch) =>
                    watch.scopes === "every" ? [EVERY_SCOPE] : watch.scopes,
                ),
                ...scopes,
            ]),
        ];
    },

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
