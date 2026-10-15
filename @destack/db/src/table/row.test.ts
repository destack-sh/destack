import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { defineTable, Table, TABLE } from "./table.ts";
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
} from "./column.ts";

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
    editedAt: integer("edited_at"),
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
        editedAt: 1790416800123,
    };

    // round-trip each value through its JSON form
    const encoded = sample[TABLE].encode({ ...row, missing: 1 });
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
    expect(Table.decode(sample, encoded)).toEqual(row);

    // keep null values as null
    expect(sample[TABLE].encode({ id: "b", views: null })).toEqual({ id: "b", views: null });
    expect(Table.decode(sample, { id: "b", views: null })).toEqual({ id: "b", views: null });
});

test("conceal columns by masking the personal values their schemas mark and leaving out the rest", () => {
    // keep a call's personal address inside its data, a personal email and a plain note
    const entry = defineTable("conceal_sample", {
        /** The identity. */
        id: text("id").primaryKey(),
        /** A call, its address personal. */
        data: json(
            "data",
            schema.object({
                method: schema.string(),
                address: schema.sensitive(schema.string(), "personal"),
            }),
        ),
        /** A personal email. */
        email: text("email").personal(),
        /** A plain note. */
        note: text("note"),
    });
    const row = {
        id: "one",
        data: { method: "update", address: "ada@example.com" },
        email: "ada@example.com",
        note: "hello",
    };

    // mask the personal columns readers lack and leave out the plain one, in rows and their JSON form
    expect([
        entry[TABLE].conceal(row, []),
        entry[TABLE].conceal(row, ["data", "email", "note"]),
        entry[TABLE].encodeLogged(row, ["data", "note"]),
    ]).toEqual([
        row,
        { id: "one", data: { method: "update", address: "****" }, email: "****" },
        {
            id: "one",
            data: { method: "update", address: "****" },
            email: "ada@example.com",
        },
    ]);
});
