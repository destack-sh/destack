import { sql, type SQL } from "drizzle-orm";
import { schema } from "@destack/schema";
import { dialectSQL } from "../dialect/expression.ts";
import { DatabaseError } from "../error/error.ts";
import { TABLE, type Table } from "../table/table.ts";
import { Condition, Scalar } from "./condition.ts";
import type { Namespace } from "./namespace.ts";

/** The column kinds arithmetic reads, exact in JavaScript numbers and both dialects. */
const NUMERIC_KINDS: ReadonlySet<string> = new Set(["integer", "real"]);

/** The most terms one expression holds, bounding what clients compute per row. */
const EXPRESSION_TERMS = 64;

/**
 * A value computed from one row's columns, which SQL and memory compute alike.
 *
 * Arithmetic reads numbers and yields a missing value from a missing one; division is real division, missing on zero.
 */
export type Expression =
    | { readonly kind: "column"; readonly name: string }
    | { readonly kind: "literal"; readonly value: Scalar }
    | {
          readonly kind: "add" | "subtract" | "multiply" | "divide";
          readonly left: Expression;
          readonly right: Expression;
      }
    | { readonly kind: "coalesce"; readonly values: readonly Expression[] }
    | { readonly kind: "lookup"; readonly via: string; readonly column: string }
    | {
          readonly kind: "rollup";
          readonly function: Rollup;
          readonly via: string;
          readonly column?: string;
          readonly where?: Condition;
      };

/** How a rollup measures the related rows: their count, or the sum, least or greatest of a column. */
export type Rollup = "count" | "sum" | "min" | "max";

/** How an expression reads related rows: the one row a lookup relates, and the rows a rollup measures. */
export interface Related {
    /** Read a column of the one related row, missing without one. */
    lookup(via: string, column: string): Scalar;
    /** Measure the related rows meeting a condition. */
    rollup(
        measure: Rollup,
        via: string,
        column: string | undefined,
        where: Condition | undefined,
    ): Scalar;
}

/** The schema of an expression. */
const expressionSchema: schema.Schema<Expression> = schema.lazy(() =>
    schema.discriminatedUnion("kind", [
        schema.object({
            /** Read a column. */
            kind: schema.literal("column"),
            /** The column's property. */
            name: schema.string().min(1),
        }),
        schema.object({
            /** Embed a value. */
            kind: schema.literal("literal"),
            /** The value. */
            value: Scalar,
        }),
        schema.object({
            /** Combine two numbers. */
            kind: schema.enum(["add", "subtract", "multiply", "divide"]),
            /** The left operand. */
            left: expressionSchema,
            /** The right operand. */
            right: expressionSchema,
        }),
        schema.object({
            /** Take the first present value. */
            kind: schema.literal("coalesce"),
            /** The values in order. */
            values: schema.array(expressionSchema).min(1),
        }),
        schema.object({
            /** Read a column of the one row a relation relates. */
            kind: schema.literal("lookup"),
            /** The relation, by the name its host declares. */
            via: schema.string().min(1),
            /** The related row's column, by property. */
            column: schema.string().min(1),
        }),
        schema.object({
            /** Measure the rows a relation relates. */
            kind: schema.literal("rollup"),
            /** The measure. */
            function: schema.enum(["count", "sum", "min", "max"]),
            /** The relation, by the name its host declares. */
            via: schema.string().min(1),
            /** The measured column of the related rows, absent for a count. */
            column: schema.string().min(1).optional(),
            /** The condition the measured rows meet. */
            where: schema.lazy(() => Condition.schema).optional(),
        }),
    ]),
);

