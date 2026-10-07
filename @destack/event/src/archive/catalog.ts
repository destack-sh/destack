import { defineTable, index, integer, json, text } from "@destack/db";
import { schema } from "@destack/schema";

/** The most distinct values a segment lists for one query key before the catalog keeps no set for it. */
export const KEY_INDEX_LIMIT = 64;

/** The distinct values of each query key in a segment, null for a key holding more than the catalog lists. */
export const KeyIndex = schema.record(
    schema.string(),
    schema.array(schema.union([schema.string(), schema.number()])).nullable(),
);
/** The distinct values of each query key in a segment. */
export type KeyIndex = schema.Infer<typeof KeyIndex>;

/** One flushed segment of a kind's events in a scope: the catalog queries choose segments by. */
export const eventSegment = defineTable(
    "segment",
    {
        /** The segment's identity, which names its file. */
        id: text("id").primaryKey(),
        /** The event kind, by its package and name. */
        kind: text("kind").notNull(),
        /** The scope whose events it keeps. */
        scope: text("scope").notNull(),
        /** The earliest event time, in Unix microseconds. */
        from: integer("from").notNull(),
        /** The latest event time, in Unix microseconds. */
        to: integer("to").notNull(),
        /** The event count. */
        rows: integer("rows").notNull(),
        /** The file size, in bytes. */
        bytes: integer("bytes").notNull(),
        /** The distinct values of each query key. */
        keys: json("keys", KeyIndex).notNull(),
        /** The bucket key of the file. */
        file: text("file").notNull(),
        /** The SHA-256 digest of the file, in hexadecimal. */
        digest: text("digest").notNull(),
        /** The digest of the scope's previous segment of the kind, which chains them, absent for the first. */
        previous: text("previous"),
        /** When the segment was flushed, in Unix milliseconds. */
        flushedAt: integer("flushed_at").notNull(),
    },
    {
        constraints: (segment) => [
            index("segment_scope_time").on(segment.kind, segment.scope, segment.to),
            index("segment_scope_flushed").on(segment.kind, segment.scope, segment.flushedAt),
        ],
    },
);
