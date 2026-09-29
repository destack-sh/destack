import { defineSchema, schema } from "@destack/schema";
import { AuditEvent } from "../event/index.ts";

/**
 * The most events one delivery carries.
 *
 * A history inserts a hundred events in one transaction within tens of milliseconds.
 */
export const AUDIT_BATCH_EVENTS = 100;

/** Events delivered together, oldest first. */
export const AuditBatch = defineSchema(
    schema.object({
        /** The events, oldest first. */
        events: schema.array(AuditEvent).min(1).max(AUDIT_BATCH_EVENTS),
    }),
);
/** Events delivered together, oldest first. */
export type AuditBatch = schema.Infer<typeof AuditBatch>;

/** The history an outbox delivers to. */
export interface AuditDestination {
    /** Store a batch's events once in one transaction. */
    ingest(batch: AuditBatch, options?: { signal?: AbortSignal }): Promise<unknown>;
}
