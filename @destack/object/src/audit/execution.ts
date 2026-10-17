import { Instant, schema, Version } from "@destack/schema";
import { Outcome } from "@destack/sync";
import { CallOrigin } from "./origin.ts";

/** The largest stored execution, in bytes. */
export const MAX_EXECUTION_BYTES = 65536;

/** The categories of a call: a write, a read of data, or a refused call. */
export const AUDIT_CATEGORIES = ["activity", "access", "denial"] as const;

/** The ways a call ends. */
export const AUDIT_OUTCOMES = ["success", "failure", "denied", "cancelled"] as const;

/** The object a call acts on. */
export const AuditTarget = schema.object({
    /** The object type. */
    type: schema.string().min(1),
    /** The object identifier. */
    id: schema.string().min(1),
});
/** The object a call acts on. */
export type AuditTarget = schema.Infer<typeof AuditTarget>;

/** The execution of a call: who ran it where, what it affected, and how it ended. */
export const AuditExecution = schema.object({
    /** The call's identity. */
    id: schema.identifier("call"),
    /** The request the call belongs to, which retries repeat. */
    requestId: schema.string().min(1).exactOptional(),
    /** A write, a read of data, or a refused call. */
    category: schema.enum(AUDIT_CATEGORIES),
    /** The authority and origin. */
    context: CallOrigin,
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
});
/** The execution of a call: who ran it where, what it affected, and how it ended. */
export type AuditExecution = schema.Infer<typeof AuditExecution>;

/** A call with its execution, running or ended, as a journal records it. */
export const JournalEntry = schema.object({
    /** The audit action, such as page.create, of the context's package. */
    method: schema.string().min(1),
    /** The method's input, with its scope and target. */
    input: schema.jsonObject(),
    /** The release of the method's package the call was made against. */
    release: Version,
    /** Who ran the call where, what it affected, and how it ended once it ends. */
    execution: AuditExecution,
});
/** A call with its execution, running or ended, as a journal records it. */
export type JournalEntry = schema.Infer<typeof JournalEntry>;

/** A call event: an ended call with its execution. */
export const CallEvent = JournalEntry.extend({
    /** Who ran the call where, what it affected, and how it ended. */
    execution: AuditExecution.required({ outcome: true, finishedAt: true }),
});
/** A call event: an ended call with its execution. */
export type CallEvent = schema.Infer<typeof CallEvent>;
