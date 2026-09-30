import { sql, type SQL } from "drizzle-orm";
import * as language from "@destack/schema/expression";
import type { ScalarKind } from "@destack/schema/expression";
import { dialectSQL } from "../dialect/expression.ts";
import { DatabaseError } from "../error/error.ts";
import { TABLE, type Table } from "../table/table.ts";
import type { Namespace } from "../query/namespace.ts";

/** The column kinds arithmetic reads. */
const NUMERIC_KINDS: ReadonlySet<string> = new Set(["integer", "real"]);

/** The kinds an expression yields. */
type Kind = "integer" | "real" | "text" | "json";

/** The most terms one expression holds, bounding compiled SQL; computed fields use under ten. */
const EXPRESSION_TERMS = 64;

export type { Related, Rollup } from "@destack/schema/expression";

/** A value computed from one row, alike in SQL and memory. */
export type Expression = language.Expression;

/** A value computed from one row: the language, with its kind over tables and its SQL rendering. */
export const Expression = {
    ...language.Expression,

    /** Require a bounded expression over numeric or text columns. */
    require(expression: Expression, table: Table, namespace: Namespace = { computed: {} }): void {
        // bound the size
        if (language.Expression.terms(expression) > EXPRESSION_TERMS) {
            throw new DatabaseError(
                "INVALID_QUERY",
                `expression holds more than ${EXPRESSION_TERMS} terms`,
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
    kind(expression: Expression, table: Table, namespace: Namespace = { computed: {} }): Kind {
        // read an all-null expression as real
        const kind = kindOf(expression, table, namespace);
        if (kind === undefined) {
            throw new DatabaseError("INVALID_QUERY", "expression mixes numbers with other values");
        }

        return kind === "null" ? "real" : kind;
    },

    /** Render an expression as SQL, numbers as 64-bit floats. */
    render(expression: Expression, table: Table, namespace: Namespace = { computed: {} }): SQL {
        return renderAs(
            expression,
            Expression.kind(expression, table, namespace),
            table,
            namespace,
        );
    },
};

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
            }) as SQL;
        }
        case "object": {
            // pair each key with its value as JSON
            const pairs = Object.entries(expression.fields).map(([key, value]) => {
                const inner = kindOf(value, table, namespace);
                const rendered = renderAs(
                    value,
                    inner === "null" ? "text" : inner!,
                    table,
                    namespace,
                );

                return sql`CAST(${key} AS TEXT), ${rendered}`;
            });

            return dialectSQL({
                sqlite: sql`json_object(${sql.join(pairs, sql`, `)})`,
                postgresql: sql`jsonb_build_object(${sql.join(pairs, sql`, `)})`,
            }) as SQL;
        }
        case "case": {
            // compare the value with each listed one as its own kind
            const inner = kindOf(expression.of, table, namespace);
            const compared = inner === "null" ? "text" : (inner as ScalarKind);
            const of = renderAs(expression.of, compared, table, namespace);
            const branches = expression.cases.map(
                (entry) =>
                    sql` WHEN ${renderAs(language.Expression.literal(entry.when), compared, table, namespace)} THEN ${renderAs(entry.then, kind, table, namespace)}`,
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
            }) as SQL;

            return isText ? value : float(value);
        }
        case "column": {
            // read numbers as floats and JSON as JSON
            const column = table[TABLE].columns[expression.name]!;

            return kind === "text"
                ? sql`${column}`
                : kind === "json"
                  ? json(sql`${column}`)
                  : float(sql`${column}`);
        }
        case "literal":
            // type literals by kind
            return kind === "text"
                ? sql`CAST(${expression.value} AS TEXT)`
                : kind === "json"
                  ? json(sql`${JSON.stringify(expression.value)}`)
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
            // read the related value
            const value = sql`${namespace.lookup!(expression.via, expression.column).value}`;

            return kind === "text" ? value : float(value);
        }
        case "rollup": {
            // measure the related rows
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

            return NUMERIC_KINDS.has(column.definition.kind) ||
                column.definition.kind === "text" ||
                column.definition.kind === "json"
                ? (column.definition.kind as Kind)
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
            // yield integers from integers except by division, and reals otherwise
            const kinds = new Set([
                kindOf(expression.left, table, namespace),
                kindOf(expression.right, table, namespace),
            ]);
            kinds.delete("null");

            return kinds.has(undefined) || kinds.has("text") || kinds.has("json")
                ? undefined
                : kinds.has("real") || expression.kind === "divide"
                  ? "real"
                  : "integer";
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

            return NUMERIC_KINDS.has(definition.kind) || definition.kind === "text"
                ? (definition.kind as "integer" | "real" | "text")
                : undefined;
        }
        case "lookup": {
            // read the related column's kind
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

/** Mark a value as JSON, so JSON builders embed it as JSON. */
function json(value: SQL): SQL {
    return dialectSQL({
        sqlite: sql`json(${value})`,
        postgresql: sql`CAST(${value} AS JSONB)`,
    }) as SQL;
}

/** Cast a number to a 64-bit float. */
function float(value: SQL): SQL {
    return dialectSQL({
        sqlite: sql`CAST(${value} AS REAL)`,
        postgresql: sql`CAST(${value} AS DOUBLE PRECISION)`,
    }) as SQL;
}
