import { and, or, sql, type SQL, type SQLWrapper } from "drizzle-orm";
import { defineSchema, schema } from "@destack/schema";
import { dialectSQL } from "../dialect/expression.ts";
import { Column } from "../table/column.ts";
import { TABLE, type Table } from "../table/table.ts";
import { Key } from "./key.ts";
import type { Row } from "../table/row.ts";
import { Expression } from "./expression.ts";
import type { Namespace } from "./namespace.ts";
import { DatabaseError } from "../error/error.ts";

/** The column kinds that order and compare alike in SQLite, PostgreSQL and memory. */
const ORDERED_KINDS: ReadonlySet<string> = new Set([
    "text",
    "integer",
    "real",
    "boolean",
    "bigint",
    "timestamp",
]);

/** The most keys one order holds, well beyond the columns an index serves. */
const ORDER_KEYS = 16;

/** One key of an order: a column by property and its direction. */
export const OrderKey = defineSchema(
    schema.object({
        /** The column's property. */
        column: schema.string().min(1),
        /** Whether values ascend or descend; missing values sort lowest either way. */
        direction: schema.enum(["asc", "desc"]),
    }),
);
/** One key of an order: a column by property and its direction. */
export type OrderKey = schema.Infer<typeof OrderKey>;

/** How rows sort: keys in precedence, missing values lowest, text by UTF-8 bytes in every dialect. */
export type Order = readonly OrderKey[];

/** How rows sort: keys in precedence, missing values lowest, text by UTF-8 bytes in every dialect. */
export const Order = {
    /** The schema of an order. */
    schema: defineSchema(schema.array(OrderKey)),

    /** Require an order of distinct keys, at most a bounded number, over a table's orderable columns and computed values. */
    require(order: Order, table: Table, namespace: Namespace = { computed: {} }): void {
        // bound the order, which clients choose
        const columns = new Set(order.map((key) => key.column));
        if (order.length > ORDER_KEYS || columns.size < order.length) {
            throw new DatabaseError(
                "INVALID_QUERY",
                `order holds more than ${ORDER_KEYS} keys, or one column twice`,
            );
        }

        // require orderable columns, or computed values
        for (const key of order) {
            if (!Object.hasOwn(namespace.computed, key.column)) {
                Order.column(table, key.column);
            }
        }
    },

    /** Read an orderable column of a table by property. */
    column(table: Table, name: string): Column {
        const found = table[TABLE].columns[name];
        if (found === undefined) {
            throw new DatabaseError("INVALID_QUERY", `${table[TABLE].name} has no column ${name}`);
        } else if (!ORDERED_KINDS.has(found.definition.kind)) {
            throw new DatabaseError(
                "INVALID_QUERY",
                `${table[TABLE].name}.${name} is a ${found.definition.kind} column`,
            );
        }

        return found;
    },

    /** Extend an order with the table's primary key, so that it orders every row apart. */
    complete(order: Order, table: Table): Order {
        const keys = table[TABLE].key.filter((name) => !order.some((key) => key.column === name));

        return [...order, ...keys.map((name) => ({ column: name, direction: "asc" as const }))];
    },

    /** Render an order as ORDER BY expressions, missing values lowest and text by byte. */
    render(order: Order, table: Table, namespace: Namespace = { computed: {} }): SQL[] {
        return order.map((key) => {
            const expression = Order.expression(table, key.column, namespace);

            return key.direction === "asc"
                ? sql`${expression} ASC NULLS FIRST`
                : sql`${expression} DESC NULLS LAST`;
        });
    },

    /** Select the rows an order places after a row, whose computed values it holds. */
    after(
        order: Order,
        table: Table,
        row: Readonly<Record<string, unknown>>,
        namespace: Namespace = { computed: {} },
    ): SQL {
        // tie on the leading keys and follow on the next, for each key in turn
        const alternatives = order.map((key, position) =>
            and(
                ...order.slice(0, position).map((leading) => tie(table, leading, row, namespace)),
                follow(table, key, row, namespace),
            ),
        );

        return or(...alternatives) ?? sql`false`;
    },

    /** Compare two rows by an order. */
    rows(
        order: Order,
        left: Readonly<Record<string, unknown>>,
        right: Readonly<Record<string, unknown>>,
    ): number {
        for (const key of order) {
            const compared = Order.missingFirst(left[key.column], right[key.column]);
            if (compared !== 0) {
                return key.direction === "asc" ? compared : -compared;
            }
        }

        return 0;
    },

    /**
     * Merge ordered rows of several tables into the first of their shared order, each with its list's name.
     *
     * Rows tie by their list's name, then by key, so that every reader merges alike.
     */
    merge(
        order: Order,
        lists: readonly {
            readonly name: string;
            readonly table: Table;
            readonly rows: readonly Row[];
        }[],
        limit: number,
    ): { readonly name: string; readonly row: Row }[] {
        // take every row with its list's name and key
        const entries = lists.flatMap(({ name, table, rows }) =>
            rows.map((row) => ({ name, row, key: Key.name(table, row) })),
        );

        // sort by the order, then by name and key, and keep the first
        entries.sort(
            (left, right) =>
                Order.rows(order, left.row, right.row) ||
                Order.codePoints(left.name, right.name) ||
                Order.codePoints(left.key, right.key),
        );

        return entries.slice(0, limit).map(({ name, row }) => ({ name, row }));
    },

    /** Compare two values with missing values lowest, as the rendered order does. */
    missingFirst(left: unknown, right: unknown): number {
        // place missing values before present ones
        const isLeftMissing = left === null || left === undefined;
        const isRightMissing = right === null || right === undefined;
        if (isLeftMissing || isRightMissing) {
            return Number(isRightMissing) - Number(isLeftMissing);
        }

        return Order.values(left, right)!;
    },

    /** Compare two present values as SQL does, absent when either is missing or their types differ. */
    values(left: unknown, right: unknown): number | undefined {
        // leave missing values unordered
        if (left === null || left === undefined || right === null || right === undefined) {
            return undefined;
        }

        // order instants by time, text by code point, and other scalars by value
        const first = left instanceof Date ? left.getTime() : left;
        const second = right instanceof Date ? right.getTime() : right;
        if (typeof first === "string" && typeof second === "string") {
            return Order.codePoints(first, second);
        } else if (
            (typeof first === "number" || typeof first === "bigint") &&
            (typeof second === "number" || typeof second === "bigint")
        ) {
            return first < second ? -1 : first > second ? 1 : 0;
        } else if (typeof first === "boolean" && typeof second === "boolean") {
            return Number(first) - Number(second);
        }

        return undefined;
    },

    /** Compare text by code point, the byte order of UTF-8 that SQLite and the C collation use. */
    codePoints(left: string, right: string): number {
        // find the first differing UTF-16 unit, which orders the strings unless a surrogate pair begins there
        const length = Math.min(left.length, right.length);
        let index = 0;
        while (index < length && left.charCodeAt(index) === right.charCodeAt(index)) {
            index += 1;
        }
        if (index === length) {
            return Math.sign(left.length - right.length);
        }
        const first = left.charCodeAt(index);
        const second = right.charCodeAt(index);
        if (!isSurrogate(first) && !isSurrogate(second)) {
            return first < second ? -1 : 1;
        }

        // compare the code points around a surrogate, since UTF-16 units order astral characters below some others
        const start =
            index > 0 && (isLowSurrogate(first) || isLowSurrogate(second)) ? index - 1 : index;
        const leftPoint = left.codePointAt(start)!;
        const rightPoint = right.codePointAt(start)!;
        if (leftPoint === rightPoint) {
            return first < second ? -1 : 1;
        }

        return leftPoint < rightPoint ? -1 : 1;
    },

    /** Render a column or computed value to sort and compare by, text by byte in every dialect. */
    expression(table: Table, name: string, namespace: Namespace = { computed: {} }): SQLWrapper {
        // render a column collated as text, or a computed value, collating computed text alike
        const expression = namespace.computed[name];
        if (expression === undefined) {
            return Order.text(Order.column(table, name));
        }
        const rendered = Expression.render(expression, table, namespace);

        return Expression.kind(expression, table, namespace) === "text"
            ? dialectSQL({ sqlite: sql`${rendered}`, postgresql: sql`${rendered} COLLATE "C"` })
            : rendered;
    },

    /** Collate a text column by byte in every dialect, leaving other columns as they are. */
    text(expression: Column): SQLWrapper {
        return expression.definition.kind === "text"
            ? dialectSQL({
                  sqlite: sql`${expression}`,
                  postgresql: sql`${expression} COLLATE "C"`,
              })
            : expression;
    },
};

