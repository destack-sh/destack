import { schema } from "@destack/schema";
import { defineAuditAction } from "../action/index.ts";

/** Invoke a service procedure. */
export const invokeService = defineAuditAction({
    name: "Service.invoke",
    targets: schema.object({
        procedure: schema.object({ type: schema.literal("procedure"), id: schema.string().min(1) }),
    }),
    details: schema.object({ authentication: schema.enum(["public", "identity", "host"]) }),
});
