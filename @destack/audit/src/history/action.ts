import { schema } from "@destack/schema";
import { defineAuditAction } from "../declare/index.ts";

/** The scope whose history an action reads or prunes. */
const historyTarget = schema.object({
    scope: schema.object({ type: schema.literal("scope"), id: schema.string() }),
});
/** The empty details of history actions. */
const historyDetails = schema.object({});

/** List a page of audit history. */
export const auditList = defineAuditAction({
    name: "audit.list",
    targets: historyTarget,
    details: historyDetails,
});

/** Export audit history. */
export const auditExport = defineAuditAction({
    name: "audit.export",
    targets: historyTarget,
    details: historyDetails,
});

/** Prune audit history. */
export const auditPrune = defineAuditAction({
    name: "audit.prune",
    targets: historyTarget,
    details: historyDetails,
});
