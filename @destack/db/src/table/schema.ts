import { schema } from "@destack/schema";
import { TABLE, type Table } from "./table.ts";

/** The field validators of a record. */
export type Shape<Value> = { [Property in keyof Value]-?: schema.Schema<Value[Property]> };

/** Build a validator of a table's records for an operation, in application or JSON form. */
export function recordSchema(
    table: Table,
    operation: "select" | "insert" | "update",
    form: "application" | "json" = "application",
) {
    const fields: Record<string, schema.Schema> = {};

    // derive validators from the columns
    for (const [property, column] of Object.entries(table[TABLE].columns)) {
        const definition = column.definition;
        if (operation !== "select" && definition.generated) {
            continue;
        }
        let validator =
            form === "json" && definition.json !== undefined ? definition.json : definition.schema;
        if (definition.nullable) {
            validator = validator.nullable();
        }
        if (
            operation === "update" ||
            (operation === "insert" &&
                (definition.nullable ||
                    definition.default !== undefined ||
                    definition.runtimeDefault !== undefined ||
                    definition.runtimeUpdate !== undefined))
        ) {
            validator = validator.optional();
        }
        fields[property] = validator;
    }

    return schema.object(fields);
}
