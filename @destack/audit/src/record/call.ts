import { defineSchema, schema } from "@destack/schema";
import { Call } from "@destack/sync";
import { AuditExecution } from "./execution.ts";

/** A call with its execution, running or ended, as a journal records it. */
export const JournalEntry = defineSchema(
    Call.extend({
        /** Who ran the call where, what it affected, and how it ended once it ends. */
        execution: AuditExecution,
    }),
);
/** A call with its execution, running or ended, as a journal records it. */
export type JournalEntry = schema.Infer<typeof JournalEntry>;

/** An ended call with its execution, as journals deliver it and the audit history keeps it. */
export const AuditCall = defineSchema(
    Call.extend({
        /** Who ran the call where, what it affected, and how it ended. */
        execution: AuditExecution.required({ outcome: true, finishedAt: true }),
    }),
);
/** An ended call with its execution, as journals deliver it and the audit history keeps it. */
export type AuditCall = schema.Infer<typeof AuditCall>;
