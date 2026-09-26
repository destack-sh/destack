import { TABLE, type Table } from "./table.ts";
import type { JsonValue } from "./column.ts";

/** Write a row's columns in their JSON form, by property, keeping absent and null values as null. */
export function encodeRow(
    table: Table,
    row: Readonly<Record<string, unknown>>,
): Record<string, JsonValue> {
    return Object.fromEntries(
        Object.entries(table[TABLE].columns)
            .filter(([property]) => Object.hasOwn(row, property))
            .map(([property, column]) => {
                const value = row[property];

                return [
                    property,
                    value === null || value === undefined ? null : column.definition.toJson(value),
                ];
            }),
    );
}

/** Read a row's columns from their JSON form, by property, keeping null values as null. */
export function decodeRow(
    table: Table,
    row: Readonly<Record<string, unknown>>,
): Record<string, unknown> {
    return Object.fromEntries(
        Object.entries(table[TABLE].columns)
            .filter(([property]) => Object.hasOwn(row, property))
            .map(([property, column]) => {
                const value = row[property];

                return [
                    property,
                    value === null || value === undefined
                        ? null
                        : column.definition.fromJson(value),
                ];
            }),
    );
}
