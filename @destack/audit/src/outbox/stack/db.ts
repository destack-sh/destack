import {
    identifier,
    integer,
    json,
    defineTable,
    text,
    index,
    uniqueIndex,
    type Table,
} from "@destack/db";
import { AuditEvent } from "../../event/index.ts";

/** Events awaiting acknowledgement from the configured history. */
export const auditOutbox = defineTable(
    "outbox",
    {
        /** The event identity. */
        id: identifier("id", "audit-event").primaryKey().notNull(),
        /** The complete validated event. */
        event: json("event", AuditEvent).notNull(),
        /** The local commit time, which orders pending deliveries. */
        recordedAt: integer("recorded_at").notNull(),
    },
    {
        constraints: (entry) => [index("outbox_order").on(entry.recordedAt, entry.id)],
    },
);

/** Persistent delivery position for one outbox and destination. */
export const auditSender = defineTable(
    "sender",
    {
        /** The destination the sender delivers to, one per installed outbox. */
        name: text("name").primaryKey().notNull(),
        /** The producer identity the history knows the sender by. */
        id: identifier("id", "audit-producer").notNull(),
        /** The last acknowledged delivery position. */
        sequence: integer("sequence").notNull(),
        /** The event awaiting acknowledgement, null while none is in flight. */
        eventId: identifier("event_id", "audit-event").references(() => auditOutbox.id),
    },
    { constraints: (sender) => [uniqueIndex("sender_id").on(sender.id)] },
);

/** The tables of transactional audit storage, installed beside application tables. */
export const auditOutboxTables: readonly Table[] = [auditOutbox, auditSender];
