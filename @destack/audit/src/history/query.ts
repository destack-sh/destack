import { defineSchema, schema } from "@destack/schema";
import { identifier } from "@destack/schema/identifier";
import { AuditEvent } from "../event/index.ts";
import { AuditActionName } from "../action/index.ts";
import { PackageId } from "@destack/package";

/** The independently authorized audit collection. */
export const AuditScope = defineSchema(
    schema.discriminatedUnion("type", [
        schema.object({ type: schema.literal("global") }),
        schema.object({ type: schema.literal("account"), accountId: identifier("account") }),
        schema.object({
            type: schema.literal("space"),
            accountId: identifier("account"),
            spaceId: identifier("space"),
        }),
        schema.object({ type: schema.literal("host"), hostId: identifier("host") }),
    ]),
);
/** The requested collection. */
export type AuditScope = schema.Infer<typeof AuditScope>;

/** A stable ingestion-order continuation, independent of producer clock skew. */
export const AuditCursor = defineSchema(
    schema.object({
        recordedAt: schema.number().int().nonnegative(),
        id: identifier("audit-event"),
    }),
);
/** A bounded, authorized history query. */
export const AuditQuery = defineSchema(
    schema.object({
        scope: AuditScope,
        action: AuditActionName.optional(),
        packageId: PackageId.optional(),
        actor: schema.string().min(1).optional(),
        target: schema
            .object({ type: schema.string().min(1), id: schema.string().min(1) })
            .optional(),
        attemptId: identifier("audit-event").optional(),
        outcome: schema.enum(["success", "failure", "denied", "cancelled"]).optional(),
        /** Return retained attempts without a retained result. */
        unresolved: schema.boolean().optional(),
        from: schema.number().int().nonnegative().optional(),
        before: schema.number().int().nonnegative().optional(),
        cursor: AuditCursor.optional(),
        limit: schema.number().int().min(1).max(1000),
    }),
);
/** A validated history query. */
export type AuditQuery = schema.Infer<typeof AuditQuery>;

/** An event and its durable ingestion time. */
export const AuditRecord = defineSchema(
    schema.object({ event: AuditEvent, recordedAt: schema.number().int().nonnegative() }),
);
/** A page that can be continued without repeating accepted events. */
export const AuditPage = defineSchema(
    schema.object({ items: schema.array(AuditRecord), cursor: AuditCursor.nullable() }),
);
/** A page of history. */
export type AuditPage = schema.Infer<typeof AuditPage>;
