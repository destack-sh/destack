import { defineSchema, schema } from "@destack/schema";
import { Call } from "@destack/sync";
import { AuditExecution } from "./execution.ts";

/** A call with its execution, as journals record and the audit history keeps it. */
export const AuditCall = defineSchema(
    Call.extend({
        /** Who ran the call where, what it affected, and how it ended. */
        execution: AuditExecution,
    }),
);
/** A call with its execution, as journals record and the audit history keeps it. */
export type AuditCall = schema.Infer<typeof AuditCall>;
