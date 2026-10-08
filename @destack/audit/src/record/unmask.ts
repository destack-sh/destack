import { schema } from "@destack/schema";
import { defineAuditAction } from "../declare/index.ts";

/** A read of a scope's events that showed their personal values unmasked. */
export const eventUnmask = defineAuditAction({
    name: "event.unmask",
    /** The object whose grants decided the read: the scope's own or the one the read narrowed to. */
    target: schema.object({ type: schema.string().min(1), id: schema.string().min(1) }),
    details: schema.object({
        /** The kind read. */
        kind: schema.string().min(1),
        /** The read, such as query or tail. */
        operation: schema.string().min(1),
    }),
});
