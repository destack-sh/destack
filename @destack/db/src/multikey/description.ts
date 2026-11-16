import { schema } from "@destack/schema";

/** A multikey index as stored: its source table, array column and entry table. */
export const MultikeyIndexDescription = schema.object({
    /** The index's name. */
    name: schema.string().min(1),
    /** The SQL name of the source table. */
    table: schema.string().min(1),
    /** The source table's SQL column identifying its rows. */
    id: schema.string().min(1),
    /** The source table's SQL column with the JSON array. */
    from: schema.string().min(1),
    /** Whether elements whose element keys are all null get no entry. */
    isSparse: schema.boolean(),
    /** The entry columns, read at an element's path or copied from the source row. */
    fields: schema.array(
        schema.union([
            schema.object({
                /** The entry table's SQL column. */
                column: schema.string().min(1),
                /** The keys leading from the element to the value. */
                path: schema.array(schema.string().min(1)).min(1),
                /** How the value is stored: text, an integer, or JSON. */
                kind: schema.enum(["text", "integer", "json"]),
            }),
            schema.object({
                /** The entry table's SQL column. */
                column: schema.string().min(1),
                /** The source table's SQL column the entry copies. */
                source: schema.string().min(1),
                /** How the value is stored: text, an integer, or JSON. */
                kind: schema.enum(["text", "integer", "json"]),
            }),
        ]),
    ),
    /** The SQL name of the entry table. */
    entryTable: schema.string().min(1),
});
/** A multikey index as stored. */
export type MultikeyIndexDescription = schema.Infer<typeof MultikeyIndexDescription>;
