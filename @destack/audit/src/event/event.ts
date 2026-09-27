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

/** An immutable event acknowledged by durable recording. */
export const AuditEvent = defineSchema(
    schema.object({
        /** The producer-generated delivery identity. */
        id: identifier("audit-event"),
        /** The preceding attempt, omitted for atomic database results. */
        attemptId: identifier("audit-event").optional(),
        /** The declaring package and versioned action. */
        action: AuditActionReference,
        /** The producer's timestamp, in UTC epoch milliseconds. */
        occurredAt: schema.number().int().nonnegative(),
        /** The host-attested origin and authority. */
        context: AuditContext,
        /** Named historical object references. */
        targets: schema.record(schema.string().min(1), AuditTarget),
        /** Fields accepted by the declared details schema. */
        details: schema.json(),
        /** The recorded stage: an attempt without an outcome, or a result with its outcome. */
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
/** A validated immutable event. */
export type AuditEvent = schema.Infer<typeof AuditEvent>;
