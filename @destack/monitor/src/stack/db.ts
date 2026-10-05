import { journal } from "@destack/audit/stack";
import { outbox } from "@destack/service/outbox";
import {
    defineDatabase,
    defineTable,
    foreignKey,
    identifier,
    index,
    integer,
    text,
} from "@destack/db";
import { activity, announcement, subscription } from "@destack/notification";
import { comment, reaction } from "@destack/social";
import { installation, installationRevision, space } from "@destack/space/object";
import { setting } from "@destack/setting/object";
import { host } from "@destack/account/object";
import { alert, alertRule, issue } from "../object/index.ts";

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

/** A person affected by an issue, with the last time they were. */
export const issuePerson = defineTable(
    "issue_person",
    {
        /** The issue's space. */
        scope: text("scope").notNull(),
        /** The issue. */
        issueId: identifier("issue_id", "issue").primaryKey(),
        /** The person's identifier as the exception named it. */
        person: text("person").primaryKey(),
        /** When the person was last affected, in Unix milliseconds. */
        lastSeenAt: integer("last_seen_at").notNull(),
    },
    {
        constraints: (entry) => [
            foreignKey({ columns: [entry.issueId], foreignColumns: [issue.table.id] }).onDelete(
                "cascade",
            ),
        ],
    },
);

/** The monitor's database: the segment catalog, the issues with their people, the alert rules and alerts, the journal and the outbox, with copies of the scopes it monitors, their installations and revisions, and their settings. */
export const monitorDatabase = defineDatabase({
    name: "main",
    tables: [
        monitorSegment,
        ...[
            issue,
            alertRule,
            alert,
            comment,
            reaction,
            activity,
            announcement,
            subscription,
        ].flatMap((object) => object.tables),
        issuePerson,
        journal,
        outbox,
    ],
    copies: [
        space.table,
        host.table,
        installation.table,
        installationRevision.table,
        setting.table,
    ],
});
