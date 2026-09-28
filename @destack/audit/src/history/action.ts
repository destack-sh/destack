import { schema } from "@destack/schema";
import { defineAuditAction } from "../action/index.ts";

/** The scope whose history an action reads or prunes. */
const historyTarget = schema.object({
    scope: schema.object({ type: schema.literal("scope"), id: schema.string() }),
});
/** The empty details of history actions. */
const historyDetails = schema.object({});

/** Export audit history. */
export const auditExport = defineAuditAction({
    name: "Audit.export",
    version: 1,
    targets: historyTarget,
    details: historyDetails,
});

/** Prune audit history. */
export const auditPrune = defineAuditAction({
    name: "Audit.prune",
    version: 1,
    targets: historyTarget,
    details: historyDetails,
});
