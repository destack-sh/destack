import { expect, onTestFinished, test } from "@destack/test";
import { text } from "../table/column.ts";
import { uniqueIndex } from "../table/constraint.ts";
import { defineTable, TABLE } from "../table/table.ts";
import { TEST_DIALECTS, TestDatabase } from "../test/database.ts";
import { classifyError, DatabaseError, UniqueKey } from "./error.ts";

/** Labels of folders, each name once per folder. */
const label = defineTable(
    "error_label",
    {
        /** The label's key. */
        id: text("id").primaryKey(),
        /** The folder the label belongs to. */
        folder: text("folder").notNull(),
        /** The label's name, unique in its folder. */
        name: text("name").notNull(),
    },
    { constraints: (table) => [uniqueIndex("error_label_name").on(table.folder, table.name)] },
);

test("classify driver failures and leave domain errors that wrap them alone", () => {
    // a driver's unique violation
    const driver = Object.assign(new Error("duplicate key value violates unique constraint"), {
        code: "23505",
    });

    // a domain error explaining the same failure its own way
    const domain = new Error("package version was already published", { cause: driver });

    // classify the driver failure, and keep the domain error as thrown
    const classified = [classifyError(driver), classifyError(domain)];
    expect(
        classified.map((error) => (error instanceof DatabaseError ? error.code : error)),
    ).toEqual(["DUPLICATE", domain]);
});

test.for(TEST_DIALECTS)(
    "read the table and key columns a duplicate violates on %s",
    async (dialect) => {
        // insert two labels of one folder under one name
        const storage = await TestDatabase.create(dialect, [label], { isMigrated: true });
        onTestFinished(() => storage.close());
        await storage.database.insert(label).values({ id: "l1", folder: "inbox", name: "red" });
        const failure = await storage.database
            .insert(label)
            .values({ id: "l2", folder: "inbox", name: "red" })
            .then(
                () => undefined,
                (error: unknown) => error,
            );

        // read the violated key by its SQL names
        const table = label[TABLE].sqlName;
        expect(failure instanceof DatabaseError ? [failure.code, failure.key] : failure).toEqual([
            "DUPLICATE",
            { table, columns: ["folder", "name"] },
        ]);
    },
);

test("read the unique key a duplicate violates from SQLite's message, a namespaced one included, and PostgreSQL's detail, absent for an index or expression", () => {
    const postgres = Object.assign(new Error("duplicate key value violates unique constraint"), {
        table_name: "note",
        detail: "Key (scope, name)=(space-1, notes) already exists.",
    });
    const expression = Object.assign(new Error("duplicate key value violates unique constraint"), {
        table_name: "note",
        detail: "Key (lower(name))=(notes) already exists.",
    });

    expect([
        UniqueKey.violated(new Error("UNIQUE constraint failed: note.scope, note.name")),
        UniqueKey.violated(new Error("UNIQUE constraint failed: work.note.scope, work.note.name")),
        UniqueKey.violated(new Error("UNIQUE constraint failed: index 'note_title'")),
        UniqueKey.violated(postgres),
        UniqueKey.violated(expression),
    ]).toEqual([
        { table: "note", columns: ["scope", "name"] },
        { table: "note", columns: ["scope", "name"] },
        undefined,
        { table: "note", columns: ["scope", "name"] },
        undefined,
    ]);
});
