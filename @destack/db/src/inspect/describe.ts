import { SQL, sql, type SQLChunk } from "drizzle-orm";
import { Column } from "../table/column.ts";
import { TABLE, type Table } from "../table/table.ts";
import { TableDescription } from "./table.ts";
import type { Dialect } from "../dialect/dialect.ts";
import { compileExpression } from "../dialect/expression.ts";
import { boundedName, constraintName } from "../table/namespace.ts";
import { assertNever } from "../error/error.ts";
import { literal, quote } from "../dialect/quote.ts";

/** Describe logical fields and constraints without opening a database. */
export function describeTable(table: Table, dialect: Dialect): TableDescription {
    // collect the declared constraints for the dialect
    const definition = table[TABLE];
    const constraints = table.constraints(dialect);
    const columns = Object.values(definition.columns);

    // gather primary keys and unique constraints declared on columns and on the table
    const keys: { kind: "primaryKey" | "unique"; name?: string; columns: readonly Column[] }[] = [
        ...columns
            .filter((column) => column.definition.primaryKey)
            .map((column) => ({ kind: "primaryKey" as const, columns: [column] })),
        ...constraints.flatMap((constraint) =>
            constraint.kind === "primaryKey"
                ? [{ ...constraint, kind: "primaryKey" as const }]
                : [],
        ),
        ...columns
            .filter((column) => column.definition.unique !== undefined)
            .map((column) => ({
                kind: "unique" as const,
                ...(column.definition.unique!.name === undefined
                    ? {}
                    : { name: column.definition.unique!.name }),
                columns: [column],
            })),
        ...constraints.flatMap((constraint) =>
            constraint.kind === "unique" ? [{ ...constraint, kind: "unique" as const }] : [],
        ),
    ];

    return {
        dialect,
        name: definition.sqlName,
        columns: columns.map((column) => ({
            name: column.definition.name,
            type: column.definition.types[dialect],
            nullable: column.definition.nullable,
            ...(column.definition.default === undefined
                ? {}
                : {
                      default: inlineExpression(
                          column.definition.default instanceof SQL
                              ? column.definition.default
                              : column.definition.encode(column.definition.default, dialect),
                          dialect,
                      ),
                  }),
            ...(column.definition.generated === undefined
                ? {}
                : {
                      generated: {
                          mode: column.definition.generated.mode,
                          expression: inlineExpression(
                              typeof column.definition.generated.expression === "function"
                                  ? column.definition.generated.expression()
                                  : column.definition.generated.expression,
                              dialect,
                          ),
                      },
                  }),
        })),
        constraints: [
            ...keys.map((key) => ({
                kind: key.kind,
                name:
                    constraintName(definition.package, key.name) ??
                    derivedName(
                        definition.sqlName,
                        key.columns,
                        key.kind === "primaryKey" ? "pk" : "unique",
                    ),
                columns: key.columns.map((column) => column.definition.name),
            })),
            ...constraints
                .filter((value) => value.kind === "foreignKey")
                .map((key) => ({
                    kind: "foreignKey" as const,
                    name:
                        key.name ??
                        derivedName(
                            definition.sqlName,
                            key.columns,
                            `${key.foreignColumns[0].table}_${key.foreignColumns
                                .map((column) => column.definition.name)
                                .join("_")}_fk`,
                        ),
                    columns: key.columns.map((column) => column.definition.name),
                    table: key.foreignColumns[0].table,
                    references: key.foreignColumns.map((column) => column.definition.name),
                    ...(key.actions.onDelete === undefined
                        ? {}
                        : { onDelete: key.actions.onDelete }),
                    ...(key.actions.onUpdate === undefined
                        ? {}
                        : { onUpdate: key.actions.onUpdate }),
                })),
            ...constraints
                .filter((value) => value.kind === "check")
                .map((check) => ({
                    kind: "check" as const,
                    name: check.name,
                    expression: inlineExpression(check.expression, dialect),
                })),
        ],
        indexes: constraints
            .filter((value) => value.kind === "index")
            .map((index) => ({
                name: constraintName(definition.package, index.name),
                unique: index.isUnique,
                columns: index.columns.map((column) =>
                    column instanceof Column
                        ? { column: column.definition.name }
                        : { expression: inlineExpression(column, dialect) },
                ),
                ...(index.predicate === undefined
                    ? {}
                    : { where: inlineExpression(index.predicate, dialect) }),
            })),
    };
}

/** Derive a constraint name from a table, its columns and a suffix, bounded to the identifier limit. */
function derivedName(table: string, columns: readonly Column[], suffix: string): string {
    return boundedName(
        [table, ...columns.map((column) => column.definition.name), suffix].join("_"),
    );
}

/** Render a declaration expression with quoted SQL identifiers and literals. */
export function inlineExpression(value: unknown, dialect: Dialect): string {
    // write bigints as integer literals
    if (typeof value === "bigint") {
        return value.toString();
    }

    // write bytes as hexadecimal literals
    if (value instanceof Uint8Array) {
        const hexadecimal = value.toHex();

        // write a SQLite blob literal
        if (dialect === "sqlite") {
            return `X'${hexadecimal}'`;
        }
        // decode hexadecimal in PostgreSQL
        else if (dialect === "postgresql") {
            return `decode('${hexadecimal}', 'hex')`;
        }
        // reject other dialects
        else {
            return assertNever(dialect);
        }
    }

    // render the expression with unqualified columns
    const expression = value instanceof SQL ? value : sql`${value}`;
    const unqualified = (chunk: SQLChunk) =>
        chunk instanceof Column ? sql.identifier(chunk.definition.name) : chunk;

    return compileExpression(expression, dialect, unqualified).toQuery({
        escapeName: quote,
        escapeString: literal,
        escapeParam: (index) => `$${index + 1}`,
        inlineParams: true,
    }).sql;
}
