import {
    Change,
    type Write,
    Key,
    TABLE,
    type DatabaseConnection,
    type Row,
    type Table,
    type Overlay,
} from "@destack/db";
import { found, schema, type JsonObject } from "@destack/schema";
import type { ObjectType } from "../object/index.ts";

/** The columns a stored branch row keeps: its identifier, table, key, and both sides, absent or null when missing. */
const STORED_ROW = schema.looseObject({
    id: schema.string(),
    table: schema.string(),
    key: schema.record(schema.string(), schema.json()),
    row: schema.record(schema.string(), schema.json()).nullish(),
    before: schema.record(schema.string(), schema.json()).nullish(),
});

/** A write a branch makes to one row over the main line, encoded for storage. */
export interface BranchWrite {
    /** The SQL name of the row's table. */
    readonly table: string;
    /** The row's key columns. */
    readonly key: JsonObject;
    /** The row as the branch leaves it, null when the branch removes it. */
    readonly row: JsonObject | null;
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

/** The writes branches make: captured from calls, written over a database, and read as overlays and changes. */
export const BranchWrite = {
    /** Read a stored branch row with its identifier, a missing side as null. */
    read(stored: Row): BranchWrite & { readonly id: string } {
        const { id, table, key, row, before } = STORED_ROW.parse(stored);

        return { id, table, key, row: row ?? null, before: before ?? null };
    },

    /** Capture the rows a transaction's changes leave different from the main line. */
    capture(written: readonly Write[]): BranchWrite[] {
        // keep each key's first image and last change
        const first = new Map<string, Row | null>();
        const last = new Map<
            string,
            { readonly table: Table; readonly change: (typeof written)[number] }
        >();
        for (const change of written) {
            const table = change.table;
            const name = Key.name(table, change.key);
            if (!first.has(name)) {
                first.set(name, Change.before(change));
            }
            last.set(name, { table, change });
        }

        // keep the rows that differ from the main line's
        return [...last].flatMap(([name, { table, change }]) => {
            // encode the main line's row and the branch's
            const main = first.get(name) ?? null;
            const before = main === null ? null : table[TABLE].encode(main);
            const after = Change.after(change);
            const row = after === null ? null : table[TABLE].encode(after);

            return JSON.stringify(before) === JSON.stringify(row)
                ? []
                : [
                      {
                          table: table[TABLE].sqlName,
                          key: table[TABLE].encode(change.key),
                          row,
                          before,
                      },
                  ];
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
            if (entry.row === null) {
                await database
                    .delete(table)
                    .where(Key.match(table, table[TABLE].decode(entry.key)));
            } else {
                await database.upsert(table, [table[TABLE].decode(entry.row)]);
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
                Key.name(table, table[TABLE].decode(entry.key)),
                entry.row === null ? null : table[TABLE].decode(entry.row),
            );
        }

        return async (table) => byTable.get(table[TABLE].sqlName) ?? new Map();
    },

    /** Decode a branch row as the change it makes to its object, absent when no written field differs. */
    change(entry: BranchWrite, object: ObjectType): BranchChange | undefined {
        // decode both sides
        const table = object.table;
        const before = entry.before === null ? undefined : table[TABLE].decode(entry.before);
        const after = entry.row === null ? undefined : table[TABLE].decode(entry.row);

        // keep the written fields that differ, skipping derived changes alone
        const fields = object.written.filter(
            (name) => JSON.stringify(before?.[name]) !== JSON.stringify(after?.[name]),
        );
        if (fields.length === 0) {
            return undefined;
        }

        // read the object's identifier from its key
        const id = table[TABLE].decode(entry.key)["id"];
        if (typeof id !== "string") {
            throw new TypeError(`branch row key of ${entry.table} has no string id`);
        }

        return {
            object,
            id,
            change: changeOf(before, after),
            ...(before === undefined ? {} : { before }),
            ...(after === undefined ? {} : { after }),
            fields,
        };
    },
};

/** Tell whether a branch creates, removes or updates an object from its rows on both sides. */
function changeOf(before: Row | undefined, after: Row | undefined): BranchChange["change"] {
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
}
