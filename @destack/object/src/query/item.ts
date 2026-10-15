import type * as db from "@destack/db";
import type { ColumnValue, Extras, Relation } from "@destack/db";
import { included, type Item, type Query } from "@destack/sync";
import type * as sync from "@destack/sync";
import type { ObjectType } from "../object/object.ts";
import type { ModelOf } from "./relation.ts";

/** A relational read of an object type's rows, with the trash a root selects. */
export type FindOptions<
    Object extends ObjectType,
    Objects extends ObjectType,
    Cardinality extends "one" | "many" = "many",
    Computed extends Extras = Extras,
> = db.FindOptions<ModelOf<Object, Objects>, Cardinality, Computed> & {
    /** Which trashed rows of a recoverable type to select, none by default. */
    readonly deleted?: "exclude" | "include" | "only";
};

/** Measures of an object type's rows per group, live like any read. */
export type AggregateOptions<
    Object extends ObjectType,
    Objects extends ObjectType,
> = sync.AggregateOptions<ModelOf<Object, Objects>>;

/** The condition of an object type's rows, over logged fields, a read's extras and relations. */
export type ConditionOf<
    Object extends ObjectType,
    Objects extends ObjectType,
    Computed extends Extras = {},
> = db.ConditionOf<ModelOf<Object, Objects>, Computed>;

/** One relational read result of an object type: selected fields, extras and relation results. */
export type FindResult<
    Object extends ObjectType,
    Objects extends ObjectType,
    Options,
> = db.FindResult<ModelOf<Object, Objects>, Options>;

/** An item as callers read it: its row with each relation's one item or null, or its items. */
export type NestedItem = {
    readonly [property: string]: ColumnValue | NestedItem | readonly NestedItem[];
};

/** Nest an item's includes beside its row as its relations read, without aggregate includes. */
export function nestItem(item: Item, query: Query): NestedItem {
    return {
        ...item.row,
        ...Object.fromEntries(
            Object.entries(item.with).map(([name, items]) => {
                const include = included(query, name);

                return [
                    name,
                    relateItems(
                        include.relation,
                        items.map((entry) => nestItem(entry, include.query)),
                    ),
                ];
            }),
        ),
    };
}

/** Read a relation's items: a one relation's item or null, else every item. */
export function relateItems<Value>(
    relation: Relation,
    items: readonly Value[],
): Value | null | readonly Value[] {
    return relation.cardinality === "one" ? (items[0] ?? null) : items;
}
