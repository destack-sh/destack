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

/** An audience that admits and reads every row. */
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

/** Who a subscription serves: the rows it may have and the columns it may read. */
export interface Audience {
    /** The tables whose changes decide visibility. */
    readonly watches: readonly Watch[];
    /** Match the rows of a table the subscriber may have, as SQL, or "memory" when only `admits` decides. */
    where(table: Table): SQL | "memory";
    /** The name of everything the audience decides. */
    readonly key: string;
    /** Decide which rows of a table the subscriber may have, by index. */
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

/** The tables and scopes whose changes decide what a subscriber has. */
export interface Watch {
    /** The logged table. */
    readonly table: Table;
    /** The scopes whose changes matter, or every scope. */
    readonly scopes: Query["scopes"];
    /** The rows whose changes matter, absent for every row. */
    readonly where?: Condition;
}

/** The compiled row conditions of watches. */
const MATCHES = new WeakMap<Watch, Match>();

/** Watches of logged tables. */
export const Watch = {
    /** Decide whether a change is of a watched table, scope and row. */
    matches(watches: readonly Watch[], change: Change): boolean {
        return watches.some((entry) => {
            // require the table and scope
            if (
                entry.table !== change.table ||
                (entry.scopes !== "every" && !entry.scopes.includes(change.scope))
            ) {
                return false;
            } else if (entry.where === undefined) {
                return true;
            }

            // require the row to match before or after the change
            let match = MATCHES.get(entry);
            if (match === undefined) {
                const predicate = Condition.resolve(entry.where, Namespace.fields(entry.table));
                match = Predicate.compile(predicate, entry.table);
                MATCHES.set(entry, match);
            }

            return [Change.before(change), Change.after(change)].some(
                (image) => image !== null && Predicate.matches(match, image),
            );
        });
    },
};

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
