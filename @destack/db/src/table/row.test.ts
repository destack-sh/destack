import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { defineTable } from "./table.ts";
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
} from "./column.ts";
import { decodeRow, encodeRow } from "./row.ts";

/** A row of every column kind. */
const sample = defineTable("row_sample", {
    /** Text. */
    id: text("id").primaryKey(),
    /** A typed identifier. */
    owner: identifier("owner", "user"),
    /** An integer. */
    count: integer("count"),
    /** A real number. */
    ratio: real("ratio"),
    /** A boolean. */
    isOpen: boolean("is_open"),
    /** Structured JSON. */
    labels: json("labels", schema.array(schema.string())),
    /** Bytes. */
    content: binary("content"),
    /** An exact integer beyond the safe range. */
    views: bigint("views"),
    /** An exact decimal. */
    price: numeric("price"),
    /** An instant. */
    editedAt: timestamp("edited_at"),
});

test("roundtrip every column kind through its JSON form as text", () => {
    const row = {
        id: "a",
        owner: "user-01996ab0-0000-7000-8000-000000000001",
        count: 3,
        ratio: 0.5,
        isOpen: true,
        labels: ["draft"],
        content: new Uint8Array([1, 2, 255]),
        views: 9_007_199_254_740_993n,
        price: "12.50",
        editedAt: new Date("2026-09-26T10:00:00.123Z"),
    };

    // write each value in its JSON form, and read it back after it travels as text
    const encoded = encodeRow(sample, { ...row, missing: 1 });
    expect(encoded).toEqual({
        id: "a",
        owner: "user-01996ab0-0000-7000-8000-000000000001",
        count: 3,
        ratio: 0.5,
        isOpen: true,
        labels: ["draft"],
        content: "AQL/",
        views: "9007199254740993",
        price: "12.50",
        editedAt: 1790416800123,
    });
    expect(decodeRow(sample, JSON.parse(JSON.stringify(encoded)))).toEqual(row);

    // keep null values as null in both directions
    expect(encodeRow(sample, { id: "b", views: null })).toEqual({ id: "b", views: null });
    expect(decodeRow(sample, { id: "b", views: null })).toEqual({ id: "b", views: null });
});
