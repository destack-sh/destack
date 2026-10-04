import {
    type QueryOptions,
    type Relation,
    Relations,
    type Row,
    type Scalar,
    type Table,
} from "@destack/db";

/** Rows of one logged table a subscriber keeps, with the related rows they include, or aggregates of them. */
export interface Query extends QueryOptions {
    /** The logged table. */
    readonly table: Table;
    /** The scopes whose rows the query selects, or every scope. */
    readonly scopes: readonly string[] | "every";
    /** The schema's relations, which `with`, `exists`, lookups and rollups name. */
    readonly relations?: Relations;
}

/** One result of a query: its row, and what each include selects for it by the include's name. */
export type Item = {
    /** The row's readable columns and computed values. */
    readonly row: Row;
    /** Each include's items, a key include's one item as a one-element list and none as an empty list. */
    readonly with: Readonly<Record<string, readonly Item[]>>;
    /** Each aggregate include's groups, a whole include's measures as one group without group values. */
    readonly measures: Readonly<Record<string, readonly AggregateRow[]>>;
};

/** One group of an aggregate's rows: its group values and its measures. */
export type AggregateRow = {
    /** The group's values in JSON form, by column. */
    readonly group: Readonly<Record<string, Scalar>>;
    /** The group's measures, by name. */
    readonly values: Readonly<Record<string, Scalar>>;
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
