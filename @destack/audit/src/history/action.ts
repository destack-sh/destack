import { schema } from "@destack/schema";
import { defineAuditAction } from "../action/index.ts";
import manifest from "../../package.json" with { type: "json" };
import definition from "../../destack.json" with { type: "json" };
import { PackageId } from "@destack/package";

/** Package declaring audit-history operations. */
const auditPackage = {
    id: PackageId.parse(definition.id),
    name: manifest.name,
    version: manifest.version,
};
/** Selected audit-history collection. */
const auditTarget = schema.object({
    collection: schema.object({ type: schema.string(), id: schema.string() }),
});
/** History actions omit filters and event contents. */
const auditDetails = schema.object({});

/** Record audit-history get requests and outcomes. */
export const auditGet = defineAuditAction({
    package: auditPackage,
    name: "audit.get",
    version: 1,
    targets: auditTarget,
    details: auditDetails,
});

/** Record audit-history list requests and outcomes. */
export const auditList = defineAuditAction({
    package: auditPackage,
    name: "audit.list",
    version: 1,
    targets: auditTarget,
    details: auditDetails,
});

/** Record audit-history export requests and outcomes. */
export const auditExport = defineAuditAction({
    package: auditPackage,
    name: "audit.export",
    version: 1,
    targets: auditTarget,
    details: auditDetails,
});

/** Record audit-history prune requests and outcomes. */
export const auditPrune = defineAuditAction({
    package: auditPackage,
    name: "audit.prune",
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
