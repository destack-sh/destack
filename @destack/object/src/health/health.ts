import { schema } from "@destack/schema";
import { Package } from "@destack/package";
import { defineProcedure, eventIterator } from "../procedure/index.ts";

/** The readiness of a package's server. */
export const HealthStatus = schema.enum([
    "starting",
    "serving",
    "not-serving",
    "draining",
    "stopped",
]);
/** The readiness of a package's server. */
export type HealthStatus = schema.Infer<typeof HealthStatus>;

/** The health of one package's server. */
export const HealthDescription = schema.object({
    /** The package name. */
    name: schema.string().min(1),
    /** The status. */
    status: HealthStatus,
});

/** The health procedures. */
export const health = {
    check: defineProcedure({
        package: Package.of(import.meta.destack.package),
        authentication: "public",
        permission: "anyone",
        audit: false,
    })
        .route({ method: "GET", path: "/health" })
        .output(HealthDescription),
    watch: defineProcedure({
        package: Package.of(import.meta.destack.package),
        authentication: "public",
        permission: "anyone",
        audit: false,
    })
        .route({ method: "GET", path: "/health/watch" })
        .output(eventIterator(HealthDescription)),
};
