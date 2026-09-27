import { TABLE, type Table } from "./table.ts";
import type { Dialect } from "../dialect/dialect.ts";
import type { Column, JsonValue } from "./column.ts";

/** A row as the application holds it, by column property. */
export type Row = Readonly<Record<string, unknown>>;

/** Write a row's own columns in their JSON form, by property, writing undefined and null values as null. */
export function encodeRow(table: Table, row: Row): Record<string, JsonValue> {
    return encodeColumns(table[TABLE].entries, row, []);
}

/** Write a row's own values in some columns in their JSON form, by property, writing undefined and null values as null, and leaving out some. */
export function encodeColumns(
    columns: readonly (readonly [string, Column])[],
    row: Row,
    omitted: readonly string[],
): Record<string, JsonValue> {
    const encoded: Record<string, JsonValue> = {};
    for (const [property, column] of columns) {
        // write the row's own columns the caller keeps
        if (Object.hasOwn(row, property) && !omitted.includes(property)) {
            const value = row[property];
            encoded[property] =
                value === null || value === undefined ? null : column.definition.toJson(value);
        }
    }

    return encoded;
}

/** Read a row's own columns from their JSON form, by property, keeping undefined and null values as null. */
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

/** Read a row a driver returned as values in the order of some columns, decoding each value for a dialect, by property. */
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
