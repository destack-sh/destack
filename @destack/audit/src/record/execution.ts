import { defineSchema, Instant, schema } from "@destack/schema";
import { Outcome } from "@destack/sync";
import { AuditContext } from "./context.ts";

/** The largest stored execution, in bytes. */
export const MAX_EXECUTION_BYTES = 65536;

/** The categories of a call: a write, a read of data, or a refused call. */
export const AUDIT_CATEGORIES = ["activity", "access", "denial"] as const;

/** The ways a call ends. */
export const AUDIT_OUTCOMES = ["success", "failure", "denied", "cancelled"] as const;

/** The object a call acts on. */
export const AuditTarget = defineSchema(
    schema.object({
        /** The object type. */
        type: schema.string().min(1),
        /** The object identifier. */
        id: schema.string().min(1),
    }),
);
/** The object a call acts on. */
export type AuditTarget = schema.Infer<typeof AuditTarget>;

/** The execution of a call: who ran it where, what it affected, and how it ended. */
export const AuditExecution = defineSchema(
    schema.object({
        /** The call's identity. */
        id: schema.identifier("call"),
        /** The request the call belongs to, which retries repeat. */
        requestId: schema.string().min(1).exactOptional(),
        /** A write, a read of data, or a refused call. */
        category: schema.enum(AUDIT_CATEGORIES),
        /** The authority and origin. */
        context: AuditContext,
        /** The object the call acts on. */
        target: AuditTarget,
        /** The details the method's audit schema accepts, sensitive values redacted. */
        details: schema.json(),
        /** The digest of the input, sensitive values fingerprinted under the call key. */
        digest: schema.string().exactOptional(),
        /** The transaction identity of the call's logged changes. */
        transaction: schema.string().exactOptional(),
        /** How the call ended, absent while it runs. */
        outcome: Outcome.exactOptional(),
        /** The start time, in UTC epoch milliseconds. */
        startedAt: Instant,
        /** The end time, in UTC epoch milliseconds, absent while it runs. */
        finishedAt: Instant.exactOptional(),
    }),
);
/** The execution of a call: who ran it where, what it affected, and how it ended. */
export type AuditExecution = schema.Infer<typeof AuditExecution>;
