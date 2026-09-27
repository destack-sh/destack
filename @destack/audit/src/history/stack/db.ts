import {
    identifier,
    integer,
    text,
    json,
    defineTable,
    index,
    uniqueIndex,
    type Table,
} from "@destack/db";
import { AuditEvent, AuditOutcome } from "../../event/index.ts";

/** Immutable searchable history with independent acceptance timestamps. */
export const auditEvent = defineTable(
    "event",
    {
        /** The event identity. */
        id: identifier("id", "audit-event").primaryKey().notNull(),
        /** The attempt a result completes, null for an attempt or a standalone event. */
        attemptId: identifier("attempt_id", "audit-event"),
        /** The scope whose history holds the event, routing its changes. */
        scope: text("scope").notNull(),
        /** The action's name. */
        action: text("action").notNull(),
        /** The package declaring the action. */
        packageId: identifier("package_id", "package").notNull(),
        /** The actor's key. */
        actor: text("actor").notNull(),
        /** Whether the event records an attempt or its result. */
        stage: text("stage", { enum: ["attempt", "result"] }).notNull(),
        /** The result's outcome, null for an attempt. */
        outcome: text("outcome", {
            enum: AuditOutcome.options as [AuditOutcome, ...AuditOutcome[]],
        }),
        /** The producer's time of the event in UTC milliseconds. */
        occurredAt: integer("occurred_at").notNull(),
        /** The time the history accepted the event in UTC milliseconds. */
        recordedAt: integer("recorded_at").notNull(),
        /** The complete validated event. */
        event: json("event", AuditEvent).notNull(),
    },
    {
        log: { tier: "window" },
        constraints: (event) => [
            uniqueIndex("event_attempt_result").on(event.attemptId),
            index("event_scope_time").on(event.scope, event.recordedAt, event.id),
            index("event_action_time").on(event.action, event.recordedAt, event.id),
            index("event_package_time").on(event.packageId, event.recordedAt, event.id),
            index("event_actor_time").on(event.actor, event.recordedAt, event.id),
        ],
    },
);

/** Historical object references indexed independently of live application tables. */
export const auditTarget = defineTable(
    "target",
    {
        /** The target identity. */
        id: identifier("id", "audit-target").primaryKey().notNull(),
        /** The event naming the target, removing the target with it. */
        event: identifier("event_id", "audit-event")
            .notNull()
            .references(() => auditEvent.id, { onDelete: "cascade" }),
        /** The scope of the event's history, routing its changes. */
        scope: text("scope").notNull(),
        /** The name of the target within its event. */
        role: text("role").notNull(),
        /** The object's type. */
        type: text("type").notNull(),
        /** The object's identifier. */
        objectId: text("object_id").notNull(),
    },
    {
        log: { tier: "window" },
        constraints: (target) => [
            uniqueIndex("target_role").on(target.event, target.role),
            index("target_object").on(target.type, target.objectId, target.event),
        ],
    },
);

/** Delivery progress retained independently of event history. */
export const auditProducer = defineTable("producer", {
    /** The authenticated producer identity, reserved for good after retirement. */
    id: identifier("id", "audit-producer").primaryKey().notNull(),
    /** The last accepted delivery position. */
    sequence: integer("sequence").notNull(),
    /** The hash of the last accepted delivery, which acknowledges its retries, null before the first. */
    digest: text("digest"),
    /** The retirement time in UTC milliseconds, after which the producer submits nothing. */
    retiredAt: integer("retired_at"),
});

/** The tables of local or regional retained audit history. */
export const auditTables: readonly Table[] = [auditEvent, auditTarget, auditProducer];
