import { schema } from "@destack/schema";
import { sql } from "drizzle-orm";
import { identifier, integer, json } from "./column.ts";
import { type ColumnBuilders, table, type TableColumns } from "./table.ts";
import type { TableConstraint } from "./constraint.ts";

/** User-defined labels indexed by name. */
export const Tags = schema.record(schema.string().min(1).max(128), schema.string().max(256));
/** User-defined labels indexed by name. */
export type Tags = schema.Infer<typeof Tags>;

/** Declare a label map with an empty database default. */
export function tags(name = "tags") {
    return json(name, Tags)
        .notNull()
        .default(sql`'{}'`);
}

/** Declare the shared record fields. */
export function recordColumns<const Prefix extends string>(prefix: Prefix) {
    return {
        /** The immutable record identifier. */
        id: identifier("id", prefix).primaryKey(),
        /** Creation time in UTC epoch milliseconds. */
        createdAt: integer("created_at").notNull(),
        /** Last modification time in UTC epoch milliseconds. */
        updatedAt: integer("updated_at").notNull(),
        /** The revision used by conditional updates. */
        revision: integer("revision").notNull().default(1),
        /** User-defined labels. */
        tags: tags(),
    };
}

/** The shared record column declarations. */
export type RecordColumns<Prefix extends string = string> = ReturnType<
    typeof recordColumns<Prefix>
>;

/** Declare a table with standard record fields. */
export function record<Name extends string, Prefix extends string, Builders extends ColumnBuilders>(
    name: Name,
    prefix: Prefix,
    columns: Builders & { [Property in keyof RecordColumns]?: never },
    constraints?: (
        columns: TableColumns<RecordColumns<Prefix> & Builders, Name>,
    ) => readonly TableConstraint[],
) {
    return table(name, { ...recordColumns(prefix), ...columns }, constraints);
}
