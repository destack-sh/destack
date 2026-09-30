import * as schema from "../validate/index.ts";
import { Version } from "../version/version.ts";
import type { JsonValue } from "../json/json.ts";
import { Condition, Scalar } from "./condition.ts";

/** The kinds a JSON scalar reads as. */
export type ScalarKind = "integer" | "real" | "text";

/**
 * A value computed from one row, alike in SQL and memory.
 *
 * Arithmetic yields a missing value from a missing operand and from division by zero.
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
      }
    | { readonly kind: "json"; readonly value: JsonValue }
    | { readonly kind: "path"; readonly of: Expression; readonly keys: readonly string[] }
    | { readonly kind: "object"; readonly fields: Readonly<Record<string, Expression>> }
    | {
          readonly kind: "case";
          readonly of: Expression;
          readonly cases: readonly {
              readonly when: string | number;
              readonly then: Expression;
          }[];
          readonly otherwise: Expression;
      }
    | { readonly kind: "scalar"; readonly of: Expression; readonly as: ScalarKind };

/** How a rollup measures related rows. */
export type Rollup = "count" | "sum" | "min" | "max";

/** The related rows an expression reads. */
export interface Related {
    /** Read a column of the related row. */
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
            /** Measure a relation's rows. */
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
        schema.object({
            /** Embed a JSON value. */
            kind: schema.literal("json"),
            /** The value. */
            value: schema.json(),
        }),
        schema.object({
            /** Read the JSON inside a JSON value. */
            kind: schema.literal("path"),
            /** The JSON value. */
            of: expressionSchema,
            /** The object keys leading to the inner value. */
            keys: schema.array(schema.string().min(1)).min(1),
        }),
        schema.object({
            /** Build a JSON object. */
            kind: schema.literal("object"),
            /** The values by key. */
            fields: schema.record(schema.string().min(1), expressionSchema),
        }),
        schema.object({
            /** Map listed values to results. */
            kind: schema.literal("case"),
            /** The value compared. */
            of: expressionSchema,
            /** The results by value, first match first. */
            cases: schema
                .array(
                    schema.object({
                        /** The value matched. */
                        when: schema.union([schema.string(), schema.number().finite()]),
                        /** The result. */
                        then: expressionSchema,
                    }),
                )
                .min(1),
            /** The result for any other value. */
            otherwise: expressionSchema,
        }),
        schema.object({
            /** Read a JSON scalar as a number or text. */
            kind: schema.literal("scalar"),
            /** The JSON value. */
            of: expressionSchema,
            /** The kind read. */
            as: schema.enum(["integer", "real", "text"]),
        }),
    ]),
);

