import { defineSchema, schema } from "@destack/schema";
import { identifier } from "@destack/schema/identifier";
import { AuditContext } from "./context.ts";
import { AuditActionReference } from "../action/action.ts";
import { AuditTarget } from "./target.ts";

/** How an action ended. */
export const AuditOutcome = defineSchema(
    schema.enum(["success", "failure", "denied", "cancelled"]),
);
/** How an action ended. */
export type AuditOutcome = schema.Infer<typeof AuditOutcome>;

/** The observed result of an action. */
export const AuditResult = defineSchema(
    schema.discriminatedUnion("outcome", [
        schema.object({
            /** The action succeeded. */
            outcome: AuditOutcome.extract(["success"]),
        }),
        schema.object({
            /** The action failed, was denied or was cancelled. */
            outcome: AuditOutcome.exclude(["success"]),
            /** The stable code of the failure. */
            errorCode: schema.string().min(1),
        }),
    ]),
);
/** The observed result of an action. */
export type AuditResult = schema.Infer<typeof AuditResult>;

/** A recorded event. */
export const AuditEvent = defineSchema(
    schema.object({
        /** The event identifier. */
        id: identifier("audit-event"),
        /** The attempt this result completes. */
        attemptId: identifier("audit-event").optional(),
        /** The versioned action. */
        action: AuditActionReference,
        /** A committed write, a read of data, or a refused call. */
        category: schema.enum(["activity", "access", "denial"]),
        /** The producer's timestamp, in UTC epoch milliseconds. */
        occurredAt: schema.number().int().nonnegative(),
        /** The origin and authority of the event. */
        context: AuditContext,
        /** The named affected objects. */
        targets: schema.record(schema.string().min(1), AuditTarget),
        /** The details the action schema accepts. */
        details: schema.json(),
        /** The attempt, or the result with its outcome. */
        result: schema.union([
            schema.object({
                /** The event records an attempt. */
                stage: schema.literal("attempt"),
            }),
            ...AuditResult.options.map((result) =>
                result.extend({
                    /** The event records a result. */
                    stage: schema.literal("result"),
                }),
            ),
        ]),
    }),
);
/** A validated event. */
export type AuditEvent = schema.Infer<typeof AuditEvent>;
