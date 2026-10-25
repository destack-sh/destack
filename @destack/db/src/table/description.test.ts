import { sql } from "../sql/index.ts";
import { expect, onTestFinished, test, refusal } from "@destack/test";
import { defineTable, Table, TABLE } from "../table/table.ts";
import { integer, json, text, textEnum } from "../table/column.ts";
import { index, unique } from "./constraint.ts";
import { describeTable } from "./description.ts";
import { declareState } from "../migration/state.ts";
import { present, schema } from "@destack/schema";
import { TEST_CIPHER, TestDatabase } from "../test/database.ts";
import { testSimulation } from "@destack/test/world";

/** Lamps with an enumerated state. */
const lamp = defineTable("describe_lamp", {
    /** The lamp's identifier. */
    id: text("id").primaryKey(),
    /** The state, absent before wiring. */
    state: textEnum("state", ["on", "off"]),
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
        /** The wiring key, left out of the log. */
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

test("fill JSON columns with their declared defaults", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create([shelf], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());

    // insert through raw SQL to use the database defaults
    await storage.database.run(
        sql`INSERT INTO ${sql.identifier(shelf[TABLE].sqlName)} (id) VALUES ('a')`,
    );
    expect(await storage.database.select().from(shelf)).toEqual([
        { id: "a", labels: [], layout: {} },
    ]);
});

test("refuse a value outside an enum column's values in the database itself", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create([lamp], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());

    // accept each declared value and none
    await storage.database.insert(lamp).values([
        { id: "a", state: "on" },
        { id: "b", state: null },
    ]);
    expect(
        await refusal(
            storage.database.all(
                sql`INSERT INTO ${sql.identifier(lamp[TABLE].sqlName)} (id, state) VALUES ('c', 'dim')`,
            ),
        ),
    ).toEqual(["BAD_REQUEST", "INVALID_RECORD", "a record fails a declared check"]);
    expect((await storage.database.select().from(lamp)).length).toBe(2);
});

test("rebuild a declared table from its state under its properties, reading the rows it writes", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create([light], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await storage.database.transaction((transaction) =>
        transaction
            .insert(light)
            .values({
                id: "a",
                scope: "space-1",
                switchedAt: 1_790_000_000_000,
                wiringKey: "secret",
            })
            .execute(),
    );

    // rebuild the table from its state, with the switch time under its property name
    const [state] = declareState([light], TestDatabase.target());
    const rebuilt = Table.describe(present(state, "the light's state"), {
        switched_at: "switchedAt",
        wiring_key: "wiringKey",
    });

    // describe its columns as the declared ones, and read its rows by property
    const [copied] = await storage.database.select().from(rebuilt);
    expect([
        describeTable(rebuilt, TestDatabase.target()).columns,
        Object.keys(rebuilt[TABLE].logged),
        { ...copied, wiringKey: undefined },
    ]).toEqual([
        describeTable(light, TestDatabase.target()).columns,
        ["id", "scope", "switchedAt"],
        { id: "a", scope: "space-1", switchedAt: 1_790_000_000_000, wiringKey: undefined },
    ]);

    // copy the unlogged column's sealed text as it is, which the database's cipher opens
    expect(TEST_CIPHER.open(present(copied, "the copied row")["wiringKey"], "wiring_key")).toBe(
        "secret",
    );
});

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
