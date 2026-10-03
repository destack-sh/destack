import { dialectSQL, sql, type SQL } from "../sql/index.ts";
import { DatabaseError } from "../error/error.ts";
import { TABLE, type Table } from "../table/table.ts";
import type { ColumnKind, ColumnValue } from "../table/column.ts";
import type { Namespace } from "../query/namespace.ts";
import { JsonValue, schema, Version } from "@destack/schema";
import { Condition, Scalar, type JsonCondition } from "../query/condition.ts";

/** The most terms one expression has, bounding compiled SQL; computed fields use under ten. */
const EXPRESSION_TERMS = 64;

/** The kind an expression reads from a column of each kind it can compute over. */
const KINDS: Readonly<Partial<Record<ColumnKind, Kind>>> = {
    integer: "integer",
    real: "real",
    text: "text",
    json: "json",
};

/** The kind a related column yields to a lookup or rollup, for the scalar kinds. */
const SCALAR_KINDS: Readonly<Partial<Record<ColumnKind, ScalarKind>>> = {
    integer: "integer",
    real: "real",
    text: "text",
};

/** A selected text value. */
const SELECTED_TEXT = schema.string();

/** A selected number, as drivers return it: a number, an exact integer or PostgreSQL's decimal text. */
const SELECTED_NUMBER = schema
    .union([schema.number(), schema.bigint(), schema.string()])
    .transform((value) => Number(value))
    .pipe(schema.number());

/** The kinds an expression yields. */
type Kind = "integer" | "real" | "text" | "json";

/** The kinds a JSON scalar reads as. */
export const ScalarKind = schema.enum(["integer", "real", "text"]);
/** The kinds a JSON scalar reads as. */
export type ScalarKind = schema.Infer<typeof ScalarKind>;

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
          readonly where: JsonCondition;
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
export const Rollup = schema.enum(["count", "sum", "min", "max"]);
/** How a rollup measures related rows. */
export type Rollup = schema.Infer<typeof Rollup>;

