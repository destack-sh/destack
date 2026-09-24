import { SQL, sql } from "drizzle-orm";
import { Column } from "../table/column.ts";
import { TABLE, type Table } from "../table/table.ts";
import { type ColumnDescription, describeColumnSchema, TableDescription } from "./table.ts";
import type { Dialect } from "../dialect/dialect.ts";
import type { DatabaseSchema } from "../schema/schema.ts";
import type { DatabaseSchemaDescription } from "./schema.ts";
import { compileExpression } from "../dialect/expression.ts";
import { assertNever } from "../error/error.ts";

/** Describe logical fields and constraints without opening a database. */
export function describeTable(table: Table, dialect: Dialect): TableDescription {
    const definition = table[TABLE];
    const constraints = table.constraints(dialect);

    return {
        dialect,
        name: definition.name,
        columns: Object.entries(definition.columns).map(([property, column]) => ({
            property,
            name: column.definition.name,
            type: column.definition.types[dialect],
            dataType: column.definition.kind,
            enumValues: column.definition.enumValues && [...column.definition.enumValues],
            nullable: column.definition.nullable,
            primaryKey: column.definition.primaryKey ?? false,
            unique: column.definition.unique !== undefined,
            uniqueName: column.definition.unique?.name,
            autoIncrement: false,
            hasDefault:
                column.definition.default !== undefined ||
                column.definition.runtimeDefault !== undefined ||
                column.definition.runtimeUpdate !== undefined ||
                column.definition.generated !== undefined,
            default:
                column.definition.default === undefined
                    ? undefined
                    : expression(
                          column.definition.default instanceof SQL
                              ? column.definition.default
                              : column.definition.encode(column.definition.default, dialect),
                          dialect,
                      ),
            hasRuntimeDefault: column.definition.runtimeDefault !== undefined,
            hasRuntimeUpdate: column.definition.runtimeUpdate !== undefined,
            generated: column.definition.generated
                ? {
                      mode: column.definition.generated.mode,
                      expression: expression(
                          typeof column.definition.generated.expression === "function"
                              ? column.definition.generated.expression()
                              : column.definition.generated.expression,
                          dialect,
                      ),
                  }
                : undefined,
            jsonSchema: describeColumnSchema(column.definition) as NonNullable<
                ColumnDescription["jsonSchema"]
            >,
        })),
        primaryKeys: constraints
            .filter((value) => value.kind === "primaryKey")
            .map((key) => ({
                name:
                    key.name ??
                    `${definition.name}_${key.columns
                        .map((column) => column.definition.name)
                        .join("_")}_pk`,
                columns: key.columns.map((column) => column.definition.name),
            })),
        uniqueConstraints: constraints
            .filter((value) => value.kind === "unique")
            .map((key) => ({
                name:
                    key.name ??
                    `${definition.name}_${key.columns
                        .map((column) => column.definition.name)
                        .join("_")}_unique`,
                columns: key.columns.map((column) => column.definition.name),
            })),
        indexes: constraints
            .filter((value) => value.kind === "index")
            .map((index) => ({
                name: index.name,
                unique: index.unique,
                columns: index.columns.map((column) =>
                    column instanceof Column
                        ? { column: column.definition.name }
                        : { expression: expression(column, dialect) },
                ),
                where: index.predicate && expression(index.predicate, dialect),
            })),
        foreignKeys: constraints
            .filter((value) => value.kind === "foreignKey")
            .map((key) => ({
                name:
                    key.name ??
                    `${definition.name}_${key.columns
                        .map((column) => column.definition.name)
                        .join("_")}_${key.foreignColumns[0].table}_${key.foreignColumns
                        .map((column) => column.definition.name)
                        .join("_")}_fk`,
                columns: key.columns.map((column) => column.definition.name),
                table: key.foreignColumns[0].table,
                references: key.foreignColumns.map((column) => column.definition.name),
                onDelete: key.actions.onDelete,
                onUpdate: key.actions.onUpdate,
            })),
        checks: constraints
            .filter((value) => value.kind === "check")
            .map((check) => ({
                name: check.name,
                expression: expression(check.expression, dialect),
            })),
    };
}

/** Render a declaration expression with quoted SQL identifiers and literals. */
function expression(value: unknown, dialect: Dialect): string {
    if (typeof value === "bigint") {
        return value.toString();
    }
    if (value instanceof Uint8Array) {
        const hexadecimal = value.toHex();

        if (dialect === "sqlite") {
            return `X'${hexadecimal}'`;
        } else if (dialect === "postgresql") {
            return `decode('${hexadecimal}', 'hex')`;
        } else {
            return assertNever(dialect);
        }
    }

    // render the default as inline SQL
    const expression = value instanceof SQL ? value : sql`${value}`;

    return compileExpression(expression, dialect).toQuery({
        escapeName: (name) => `"${name.replaceAll('"', '""')}"`,
        escapeString: (value) => `'${value.replaceAll("'", "''")}'`,
        escapeParam: (index) => `$${index + 1}`,
        inlineParams: true,
    }).sql;
}

/** Describe a managed schema for its selected SQL dialect. */
export function describeSchema(
    definition: DatabaseSchema,
    dialect: Dialect,
): DatabaseSchemaDescription {
    return {
        name: definition.name,
        version: 1,
        dialect,
        tables: Object.values(definition.tables).map((table) => describeTable(table, dialect)),
        ...(definition.trees?.length
            ? { trees: definition.trees.map((tree) => tree.describe()) }
            : {}),
        ...(definition.dependencies?.length
            ? { dependencies: definition.dependencies.map((schema) => schema.name) }
            : {}),
    };
}
