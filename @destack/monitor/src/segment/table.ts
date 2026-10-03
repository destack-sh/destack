import { defineTable, identifier, index, integer, text } from "@destack/db";

/** The sealed segments of each scope's entries: the catalog searches prune by. */
export const monitorSegment = defineTable(
    "segment",
    {
        /** The segment identity. */
        id: identifier("id", "segment").primaryKey(),
        /** The scope whose entries the segment keeps. */
        scope: text("scope").notNull(),
        /** The installation whose entries the segment keeps, absent for the scope's host. */
        installationId: identifier("installation_id", "installation"),
        /** The earliest entry time, in Unix microseconds. */
        from: integer("from").notNull(),
        /** The latest entry time, in Unix microseconds. */
        to: integer("to").notNull(),
        /** The compaction level: 0 for a minute's segment, 1 for an hour's merged one. */
        level: integer("level").notNull(),
        /** The entry count. */
        rows: integer("rows").notNull(),
        /** The file size, in bytes. */
        bytes: integer("bytes").notNull(),
        /** The kinds present: bit 0 for spans, bits 1 to 24 for log severities, bit 25 for points. */
        contents: integer("contents").notNull(),
        /** The bucket key of the segment's file. */
        key: text("key").notNull(),
        /** The sealing time, in Unix milliseconds. */
        createdAt: integer("created_at").notNull(),
    },
    {
        constraints: (segment) => [
            index("segment_installation_time").on(
                segment.scope,
                segment.installationId,
                segment.to,
            ),
        ],
    },
);

/** The monitor's tables. */
export const monitorTables = [monitorSegment];