/** The related rows an expression reads. */
export interface Related {
    /** Read a column of the related row, as rows hold it. */
    lookup(via: string, column: string): ColumnValue;
    /** Measure the related rows meeting a condition. */
    rollup(measure: Rollup, via: string, column: string | undefined, where: Condition): ColumnValue;
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
            function: Rollup,
            /** The relation, by the name its host declares. */
            via: schema.string().min(1),
            /** The measured column of the related rows, absent for a count. */
            column: schema.string().min(1).exactOptional(),
            /** The condition the measured rows meet. */
            where: schema.lazy(() => Condition.schema),
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
                        when: schema.union([schema.string(), schema.number()]),
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
            as: ScalarKind,
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
    rollup: (
        measure: Rollup,
        via: string,
        column?: string,
        where: JsonCondition = {},
    ): Expression => ({
        kind: "rollup",
        function: measure,
        via,
        ...(column === undefined ? {} : { column }),
        where,
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
        const converted: Record<string, JsonValue> = { ...record };
        for (const [, assignments] of Version.between(convert, from, to)) {
            // compute each field, skipping nothing computed from absent fields alone
            const fields: Record<string, JsonValue> = {};
            for (const [field, expression] of Object.entries(assignments)) {
                const value = Expression.evaluate(expression, converted);
                const isAbsent = [...Expression.columns(expression)].every(
                    (column) => !(column in converted),
                );
                if (value !== null || !isAbsent) {
                    fields[field] = value;
                }
            }
            Object.assign(converted, fields);
        }

        return converted;
    },

    /** Compute an expression over a row as SQL does. */
    evaluate(
        expression: Expression,
        row: Readonly<Record<string, ColumnValue>>,
        related?: Related,
    ): JsonValue {
        switch (expression.kind) {
            case "json":
                return expression.value;
            case "path": {
                // walk the keys through objects, missing past any other value
                let value = Expression.evaluate(expression.of, row, related);
                for (const key of expression.keys) {
                    value = JsonValue.isObject(value) ? (value[key] ?? null) : null;
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
            case "column":
                return jsonOf(row[expression.name], expression.name);
            case "literal":
                return expression.value;
            case "add":
            case "subtract":
            case "multiply":
            case "divide": {
                // yield a missing value from a missing or structured operand, or division by zero
                const left = Expression.evaluate(expression.left, row, related);
                const right = Expression.evaluate(expression.right, row, related);
                if (typeof left === "object" || typeof right === "object") {
                    return null;
                }

                return arithmetic(expression.kind, Number(left), Number(right));
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
                    ? jsonOf(related.lookup(expression.via, expression.column), expression.column)
                    : jsonOf(
                          related.rollup(
                              expression.function,
                              expression.via,
                              expression.column,
                              expression.where,
                          ),
                          expression.column ?? expression.via,
                      );
        }
    },

    /** Require a bounded expression over numeric or text columns. */
    require(expression: Expression, table: Table, namespace: Namespace = { extras: {} }): void {
        // bound the size
        if (Expression.terms(expression) > EXPRESSION_TERMS) {
            throw new DatabaseError(
                "INVALID_QUERY",
                `expression has more than ${EXPRESSION_TERMS} terms`,
            );
        }

        // require arithmetic over numbers, and a number or text result
        const kind = kindOf(expression, table, namespace);
        if (kind === undefined) {
            throw new DatabaseError("INVALID_QUERY", "expression mixes numbers with other values");
        } else if (kind === "json") {
            throw new DatabaseError("INVALID_QUERY", "expression yields JSON; read a scalar of it");
        }
    },

    /** Read the kind an expression yields. */
    kind(expression: Expression, table: Table, namespace: Namespace = { extras: {} }): Kind {
        // read an all-null expression as real
        const kind = kindOf(expression, table, namespace);
        if (kind === undefined) {
            throw new DatabaseError("INVALID_QUERY", "expression mixes numbers with other values");
        }

        return kind === "null" ? "real" : kind;
    },

    /** Render an expression as SQL, numbers as 64-bit floats. */
    render(expression: Expression, table: Table, namespace: Namespace = { extras: {} }): SQL {
        return renderAs(
            expression,
            Expression.kind(expression, table, namespace),
            table,
            namespace,
        );
    },

    /** Render a scalar expression for a selection, read back as text or a number by its kind. */
    select(expression: Expression, table: Table, namespace: Namespace = { extras: {} }): SQL {
        const rendered = Expression.render(expression, table, namespace);

        return Expression.kind(expression, table, namespace) === "text"
            ? rendered.mapWith((value) => SELECTED_TEXT.parse(value))
            : rendered.mapWith((value) => SELECTED_NUMBER.parse(value));
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

/** Render an expression as SQL of a kind. */
function renderAs(expression: Expression, kind: Kind, table: Table, namespace: Namespace): SQL {
    switch (expression.kind) {
        case "json":
            return json(sql`${JSON.stringify(expression.value)}`);
        case "path": {
            // follow the keys, missing past a missing key
            const of = renderAs(expression.of, "json", table, namespace);
            const keys = expression.keys;

            return dialectSQL({
                sqlite: sql`(${of} -> ${`$${keys.map((key) => `.${JSON.stringify(key)}`).join("")}`})`,
                postgresql: sql`(${of} #> CAST(${`{${keys.map((key) => JSON.stringify(key)).join(",")}}`} AS TEXT[]))`,
            });
        }
        case "object": {
            // pair each key with its value as JSON
            const pairs = Object.entries(expression.fields).map(([key, value]) => {
                const inner = requireKind(kindOf(value, table, namespace));
                const rendered = renderAs(
                    value,
                    inner === "null" ? "text" : inner,
                    table,
                    namespace,
                );

                return sql`CAST(${key} AS TEXT), ${rendered}`;
            });

            return dialectSQL({
                sqlite: sql`json_object(${sql.join(pairs, sql`, `)})`,
                postgresql: sql`jsonb_build_object(${sql.join(pairs, sql`, `)})`,
            });
        }
        case "case": {
            // compare the value with each listed one by the listed one's kind
            const inner = requireKind(kindOf(expression.of, table, namespace));
            if (inner === "json") {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    "expression compares JSON in a case; read a scalar of it",
                );
            }
            const compared = inner === "null" ? "text" : inner;
            const of = renderAs(expression.of, compared, table, namespace);
            const branches = expression.cases.map(
                (entry) =>
                    sql` WHEN ${renderAs(Expression.literal(entry.when), compared, table, namespace)} THEN ${renderAs(entry.then, kind, table, namespace)}`,
            );
            const otherwise = renderAs(expression.otherwise, kind, table, namespace);

            return sql`(CASE ${of}${sql.join(branches, sql``)} ELSE ${otherwise} END)`;
        }
        case "scalar": {
            // read a JSON string as text and a JSON number as a number, anything else as missing
            const of = renderAs(expression.of, "json", table, namespace);
            const isText = expression.as === "text";
            const value = dialectSQL({
                sqlite: isText
                    ? sql`(CASE json_type(${of}) WHEN 'text' THEN (${of} ->> '$') END)`
                    : sql`(CASE json_type(${of}) WHEN 'integer' THEN (${of} ->> '$') WHEN 'real' THEN (${of} ->> '$') END)`,
                postgresql: isText
                    ? sql`(CASE jsonb_typeof(${of}) WHEN 'string' THEN (${of} #>> '{}') END)`
                    : sql`(CASE jsonb_typeof(${of}) WHEN 'number' THEN CAST((${of} #>> '{}') AS DOUBLE PRECISION) END)`,
            });

            return isText ? value : float(value);
        }
        case "column": {
            // read text as it is
            const column = table[TABLE].column(expression.name);
            if (kind === "text") {
                return sql`${column}`;
            }
            // read JSON as JSON
            else if (kind === "json") {
                return json(sql`${column}`);
            }
            // read numbers as floats
            else {
                return float(sql`${column}`);
            }
        }
        case "literal": {
            // type text literals as text
            if (kind === "text") {
                return sql`CAST(${expression.value} AS TEXT)`;
            }
            // type JSON literals as JSON
            else if (kind === "json") {
                return json(sql`${JSON.stringify(expression.value)}`);
            }
            // type numeric literals as floats
            else {
                return float(sql`${expression.value}`);
            }
        }
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
            // read the related value
            const value = sql`${requireRelation(namespace.lookup, "looks up", expression.via, table)(expression.via, expression.column).value}`;

            return kind === "text" ? value : float(value);
        }
        case "rollup": {
            // measure the related rows
            const rollup = requireRelation(namespace.rollup, "measures", expression.via, table);
            const value = sql`${rollup(expression.function, expression.via, expression.column, expression.where).value}`;

            return kind === "text" ? value : float(value);
        }
    }
}

/** Read the kind an expression yields, absent when kinds mix or columns are unknown. */
function kindOf(
    expression: Expression,
    table: Table,
    namespace: Namespace,
): Kind | "null" | undefined {
    switch (expression.kind) {
        case "json":
            return "json";
        case "path":
        case "scalar":
            // read inside JSON only
            if (kindOf(expression.of, table, namespace) !== "json") {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    `expression reads a ${expression.kind} of a value that is not JSON`,
                );
            }

            return expression.kind === "path" ? "json" : expression.as;
        case "object": {
            // require every field of a known kind
            const kinds = Object.values(expression.fields).map((value) =>
                kindOf(value, table, namespace),
            );

            return kinds.includes(undefined) ? undefined : "json";
        }
        case "case": {
            // compare numbers or text, and take the results' shared kind, reals over integers
            const compared = kindOf(expression.of, table, namespace);
            if (compared === "json") {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    "expression compares JSON in a case; read a scalar of it",
                );
            } else if (compared === undefined) {
                return undefined;
            }
            const kinds = new Set(
                [...expression.cases.map((entry) => entry.then), expression.otherwise].map(
                    (value) => kindOf(value, table, namespace),
                ),
            );
            kinds.delete("null");
            if (kinds.has("integer") && kinds.has("real")) {
                kinds.delete("integer");
            }

            return kinds.has(undefined) || kinds.size > 1 ? undefined : ([...kinds][0] ?? "null");
        }
        case "column": {
            const column = table[TABLE].columns[expression.name];
            if (column === undefined) {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    `${table[TABLE].name} has no column ${expression.name}`,
                );
            }

            return KINDS[column.definition.kind];
        }
        case "literal":
            return literalKind(expression.value);
        case "add":
        case "subtract":
        case "multiply":
        case "divide": {
            // yield integers from integers except by division, and reals otherwise
            const kinds = new Set([
                kindOf(expression.left, table, namespace),
                kindOf(expression.right, table, namespace),
            ]);
            kinds.delete("null");

            // refuse operands without a numeric kind
            if (kinds.has(undefined) || kinds.has("text") || kinds.has("json")) {
                return undefined;
            }
            // yield reals from a real operand or a division
            else if (kinds.has("real") || expression.kind === "divide") {
                return "real";
            }
            // yield integers otherwise
            else {
                return "integer";
            }
        }
        case "coalesce": {
            // take the shared kind, reals over integers
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
            // read the kind of a rollup
            const rollup = requireRelation(namespace.rollup, "measures", expression.via, table);
            const { definition } = rollup(
                expression.function,
                expression.via,
                expression.column,
                expression.where,
            );
            // count to an integer
            if (expression.function === "count") {
                return "integer";
            }
            // read the measured column
            else if (definition === undefined) {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    `rollup ${expression.function} of ${expression.via} measures a column`,
                );
            }
            // sum integers only
            else if (expression.function === "sum") {
                if (definition.kind !== "integer") {
                    throw new DatabaseError(
                        "INVALID_QUERY",
                        `rollup sum of ${expression.via} measures ${definition.kind} values; sums take integers`,
                    );
                }

                return "integer";
            }

            return SCALAR_KINDS[definition.kind];
        }
        case "lookup": {
            // read the related column's kind
            const lookup = requireRelation(namespace.lookup, "looks up", expression.via, table);

            return SCALAR_KINDS[lookup(expression.via, expression.column).definition.kind];
        }
    }
}

