import { getTableColumns, getTableName, is, SQL, sql } from "drizzle-orm";
import {
    getTableConfig,
    SQLiteBaseInteger,
    SQLiteColumn,
    SQLiteDialect,
    type SQLiteTable,
    SQLiteTimestamp,
} from "drizzle-orm/sqlite-core";
import type { JsonSchema } from "@destack/schema/inspect";
import { TableDescription } from "../../inspect/index.ts";

/** Describe columns and SQL constraints without opening a database. */
export function describeTable(table: SQLiteTable): TableDescription {
    const definition = getTableConfig(table);

    return TableDescription.parse({
        dialect: "sqlite",
        name: definition.name,
        columns: Object.entries(getTableColumns(table)).map(([property, column]) => ({
            property,
            name: column.name,
            type: column.getSQLType(),
            dataType: column.dataType,
            mode: is(column, SQLiteTimestamp) ? column.mode : undefined,
            enumValues: column.enumValues,
            nullable: !column.notNull,
            primaryKey: column.primary,
            unique: column.isUnique,
            uniqueName: column.isUnique ? column.uniqueName : undefined,
            autoIncrement: is(column, SQLiteBaseInteger) && column.autoIncrement,
            hasDefault: column.hasDefault,
            default:
                column.default === undefined
                    ? undefined
                    : expression(
                          is(column.default, SQL)
                              ? column.default
                              : column.default === null
                                ? null
                                : column.mapToDriverValue(column.default),
                      ),
            hasRuntimeDefault: column.defaultFn !== undefined,
            hasRuntimeUpdate: column.onUpdateFn !== undefined,
            generated:
                column.generated === undefined
                    ? undefined
                    : {
                          mode: column.generated.mode,
                          expression: expression(
                              typeof column.generated.as === "function"
                                  ? column.generated.as()
                                  : column.generated.as,
                          ),
                      },
            jsonSchema: "jsonSchema" in column ? (column.jsonSchema as JsonSchema) : undefined,
        })),
        primaryKeys: definition.primaryKeys.map((key) => ({
            name: key.getName(),
            columns: key.columns.map((column) => column.name),
        })),
        uniqueConstraints: definition.uniqueConstraints.map((constraint) => ({
            name: constraint.getName(),
            columns: constraint.columns.map((column) => column.name),
        })),
        indexes: definition.indexes.map(({ config }) => ({
            name: config.name,
            unique: config.unique,
            columns: config.columns.map((column) =>
                is(column, SQLiteColumn)
                    ? { column: column.name }
                    : { expression: expression(column) },
            ),
            where: config.where === undefined ? undefined : expression(config.where),
        })),
        foreignKeys: definition.foreignKeys.map((key) => {
            const reference = key.reference();

            return {
                name: key.getName(),
                columns: reference.columns.map((column) => column.name),
                table: getTableName(reference.foreignTable),
                references: reference.foreignColumns.map((column) => column.name),
                onUpdate: key.onUpdate,
                onDelete: key.onDelete,
            };
        }),
        checks: definition.checks.map((check) => ({
            name: check.name,
            expression: expression(check.value),
        })),
    });
}

/** Compile a declaration expression using SQLite quoting and literal encoding. */
function expression(value: unknown): string {
    if (typeof value === "bigint") {
        return value.toString();
    }

    // preserve binary literals without decoding bytes as text
    if (value instanceof ArrayBuffer || ArrayBuffer.isView(value)) {
        const bytes =
            value instanceof ArrayBuffer
                ? new Uint8Array(value)
                : new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
        const hexadecimal = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join(
            "",
        );

        return `X'${hexadecimal}'`;
    }

    // compile SQL expressions and scalar literals with the dialect's escaping
    const query = is(value, SQL) ? value : sql`${value}`;

    return new SQLiteDialect().sqlToQuery(sql`${query}`.inlineParams()).sql;
}
