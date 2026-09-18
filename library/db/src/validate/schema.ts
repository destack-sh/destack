import {
    getTableColumns,
    type InferInsertModel,
    type InferSelectModel,
    type Table,
} from "drizzle-orm";
import * as drizzle from "drizzle-orm/zod";
import * as schema from "@destack/schema/validate";

/** Validation fields matching an inferred SQL record. */
type Shape<T> = { [Key in keyof T]-?: schema.Schema<T[Key]> };

/** Validate a selected record, including declared JSON fields. */
export function createSelectSchema<T extends Table>(
    table: T,
): schema.Object<Shape<InferSelectModel<T>>> {
    return createSchema(table, "select") as schema.Object<Shape<InferSelectModel<T>>>;
}

/** Validate an inserted record, including defaults, generated fields, and JSON fields. */
export function createInsertSchema<T extends Table>(
    table: T,
): schema.Object<Shape<InferInsertModel<T>>> {
    return createSchema(table, "insert") as schema.Object<Shape<InferInsertModel<T>>>;
}

/** Validate a partial update while excluding generated fields. */
export function createUpdateSchema<T extends Table>(
    table: T,
): schema.Object<Shape<Partial<InferInsertModel<T>>>> {
    return createSchema(table, "update") as schema.Object<
        Shape<Partial<InferInsertModel<T>>>
    >;
}

/** Combine Drizzle's column validation with Destack's declared custom schemas. */
function createSchema(table: Table, operation: "select" | "insert" | "update") {
    const definition = operation === "select"
        ? drizzle.createSelectSchema(table)
        : operation === "insert"
        ? drizzle.createInsertSchema(table)
        : drizzle.createUpdateSchema(table);
    const fields: Record<string, schema.Schema> = { ...definition.shape };

    // validate custom fields with the schema retained on the column
    for (const [name, column] of Object.entries(getTableColumns(table))) {
        if (!(name in fields) || column.dataType !== "custom") continue;
        if (!("schema" in column) || !(column.schema instanceof schema.Schema)) {
            throw new TypeError(`Missing custom column validator: ${name}.`);
        }
        let validator = column.schema;
        if (!column.notNull) validator = validator.nullable();
        if (
            operation === "update" ||
            (operation === "insert" && (!column.notNull || column.hasDefault))
        ) {
            validator = validator.optional();
        }
        fields[name] = validator;
    }

    return schema.object(fields);
}
