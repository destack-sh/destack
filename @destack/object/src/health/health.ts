import { schema } from "@destack/schema";
import { PackageVersion } from "@destack/package";
import { Procedure, eventIterator } from "../procedure/index.ts";

/** The readiness of a package's server. */
export const HealthStatus = schema.enum(["starting", "up", "down", "draining", "stopped"]);
/** The readiness of a package's server. */
export type HealthStatus = schema.Infer<typeof HealthStatus>;

/** The health of one package's server. */
export const HealthDescription = schema.object({
    /** The package name. */
    name: schema.string().min(1),
    /** The status. */
    status: HealthStatus,
    /** Why the status holds. */
    reason: schema.string().min(1),
});

/** The health procedures. */
export const health = {
    check: Procedure.of({
        package: PackageVersion.of(import.meta.destack.package),
        authentication: "public",
        permission: "anyone",
        audit: false,
    })
        .route({ method: "GET", path: "/health" })
        .output(HealthDescription),
    watch: Procedure.of({
        package: PackageVersion.of(import.meta.destack.package),
        authentication: "public",
        permission: "anyone",
        audit: false,
    })
        .route({ method: "GET", path: "/health/watch" })
        .output(eventIterator(HealthDescription)),
};
