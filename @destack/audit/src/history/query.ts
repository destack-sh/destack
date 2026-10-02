import { AuditActor } from "../record/actor.ts";
import { AuditCall } from "../record/call.ts";
import { defineSchema, Instant, schema, identifier } from "@destack/schema";
import { AuditActionName } from "../action/index.ts";
import { PackageId } from "@destack/package";

/** The most calls one page returns, one batch carries or one prune removes. */
export const MAX_AUDIT_BATCH = 1000;

/** The scope whose history a query reads. */
export const AuditScope = defineSchema(schema.string().min(1));
/** The requested scope. */
export type AuditScope = schema.Infer<typeof AuditScope>;

/** The position after a call in acceptance order. */
export const AuditCursor = defineSchema(
    schema.object({
        /** The acceptance time of the last returned call, in UTC milliseconds. */
        recordedAt: Instant,
        /** The last returned call. */
        id: identifier("call"),
    }),
);
/** A history query. */
export const AuditQuery = defineSchema(
    schema.object({
        /** The scope whose history the query reads. */
        scope: AuditScope,
        /** The method the calls ran. */
        method: AuditActionName.exactOptional(),
        /** The package serving the calls. */
        packageId: PackageId.exactOptional(),
        /** The actor the calls record. */
        actor: AuditActor.exactOptional(),
        /** An object the calls name. */
        target: schema
            .object({ type: schema.string().min(1), id: schema.string().min(1) })
            .exactOptional(),
        /** The category of the calls. */
        category: schema.enum(["activity", "access", "denial"]).exactOptional(),
        /** How the calls ended. */
        outcome: schema.enum(["success", "failure", "denied", "cancelled"]).exactOptional(),
        /** Whether to return only calls still running. */
        isRunning: schema.boolean().exactOptional(),
        /** The inclusive earliest acceptance time, in UTC milliseconds. */
        from: Instant.exactOptional(),
        /** The exclusive latest acceptance time, in UTC milliseconds. */
        before: Instant.exactOptional(),
        /** The position after which the page continues. */
        cursor: AuditCursor.exactOptional(),
        /** The maximum number of calls in the page. */
        limit: schema.number().int().min(1).max(MAX_AUDIT_BATCH),
    }),
);
/** A validated history query. */
export type AuditQuery = schema.Infer<typeof AuditQuery>;

/** A call and its acceptance time. */
export const AuditRecord = defineSchema(
    schema.object({
        /** The call. */
        call: AuditCall,
        /** The time the history accepted the call, in UTC milliseconds. */
        recordedAt: Instant,
    }),
);

/** Calls delivered together, oldest first. */
export const AuditBatch = defineSchema(
    schema.object({
        /** The calls, oldest first. */
        calls: schema.array(AuditCall).min(1).max(MAX_AUDIT_BATCH),
    }),
);
/** Calls delivered together, oldest first. */
export type AuditBatch = schema.Infer<typeof AuditBatch>;
/** A page of history. */
export const AuditPage = defineSchema(
    schema.object({
        /** The records of the page, in acceptance order. */
        items: schema.array(AuditRecord),
        /** The position after the last record, null when the history ends. */
        cursor: AuditCursor.nullable(),
    }),
);
/** A page of history. */
export type AuditPage = schema.Infer<typeof AuditPage>;

/** A removal of a scope's calls accepted before a time. */
export const AuditPrune = defineSchema(
    schema.object({
        /** The scope whose history loses the calls. */
        scope: AuditScope,
        /** The exclusive latest acceptance time, in UTC milliseconds. */
        before: Instant,
        /** The most calls the prune removes. */
        limit: schema.number().int().min(1).max(MAX_AUDIT_BATCH),
    }),
);
/** A removal of a scope's calls accepted before a time. */
export type AuditPrune = schema.Infer<typeof AuditPrune>;
