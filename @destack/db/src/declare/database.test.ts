import { expect, onTestFinished, test } from "@destack/test";
import { TestDatabase } from "../test/database.ts";
import { text } from "../table/column.ts";
import { defineTable, TABLE } from "../table/table.ts";
import { defineDatabase } from "./database.ts";

/** Users, which another package owns. */
const user = defineTable("user", { id: text("id").primaryKey() });

/** Folders, a tree another package owns. */
const folder = defineTable(
    "folder",
    {
        id: text("id").primaryKey(),
        scope: text("scope").notNull(),
        parentId: text("parent_id"),
    },
    { tree: { id: "id", scope: "scope", parent: "parentId" } },
);

/** Notes, which this package owns. */
const note = defineTable("note", { id: text("id").primaryKey() });

test("keep owned and replicated tables, replicating a replicated tree's index, and refuse a table listed twice", () => {
    // own the notes and replicate the users and folders with their tree index
    const database = defineDatabase({ name: "main", tables: [note], replicated: [user, folder] });
    const tree = folder[TABLE].tree;
    expect({
        tables: database.tables,
        replicated: database.spec.replicated,
        isReplicated: database.tables.map((table) => database.replicates(table)),
    }).toEqual({
        tables: [note, user, folder, tree?.ancestors, tree?.revision],
        replicated: [
            "destack__db__user",
            "destack__db__folder",
            "destack__db__folder_tree_ancestor",
            "destack__db__folder_tree_ancestor_revision",
        ],
        isReplicated: [false, true, true, true, true],
    });

    // refuse a table both owned and replicated
    expect(() =>
        defineDatabase({ name: "main", tables: [note, user], replicated: [user] }),
    ).toThrow(new TypeError("duplicate SQL table: destack__db__user"));
});

test("decide replicated tables on a connection by its declaration, and none over bare tables", async () => {
    // open a database replicating the users and the same tables bare
    const tables = [user, note];
    const opened = await Promise.all([
        TestDatabase.create(
            "sqlite",
            defineDatabase({ name: "main", tables: [note], replicated: [user] }),
        ),
        TestDatabase.create("sqlite", tables),
    ]);
    onTestFinished(async () => {
        await Promise.all(opened.map((opening) => opening.close()));
    });

    // replicate the users only in the declared database
    expect(opened.map(({ database }) => tables.map((table) => database.replicates(table)))).toEqual(
        [
            [true, false],
            [false, false],
        ],
    );
});
