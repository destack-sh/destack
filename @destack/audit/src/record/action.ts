import { schema } from "@destack/schema";
import { defineAuditAction } from "../declare/index.ts";

/** Invoke a service procedure. */
export const invokeService = defineAuditAction({
    name: "service.invoke",
    targets: schema.object({
        procedure: schema.object({ type: schema.literal("procedure"), id: schema.string().min(1) }),
    }),
    details: schema.object({ authentication: schema.enum(["public", "identity", "host"]) }),
});
