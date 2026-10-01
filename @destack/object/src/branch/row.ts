import { Key, TABLE, type DatabaseConnection, type Row, type Table } from "@destack/db";
import type { Change, Overlay } from "@destack/db/log";
import type { ObjectType } from "../object/index.ts";

/** A row a branch changes over the main line, encoded for storage. */
export interface BranchRow {
    /** The SQL name of the row's table. */
    readonly table: string;
    /** The row's key columns. */
    readonly key: Readonly<Record<string, unknown>>;
    /** The row as the branch leaves it, null when the branch removes it. */
    readonly row: Readonly<Record<string, unknown>> | null;
    /** The main line's row the branch replaces, null when the branch creates it. */
    readonly before: Readonly<Record<string, unknown>> | null;
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

/** The rows branches change: captured from writes, written over a database, and read as overlays and changes. */
export const BranchRow = {
    /** Capture the rows a transaction's changes leave different from the main line. */
    capture(written: readonly Omit<Change<Table>, "sequence">[]): BranchRow[] {
        // keep each key's first image and last change
        const first = new Map<string, Row | null>();
        const last = new Map<
            string,
            { readonly table: Table; readonly change: (typeof written)[number] }
        >();
        for (const change of written) {
            const table = change.table as Table;
            const name = Key.name(table, change.key);
            if (!first.has(name)) {
                first.set(name, (change.before as Row | undefined) ?? null);
            }
            last.set(name, { table, change });
        }

        // keep the rows that differ from the main line's
        return [...last].flatMap(([name, { table, change }]) => {
            // encode the main line's row and the branch's
            const main = first.get(name) ?? null;
            const before = main === null ? null : table.encode(main);
            const row = change.after === undefined ? null : table.encode(change.after as Row);

            return JSON.stringify(before) === JSON.stringify(row)
                ? []
                : [
                      {
                          table: table[TABLE].sqlName,
                          key: table.encode(change.key as Row),
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
        rows: readonly BranchRow[],
    ): Promise<void> {
        for (const entry of rows) {
            // remove a row the branch removes, and put the others
            const table = tables.get(entry.table)!;
            if (entry.row === null) {
                await database.delete(table).where(Key.match(table, table.decode(entry.key)));
            } else {
                await database.upsert(table, [table.decode(entry.row)]);
            }
        }
    },

    /** Read branch rows as an overlay, by table and key. */
    overlay(rows: readonly BranchRow[], tables: ReadonlyMap<string, Table>): Overlay {
        const byTable = new Map<string, Map<string, Row | null>>();
        for (const entry of rows) {
            const table = tables.get(entry.table)!;
            const keyed = byTable.get(entry.table) ?? new Map<string, Row | null>();
            byTable.set(entry.table, keyed);
            keyed.set(
                Key.name(table, table.decode(entry.key)),
                entry.row === null ? null : table.decode(entry.row),
            );
        }

        return async (table) => byTable.get(table[TABLE].sqlName) ?? new Map();
    },

    /** Decode a branch row as the change it makes to its object, absent when no written field differs. */
    change(entry: BranchRow, object: ObjectType): BranchChange | undefined {
        // decode both sides
        const table = object.table as Table;
        const before = entry.before === null ? undefined : table.decode(entry.before);
        const after = entry.row === null ? undefined : table.decode(entry.row);

        // keep the written fields that differ, skipping derived changes alone
        const fields = (object.written as readonly string[]).filter(
            (name) => JSON.stringify(before?.[name]) !== JSON.stringify(after?.[name]),
        );
        if (fields.length === 0) {
            return undefined;
        }

        return {
            object,
            id: String(table.decode(entry.key).id),
            change: before === undefined ? "created" : after === undefined ? "removed" : "updated",
            ...(before === undefined ? {} : { before }),
            ...(after === undefined ? {} : { after }),
            fields,
        };
    },
};
