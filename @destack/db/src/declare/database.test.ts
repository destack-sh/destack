import { expect, onTestFinished, test } from "@destack/test";
import { TestDatabase } from "../test/database.ts";
import { text } from "../table/column.ts";
import { defineTable, TABLE } from "../table/table.ts";
import { defineDatabase } from "./database.ts";

/** Users, which another service owns. */
const user = defineTable("user", { id: text("id").primaryKey() });

/** Folders, a tree another service owns. */
const folder = defineTable(
    "folder",
    {
        id: text("id").primaryKey(),
        scope: text("scope").notNull(),
        parentId: text("parent_id"),
    },
    { tree: { id: "id", scope: "scope", parent: "parentId" } },
);

/** Notes, which this service owns. */
const note = defineTable("note", { id: text("id").primaryKey() });

test("keep owned tables and copies, copying a copied tree's index, and refuse a table listed twice", () => {
    // own the notes and copy the users and folders with their tree index
    const database = defineDatabase({ name: "main", tables: [note], copies: [user, folder] });
    const tree = folder[TABLE].tree;
    expect({
        tables: database.tables,
        copies: database.spec.copies,
        isCopied: database.tables.map((table) => database.copies(table)),
    }).toEqual({
        tables: [note, user, folder, tree?.ancestors, tree?.revision],
        copies: [
            "destack__db__user",
            "destack__db__folder",
            "destack__db__folder_tree_ancestor",
            "destack__db__folder_tree_ancestor_revision",
        ],
        isCopied: [false, true, true, true, true],
    });

    // refuse a table both owned and copied
    expect(() => defineDatabase({ name: "main", tables: [note, user], copies: [user] })).toThrow(
        new TypeError("duplicate SQL table: destack__db__user"),
    );
});

test("decide copies on a connection by its declaration, and none over bare tables", async () => {
    // open a database copying the users and the same tables bare
    const tables = [user, note];
    const opened = await Promise.all([
        TestDatabase.create(
            "sqlite",
            defineDatabase({ name: "main", tables: [note], copies: [user] }),
        ),
        TestDatabase.create("sqlite", tables),
    ]);
    onTestFinished(async () => {
        await Promise.all(opened.map((opening) => opening.close()));
    });

    // copy the users only in the declared database
    expect(opened.map(({ database }) => tables.map((table) => database.copies(table)))).toEqual([
        [true, false],
        [false, false],
    ]);
});
