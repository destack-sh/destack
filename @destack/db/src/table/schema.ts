import { schema } from "@destack/schema";
import type { TableDefinition } from "./table.ts";
import type { JsonOf } from "./column.ts";

/** The field validators of a record. */
export type Shape<Value> = { [Property in keyof Value]-?: schema.Schema<Value[Property]> };

/** The field validators of a record in JSON form. */
export type JsonShape<Value> = {
    [Property in keyof Value]-?: schema.Schema<JsonOf<Exclude<Value[Property], undefined>>>;
};

/** Build a validator of a table's records for an operation, in application or JSON form. */
export function recordSchema(
    table: TableDefinition,
    operation: "select" | "insert" | "update",
    form: "application" | "json" = "application",
) {
    const fields: Record<string, schema.Schema> = {};

    // derive validators from the columns
    for (const [property, column] of table.entries) {
        const definition = column.definition;
        if (operation !== "select" && definition.generated !== undefined) {
            continue;
        }
        let validator: schema.Schema = form === "json" ? definition.json : definition.schema;
        if (definition.nullable) {
            validator = validator.nullable();
        }
        if (
            operation === "update" ||
            (operation === "insert" && (definition.nullable || definition.default !== undefined))
        ) {
            validator = validator.exactOptional();
        }
        fields[property] = validator;
    }

    return schema.object(fields);
}
