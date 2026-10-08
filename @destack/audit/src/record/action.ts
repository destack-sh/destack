import { schema } from "@destack/schema";
import { defineAuditAction } from "../declare/index.ts";

/** Invoke a service procedure. */
export const serviceInvoke = defineAuditAction({
    name: "service.invoke",
    target: schema.object({ type: schema.literal("procedure"), id: schema.string().min(1) }),
    details: schema.object({ authentication: schema.enum(["public", "identity", "machine"]) }),
});
