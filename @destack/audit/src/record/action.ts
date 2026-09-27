import { schema } from "@destack/schema";
import { defineAuditAction } from "../action/index.ts";

/** Procedure execution, separate from any domain action committed by its handler. */
export const invokeService = defineAuditAction({
    name: "Service.invoke",
    version: 1,
    targets: schema.object({
        procedure: schema.object({ type: schema.literal("procedure"), id: schema.string().min(1) }),
    }),
    details: schema.object({ authentication: schema.enum(["public", "identity", "host"]) }),
});
