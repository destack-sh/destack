import { defineSchema, schema } from "@destack/schema";
import { identifier } from "@destack/schema/identifier";
import { AuditEvent } from "../event/index.ts";

/** Stable identity of one outbox and its configured destination. */
export const AuditProducerId = identifier("audit-producer");

/** One immutable event at a persistent producer position. */
export const AuditEntry = defineSchema(
    schema.object({
        /** The producer delivering the event. */
        producerId: AuditProducerId,
        /** The producer's delivery position, one past its last acknowledged one. */
        sequence: schema.number().int().positive().max(Number.MAX_SAFE_INTEGER),
        /** The delivered event. */
        event: AuditEvent,
    }),
);
/** A transport request, independent of the action declaration. */
export type AuditEntry = schema.Infer<typeof AuditEntry>;

/** Durable acknowledgement of one producer position. */
export const AuditAcknowledgement = defineSchema(
    schema.object({
        /** The producer whose delivery the history accepted. */
        producerId: AuditProducerId,
        /** The accepted delivery position. */
        sequence: schema.number().int().positive().max(Number.MAX_SAFE_INTEGER),
    }),
);
/** The accepted producer position. */
export type AuditAcknowledgement = schema.Infer<typeof AuditAcknowledgement>;

/** Authenticated history destination supplied by the host. */
export interface AuditDestination {
    /** Accept the event and its producer position in one transaction. */
    ingest(request: AuditEntry, options?: { signal?: AbortSignal }): Promise<AuditAcknowledgement>;
}

/** Background delivery settings supplied by the host. */
export interface AuditOutboxOptions {
    /** Stop delivery and interrupt retry delays. */
    signal: AbortSignal;
    /** Report every failed delivery without exposing event contents. */
    report(error: unknown): void;
    /** The first retry delay after a failed delivery, in milliseconds, doubling up to a minute. */
    retryDelay?: number;
}
