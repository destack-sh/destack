import {
    type RowChange,
    Key,
    TABLE,
    type DatabaseConnection,
    type Row,
    type Overlay,
    Table,
} from "@destack/db";
import { found, present, schema, type JsonObject } from "@destack/schema";
import type { ObjectType } from "../object/index.ts";

/** The columns of a stored branch row: identifier, table, key and both sides, or null. */
const STORED_CHANGE = schema.looseObject({
    id: schema.string(),
    table: schema.string(),
    key: schema.jsonObject(),
    after: schema.jsonObject().nullish(),
    before: schema.jsonObject().nullish(),
});

/** A write a branch makes to one row over the main line, encoded for storage. */
export interface BranchWrite {
    /** The SQL name of the row's table. */
    readonly table: string;
    /** The row's key columns. */
    readonly key: JsonObject;
    /** The row as the branch leaves it, null when the branch removes it. */
    readonly after: JsonObject | null;
    /** The main line's row the branch replaces, null when the branch creates it. */
    readonly before: JsonObject | null;
}

/** A change a branch makes to one object over the main line. */
export interface BranchChange {
    /** The object's type. */
    readonly object: ObjectType;
    /** The object's identifier. */
    readonly id: string;
    /** Whether the branch creates, updates or removes the object. */
    readonly change: "created" | "updated" | "removed";
    /** The object on the main line, absent when the branch creates it. */
    readonly before?: Row;
    /** The object on the branch, absent when the branch removes it. */
    readonly after?: Row;
    /** The fields callers write whose values differ. */
    readonly fields: readonly string[];
}

/** The writes of branches: captured from calls, written over a database, read as overlays. */
export const BranchWrite = {
    /** Read a stored branch row with its identifier, a missing side as null. */
    read(stored: Row): BranchWrite & { readonly id: string } {
        const { id, table, key, after, before } = STORED_CHANGE.parse(stored);

        return { id, table, key, after: after ?? null, before: before ?? null };
    },

    /** Capture the rows a transaction's changes leave different from the main line. */
    capture(changes: readonly RowChange[]): BranchWrite[] {
        return changes.map(({ table, before, after }) => {
            const image = present(after ?? before, "a changed row's image");

            return {
                table: table[TABLE].sqlName,
                key: table[TABLE].encode(Key.of(table, image)),
                after: after === null ? null : table[TABLE].encode(after),
                before: before === null ? null : table[TABLE].encode(before),
            };
        });
    },

    /** Write branch rows over a database's rows. */
    async write(
        database: DatabaseConnection,
        tables: ReadonlyMap<string, Table>,
        rows: readonly BranchWrite[],
    ): Promise<void> {
        for (const entry of rows) {
            // remove a row the branch removes, and put the others
            const table = found(tables, entry.table);
            if (entry.after === null) {
                await database
                    .delete(table)
                    .where(Key.match(table, Table.decode(table, entry.key)));
            } else {
                await database.upsert(table, [Table.decode(table, entry.after)]);
            }
        }
    },

    /** Read branch rows as an overlay, by table and key. */
    overlay(rows: readonly BranchWrite[], tables: ReadonlyMap<string, Table>): Overlay {
        const byTable = new Map<string, Map<string, Row | null>>();
        for (const entry of rows) {
            const table = found(tables, entry.table);
            const keyed = byTable.get(entry.table) ?? new Map<string, Row | null>();
            byTable.set(entry.table, keyed);
            keyed.set(
                Key.name(table, Table.decode(table, entry.key)),
                entry.after === null ? null : Table.decode(table, entry.after),
            );
        }

        return async (table) => byTable.get(table[TABLE].sqlName) ?? new Map();
    },

    /** Decode a branch row as its object change, absent when no written field differs. */
    change(entry: BranchWrite, object: ObjectType): BranchChange | undefined {
        // decode both sides
        const table = object.table;
        const before = entry.before === null ? undefined : Table.decode(table, entry.before);
        const after = entry.after === null ? undefined : Table.decode(table, entry.after);

        // take the written fields that differ, skipping changes that are only derived
        const fields = object.written.filter(
            (name) => JSON.stringify(before?.[name]) !== JSON.stringify(after?.[name]),
        );
        if (fields.length === 0) {
            return undefined;
        }

        // read the object's identifier from its key
        const id = Table.decode(table, entry.key)["id"];
        if (typeof id !== "string") {
            throw new TypeError(`branch row key of ${entry.table} has no string id`);
        }

        return {
            object,
            id,
            change: BranchWrite.readChange(before, after),
            ...(before === undefined ? {} : { before }),
            ...(after === undefined ? {} : { after }),
            fields,
        };
    },

    /** Tell whether a branch creates, removes or updates an object from its rows on both sides. */
    readChange(before: Row | undefined, after: Row | undefined): BranchChange["change"] {
        // create an object missing on the main line
        if (before === undefined) {
            return "created";
        }
        // remove an object missing on the branch
        else if (after === undefined) {
            return "removed";
        }
        // update an object on both sides
        else {
            return "updated";
        }
    },
};
