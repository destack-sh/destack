import { defineAuditAction } from "@destack/audit";
import { identifier, schema } from "@destack/schema";
import { daemonPackage } from "./action.ts";

/** Record instance start under its verified request or execution identity. */
export const startInstance = defineAuditAction({
    package: daemonPackage,
    name: "instance.start",
    version: 1,
    targets: schema.object({
        instance: schema.object({ type: schema.literal("instance"), id: identifier("instance") }),
    }),
    details: schema.object({
        /** Space authorizing the operation. */
        spaceId: identifier("space"),
        /** Prepared deployment selected for execution. */
        deploymentId: identifier("deployment"),
        /** Host authorization epoch checked before execution. */
        hostEpoch: schema.number().int().positive(),
    }),
});

/** Record instance stop under its verified request or execution identity. */
export const stopInstance = defineAuditAction({
    package: daemonPackage,
    name: "instance.stop",
    version: 1,
    targets: schema.object({
        instance: schema.object({ type: schema.literal("instance"), id: identifier("instance") }),
    }),
    details: schema.object({
        /** Space authorizing the operation. */
        spaceId: identifier("space"),
        /** Prepared deployment selected for execution. */
        deploymentId: identifier("deployment"),
        /** Host authorization epoch checked before execution. */
        hostEpoch: schema.number().int().positive(),
    }),
});

/** Record instance restart under its verified request or execution identity. */
export const restartInstance = defineAuditAction({
    package: daemonPackage,
    name: "instance.restart",
    version: 1,
    targets: schema.object({
        instance: schema.object({ type: schema.literal("instance"), id: identifier("instance") }),
    }),
    details: schema.object({
        /** Space authorizing the operation. */
        spaceId: identifier("space"),
        /** Prepared deployment selected for execution. */
        deploymentId: identifier("deployment"),
        /** Host authorization epoch checked before execution. */
        hostEpoch: schema.number().int().positive(),
    }),
});
