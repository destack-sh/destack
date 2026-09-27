import { schema } from "@destack/schema";
import { defineAuditAction } from "../action/index.ts";

/** The scope whose history an action reads or prunes. */
const historyTarget = schema.object({
    scope: schema.object({ type: schema.literal("scope"), id: schema.string() }),
});
/** History actions omit filters and event contents. */
const historyDetails = schema.object({});

/** Record audit-history export requests and outcomes. */
export const auditExport = defineAuditAction({
    name: "Audit.export",
    version: 1,
    targets: historyTarget,
    details: historyDetails,
});

/** Record audit-history prune requests and outcomes. */
export const auditPrune = defineAuditAction({
    name: "Audit.prune",
    version: 1,
    targets: historyTarget,
    details: historyDetails,
});
