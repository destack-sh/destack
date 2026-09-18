import { type BuildColumns, type ColumnBuilderBase, sql } from "drizzle-orm";
import { check, sqliteTable, type SQLiteTableExtraConfigValue } from "drizzle-orm/sqlite-core";
import type { JsonSchema } from "@destack/schema/inspect";

/** Define a SQLite table and retain structured column descriptions. */
export function table<Name extends string, Columns extends Record<string, ColumnBuilderBase>>(
    name: Name,
    columns: Columns,
    extra?: (table: BuildColumns<Name, Columns, "sqlite">) => SQLiteTableExtraConfigValue[],
) {
    const definition = sqliteTable(name, columns, (fields) => {
        const constraints = [...(extra?.(fields) ?? [])];

        // preserve non-null text primary keys when migration generators omit NOT NULL
        for (const column of Object.values(fields)) {
            if (column.primary && column.notNull && column.getSQLType() !== "integer") {
                constraints.push(
                    check(`${name}_${column.name}_not_null`, sql`${column} IS NOT NULL`),
                );
            }
        }

        return constraints;
    });
    const names = new Set<string>();

    // reject duplicate SQL names and retain JSON descriptions on the finished columns
    for (const property of Object.keys(columns) as (keyof Columns & string)[]) {
        const name = definition[property].name;
        if (names.has(name)) throw new TypeError(`Duplicate SQL column: ${name}.`);
        names.add(name);

        const column = columns[property];
        if ("jsonSchema" in column) {
            if (!("schema" in column)) throw new TypeError(`Missing JSON validator: ${property}.`);
            Object.assign(definition[property], {
                schema: column.schema,
                jsonSchema: column.jsonSchema as JsonSchema,
            });
        }
    }

    return definition;
}
