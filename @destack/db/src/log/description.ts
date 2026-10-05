import { defineSchema, schema } from "@destack/schema";

/** How long a table's committed changes stay in the log. */
export const ChangeRetention = defineSchema(schema.enum(["none", "window", "history"]));
/** How long a table's committed changes stay in the log. */
export type ChangeRetention = schema.Infer<typeof ChangeRetention>;

/** The columns a table's change triggers record. */
export const ChangeDescription = defineSchema(
    schema.object({
        /** The table's SQL name. */
        table: schema.string().min(1),
        /** Whether changes outlive the compaction window. */
        retention: schema.enum(["window", "history"]),
        /** The primary key's SQL column names in key order. */
        key: schema.array(schema.string().min(1)).min(1),
        /** The recorded SQL column names, without sensitive columns. */
        columns: schema.array(schema.string().min(1)),
        /** The recorded columns converted to text for exact precision. */
        exact: schema.array(schema.string().min(1)),
        /** The recorded binary columns, converted to hexadecimal text. */
        binary: schema.array(schema.string().min(1)),
        /** Every SQL column name, to skip updates that change nothing. */
        compared: schema.array(schema.string().min(1)),
        /** The SQL column with each row's scope, absent when every row takes the database's scope. */
        scope: schema.string().min(1).exactOptional(),
    }),
);
/** The columns a table's change triggers record. */
export type ChangeDescription = schema.Infer<typeof ChangeDescription>;
