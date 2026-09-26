import { defineTable } from "../../table/table.ts";
import { bigint, binary, integer, json, text, timestamp } from "../../table/column.ts";
import { primaryKey } from "../../table/constraint.ts";
import { schema } from "@destack/schema";

/** Notes whose changes stay within the compaction window. */
export const note = defineTable(
    "note",
    {
        /** The note identity. */
        id: text("id").primaryKey(),
        /** The title. */
        title: text("title").notNull(),
        /** The folder routing the note's changes. */
        folder: text("folder").notNull(),
        /** An optional summary. */
        summary: text("summary"),
        /** An exact counter beyond the safe integer range. */
        views: bigint("views").notNull(),
        /** Structured labels. */
        labels: json("labels", schema.array(schema.string())).notNull(),
        /** The last edit time. */
        editedAt: timestamp("edited_at").notNull(),
        /** Attached bytes, left out of the log. */
        attachment: binary("attachment"),
    },
    { log: { route: "folder" } },
);

/** Note revisions whose changes are kept forever. */
export const revision = defineTable(
    "revision",
    {
        /** The revised note. */
        noteId: text("note_id").notNull(),
        /** The revision number. */
        number: integer("number").notNull(),
        /** The revised title. */
        title: text("title").notNull(),
    },
    {
        constraints: (revision) => [primaryKey({ columns: [revision.noteId, revision.number] })],
        log: { tier: "history" },
    },
);

/** Short-lived locks whose changes are not logged. */
export const lease = defineTable(
    "lease",
    {
        /** The locked name. */
        name: text("name").primaryKey(),
        /** The expiry in UTC epoch milliseconds. */
        expiresAt: integer("expires_at").notNull(),
    },
    { log: { tier: "none" } },
);

/** The log example's tables. */
export const changeTables = [note, revision, lease];
