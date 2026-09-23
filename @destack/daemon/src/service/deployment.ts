import { identifier, schema } from "@destack/schema";
import { PackageId } from "@destack/package";
import { defineProcedure, eventIterator } from "@destack/service";
import { Creation } from "@destack/service/procedure";
import { page, PageRequest } from "@destack/service/page";
import definition from "../../destack.json" with { type: "json" };

/** Desired execution accepted from the space authority for this host. */
export const DeploymentExecution = schema.object({
    /** Authoritative deployment identity. */
    deploymentId: identifier("deployment"),
    /** Space authorizing execution. */
    spaceId: identifier("space"),
    /** Accepted installation generation. */
    generation: schema.number().int().positive(),
    /** Host authorization epoch verified at acceptance. */
    hostEpoch: schema.number().int().positive(),
    /** Exclusive authorization expiry, in UTC milliseconds. */
    authorizedUntil: schema.number().int(),
    /** Desired execution availability. */
    status: schema.enum(["active", "draining", "stopped"]),
    /** Number of concurrent instances requested on this host. */
    instances: schema.number().int().nonnegative(),
    /** Authority-selected behavior after an instance exits. */
    restart: schema.enum(["never", "on-failure", "always"]),
    /** Last authority verification, in UTC milliseconds. */
    verifiedAt: schema.number().int(),
});

/** Select an authoritative deployment independently of its execution lifetimes. */
const key = schema.object({ spaceId: identifier("space"), deploymentId: identifier("deployment") });

/** Reconcile durable host execution with authoritative deployment configuration. */
export const deployment = {
    list: access("read")
        .route({ method: "GET", path: "/deployments" })
        .input(PageRequest.extend({ spaceId: identifier("space").optional() }))
        .output(page(DeploymentExecution)),
    get: access("read")
        .route({ method: "GET", path: "/deployments/{deploymentId}" })
        .input(key)
        .output(DeploymentExecution),
    watch: access("read")
        .route({ method: "GET", path: "/deployments/{deploymentId}/watch" })
        .input(key)
        .output(eventIterator(DeploymentExecution)),
    /** Fetch current authority instructions; request fields grant no execution permissions. */
    apply: access("apply")
        .route({ method: "POST", path: "/deployments/{deploymentId}/apply" })
        .input(key.extend(Creation.shape).extend({ generation: schema.number().int().positive() }))
        .output(DeploymentExecution),
};

/** Require private host administration before accepting deployment instructions. */
function access(name: "read" | "apply") {
    return defineProcedure({
        authentication: "host",
        permission: { packageId: PackageId.parse(definition.id), type: "deployment", name },
        audit: name !== "read",
    });
}