/** A value computed from one row, alike in SQL and memory. */
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
    /** Read a column of the related row. */
    lookup: (via: string, column: string): Expression => ({ kind: "lookup", via, column }),
    /** Measure a relation's rows meeting a condition. */
    rollup: (measure: Rollup, via: string, column?: string, where?: Condition): Expression => ({
        kind: "rollup",
        function: measure,
        via,
        ...(column === undefined ? {} : { column }),
        ...(where === undefined ? {} : { where }),
    }),
    /** Embed a JSON value. */
    json: (value: JsonValue): Expression => ({ kind: "json", value }),
    /** Read the JSON inside a JSON value by object keys, missing when absent. */
    path: (of: Expression, ...keys: string[]): Expression => ({ kind: "path", of, keys }),
    /** Build a JSON object from values by key. */
    object: (fields: Readonly<Record<string, Expression>>): Expression => ({
        kind: "object",
        fields,
    }),
    /** Map listed values to results, and any other value to a fallback result. */
    case: (
        of: Expression,
        cases: readonly { readonly when: string | number; readonly then: Expression }[],
        otherwise: Expression,
    ): Expression => ({ kind: "case", of, cases, otherwise }),
    /** Read a JSON scalar as a number or text. */
    scalar: (of: Expression, as: ScalarKind): Expression => ({ kind: "scalar", of, as }),

    /** Count an expression's terms. */
    terms(expression: Expression): number {
        let count = 0;
        visit(expression, () => {
            count += 1;
        });

        return count;
    },

    /** List the lookups of an expression. */
    lookups(expression: Expression): { readonly via: string; readonly column: string }[] {
        const found: { readonly via: string; readonly column: string }[] = [];
        visit(expression, (entry) => {
            if (entry.kind === "lookup") {
                found.push({ via: entry.via, column: entry.column });
            }
        });

        return found;
    },

    /** List the rollups of an expression. */
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

    /** Assign fields computed from a row's fields as it stands, keeping its other fields. */
    assign(
        assignments: Readonly<Record<string, Expression>>,
        row: Readonly<Record<string, JsonValue>>,
    ): Record<string, JsonValue> {
        const fields = Object.entries(assignments).map(([field, expression]) => [
            field,
            Expression.evaluate(expression, row),
        ]);

        return { ...row, ...Object.fromEntries(fields) };
    },

    /**
     * Upgrade a partial record of an earlier release through each later release's assignments, up to a release.
     *
     * An assignment computing nothing from absent fields leaves its field absent, so partial records stay partial.
     */
    upgrade(
        convert: Readonly<Record<Version, Readonly<Record<string, Expression>>>>,
        record: Readonly<Record<string, JsonValue>>,
        from: Version,
        to: Version,
    ): Record<string, JsonValue> {
        let converted: Record<string, JsonValue> = { ...record };
        for (const release of Version.between(Object.keys(convert), from, to)) {
            // compute each field, skipping nothing computed from absent fields alone
            const fields: Record<string, JsonValue> = {};
            for (const [field, expression] of Object.entries(convert[release]!)) {
                const value = Expression.evaluate(expression, converted);
                const isAbsent = [...Expression.columns(expression)].every(
                    (column) => !(column in converted),
                );
                if (value !== null || !isAbsent) {
                    fields[field] = value;
                }
            }
            converted = Object.assign(converted, fields);
        }

        return converted;
    },

    /** Compute an expression over a row as SQL does. */
    evaluate(
        expression: Expression,
        row: Readonly<Record<string, unknown>>,
        related?: Related,
    ): JsonValue {
        switch (expression.kind) {
            case "json":
                return expression.value;
            case "path": {
                // walk the keys through objects, missing past any other value
                let value = Expression.evaluate(expression.of, row, related);
                for (const key of expression.keys) {
                    const isObject =
                        typeof value === "object" && value !== null && !Array.isArray(value);
                    value = isObject ? ((value as Record<string, JsonValue>)[key] ?? null) : null;
                }

                return value;
            }
            case "object":
                return Object.fromEntries(
                    Object.entries(expression.fields).map(([key, value]) => [
                        key,
                        Expression.evaluate(value, row, related),
                    ]),
                );
            case "case": {
                // take the first matching result, else the fallback
                const value = Expression.evaluate(expression.of, row, related);
                const match = expression.cases.find((entry) => entry.when === value);

                return Expression.evaluate(match?.then ?? expression.otherwise, row, related);
            }
            case "scalar": {
                // read a JSON string as text and a JSON number as a number, anything else as missing
                const value = Expression.evaluate(expression.of, row, related);
                const isRead =
                    expression.as === "text"
                        ? typeof value === "string"
                        : typeof value === "number";

                return isRead ? value : null;
            }
            case "column": {
                const value = row[expression.name];

                return value === undefined ? null : (value as JsonValue);
            }
            case "literal":
                return expression.value;
            case "add":
            case "subtract":
            case "multiply":
            case "divide": {
                // yield a missing value from a missing operand or division by zero
                const left = Expression.evaluate(expression.left, row, related) as Scalar;
                const right = Expression.evaluate(expression.right, row, related) as Scalar;
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
                // read the resolved related rows
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

/** Visit an expression and its subexpressions. */
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
    } else if (expression.kind === "path" || expression.kind === "scalar") {
        visit(expression.of, visitor);
    } else if (expression.kind === "object") {
        for (const value of Object.values(expression.fields)) {
            visit(value, visitor);
        }
    } else if (expression.kind === "case") {
        visit(expression.of, visitor);
        for (const entry of expression.cases) {
            visit(entry.then, visitor);
        }
        visit(expression.otherwise, visitor);
    }
}