/** Match rows holding a row's value of one key. */
function tie(
    table: Table,
    key: OrderKey,
    row: Readonly<Record<string, unknown>>,
    namespace: Namespace,
): SQL {
    // compare a column as bound through its column, and a computed value as it is
    const value = row[key.column];
    const isComputed = Object.hasOwn(namespace.computed, key.column);
    const expression = isComputed
        ? Order.expression(table, key.column, namespace)
        : Order.column(table, key.column);

    return value === null || value === undefined
        ? sql`${expression} IS NULL`
        : sql`${expression} = ${isComputed ? sql`${value}` : sql.param(value, expression as Column)}`;
}

/** Match rows placed after a row's value of one key, missing values lowest. */
function follow(
    table: Table,
    key: OrderKey,
    row: Readonly<Record<string, unknown>>,
    namespace: Namespace,
): SQL {
    // follow a missing value by every present one when ascending, and by none when descending
    const isComputed = Object.hasOwn(namespace.computed, key.column);
    const expression = isComputed
        ? Expression.render(namespace.computed[key.column]!, table, namespace)
        : Order.column(table, key.column);
    const sorted = Order.expression(table, key.column, namespace);
    const value = row[key.column];
    if (value === null || value === undefined) {
        return key.direction === "asc" ? sql`${expression} IS NOT NULL` : sql`false`;
    }

    // follow a present value by greater ones, or by smaller and missing ones when descending
    const bound = isComputed ? sql`${value}` : sql.param(value, expression as Column);

    return key.direction === "asc"
        ? sql`${sorted} > ${bound}`
        : sql`(${sorted} < ${bound} OR ${expression} IS NULL)`;
}

/** Whether a UTF-16 unit is half of a surrogate pair. */
function isSurrogate(unit: number): boolean {
    return unit >= 0xd800 && unit <= 0xdfff;
}

/** Whether a UTF-16 unit is the second half of a surrogate pair. */
function isLowSurrogate(unit: number): boolean {
    return unit >= 0xdc00 && unit <= 0xdfff;
}
