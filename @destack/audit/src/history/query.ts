import { defineSchema, schema } from "@destack/schema";
import { identifier } from "@destack/schema/identifier";
import { AuditEvent, AuditActor, AuditOutcome } from "../event/index.ts";
import { AuditActionName } from "../action/index.ts";
import { PackageId } from "@destack/package";

/** The most events one page returns or one prune removes, bounding the rows of one statement. */
export const MAX_AUDIT_BATCH = 1000;

/** The scope whose history an audit query reads, independently authorized. */
export const AuditScope = defineSchema(schema.string().min(1));
/** The requested scope. */
export type AuditScope = schema.Infer<typeof AuditScope>;

/** A stable ingestion-order continuation, independent of producer clock skew. */
export const AuditCursor = defineSchema(
    schema.object({
        /** The acceptance time of the last returned event, in UTC milliseconds. */
        recordedAt: schema.number().int().nonnegative(),
        /** The last returned event. */
        id: identifier("audit-event"),
    }),
);
/** A bounded, authorized history query. */
export const AuditQuery = defineSchema(
    schema.object({
        /** The scope whose history the query reads. */
        scope: AuditScope,
        /** The action name the events carry. */
        action: AuditActionName.optional(),
        /** The package declaring the events' action. */
        packageId: PackageId.optional(),
        /** The actor the events record. */
        actor: AuditActor.optional(),
        /** An object the events name. */
        target: schema
            .object({ type: schema.string().min(1), id: schema.string().min(1) })
            .optional(),
        /** The attempt the events are or complete. */
        attemptId: identifier("audit-event").optional(),
        /** The outcome the results report. */
        outcome: AuditOutcome.optional(),
        /** Return retained attempts without a retained result. */
        unresolved: schema.boolean().optional(),
        /** The inclusive earliest acceptance time, in UTC milliseconds. */
        from: schema.number().int().nonnegative().optional(),
        /** The exclusive latest acceptance time, in UTC milliseconds. */
        before: schema.number().int().nonnegative().optional(),
        /** The position after which the page continues. */
        cursor: AuditCursor.optional(),
        /** The maximum number of events in the page. */
        limit: schema.number().int().min(1).max(MAX_AUDIT_BATCH),
    }),
);
/** A validated history query. */
export type AuditQuery = schema.Infer<typeof AuditQuery>;

/** An event and its durable ingestion time. */
export const AuditRecord = defineSchema(
    schema.object({
        /** The event. */
        event: AuditEvent,
        /** The time the history accepted the event, in UTC milliseconds. */
        recordedAt: schema.number().int().nonnegative(),
    }),
);
/** A page that can be continued without repeating accepted events. */
export const AuditPage = defineSchema(
    schema.object({
        /** The records of the page, in acceptance order. */
        items: schema.array(AuditRecord),
        /** The position after the last record, null when the history ends. */
        cursor: AuditCursor.nullable(),
    }),
);
/** A page of history. */
export type AuditPage = schema.Infer<typeof AuditPage>;

/** A bounded removal of the events a scope's history accepted before a time. */
export const AuditPrune = defineSchema(
    schema.object({
        /** The scope whose history loses the events. */
        scope: AuditScope,
        /** The exclusive latest acceptance time, in UTC milliseconds. */
        before: schema.number().int().nonnegative(),
        /** The most events the prune removes. */
        limit: schema.number().int().min(1).max(MAX_AUDIT_BATCH),
    }),
);
/** A bounded removal of the events a scope's history accepted before a time. */
export type AuditPrune = schema.Infer<typeof AuditPrune>;
