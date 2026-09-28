import { TABLE, type Table } from "./table.ts";
import type { Dialect } from "../dialect/dialect.ts";
import type { Column, JsonValue } from "./column.ts";

/** A row by column property. */
export type Row = Readonly<Record<string, unknown>>;

/** Write a row's own columns in JSON form, nulls for missing values. */
export function encodeRow(table: Table, row: Row): Record<string, JsonValue> {
    return encodeColumns(table[TABLE].entries, row, []);
}

/** Write a row's own values in some columns in JSON form, leaving out some. */
export function encodeColumns(
    columns: readonly (readonly [string, Column])[],
    row: Row,
    omitted: readonly string[],
): Record<string, JsonValue> {
    const encoded: Record<string, JsonValue> = {};
    for (const [property, column] of columns) {
        // write the kept columns
        if (Object.hasOwn(row, property) && !omitted.includes(property)) {
            const value = row[property];
            encoded[property] =
                value === null || value === undefined ? null : column.definition.toJson(value);
        }
    }

    return encoded;
}

/** Read a row's own columns from JSON form. */
export function decodeRow(table: Table, row: Row): Record<string, unknown> {
    const decoded: Record<string, unknown> = {};
    for (const [property, column] of table[TABLE].entries) {
        // read the row's own columns
        if (Object.hasOwn(row, property)) {
            const value = row[property];
            decoded[property] =
                value === null || value === undefined ? null : column.definition.fromJson(value);
        }
    }

    return decoded;
}

/** Read a driver row of positional values by property. */
export function fromDriver(
    columns: readonly (readonly [string, Column])[],
    values: readonly unknown[],
    dialect: Dialect,
): Record<string, unknown> {
    const read: Record<string, unknown> = {};
    for (const [position, [property, column]] of columns.entries()) {
        const value = values[position];
        read[property] = value === null ? null : column.definition.decode(value, dialect);
    }

    return read;
}
