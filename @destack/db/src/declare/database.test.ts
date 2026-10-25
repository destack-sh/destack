import { expect, onTestFinished, test } from "@destack/test";
import { TestDatabase } from "../test/database.ts";
import { text } from "../table/column.ts";
import { defineTable, TABLE } from "../table/table.ts";
import { defineDatabase } from "./database.ts";
import { testSimulation } from "@destack/test/world";

/** Users, which another package declares. */
const user = defineTable("user", { id: text("id").primaryKey() });

/** Folders, a tree another package declares. */
const folder = defineTable(
    "folder",
    {
        id: text("id").primaryKey(),
        scope: text("scope").notNull(),
        parentId: text("parent_id"),
    },
    { tree: { id: "id", scope: "scope", parent: "parentId" } },
);

/** Notes, which this package declares. */
const note = defineTable("note", { id: text("id").primaryKey() });

test("declare local and replicated tables, replicating a replicated tree's index, and refuse a table listed twice", () => {
    // store the notes and replicate the users and folders with their tree index
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

    // refuse a table both declared and replicated
    expect(() =>
        defineDatabase({ name: "main", tables: [note, user], replicated: [user] }),
    ).toThrow(new TypeError("duplicate SQL table: destack__db__user"));
});

test("decide replicated tables on a connection by its declaration, and none over bare tables", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // open a database replicating the users and the same tables bare
    const tables = [user, note];
    const opened = await Promise.all([
        TestDatabase.create(defineDatabase({ name: "main", tables: [note], replicated: [user] }), {
            environment: world,
        }),
        TestDatabase.create(tables, { environment: world }),
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
