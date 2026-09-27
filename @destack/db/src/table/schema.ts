import * as schema from "@destack/schema/validate";
import { type Insert, type Select, TABLE, type Table } from "./table.ts";

/** Validation fields matching an application record. */
type Shape<Value> = { [Property in keyof Value]-?: schema.Schema<Value[Property]> };

/** Validate every selected field, as the application holds it or in its JSON form. */
export function createSelectSchema<Definition extends Table>(
    table: Definition,
    form: "application" | "json" = "application",
): schema.Object<Shape<Select<Definition>>> {
    return createSchema(table, "select", form) as schema.Object<Shape<Select<Definition>>>;
}

/** Validate inserted fields, including nullable columns and defaults, as the application holds them or in their JSON form. */
export function createInsertSchema<Definition extends Table>(
    table: Definition,
    form: "application" | "json" = "application",
): schema.Object<Shape<Insert<Definition>>> {
    return createSchema(table, "insert", form) as schema.Object<Shape<Insert<Definition>>>;
}

/** Validate a partial application record update. */
export function createUpdateSchema<Definition extends Table>(
    table: Definition,
): schema.Object<Shape<Partial<Insert<Definition>>>> {
    return createSchema(table, "update") as schema.Object<Shape<Partial<Insert<Definition>>>>;
}

/** Apply insertion and nullability rules to the declared field validators. */
function createSchema(
    table: Table,
    operation: "select" | "insert" | "update",
    form: "application" | "json" = "application",
) {
    const fields: Record<string, schema.Schema> = {};

    // derive API values directly from logical columns without loading a database driver
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
