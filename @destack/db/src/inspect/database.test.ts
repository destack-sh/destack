import { expect, test } from "@destack/test";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { defineDatabase } from "../declare/database.ts";
import { defineTable } from "../table/table.ts";
import { text } from "../table/column.ts";
import { compareDatabase, describeDatabase } from "./database.ts";

/** Notes of the earlier release, with their titles. */
const earlierNote = defineTable("note", { id: text("id").primaryKey(), title: text("title") });

/** Drafts the later release drops. */
const draft = defineTable("draft", { id: text("id").primaryKey() });

/** Notes of the later release, gaining a body. */
const laterNote = defineTable("note", {
    id: text("id").primaryKey(),
    title: text("title"),
    body: text("body"),
});

/** The package whose releases the entries declare. */
const PACKAGE = {
    id: PackageId.parse("package-01996ab0-0000-7000-8000-00000000c001"),
    name: "@destack/compare-fixture",
};

/** A release's manifest entry of a declaration, its description read back from JSON. */
const entry = (description: unknown, version: string) => ({
    description: schema
        .record(schema.string(), schema.json())
        .parse(JSON.parse(JSON.stringify(description))),
    symbol: {
        package: { ...PACKAGE, version },
        symbol: { module: "src/index.ts", name: "declared" },
    },
});

test("plan a database's table changes between releases once across dialects", () => {
    // describe two releases of the notes database
    const before = describeDatabase(
        defineDatabase({ name: "notes", tables: [earlierNote, draft] }),
    );
    const after = describeDatabase(defineDatabase({ name: "notes", tables: [laterNote] }));

    // add the body column and drop the drafts
    expect(compareDatabase(entry(before, "2026.8.0"), entry(after, "2026.9.0")).steps).toEqual([
        {
            action: "create",
            target: "database/notes/table/destack__db__note/column/body",
            risk: "safe",
            detail: "add column body",
        },
        {
            action: "delete",
            target: "database/notes/table/destack__db__draft",
            risk: "destructive",
            detail: "drop table",
        },
    ]);
});
