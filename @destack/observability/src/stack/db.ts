import { defineTable, foreignKey, identifier, integer, text, type Table } from "@destack/db";
import { alert, alertRule, issue } from "../object/index.ts";

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

/** The salt of the current day, which hashes a space's anonymous visitors and is deleted once its day ends. */
export const visitorSalt = defineTable("visitor_salt", {
    /** The day, in UTC, such as 2026-10-07. */
    day: text("day").primaryKey(),
    /** The random salt, in hexadecimal. */
    salt: text("salt").notNull(),
});

/** The observability kind's tables in a space: the issues with their people, the alert rules and alerts, and the visitor salt. */
export const observabilityTables: readonly Table[] = [
    ...[issue, alertRule, alert].flatMap((object) => object.tables),
    issuePerson,
    visitorSalt,
];
