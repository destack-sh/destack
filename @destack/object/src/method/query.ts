import type * as db from "@destack/db";
import { Comparable, type Condition, type Row, type Table } from "@destack/db";
import { waitAborted } from "@destack/environment";
import { included, type Item, type Query } from "@destack/sync";
import type { LiveItems, Subscriber } from "../client/query.ts";
import { ObjectQueryBuilder } from "../client/query.ts";
import type { ObjectQueries } from "../client/client.ts";
import { ObjectError } from "../error/error.ts";
import type { ObjectType } from "../object/object.ts";
import type { ObjectQuery } from "../replica/replica.ts";

/** Read a table's rows in a call's view, recording each read. */
export type TableReader = (table: Table) => db.RelationalQueryBuilder<Table>;

/** An item as a level reads it, before its relations are set. */
type LevelItem = {
    readonly row: Row;
    readonly with: Record<string, Item[]>;
    readonly measures: {};
};

/** Follow queries over a call's view, which stays the same through the call: one read each. */
export class ViewSubscriber implements Subscriber {
    /** The object types the queries' relations reach. */
    readonly #objects: readonly ObjectType[];
    /** Read a table's rows in the view. */
    readonly #read: TableReader;

    /** Follow queries of some object types through a view's table reads. */
    constructor(objects: readonly ObjectType[], read: TableReader) {
        this.#objects = objects;
        this.#read = read;
    }

    /** Query each of some object types in a view, by key, as clients query them. */
    static queries<const Objects extends Readonly<Record<string, ObjectType>>>(
        objects: Objects,
        read: TableReader,
    ): ObjectQueries<Objects> {
        const subscriber = new ViewSubscriber(Object.values(objects), read);

        return ObjectQueryBuilder.of(subscriber, objects);
    }

    /** Follow a query's items in the view, read once since the view stays the same. */
    rows(object: ObjectType, query: Omit<ObjectQuery, "object" | "package">): LiveItems {
        const compiled: Query = { ...object.compile(query, this.#objects), scopes: "every" };
        const read = () => this.#items(compiled, undefined);

        return {
            query: compiled,
            ready: Promise.resolve(),
            read,
            watch: (signal) => ViewSubscriber.#once(read, signal),
            close: async () => {},
        };
    }

    /** Refuse an aggregate, which a call's reads measure nothing for. */
    results(): never {
        throw ObjectError.of("READ_UNSUPPORTED", "a method's reads measure no aggregates");
    }

    /** Yield the view's one read, then wait out the follow, which sees no change. */
    static async *#once(
        read: () => Promise<readonly Item[]>,
        signal: AbortSignal,
    ): AsyncGenerator<readonly Item[]> {
        yield await read();
        await waitAborted(signal);
    }

    /** Read a level's items meeting a condition too, each with its included relations. */
    async #items(query: Query, condition: Condition | undefined): Promise<LevelItem[]> {
        // refuse what recorded reads cannot compute
        if (query.extras !== undefined && Object.keys(query.extras).length > 0) {
            throw ObjectError.of("READ_UNSUPPORTED", "a method's reads compute no extras");
        } else if (query.aggregate !== undefined) {
            throw ObjectError.of("READ_UNSUPPORTED", "a method's reads measure no aggregates");
        }

        // read the level's rows, a first unordered row by one point read where a key names it
        const where = ViewSubscriber.#joined([query.where, condition]);
        const reader = this.#read(query.table);
        const rows =
            query.limit === 1 && query.orderBy === undefined
                ? ViewSubscriber.#listed(
                      await reader.findFirst(where === undefined ? {} : { where }),
                  )
                : await reader.findMany({
                      ...(where === undefined ? {} : { where }),
                      ...(query.orderBy === undefined ? {} : { orderBy: query.orderBy }),
                      ...(query.limit === undefined ? {} : { limit: query.limit }),
                  });
        const items = rows.map((row): LevelItem => ({ row, with: {}, measures: {} }));

        // include each relation's items
        for (const name of Object.keys(query.with ?? {})) {
            await this.#include(query, name, items);
        }

        return items;
    }

    /** Set a relation's items on each item, one batched read per relation. */
    async #include(query: Query, name: string, items: LevelItem[]): Promise<void> {
        // meet the relation's condition and the include's
        const { relation, query: nested } = included(query, name);
        const where = ViewSubscriber.#joined([relation.where, nested.where]);
        const target = where === undefined ? nested : { ...nested, where };
        const { on } = relation;
        const rows = items.map((item) => item.row);

        // join a related column to the item's column
        if (on.kind === "key") {
            const related = await this.#related(
                target,
                on.column,
                ViewSubscriber.#values(rows, on.parent),
            );
            for (const item of items) {
                item.with[name] = related.get(ViewSubscriber.#key(item.row[on.parent])) ?? [];
            }
        }
        // join through a join table's rows
        else if (on.kind === "junction") {
            const values = ViewSubscriber.#values(rows, on.from.key);
            const joins = await this.#read(on.table).findMany({
                where: { [on.from.column]: { in: values } },
            });
            const related = await this.#related(
                target,
                on.to.key,
                ViewSubscriber.#values(joins, on.to.column),
            );
            for (const item of items) {
                const key = ViewSubscriber.#key(item.row[on.from.key]);
                item.with[name] = joins
                    .filter((join) => ViewSubscriber.#key(join[on.from.column]) === key)
                    .flatMap((join) => related.get(ViewSubscriber.#key(join[on.to.column])) ?? []);
            }
        }
        // refuse a tree's walk, which recorded reads cannot follow
        else {
            throw ObjectError.of(
                "READ_UNSUPPORTED",
                `a method's reads cannot follow tree relation ${name}`,
            );
        }
    }

    /** Read the related items of some key values, grouped by value: per value when limited. */
    async #related(
        query: Query,
        column: string,
        values: readonly Comparable[],
    ): Promise<Map<string, LevelItem[]>> {
        // read every value's items at once, or each value's own first items under a limit
        const reads =
            query.limit === undefined
                ? [await this.#items(query, { [column]: { in: values } })]
                : await Promise.all(values.map((value) => this.#items(query, { [column]: value })));

        return Map.groupBy(reads.flat(), (item) => ViewSubscriber.#key(item.row[column]));
    }

    /** List the distinct key values of a column among rows, leaving out null ones. */
    static #values(
        rows: readonly Readonly<Record<string, unknown>>[],
        column: string,
    ): Comparable[] {
        const values = rows.flatMap((row) => {
            const value = row[column];

            return value === null ? [] : [Comparable.parse(value)];
        });

        return [...new Map(values.map((value) => [ViewSubscriber.#key(value), value])).values()];
    }

    /** List a row found, none for none. */
    static #listed(row: Row | undefined): Row[] {
        return row === undefined ? [] : [row];
    }

    /** Read the key of a column value, comparing values of any scalar type alike. */
    static #key(value: unknown): string {
        return JSON.stringify(value);
    }

    /** Meet the present conditions, none for none. */
    static #joined(conditions: readonly (Condition | undefined)[]): Condition | undefined {
        const present = conditions.filter((condition) => condition !== undefined);

        return present.length <= 1 ? present[0] : { AND: present };
    }
}
