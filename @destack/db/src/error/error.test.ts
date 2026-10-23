import { expect, onTestFinished, test } from "@destack/test";
import { text } from "../table/column.ts";
import { uniqueIndex } from "../table/constraint.ts";
import { defineTable, TABLE } from "../table/table.ts";
import { TestDatabase } from "../test/database.ts";
import { classifyError, DatabaseError, type DeclaredNames, violatedKeyOf } from "./error.ts";

/** Notes as a database declares them, their title column named apart from its property. */
const NOTES: DeclaredNames = new Map([
    [
        "note",
        {
            name: "notes",
            columns: new Map([
                ["scope", "scope"],
                ["title_text", "title"],
            ]),
        },
    ],
]);

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

    // a domain error explaining the same failure differently
    const domain = new Error("package version was already published", { cause: driver });

    // classify the driver failure, and rethrow the domain error as thrown
    const classified = [classifyError(driver, NOTES), classifyError(domain, NOTES)];
    expect(classified.map((error) => (DatabaseError.is(error) ? error.reason : error))).toEqual([
        "DUPLICATE",
        domain,
    ]);
});

test("read the table and key columns a duplicate violates", async () => {
    // insert two labels of one folder under one name
    const storage = await TestDatabase.create([label], { isMigrated: true });
    onTestFinished(() => storage.close());
    await storage.database.insert(label).values({ id: "l1", folder: "inbox", name: "red" });
    const failure = await storage.database
        .insert(label)
        .values({ id: "l2", folder: "inbox", name: "red" })
        .then(
            () => undefined,
            (error: unknown) => error,
        );

    // read the violated key by the names the table declares
    expect(
        DatabaseError.is(failure, "DUPLICATE") ? [failure.reason, failure.data] : failure,
    ).toEqual(["DUPLICATE", { table: label[TABLE].name, columns: ["folder", "name"] }]);
});

test("read the unique key a duplicate violates by its declared names from SQLite's message, a namespaced one included, and PostgreSQL's detail, absent for an index, an expression or an undeclared table", () => {
    const postgres = Object.assign(new Error("duplicate key value violates unique constraint"), {
        table_name: "note",
        detail: "Key (scope, title_text)=(space-1, notes) already exists.",
    });
    const expression = Object.assign(new Error("duplicate key value violates unique constraint"), {
        table_name: "note",
        detail: "Key (lower(title_text))=(notes) already exists.",
    });

    expect([
        violatedKeyOf(new Error("UNIQUE constraint failed: note.scope, note.title_text"), NOTES),
        violatedKeyOf(
            new Error("UNIQUE constraint failed: work.note.scope, work.note.title_text"),
            NOTES,
        ),
        violatedKeyOf(new Error("UNIQUE constraint failed: index 'note_title'"), NOTES),
        violatedKeyOf(postgres, NOTES),
        violatedKeyOf(expression, NOTES),
        violatedKeyOf(new Error("UNIQUE constraint failed: task.scope, task.title"), NOTES),
    ]).toEqual([
        { table: "notes", columns: ["scope", "title"] },
        { table: "notes", columns: ["scope", "title"] },
        null,
        { table: "notes", columns: ["scope", "title"] },
        null,
        null,
    ]);
});
