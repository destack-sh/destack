import {
    defineDatabaseSchema,
    identifier,
    integer,
    text,
    json,
    table,
    index,
    uniqueIndex,
} from "@destack/db";
import { AuditEvent } from "../../event/index.ts";

/** Immutable searchable history with independent acceptance timestamps. */
export const auditEvent = table(
    "audit_event",
    {
        /** Event identity and optional preceding attempt. */
        id: identifier("id", "audit-event").primaryKey().notNull(),
        attemptId: identifier("attempt_id", "audit-event"),
        /** Authorized collection and producing host. */
        accountId: identifier("account_id", "account"),
        spaceId: identifier("space_id", "space"),
        hostId: identifier("host_id", "host"),
        /** Indexed action and actor fields. */
        action: text("action").notNull(),
        package: text("package").notNull(),
        actor: text("actor").notNull(),
        stage: text("stage", { enum: ["attempt", "result"] }).notNull(),
        outcome: text("outcome", { enum: ["success", "failure", "denied", "cancelled"] }),
        /** Producer time, acceptance time, and validated contents. */
        occurredAt: integer("occurred_at").notNull(),
        recordedAt: integer("recorded_at").notNull(),
        event: json("event", AuditEvent).notNull(),
    },
    (event) => [
        uniqueIndex("audit_attempt_result").on(event.attemptId),
        index("audit_account_time").on(event.accountId, event.recordedAt, event.id),
        index("audit_space_time").on(event.spaceId, event.recordedAt, event.id),
        index("audit_host_time").on(event.hostId, event.recordedAt, event.id),
        index("audit_action_time").on(event.action, event.recordedAt, event.id),
        index("audit_actor_time").on(event.actor, event.recordedAt, event.id),
    ],
);

/** Historical object references indexed independently of live application tables. */
export const auditTarget = table(
    "audit_target",
    {
        /** Remove target indexes together with retained events. */
        eventId: identifier("event_id", "audit-event")
            .notNull()
            .references(() => auditEvent.id, { onDelete: "cascade" }),
        /** Named target and stable object reference. */
        role: text("role").notNull(),
        type: text("type").notNull(),
        id: text("id").notNull(),
    },
    (target) => [
        uniqueIndex("audit_target_role").on(target.eventId, target.role),
        index("audit_target_object").on(target.type, target.id, target.eventId),
    ],
);

/** Delivery progress retained independently of event history. */
export const auditProducer = table("audit_producer", {
    /** Authenticated producer identity, permanently reserved after retirement. */
    id: identifier("id", "audit-producer").primaryKey().notNull(),
    /** Last accepted position and exact payload hash for acknowledgement retries. */
    sequence: integer("sequence").notNull(),
    digest: text("digest").notNull(),
    /** Retired producers cannot submit or replay deliveries. */
    retiredAt: integer("retired_at"),
});

/** Local or regional retained audit history. */
export const auditSchema = defineDatabaseSchema({
    name: "destack-audit",
    tables: { auditEvent, auditTarget, auditProducer },
    migrations: new URL("./migration/", import.meta.url),
});
