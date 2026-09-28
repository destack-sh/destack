import { expect, onTestFinished, test } from "@destack/test";
import { sql } from "drizzle-orm";
import { defineTable, TABLE } from "../table/table.ts";
import { json, text } from "../table/column.ts";
import { schema } from "@destack/schema";
import { TEST_DIALECTS, TestDatabase } from "../test/database.ts";
import { DatabaseError } from "../error/error.ts";

/** Lamps with an enumerated state. */
const lamp = defineTable("describe_lamp", {
    /** The lamp's identifier. */
    id: text("id").primaryKey(),
    /** The state, absent before wiring. */
    state: text("state", { enum: ["on", "off"] }),
});

/** Shelves with empty defaults. */
const shelf = defineTable("describe_shelf", {
    /** The shelf's identifier. */
    id: text("id").primaryKey(),
    /** The labels on the shelf. */
    labels: json("labels", schema.array(schema.string())).notNull().default([]),
    /** The shelf's layout. */
    layout: json("layout", schema.record(schema.string(), schema.number())).notNull().default({}),
});

test.for(TEST_DIALECTS)("fill JSON columns with their declared defaults on %s", async (dialect) => {
    const storage = await TestDatabase.create(dialect, [shelf], { isMigrated: true });
    onTestFinished(() => storage.close());

    // insert through raw SQL to use the database defaults
    await storage.database.execute(
        sql`INSERT INTO ${sql.identifier(shelf[TABLE].sqlName)} (id) VALUES ('a')`,
    );
    expect(await storage.database.select().from(shelf)).toEqual([
        { id: "a", labels: [], layout: {} },
    ]);
});

test.for(TEST_DIALECTS)(
    "refuse a value outside an enum column's values in the database itself on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [lamp], { isMigrated: true });
        onTestFinished(() => storage.close());

        // accept each declared value and none
        await storage.database.insert(lamp).values([
            { id: "a", state: "on" },
            { id: "b", state: null },
        ]);
        await expect(
            storage.database.execute(
                sql`INSERT INTO ${sql.identifier(lamp[TABLE].sqlName)} (id, state) VALUES ('c', 'dim')`,
            ),
        ).rejects.toThrow(new DatabaseError("INVALID_RECORD", "a record fails a declared check"));
        expect((await storage.database.select().from(lamp)).length).toBe(2);
    },
);