/** Compute an arithmetic operation, missing for a division by zero. */
function arithmetic(
    operation: "add" | "subtract" | "multiply" | "divide",
    first: number,
    second: number,
): number | null {
    switch (operation) {
        case "add":
            return first + second;
        case "subtract":
            return first - second;
        case "multiply":
            return first * second;
        case "divide":
            return second === 0 ? null : first / second;
    }
}

/** Read the kind a literal yields: integer, real, text or null, absent for a boolean. */
function literalKind(value: Scalar): Kind | "null" | undefined {
    // read numbers as integers or reals
    if (typeof value === "number") {
        return Number.isInteger(value) ? "integer" : "real";
    }
    // read text
    else if (typeof value === "string") {
        return "text";
    }
    // read a missing value
    else if (value === null) {
        return "null";
    }
    // leave a boolean without a kind
    else {
        return undefined;
    }
}

/** Mark a value as JSON, so JSON builders embed it as JSON. */
function json(value: SQL): SQL {
    return dialectSQL({
        sqlite: sql`json(${value})`,
        postgresql: sql`CAST(${value} AS JSONB)`,
    });
}

/** Cast a number to a 64-bit float. */
function float(value: SQL): SQL {
    return dialectSQL({
        sqlite: sql`CAST(${value} AS REAL)`,
        postgresql: sql`CAST(${value} AS DOUBLE PRECISION)`,
    });
}

/** Read a column value an expression computes over, which an expression's requirements keep to JSON. */
function jsonOf(value: ColumnValue | undefined, name: string): JsonValue {
    if (value instanceof Uint8Array || typeof value === "bigint") {
        throw new DatabaseError(
            "INVALID_QUERY",
            `expressions compute over JSON values, not column ${name}`,
        );
    }

    return value ?? null;
}

/** Require an expression of known kinds, which a required expression has. */
function requireKind<Value>(kind: Value | undefined): Value {
    if (kind === undefined) {
        throw new DatabaseError("INVALID_QUERY", "expression mixes numbers with other values");
    }

    return kind;
}

/** Require the namespace to read a relation, as the table declares it. */
function requireRelation<Read>(
    read: Read | undefined,
    verb: string,
    via: string,
    table: Table,
): Read {
    if (read === undefined) {
        throw new DatabaseError(
            "INVALID_QUERY",
            `expression ${verb} relation ${via}, which ${table[TABLE].name} does not declare`,
        );
    }

    return read;
}
