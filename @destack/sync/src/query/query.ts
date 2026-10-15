import {
    type Extras,
    type QueryOptions,
    type Relation,
    Relations,
    type Row,
    type Table,
} from "@destack/db";
import type { Aggregate, AggregateRow } from "./aggregate.ts";

/** What a subscriber's query selects at one level: rows with extras and includes, or aggregates. */
export interface QuerySelection extends QueryOptions {
    /** The values computed from each row, read like fields, by name. */
    readonly extras?: Extras;
    /** The related rows each selected row includes, by relation: every row, or a selection. */
    readonly with?: Readonly<Record<string, true | QuerySelection>>;
    /** The aggregates kept instead of rows. */
    readonly aggregate?: Aggregate;
}

/** Rows of one logged table a subscriber keeps, with their included rows, or aggregates. */
export interface Query extends QuerySelection {
    /** The logged table. */
    readonly table: Table;
    /** The scopes whose rows the query selects, or every scope. */
    readonly scopes: readonly string[] | "every";
    /** The schema's relations, which `with`, `exists`, lookups and rollups reference. */
    readonly relations?: Relations;
}

/** One result of a query: its row, and what each include selects for it by the include's name. */
export type Item = {
    /** The row's readable columns and computed values. */
    readonly row: Row;
    /** Each include's items, a key include's item as a one-element or empty list. */
    readonly with: Readonly<Record<string, readonly Item[]>>;
    /** Each aggregate include's groups, a whole include's measures as one ungrouped group. */
    readonly measures: Readonly<Record<string, readonly AggregateRow[]>>;
};

/** Resolve a query's included relation as a query of the related table, with the relation. */
export function included(
    query: Query,
    name: string,
): { readonly relation: Relation; readonly query: Query } {
    // name the relation by the root's schema that nested queries keep
    const relations = query.relations ?? new Relations();
    const relation = relations.get(query.table, name);
    const selected = query.with?.[name];

    return {
        relation,
        query: {
            ...(selected === undefined || selected === true ? {} : selected),
            table: relation.table,
            scopes: query.scopes,
            relations,
        },
    };
}
