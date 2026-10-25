import { expect, onTestFinished, test } from "@destack/test";
import * as schemas from "@destack/schema";
import { TestDatabase } from "../test/database.ts";
import {
    bigint,
    binary,
    boolean,
    identifier,
    integer,
    json,
    numeric,
    real,
    text,
    textEnum,
} from "../table/column.ts";
import { defineTable } from "../table/table.ts";
import { asc, count, eq, sql } from "../index.ts";
import { testSimulation } from "@destack/test/world";

/** One column of every kind. */
const sample = defineTable("sample", {
    /** The sample identity. */
    id: identifier("id", "sample").primaryKey(),
    /** Text from a fixed set. */
    kind: textEnum("kind", ["plain", "rich"]).notNull(),
    /** A whole number. */
    count: integer("count").notNull(),
    /** A double. */
    ratio: real("ratio").notNull(),
    /** A flag. */
    isDone: boolean("is_done").notNull(),
    /** Structured data. */
    labels: json("labels", schemas.schema.array(schemas.schema.string())).notNull(),
    /** Bytes. */
    content: binary("content").notNull(),
    /** A whole number beyond double precision. */
    views: bigint("views").notNull(),
    /** An exact decimal. */
    price: numeric("price").notNull(),
    /** An instant. */
    editedAt: integer("edited_at").notNull(),
    /** A missing value. */
    note: text("note"),
});

/** Labels attached to samples. */
const tag = defineTable("tag", {
    /** The tag identity. */
    id: text("id").primaryKey(),
    /** The tagged sample. */
    sampleId: identifier("sample_id", "sample")
        .notNull()
        .references(() => sample.id),
    /** The label. */
    label: text("label").notNull(),
});

/** The identifier of a sample. */
const SampleId = schemas.schema.identifier("sample");

/** A sample with a value of every kind. */
const row = {
    id: SampleId.parse("sample-01996ab0-0000-7000-8000-000000000001"),
    kind: "rich" as const,
    count: -3,
    ratio: 0.25,
    isDone: true,
    labels: ["draft", "ünïcode"],
    content: new Uint8Array([0, 1, 254, 255]),
    views: 9_007_199_254_740_993n,
    price: "12.3400",
    editedAt: 1790244000123,
    note: null,
};

/** Open a migrated test database. */
async function open() {
    const storage = await TestDatabase.create([sample, tag], {
        environment: testSimulation().environment("test"),
        isMigrated: true,
    });
    onTestFinished(() => storage.close());

    return storage.database;
}

test("read back every column kind as written", async () => {
    const database = await open();
    await database.insert(sample).values(row);

    expect(await database.select().from(sample)).toEqual([row]);
});

test("key joined rows by their table names", async () => {
    const database = await open();
    const second = SampleId.parse("sample-01996ab0-0000-7000-8000-000000000002");
    await database.insert(sample).values([row, { ...row, id: second }]);
    await database.insert(tag).values({ id: "t", sampleId: row.id, label: "draft" });

    // join each sample with its tags and include untagged samples
    const rows = await database
        .select({ id: sample.id, tag })
        .from(sample)
        .leftJoin(tag, eq(tag.sampleId, sample.id))
        .orderBy(asc(sample.id));
    expect(rows).toEqual([
        { id: row.id, tag: { id: "t", sampleId: row.id, label: "draft" } },
        { id: second, tag: null },
    ]);
});

test("group rows by a selected expression's name, its bound values bound once per use", async () => {
    const database = await open();
    const second = SampleId.parse("sample-01996ab0-0000-7000-8000-000000000002");
    const third = SampleId.parse("sample-01996ab0-0000-7000-8000-000000000003");
    await database
        .insert(sample)
        .values([row, { ...row, id: second }, { ...row, id: third, kind: "plain" }]);

    // group by a parameterized expression named in the selection
    const prefixed = sql`${"kind:"} || ${sample.kind}`.as("prefixed");
    const rows = await database
        .select({ prefixed, samples: count() })
        .from(sample)
        .groupBy(sql.identifier("prefixed"))
        .orderBy(asc(sql.identifier("prefixed")));

    expect(rows).toEqual([
        { prefixed: "kind:plain", samples: 1 },
        { prefixed: "kind:rich", samples: 2 },
    ]);
});
