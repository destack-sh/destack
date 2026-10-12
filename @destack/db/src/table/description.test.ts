import { sql } from "../sql/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { defineTable, Table, TABLE } from "../table/table.ts";
import { integer, json, text } from "../table/column.ts";
import { index, unique } from "./constraint.ts";
import { describeTable } from "./description.ts";
import { declareState } from "../migration/state.ts";
import { present, schema } from "@destack/schema";
import { TEST_DIALECTS, TestDatabase } from "../test/database.ts";
import { DatabaseError } from "../error/error.ts";

/** Lamps with an enumerated state. */
const lamp = defineTable("describe_lamp", {
    /** The lamp's identifier. */
    id: text("id").primaryKey(),
    /** The state, absent before wiring. */
    state: text("state", { enum: ["on", "off"] }),
});

/** Lights logged under their space, with properties apart from their SQL names. */
const light = defineTable(
    "describe_light",
    {
        /** The light's identifier. */
        id: text("id").primaryKey(),
        /** The space with the light. */
        scope: text("scope").notNull(),
        /** When the light was switched last, in UTC epoch milliseconds. */
        switchedAt: integer("switched_at"),
        /** The wiring key, kept out of the log. */
        wiringKey: text("wiring_key").sensitive(),
    },
    { log: {} },
);

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

test.for(TEST_DIALECTS)(
    "rebuild a declared table from its state under its properties, reading the rows it writes on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [light], { isMigrated: true });
        onTestFinished(() => storage.close());
        await storage.database.insert(light).values({
            id: "a",
            scope: "space-1",
            switchedAt: 1_790_000_000_000,
            wiringKey: "secret",
        });

        // rebuild the table from its state, naming the switch time by its property
        const [state] = declareState([light], dialect);
        const rebuilt = Table.describe(present(state, "the light's state"), {
            switched_at: "switchedAt",
            wiring_key: "wiringKey",
        });

        // describe its columns as the declared ones, keep the unlogged column sensitive and read its rows by property
        expect([
            describeTable(rebuilt, dialect).columns,
            Object.keys(rebuilt[TABLE].logged),
            await storage.database.select().from(rebuilt),
        ]).toEqual([
            describeTable(light, dialect).columns,
            ["id", "scope", "switchedAt"],
            [{ id: "a", scope: "space-1", switchedAt: 1_790_000_000_000, wiringKey: "secret" }],
        ]);
    },
);

test("refuse a table whose unique constraint and index share a name", () => {
    // declare a unique constraint and an index under one name
    const label = defineTable(
        "label",
        { id: text("id").primaryKey(), code: text("code").notNull() },
        {
            constraints: (columns) => [
                unique("label_code").on(columns.code),
                index("label_code").on(columns.code),
            ],
        },
    );

    expect(() => describeTable(label, "postgresql")).toThrow(
        new TypeError(`table ${label[TABLE].sqlName} declares ${label[TABLE].sqlName}_code twice`),
    );
});