/** A value computed from one row's columns, which SQL and memory compute alike. */
export const Expression = {
    /** The schema of an expression. */
    schema: expressionSchema,

    /** Read a column. */
    column: (name: string): Expression => ({ kind: "column", name }),
    /** Embed a value. */
    literal: (value: Scalar): Expression => ({ kind: "literal", value }),
    /** Add two numbers. */
    add: (left: Expression, right: Expression): Expression => ({
        kind: "add",
        left,
        right,
    }),
    /** Subtract a number from another. */
    subtract: (left: Expression, right: Expression): Expression => ({
        kind: "subtract",
        left,
        right,
    }),
    /** Multiply two numbers. */
    multiply: (left: Expression, right: Expression): Expression => ({
        kind: "multiply",
        left,
        right,
    }),
    /** Divide a number by another, missing on zero. */
    divide: (left: Expression, right: Expression): Expression => ({
        kind: "divide",
        left,
        right,
    }),
    /** Take the first present value. */
    coalesce: (...values: Expression[]): Expression => ({
        kind: "coalesce",
        values,
    }),
    /** Read a column of the one row a relation relates, missing without one. */
    lookup: (via: string, column: string): Expression => ({ kind: "lookup", via, column }),
    /** Measure the rows a relation relates that meet a condition: count them, or sum, or take the least or greatest of a column. */
    rollup: (measure: Rollup, via: string, column?: string, where?: Condition): Expression => ({
        kind: "rollup",
        function: measure,
        via,
        ...(column === undefined ? {} : { column }),
        ...(where === undefined ? {} : { where }),
    }),

    /** Require an expression of bounded size over numeric or text columns of a table and its relations, arithmetic over numbers. */
    require(expression: Expression, table: Table, namespace: Namespace = { computed: {} }): void {
        // bound the expression, which clients choose
        if (terms(expression) > EXPRESSION_TERMS) {
            throw new DatabaseError(
                "INVALID_QUERY",
                `expression holds more than ${EXPRESSION_TERMS} terms`,
            );
        }

        // require arithmetic over numbers
        const kind = kindOf(expression, table, namespace);
        if (kind === undefined) {
            throw new DatabaseError("INVALID_QUERY", "expression mixes numbers with other values");
        }
    },

    /** Read the kind of value a required expression yields: an integer, a real number or text. */
    kind(
        expression: Expression,
        table: Table,
        namespace: Namespace = { computed: {} },
    ): "integer" | "real" | "text" {
        // read an expression yielding only null as real, since it holds no value to compare
        const kind = kindOf(expression, table, namespace);
        if (kind === undefined) {
            throw new DatabaseError("INVALID_QUERY", "expression mixes numbers with other values");
        }

        return kind === "null" ? "real" : kind;
    },

    /** List the relations an expression looks up, each with the column it reads. */
    lookups(expression: Expression): { readonly via: string; readonly column: string }[] {
        const found: { readonly via: string; readonly column: string }[] = [];
        visit(expression, (entry) => {
            if (entry.kind === "lookup") {
                found.push({ via: entry.via, column: entry.column });
            }
        });

        return found;
    },

    /** List the rollups an expression holds, each measuring a relation's rows that meet its condition. */
    rollups(expression: Expression): Extract<Expression, { readonly kind: "rollup" }>[] {
        const found: Extract<Expression, { readonly kind: "rollup" }>[] = [];
        visit(expression, (entry) => {
            if (entry.kind === "rollup") {
                found.push(entry);
            }
        });

        return found;
    },

    /** List the columns an expression reads. */
    columns(expression: Expression): Set<string> {
        const names = new Set<string>();
        visit(expression, (entry) => {
            if (entry.kind === "column") {
                names.add(entry.name);
            }
        });

        return names;
    },

    /** Render an expression as SQL over a table's columns, numbers as 64-bit floats. */
    render(expression: Expression, table: Table, namespace: Namespace = { computed: {} }): SQL {
        return renderAs(
            expression,
            Expression.kind(expression, table, namespace),
            table,
            namespace,
        );
    },

    /** Compute an expression over a row of application values as SQL does, reading related rows through the host. */
    evaluate(
        expression: Expression,
        row: Readonly<Record<string, unknown>>,
        related?: Related,
    ): Scalar {
        switch (expression.kind) {
            case "column": {
                const value = row[expression.name];

                return value === undefined ? null : (value as Scalar);
            }
            case "literal":
                return expression.value;
            case "add":
            case "subtract":
            case "multiply":
            case "divide": {
                // yield a missing value from a missing operand, and from division by zero
                const left = Expression.evaluate(expression.left, row, related);
                const right = Expression.evaluate(expression.right, row, related);
                if (left === null || right === null) {
                    return null;
                }
                const [first, second] = [Number(left), Number(right)];

                return expression.kind === "add"
                    ? first + second
                    : expression.kind === "subtract"
                      ? first - second
                      : expression.kind === "multiply"
                        ? first * second
                        : second === 0
                          ? null
                          : first / second;
            }
            case "coalesce":
                return (
                    expression.values
                        .map((value) => Expression.evaluate(value, row, related))
                        .find((value) => value !== null) ?? null
                );
            case "lookup":
            case "rollup":
                // read the related rows the host resolved
                if (related === undefined) {
                    throw new TypeError(`expression reads ${expression.via} without related rows`);
                }

                return expression.kind === "lookup"
                    ? related.lookup(expression.via, expression.column)
                    : related.rollup(
                          expression.function,
                          expression.via,
                          expression.column,
                          expression.where,
                      );
        }
    },
};

/** Render an expression as SQL yielding a kind, numbers as 64-bit floats. */
function renderAs(
    expression: Expression,
    kind: "integer" | "real" | "text",
    table: Table,
    namespace: Namespace,
): SQL {
    switch (expression.kind) {
        case "column": {
            // read numbers as floats, as JavaScript holds them
            const column = table[TABLE].columns[expression.name]!;

            return kind === "text" ? sql`${column}` : float(sql`${column}`);
        }
        case "literal":
            // type literals by the kind they yield
            return kind === "text"
                ? sql`CAST(${expression.value} AS TEXT)`
                : float(sql`${expression.value}`);
        case "add":
        case "subtract":
        case "multiply": {
            const operator = { add: "+", subtract: "-", multiply: "*" }[expression.kind];
            const left = renderAs(expression.left, "real", table, namespace);
            const right = renderAs(expression.right, "real", table, namespace);

            return sql`(${left} ${sql.raw(operator)} ${right})`;
        }
        case "divide": {
            // divide, missing on zero
            const left = renderAs(expression.left, "real", table, namespace);
            const right = renderAs(expression.right, "real", table, namespace);

            return sql`(${left} / NULLIF(${right}, 0))`;
        }
        case "coalesce":
            return sql`COALESCE(${sql.join(
                expression.values.map((value) => renderAs(value, kind, table, namespace)),
                sql`, `,
            )})`;
        case "lookup": {
            // read the related row's value, numbers as floats
            const value = sql`${namespace.lookup!(expression.via, expression.column).value}`;

            return kind === "text" ? value : float(value);
        }
        case "rollup": {
            // measure the related rows, numbers as floats
            const value = sql`${
                namespace.rollup!(
                    expression.function,
                    expression.via,
                    expression.column,
                    expression.where,
                ).value
            }`;

            return kind === "text" ? value : float(value);
        }
    }
}

