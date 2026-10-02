import { and, dialectSQL, or, sql, type SQL, type SQLWrapper } from "../sql/index.ts";
import { defineSchema, schema } from "@destack/schema";
import { Column, type ColumnKind } from "../table/column.ts";
import { TABLE, type Table } from "../table/table.ts";
import { Key } from "./key.ts";
import { Expression } from "../expression/expression.ts";
import type { Namespace } from "./namespace.ts";
import { DatabaseError } from "../error/error.ts";

/** The column kinds that order alike in SQLite, PostgreSQL and memory. */
const ORDERED_KINDS: ReadonlySet<ColumnKind> = new Set<ColumnKind>([
    "text",
    "integer",
    "real",
    "boolean",
    "bigint",
]);

/** The most keys one order has, which every keyset cursor carries; orders use one to four. */
const ORDER_KEYS = 16;

/** One key of an order. */
export const OrderKey = defineSchema(
    schema.object({
        /** The column's property. */
        column: schema.string().min(1),
        /** The direction; missing values sort lowest either way. */
        direction: schema.enum(["asc", "desc"]),
    }),
);
/** One key of an order. */
export type OrderKey = schema.Infer<typeof OrderKey>;

/** How rows sort: missing values lowest, text by UTF-8 bytes. */
export type Order = readonly OrderKey[];

