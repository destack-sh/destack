import {
    defineDatabaseSchema,
    identifier,
    integer,
    json,
    table,
    text,
    index,
    uniqueIndex,
} from "@destack/db";
import { AuditEvent } from "../../event/index.ts";

/** Events awaiting acknowledgement from the configured history. */
export const auditOutbox = table(
    "audit_outbox",
    {
        /** Immutable event identity and contents. */
        id: identifier("id", "audit-event").primaryKey().notNull(),
        event: json("event", AuditEvent).notNull(),
        content: text("content").notNull(),
        /** Local commit time used to select pending work. */
        recordedAt: integer("recorded_at").notNull(),
    },
    (entry) => [index("audit_outbox_order").on(entry.recordedAt, entry.id)],
);

/** Persistent delivery position for one outbox and destination. */
export const auditSender = table(
    "audit_sender",
    {
        /** One sender per installed outbox schema. */
        name: text("name").primaryKey().notNull(),
        id: identifier("id", "audit-producer").notNull(),
        /** Last acknowledged delivery and the one currently awaiting acknowledgement. */
        sequence: integer("sequence").notNull(),
        eventId: identifier("event_id", "audit-event").references(() => auditOutbox.id),
    },
    (sender) => [uniqueIndex("audit_sender_id").on(sender.id)],
);

/** Transactional audit storage installed beside application tables. */
export const auditOutboxSchema = defineDatabaseSchema({
    name: "destack-audit-outbox",
    tables: { auditOutbox, auditSender },
    migrations: new URL("./migration/", import.meta.url),
});
