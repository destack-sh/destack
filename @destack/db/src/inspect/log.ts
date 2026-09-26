import { defineSchema, schema } from "@destack/schema";

/** How long a table's committed changes stay in the log. */
export const ChangeTier = defineSchema(schema.enum(["none", "window", "history"]));
/** How long a table's committed changes stay in the log. */
export type ChangeTier = schema.Infer<typeof ChangeTier>;

/** The physical columns a table's generated change triggers record. */
export const ChangeDescription = defineSchema(
    schema.object({
        /** The table's SQL name. */
        table: schema.string().min(1),
        /** Whether changes outlive the compaction window. */
        tier: schema.enum(["window", "history"]),
        /** The primary key's SQL column names in key order. */
        key: schema.array(schema.string().min(1)).min(1),
        /** The recorded SQL column names, binary and sensitive columns left out. */
        columns: schema.array(schema.string().min(1)),
        /** The recorded columns converted to text to keep integer and numeric precision. */
        exact: schema.array(schema.string().min(1)),
        /** Every SQL column name, compared to skip updates that change nothing. */
        compared: schema.array(schema.string().min(1)),
        /** The SQL column whose value routes each change, for reading one route's changes. */
        route: schema.string().min(1).optional(),
    }),
);
/** The physical columns a table's generated change triggers record. */
export type ChangeDescription = schema.Infer<typeof ChangeDescription>;
