import { schema } from "@destack/schema";
import { defineAuditAction } from "../action/index.ts";

/** Selected audit-history collection. */
const auditTarget = schema.object({
    collection: schema.object({ type: schema.string(), id: schema.string() }),
});
/** History actions omit filters and event contents. */
const auditDetails = schema.object({});

/** Record audit-history get requests and outcomes. */
export const auditGet = defineAuditAction({
    name: "Audit.get",
    version: 1,
    targets: auditTarget,
    details: auditDetails,
});

/** Record audit-history list requests and outcomes. */
export const auditList = defineAuditAction({
    name: "Audit.list",
    version: 1,
    targets: auditTarget,
    details: auditDetails,
});

/** Record audit-history export requests and outcomes. */
export const auditExport = defineAuditAction({
    name: "Audit.export",
    version: 1,
    targets: auditTarget,
    details: auditDetails,
});

/** Record audit-history prune requests and outcomes. */
export const auditPrune = defineAuditAction({
    name: "Audit.prune",
    version: 1,
    targets: auditTarget,
    details: auditDetails,
});

/** Audit-history actions indexed by service operation. */
export const auditAction = {
    get: auditGet,
    list: auditList,
    export: auditExport,
    prune: auditPrune,
};
