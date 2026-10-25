import { defineTable } from "../../table/table.ts";
import { bigint, binary, integer, json, text } from "../../table/column.ts";
import { schema } from "@destack/schema";

/** Notes whose changes stay within the compaction window. */
export const note = defineTable(
    "note",
    {
        /** The note identity. */
        id: text("id").primaryKey(),
        /** The title. */
        title: text("title").notNull(),
        /** The folder the note lives in. */
        scope: text("scope").notNull(),
        /** An optional subtitle. */
        subtitle: text("subtitle"),
        /** An exact counter beyond the safe integer range. */
        views: bigint("views").notNull(),
        /** Structured labels. */
        labels: json("labels", schema.array(schema.string())).notNull(),
        /** The last edit time. */
        editedAt: integer("edited_at").notNull(),
        /** Attached bytes, logged as hexadecimal text. */
        attachment: binary("attachment"),
    },
    { log: {} },
);

/** Note revisions whose changes the log retains forever. */
export const revision = defineTable(
    "revision",
    {
        /** The folder the revision lives in. */
        scope: text("scope").notNull(),
        /** The revised note. */
        noteId: text("note_id").primaryKey(),
        /** The revision number. */
        number: integer("number").primaryKey(),
        /** The revised title. */
        title: text("title").notNull(),
    },
    {
        log: { retention: "history" },
    },
);

/** Unlogged locks. */
export const lease = defineTable("lease", {
    /** The locked name. */
    name: text("name").primaryKey(),
    /** The expiry, in UTC epoch milliseconds. */
    expiresAt: integer("expires_at").notNull(),
});

/** Readings that are only ever appended, pruned once sealed elsewhere. */
export const reading = defineTable(
    "reading",
    {
        /** The reading identity. */
        id: text("id").primaryKey(),
        /** The folder the reading belongs to. */
        scope: text("scope").notNull(),
        /** The value read. */
        value: integer("value").notNull(),
    },
    { log: { appendOnly: true } },
);

/** The log example's tables. */
export const changeTables = [note, revision, lease, reading];

/** A note with exact, structured and binary values. */
export const first = {
    id: "a",
    title: "First",
    scope: "inbox",
    subtitle: null,
    views: 9_007_199_254_740_993n,
    labels: ["draft"],
    editedAt: 1790244000123,
    attachment: new Uint8Array([1, 2, 3]),
};
