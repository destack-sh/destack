import { defineSchema, schema } from "@destack/schema";
import { AuditCall } from "../record/call.ts";

/** The most calls one batch carries. */
export const MAX_AUDIT_BATCH = 1000;

/** Ended calls delivered together, oldest first. */
export const AuditBatch = defineSchema(
    schema.object({
        /** The calls, oldest first. */
        calls: schema.array(AuditCall).min(1).max(MAX_AUDIT_BATCH),
    }),
);
/** Ended calls delivered together, oldest first. */
export type AuditBatch = schema.Infer<typeof AuditBatch>;
