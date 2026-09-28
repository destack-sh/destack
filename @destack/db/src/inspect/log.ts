import { defineSchema, schema } from "@destack/schema";

/** How long a table's committed changes stay in the log. */
export const ChangeTier = defineSchema(schema.enum(["none", "window", "history"]));
/** How long a table's committed changes stay in the log. */
export type ChangeTier = schema.Infer<typeof ChangeTier>;

/** The columns a table's change triggers record. */
export const ChangeDescription = defineSchema(
    schema.object({
        /** The table's SQL name. */
        table: schema.string().min(1),
        /** Whether changes outlive the compaction window. */
        tier: schema.enum(["window", "history"]),
        /** The primary key's SQL column names in key order. */
        key: schema.array(schema.string().min(1)).min(1),
        /** The recorded SQL column names, without binary and sensitive columns. */
        columns: schema.array(schema.string().min(1)),
        /** The recorded columns converted to text for exact precision. */
        exact: schema.array(schema.string().min(1)),
        /** Every SQL column name, to skip updates that change nothing. */
        compared: schema.array(schema.string().min(1)),
        /** The SQL column holding each row's scope. */
        scope: schema.string().min(1),
    }),
);
/** The columns a table's change triggers record. */
export type ChangeDescription = schema.Infer<typeof ChangeDescription>;
