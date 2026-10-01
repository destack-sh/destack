import { defineSchema, Instant, schema } from "@destack/schema";
import { identifier } from "@destack/schema/identifier";
import { Outcome } from "@destack/sync";
import { AuditContext } from "./context.ts";

/** The largest stored execution, in bytes. */
export const MAX_EXECUTION_BYTES = 65536;

/** An object a call affected. */
export const AuditTarget = defineSchema(
    schema.object({
        /** The object type. */
        type: schema.string().min(1),
        /** The object identifier. */
        id: schema.string().min(1),
        /** The display name at the time of the call. */
        name: schema.string().optional(),
        /** The object version. */
        version: schema.string().optional(),
    }),
);
/** An object a call affected. */
export type AuditTarget = schema.Infer<typeof AuditTarget>;

/** The execution of a call: who ran it where, what it affected, and how it ended. */
export const AuditExecution = defineSchema(
    schema.object({
        /** The call's identity. */
        id: identifier("call"),
        /** The request the call belongs to, which retries repeat. */
        requestId: schema.string().min(1).optional(),
        /** A write, a read of data, or a refused call. */
        category: schema.enum(["activity", "access", "denial"]),
        /** The authority and origin. */
        context: AuditContext,
        /** The named affected objects. */
        targets: schema.record(schema.string().min(1), AuditTarget),
        /** The details the method's audit schema accepts, sensitive values redacted. */
        details: schema.json(),
        /** The digest of the input, sensitive values fingerprinted under the call key. */
        digest: schema.string().optional(),
        /** The transaction identity of the call's logged changes. */
        transaction: schema.string().optional(),
        /** How the call ended, absent while it runs. */
        outcome: Outcome.optional(),
        /** The start time, in UTC epoch milliseconds. */
        startedAt: Instant,
        /** The end time, in UTC epoch milliseconds, absent while it runs. */
        finishedAt: Instant.optional(),
    }),
);
/** The execution of a call: who ran it where, what it affected, and how it ended. */
export type AuditExecution = schema.Infer<typeof AuditExecution>;
