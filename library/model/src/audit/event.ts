import {
    check,
    dialectSQL,
    identifier,
    index,
    integer,
    json,
    type Select,
    sql,
    table,
    text,
} from "@destack/db";
import { defineSchema, schema } from "@destack/schema";
import * as identifiers from "@destack/schema/identifier";

/** The actor identity and display name recorded when an action occurs. */
export const AuditActor = defineSchema(
    schema.union([
        schema.object({
            type: schema.literal("user"),
            id: identifiers.identifier("user"),
            name: schema.string().optional(),
        }),
        schema.object({
            type: schema.literal("service-account"),
            id: identifiers.identifier("service-account"),
            name: schema.string().optional(),
        }),
        schema.object({ type: schema.literal("system"), name: schema.string().min(1) }),
        schema.object({ type: schema.literal("anonymous") }),
    ]),
);
/** The actor recorded at event time. */
export type AuditActor = schema.Infer<typeof AuditActor>;

/** An affected object, retained independently of its current record. */
export const AuditTarget = defineSchema(
    schema.object({
        /** The namespaced object type. */
        type: schema.string().min(1),
        /** The object's stable identifier. */
        id: schema.string().min(1),
        /** The display name captured at event time. */
        name: schema.string().optional(),
        /** The object or secret version actually accessed. */
        version: schema.string().optional(),
    }),
);
/** An affected object's historical reference. */
export type AuditTarget = schema.Infer<typeof AuditTarget>;

/** An immutable event written by the service performing the action. */
export const auditEvent = table(
    "audit_event",
    {
        /** The producer-assigned identifier used to deduplicate delivery. */
        id: identifier("id", "audit-event").primaryKey().notNull(),
        /** The affected account, absent before an account is known. */
        accountId: identifier("account_id", "account"),
        /** The affected space, when applicable. */
        spaceId: identifier("space_id", "space"),
        /** The namespaced action, such as secret.read or role.assign. */
        action: text("action").notNull(),
        /** The action-specific details schema version. */
        version: integer("version").notNull(),
        /** The time the action occurred, in UTC epoch milliseconds. */
        occurredAt: integer("occurred_at").notNull(),
        /** The time the audit store accepted the event. */
        recordedAt: integer("recorded_at").notNull(),
        /** The authenticated actor or explicit system or anonymous actor. */
        actor: json("actor", AuditActor).notNull(),
        /** Delegating actors, ordered from original initiator to immediate delegator. */
        delegation: json("delegation", schema.array(AuditActor))
            .notNull()
            .default(sql`'[]'`),
        /** All affected objects, with historical names and accessed versions. */
        targets: json("targets", schema.array(AuditTarget)).notNull(),
        /** Whether the operation succeeded, failed, or was denied. */
        outcome: text("outcome", { enum: ["success", "failure", "denied"] }).notNull(),
        /** A stable failure or denial code. */
        errorCode: text("error_code"),
        /** The authenticated service producing the event. */
        service: text("service").notNull(),
        /** The deployment producing the event, when applicable. */
        deploymentId: identifier("deployment_id", "deployment"),
        /** The authenticated device, when applicable. */
        deviceId: identifier("device_id", "device"),
        /** The authentication session reference, without credentials. */
        sessionId: identifier("session_id", "session"),
        /** The software token reference, without credentials. */
        serviceTokenId: identifier("service_token_id", "service-token"),
        /** The request identifier shared by related events. */
        requestId: text("request_id"),
        /** The associated telemetry trace. */
        traceId: text("trace_id"),
        /** The client address obtained from the trusted transport. */
        address: text("address"),
        /** The client-reported user agent. */
        userAgent: text("user_agent"),
        /** Action-specific fields selected and validated by the producing service. */
        details: json("details", schema.record(schema.string(), schema.json())).notNull(),
    },
    (event) => [
        check("audit_version", sql`${event.version} > 0`),
        check(
            "audit_action",
            dialectSQL({
                sqlite: sql`length(${event.action}) > 0 AND instr(${event.action}, '.') > 0`,
                postgresql: sql`length(${event.action}) > 0 AND strpos(${event.action}, '.') > 0`,
            }),
        ),
        check("audit_outcome", sql`${event.outcome} IN ('success', 'failure', 'denied')`),
        check("audit_error", sql`${event.outcome} <> 'success' OR ${event.errorCode} IS NULL`),
        check(
            "audit_space_account",
            sql`${event.spaceId} IS NULL OR ${event.accountId} IS NOT NULL`,
        ),
        index("audit_account_time").on(event.accountId, event.occurredAt, event.id),
        index("audit_space_time").on(event.spaceId, event.occurredAt, event.id),
        index("audit_request").on(event.requestId),
    ],
);
/** A persisted audit event. */
export type AuditEvent = Select<typeof auditEvent>;