/** How rows sort: missing values lowest, text by UTF-8 bytes. */
export const Order = {
    /** The schema of an order. */
    schema: defineSchema(schema.array(OrderKey)),

    /** Require a bounded order of distinct orderable keys. */
    require(order: Order, table: Table, namespace: Namespace = { computed: {} }): void {
        // bound the order
        const columns = new Set(order.map((key) => key.column));
        if (order.length > ORDER_KEYS || columns.size < order.length) {
            throw new DatabaseError(
                "INVALID_QUERY",
                `order has more than ${ORDER_KEYS} keys, or one column twice`,
            );
        }

        // require orderable columns or computed values
        for (const key of order) {
            if (!Object.hasOwn(namespace.computed, key.column)) {
                Order.column(table, key.column);
            }
        }
    },

    /** Read an orderable column. */
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

    /** Complete an order with the primary key. */
    complete(order: Order, table: Table): Order {
        const keys = table[TABLE].key.filter((name) => !order.some((key) => key.column === name));

        return [...order, ...keys.map((name) => ({ column: name, direction: "asc" as const }))];
    },

    /** Render an order as ORDER BY expressions. */
    render(order: Order, table: Table, namespace: Namespace = { computed: {} }): SQL[] {
        return order.map((key) => {
            const expression = Order.expression(table, key.column, namespace);

            return key.direction === "asc"
                ? sql`${expression} ASC NULLS FIRST`
                : sql`${expression} DESC NULLS LAST`;
        });
    },

    /** Select the rows after a row in an order. */
    after(
        order: Order,
        table: Table,
        row: Readonly<Record<string, unknown>>,
        namespace: Namespace = { computed: {} },
    ): SQL {
        // tie on the leading keys and follow on the next
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

    /** Merge ordered rows of several tables with their list names. */
    merge<Listed extends Readonly<Record<string, unknown>>>(
        order: Order,
        lists: readonly {
            readonly name: string;
            readonly table: Table;
            readonly rows: readonly Listed[];
        }[],
        limit: number,
    ): { readonly name: string; readonly row: Listed }[] {
        // take every row with its list name and key
        const entries = lists.flatMap(({ name, table, rows }) =>
            rows.map((row) => ({ name, row, key: Key.name(table, Key.of(table, row)) })),
        );

        // sort and keep the first
        entries.sort(
            (left, right) =>
                Order.rows(order, left.row, right.row) ||
                Order.codePoints(left.name, right.name) ||
                Order.codePoints(left.key, right.key),
        );

        return entries.slice(0, limit).map(({ name, row }) => ({ name, row }));
    },

    /** Compare two values with missing values lowest. */
    missingFirst(left: unknown, right: unknown): number {
        // place missing values first
        const isLeftMissing = left === null || left === undefined;
        const isRightMissing = right === null || right === undefined;
        if (isLeftMissing || isRightMissing) {
            return Number(isRightMissing) - Number(isLeftMissing);
        }

        // order present values of one kind
        const compared = Order.values(left, right);
        if (compared === undefined) {
            throw new DatabaseError("INVALID_QUERY", "values of different kinds have no order");
        }

        return compared;
    },

    /** Compare two present values as SQL does, absent for missing or mixed values. */
    values(left: unknown, right: unknown): number | undefined {
        // leave missing values unordered
        if (left === null || left === undefined || right === null || right === undefined) {
            return undefined;
        }

        // order text by code point, and numbers and booleans by value
        if (typeof left === "string" && typeof right === "string") {
            return Order.codePoints(left, right);
        } else if (
            (typeof left === "number" || typeof left === "bigint") &&
            (typeof right === "number" || typeof right === "bigint")
        ) {
            return left < right ? -1 : left > right ? 1 : 0;
        } else if (typeof left === "boolean" && typeof right === "boolean") {
            return Number(left) - Number(right);
        }

        return undefined;
    },

    /** Compare text by code point, the UTF-8 byte order. */
    codePoints(left: string, right: string): number {
        // find the first differing UTF-16 unit
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

        // compare the code points around a surrogate
        const start =
            index > 0 && (isLowSurrogate(first) || isLowSurrogate(second)) ? index - 1 : index;
        const leftPoint = codePoint(left, start);
        const rightPoint = codePoint(right, start);
        if (leftPoint === rightPoint) {
            return first < second ? -1 : 1;
        }

        return leftPoint < rightPoint ? -1 : 1;
    },

    /** Render a column or computed value to sort by, text by byte. */
    expression(table: Table, name: string, namespace: Namespace = { computed: {} }): SQLWrapper {
        // render a column or computed value, collating text by byte
        const expression = namespace.computed[name];
        if (expression === undefined) {
            return Order.text(Order.column(table, name));
        }
        const rendered = Expression.render(expression, table, namespace);

        return Expression.kind(expression, table, namespace) === "text"
            ? dialectSQL({ sqlite: sql`${rendered}`, postgresql: sql`${rendered} COLLATE "C"` })
            : rendered;
    },

    /** Collate a text column by byte. */
    text(column: Column): SQLWrapper {
        return column.definition.kind === "text"
            ? dialectSQL({
                  sqlite: sql`${column}`,
                  postgresql: sql`${column} COLLATE "C"`,
              })
            : column;
    },
};

/** Match rows tying a row's value of one key. */
function tie(
    table: Table,
    key: OrderKey,
    row: Readonly<Record<string, unknown>>,
    namespace: Namespace,
): SQL {
    // match a missing value, a computed one as it is, and a column value bound through its column
    const value = row[key.column];
    if (Object.hasOwn(namespace.computed, key.column)) {
        const expression = Order.expression(table, key.column, namespace);

        return value === null || value === undefined
            ? sql`${expression} IS NULL`
            : sql`${expression} = ${value}`;
    }
    const column = Order.column(table, key.column);

    return value === null || value === undefined
        ? sql`${column} IS NULL`
        : sql`${column} = ${sql.param(value, column)}`;
}

/** Match rows after a row's value of one key. */
function follow(
    table: Table,
    key: OrderKey,
    row: Readonly<Record<string, unknown>>,
    namespace: Namespace,
): SQL {
    // follow a missing value by every present one when ascending
    const computed = namespace.computed[key.column];
    const column = computed === undefined ? Order.column(table, key.column) : undefined;
    const expression =
        computed === undefined ? column : Expression.render(computed, table, namespace);
    const sorted = Order.expression(table, key.column, namespace);
    const value = row[key.column];
    if (value === null || value === undefined) {
        return key.direction === "asc" ? sql`${expression} IS NOT NULL` : sql`false`;
    }

    // follow a present value by greater ones, or smaller and missing ones when descending
    const bound = column === undefined ? sql`${value}` : sql.param(value, column);

    return key.direction === "asc"
        ? sql`${sorted} > ${bound}`
        : sql`(${sorted} < ${bound} OR ${expression} IS NULL)`;
}

/** Read the code point at a position the comparison found inside the text. */
function codePoint(text: string, index: number): number {
    const point = text.codePointAt(index);
    if (point === undefined) {
        throw new RangeError(`no code point at ${index} of a ${text.length} unit text`);
    }

    return point;
}

/** Report whether a UTF-16 unit is half of a surrogate pair. */
function isSurrogate(unit: number): boolean {
    return unit >= 0xd800 && unit <= 0xdfff;
}

/** Report whether a UTF-16 unit is the second half of a surrogate pair. */
function isLowSurrogate(unit: number): boolean {
    return unit >= 0xdc00 && unit <= 0xdfff;
}