/** Read the kind of value an expression yields, absent when it mixes numbers with other values or reads other columns. */
function kindOf(
    expression: Expression,
    table: Table,
    namespace: Namespace,
): "integer" | "real" | "text" | "null" | undefined {
    switch (expression.kind) {
        case "column": {
            const column = table[TABLE].columns[expression.name];
            if (column === undefined) {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    `${table[TABLE].name} has no column ${expression.name}`,
                );
            }

            return NUMERIC_KINDS.has(column.definition.kind) || column.definition.kind === "text"
                ? (column.definition.kind as "integer" | "real" | "text")
                : undefined;
        }
        case "literal":
            return typeof expression.value === "number"
                ? Number.isInteger(expression.value)
                    ? "integer"
                    : "real"
                : typeof expression.value === "string"
                  ? "text"
                  : expression.value === null
                    ? "null"
                    : undefined;
        case "add":
        case "subtract":
        case "multiply":
        case "divide": {
            // yield integers from integers, except by division, and reals from any other numbers
            const kinds = new Set([
                kindOf(expression.left, table, namespace),
                kindOf(expression.right, table, namespace),
            ]);
            kinds.delete("null");

            return kinds.has(undefined) || kinds.has("text")
                ? undefined
                : kinds.has("real") || expression.kind === "divide"
                  ? "real"
                  : "integer";
        }
        case "coalesce": {
            // take the one kind the present values share, reals over integers
            const kinds = new Set(
                expression.values.map((value) => kindOf(value, table, namespace)),
            );
            kinds.delete("null");
            if (kinds.has("integer") && kinds.has("real")) {
                kinds.delete("integer");
            }

            return kinds.has(undefined) || kinds.size > 1 ? undefined : ([...kinds][0] ?? "null");
        }
        case "rollup": {
            // count to an integer, sum integers, and take the least or greatest of numbers or text
            if (namespace.rollup === undefined) {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    `expression measures relation ${expression.via}, which ${table[TABLE].name} does not declare`,
                );
            }
            const { definition } = namespace.rollup(
                expression.function,
                expression.via,
                expression.column,
                expression.where,
            );
            // count to an integer
            if (expression.function === "count") {
                return "integer";
            }
            // measure a column otherwise
            else if (definition === undefined) {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    `rollup ${expression.function} of ${expression.via} measures a column`,
                );
            }
            // sum integers only, which tallies add and remove exactly
            else if (expression.function === "sum") {
                if (definition.kind !== "integer") {
                    throw new DatabaseError(
                        "INVALID_QUERY",
                        `rollup sum of ${expression.via} measures ${definition.kind} values; sums take integers`,
                    );
                }

                return "integer";
            }

            return NUMERIC_KINDS.has(definition.kind) || definition.kind === "text"
                ? (definition.kind as "integer" | "real" | "text")
                : undefined;
        }
        case "lookup": {
            // read the related column's kind, which the host declares
            if (namespace.lookup === undefined) {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    `expression looks up relation ${expression.via}, which ${table[TABLE].name} does not declare`,
                );
            }
            const definition = namespace.lookup(expression.via, expression.column).definition;

            return NUMERIC_KINDS.has(definition.kind) || definition.kind === "text"
                ? (definition.kind as "integer" | "real" | "text")
                : undefined;
        }
    }
}

/** Cast a number to a 64-bit float in every dialect. */
function float(value: SQL): SQL {
    return dialectSQL({
        sqlite: sql`CAST(${value} AS REAL)`,
        postgresql: sql`CAST(${value} AS DOUBLE PRECISION)`,
    }) as SQL;
}

/** Count an expression's terms. */
function terms(expression: Expression): number {
    let count = 0;
    visit(expression, () => {
        count += 1;
    });

    return count;
}

/** Visit an expression and every expression within it. */
function visit(expression: Expression, visitor: (entry: Expression) => void): void {
    visitor(expression);
    if (
        expression.kind === "add" ||
        expression.kind === "subtract" ||
        expression.kind === "multiply" ||
        expression.kind === "divide"
    ) {
        visit(expression.left, visitor);
        visit(expression.right, visitor);
    } else if (expression.kind === "coalesce") {
        for (const value of expression.values) {
            visit(value, visitor);
        }
    }
}
