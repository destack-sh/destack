import type { BuildColumns, ColumnBuilderBase } from "drizzle-orm";
import { integer, type SQLiteTableExtraConfigValue } from "drizzle-orm/sqlite-core";
import { identifier } from "../column/identifier.ts";
import { tags } from "../column/tags.ts";
import { table } from "../table/index.ts";

/** Create fresh column builders for the standard application record. */
export function recordColumns<const Prefix extends string>(prefix: Prefix) {
    return {
        /** The immutable record identifier. */
        id: identifier("id", prefix).primaryKey().notNull(),
        /** Creation time in UTC epoch milliseconds. */
        createdAt: integer("created_at").notNull(),
        /** Last modification time in UTC epoch milliseconds. */
        updatedAt: integer("updated_at").notNull(),
        /** The revision used for optimistic concurrency checks. */
        revision: integer("revision").notNull().default(1),
        /** User-defined labels shared by applications. */
        tags: tags(),
    };
}

/** Column builders shared by standard application records. */
export type RecordColumns<Prefix extends string = string> = ReturnType<
    typeof recordColumns<Prefix>
>;

/** Define a typed record; callers update timestamps and revisions in their transactions. */
export function record<
    Name extends string,
    Prefix extends string,
    Columns extends Record<string, ColumnBuilderBase>,
>(
    name: Name,
    prefix: Prefix,
    columns: Columns & { [Key in keyof RecordColumns]?: never },
    extra?: (
        table: BuildColumns<Name, RecordColumns<Prefix> & Columns, "sqlite">,
    ) => SQLiteTableExtraConfigValue[],
) {
    const base = recordColumns(prefix);

    // reject reserved properties even when called from untyped code
    for (const key of Object.keys(base)) {
        if (key in columns) {
            throw new TypeError(`Reserved record column: ${key}.`);
        }
    }

    // build the table using the common column validation and descriptions
    const definition = table(name, { ...base, ...columns }, extra);

    return definition;
}
