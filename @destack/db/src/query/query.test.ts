import { expect, onTestFinished, test } from "@destack/test";
import * as schemas from "@destack/schema";
import { migrate } from "../migration/database.ts";
import { TEST_DIALECTS, TestDatabase } from "../test/database.ts";
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
    timestamp,
} from "../table/column.ts";
import { defineTable } from "../table/table.ts";
import { asc, eq } from "../index.ts";

/** One column of every kind. */
const sample = defineTable("sample", {
    /** The sample identity. */
    id: identifier("id", "sample").primaryKey(),
    /** Text from a fixed set. */
    kind: text("kind", { enum: ["plain", "rich"] }).notNull(),
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
    /** A decimal kept exact. */
    price: numeric("price").notNull(),
    /** An instant. */
    editedAt: timestamp("edited_at").notNull(),
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
const SampleId = schemas.identifier("sample");

/** A sample holding a value of every kind. */
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
    editedAt: new Date("2026-09-24T10:00:00.123Z"),
    note: null,
};

/** Open a migrated test database of samples and tags. */
async function open(dialect: (typeof TEST_DIALECTS)[number]) {
    const test = await TestDatabase.create(dialect, [sample, tag]);
    onTestFinished(() => test.close());
    await migrate(test.database, [sample, tag]);

    return test.database;
}

test.for(TEST_DIALECTS)("read back every column kind as written on %s", async (dialect) => {
    const database = await open(dialect);
    await database.insert(sample).values(row);

    expect(await database.select().from(sample)).toEqual([row]);
});

test.for(TEST_DIALECTS)("key joined rows by their table names on %s", async (dialect) => {
    const database = await open(dialect);
    const second = SampleId.parse("sample-01996ab0-0000-7000-8000-000000000002");
    await database.insert(sample).values([row, { ...row, id: second }]);
    await database.insert(tag).values({ id: "t", sampleId: row.id, label: "draft" });

    // join each sample with its tags, keeping untagged samples
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
