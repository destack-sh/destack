import { defineAuditAction } from "@destack/audit";
import { schema } from "@destack/schema";

/** The machine a tunnel reaches. */
const tunnelTargets = schema.object({
    machine: schema.object({ type: schema.literal("machine"), id: schema.identifier("machine") }),
});

/** A machine opening its tunnel to the relay under the name the relay routes to it. */
export const tunnelOpen = defineAuditAction({
    name: "tunnel.open",
    targets: tunnelTargets,
    details: schema.object({ name: schema.string().min(1) }),
});

/** The relay closing a machine's tunnel once the machine is revoked or keeps no standing key. */
export const tunnelClose = defineAuditAction({
    name: "tunnel.close",
    targets: tunnelTargets,
    details: schema.object({ reason: schema.enum(["machine-revoked", "key-revoked"]) }),
});
