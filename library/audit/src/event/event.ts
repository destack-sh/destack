import { defineSchema, schema } from "@destack/schema";
import { identifier } from "@destack/schema/identifier";
import { AuditContext } from "./context.ts";
import { AuditActionReference } from "../action/action.ts";
import { AuditTarget } from "./target.ts";

/** The observed result of an action. */
export const AuditResult = defineSchema(
    schema.discriminatedUnion("outcome", [
        schema.object({ outcome: schema.literal("success") }),
        schema.object({
            outcome: schema.enum(["failure", "denied", "cancelled"]),
            errorCode: schema.string().min(1),
        }),
    ]),
);
/** An observed outcome. */
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
        /** An attempt has no outcome until a separate result is recorded. */
        result: schema.union([
            schema.object({ stage: schema.literal("attempt") }),
            schema.object({ stage: schema.literal("result"), outcome: schema.literal("success") }),
            schema.object({
                stage: schema.literal("result"),
                outcome: schema.enum(["failure", "denied", "cancelled"]),
                errorCode: schema.string().min(1),
            }),
        ]),
    }),
);
/** A validated immutable event. */
export type AuditEvent = schema.Infer<typeof AuditEvent>;
