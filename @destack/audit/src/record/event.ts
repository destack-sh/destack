import { defineEventKind } from "@destack/event/declare";
import { schema } from "@destack/schema";
import { audit } from "./access.ts";
import { AuditCall } from "./call.ts";
import { AUDIT_CATEGORIES, AUDIT_OUTCOMES } from "./execution.ts";

/** How long an hour is, in milliseconds. */
const HOUR = 60 * 60 * 1000;

/** How long a scope keeps its calls unless its policy says otherwise: 400 days, as admin activity logs are kept. */
const RETENTION = 400 * 24 * HOUR;

/** A call executed in a scope: who ran which method on what, and how it ended, copied to every scope enclosing it. */
export const call = defineEventKind({
    name: "call",
    description: "A call executed in a scope: who ran which method on what, and how it ended.",
    keys: schema.object({
        /** The actor's key. */
        actor: schema.string(),
        /** The object type and method, such as page.create. */
        method: schema.string(),
        /** The package serving the call. */
        packageId: schema.string(),
        /** A write, a read of data, or a refused call. */
        category: schema.enum(AUDIT_CATEGORIES),
        /** How the call ended. */
        outcome: schema.enum(AUDIT_OUTCOMES),
        /** The identifier of the object the call acts on. */
        target: schema.string(),
        /** The type of the object the call acts on. */
        targetType: schema.string(),
    }),
    data: AuditCall,
    delivery: "exactly-once",
    policy: { flush: { maxAge: HOUR, maxRows: 100_000 }, retention: RETENTION },
    route: "enclosing",
    isLocked: true,
    subject: "actor",
    access: { read: audit.permission("read"), unmask: audit.permission("unmask") },
});
