import { AUDIT_CATEGORIES, AUDIT_OUTCOMES, AuditCall } from "./execution.ts";
import { field } from "../field/field.ts";
import { defineObject } from "../object/object.ts";

/** A call executed in a scope: who ran which method on what, and how it ended. */
export const call = defineObject({
    name: "call",
    plural: "calls",
    // keep every call once, its segments locked for 400 days
    storage: {
        kind: "events",
        flush: { maxAge: { hours: 1 }, maxRows: 100_000 },
        retention: { days: 400 },
        delivery: "exactly-once",
        isLocked: true,
        subject: "actor",
    },
    fields: {
        /** The ended call, its personal values masked unless the reader may unmask them. */
        data: field.json(AuditCall).guard({ read: "unmask" }),
        /** The actor's key. */
        actor: field.string(),
        /** The object type and method, such as page.create. */
        method: field.string(),
        /** The package serving the call. */
        packageId: field.string(),
        /** A write, a read of data, or a refused call. */
        category: field.enum(AUDIT_CATEGORIES),
        /** How the call ended. */
        outcome: field.enum(AUDIT_OUTCOMES),
        /** The identifier of the object the call acts on. */
        target: field.string(),
        /** The type of the object the call acts on. */
        targetType: field.string(),
    },
    permissions: ["read", "unmask"],
}).methods();
