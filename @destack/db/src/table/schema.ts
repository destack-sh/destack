import { type JsonValue, schema } from "@destack/schema";
import type { TableDefinition } from "./table.ts";
import type { ColumnDefinition } from "./column.ts";

/** The field validators of a record. */
export type Shape<Value> = { [Property in keyof Value]-?: schema.Schema<Value[Property]> };

/** The operations a record's validator checks: a selected, inserted or updated record. */
export type RecordOperation = "select" | "insert" | "update";

/** Build the field validators of a table's records for an operation. */
export function recordShape(
    table: TableDefinition,
    operation: RecordOperation,
): Record<string, schema.Schema> {
    return Object.fromEntries(
        fieldsOf(table, operation).map(([property, definition]) => [
            property,
            fieldOf(definition.schema, definition, operation),
        ]),
    );
}

/** Build the field validators of a table's records for an operation, in JSON form. */
export function jsonShape(
    table: TableDefinition,
    operation: RecordOperation,
): Record<string, schema.Schema<JsonValue>> {
    return Object.fromEntries(
        fieldsOf(table, operation).map(([property, definition]) => [
            property,
            fieldOf(definition.json, definition, operation),
        ]),
    );
}

/** List the columns a record of an operation has, generated ones selected alone. */
function fieldsOf(
    table: TableDefinition,
    operation: RecordOperation,
): (readonly [string, ColumnDefinition])[] {
    return table.entries.flatMap(([property, column]) =>
        operation !== "select" && column.definition.generated !== undefined
            ? []
            : [[property, column.definition] as const],
    );
}

/** Make a column's validator nullable where the column is, and optional where the operation is. */
function fieldOf<Value>(
    validator: schema.Schema<Value>,
    definition: ColumnDefinition,
    operation: RecordOperation,
): schema.Schema<Value | null> {
    // accept null in a nullable column
    const nullable = definition.nullable ? validator.nullable() : validator;

    // leave out an updated column, or an inserted one with a default or null
    const isOptional =
        operation === "update" ||
        (operation === "insert" && (definition.nullable || definition.default !== undefined));

    return isOptional ? nullable.exactOptional() : nullable;
}
