import { schema } from "@destack/schema";
import { defineAuditAction } from "./action.ts";
import { AuditTarget } from "./execution.ts";

/** A read that showed guarded personal values unmasked. */
export const objectUnmask = defineAuditAction({
    name: "object.unmask",
    /** The object read, or the scope a list read. */
    target: AuditTarget,
    details: schema.object({
        /** The method that read, such as list or get. */
        operation: schema.string().min(1),
    }),
});
