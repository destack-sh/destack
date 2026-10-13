import { defineSchema, schema, Version } from "@destack/schema";
import { AuditExecution } from "./execution.ts";

/** A call with its execution, running or ended, as a journal records it. */
export const JournalEntry = defineSchema(
    schema.object({
        /** The audit action, such as page.create, of the package the context keeps. */
        method: schema.string().min(1),
        /** The method's input, with its scope and target. */
        input: schema.record(schema.string(), schema.json()),
        /** The release of the method's package the call was made against. */
        release: Version,
        /** Who ran the call where, what it affected, and how it ended once it ends. */
        execution: AuditExecution,
    }),
);
/** A call with its execution, running or ended, as a journal records it. */
export type JournalEntry = schema.Infer<typeof JournalEntry>;

/** An ended call with its execution, as journals deliver it and the audit history keeps it. */
export const AuditCall = defineSchema(
    JournalEntry.extend({
        /** Who ran the call where, what it affected, and how it ended. */
        execution: AuditExecution.required({ outcome: true, finishedAt: true }),
    }),
);
/** An ended call with its execution, as journals deliver it and the audit history keeps it. */
export type AuditCall = schema.Infer<typeof AuditCall>;
