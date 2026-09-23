import { PackageId } from "@destack/package/package";
import packageDefinition from "../../destack.json" with { type: "json" };
import { identifier, schema } from "@destack/schema";
import { eventIterator } from "@destack/service";
import { Creation, defineProcedure } from "@destack/service/procedure";
import { page, PageRequest } from "@destack/service/page";
import { OperationError } from "@destack/service/operation";
import { Conditions } from "@destack/model";

/** An observed instance of an authorized deployment on this host. */
export const Instance = schema.object({
    /** Running instance identifier, independent of its installation and deployment. */
    id: identifier("instance"),
    /** Space supplying resources and authorization. */
    spaceId: identifier("space"),
    /** Prepared deployment executed by this instance. */
    deploymentId: identifier("deployment"),
    /** Host executing this instance. */
    hostId: identifier("host"),
    /** Host authorization epoch checked before execution. */
    hostEpoch: schema.number().int().positive(),
    /** Current process lifecycle status. */
    status: schema.enum(["starting", "running", "draining", "stopped", "failed"]),
    /** Last observation time in UTC milliseconds. */
    observedAt: schema.number().int(),
    /** Execution start time in UTC milliseconds. */
    startedAt: schema.number().int().nullable(),
    /** Execution stop time in UTC milliseconds. */
    stoppedAt: schema.number().int().nullable(),
    /** Last execution failure, absent when the instance has not failed. */
    error: OperationError.nullable(),
    /** Readiness and health observations, independent of process lifecycle status. */
    conditions: Conditions,
});

/** Workload supervision requires a prepared deployment and current host authorization. */
export const instance = {
    /** Replace an execution with a new identity without changing deployment availability. */
    restart: access("restart")
        .route({ method: "POST", path: "/instances/{instanceId}/restart" })
        .input(
            Creation.extend({
                instanceId: identifier("instance"),
                /** Maximum graceful shutdown time in milliseconds. */
                gracePeriodMs: schema.number().int().min(0).max(300000),
                /** Current host authorization, rechecked before replacement starts. */
                hostEpoch: schema.number().int().positive(),
            }),
        )
        .output(Instance),
    list: access("read")
        .route({ method: "GET", path: "/instances" })
        .input(
            PageRequest.extend({
                spaceId: identifier("space").optional(),
                deploymentId: identifier("deployment").optional(),
            }),
        )
        .output(page(Instance)),
    get: access("read")
        .route({ method: "GET", path: "/instances/{instanceId}" })
        .input(schema.object({ instanceId: identifier("instance") }))
        .output(Instance),
    start: access("start")
        .route({ method: "POST", path: "/instances" })
        .input(
            Creation.extend({
                spaceId: identifier("space"),
                deploymentId: identifier("deployment"),
                hostEpoch: schema.number().int().positive(),
            }),
        )
        .output(Instance),
    stop: access("stop")
        .route({ method: "POST", path: "/instances/{instanceId}/stop" })
        .input(
            Creation.extend({
                instanceId: identifier("instance"),
                /** Maximum graceful shutdown time in milliseconds. */
                gracePeriodMs: schema.number().int().min(0).max(300000),
            }),
        )
        .output(Instance),
    watch: access("read")
        .route({ method: "GET", path: "/instances/{instanceId}/watch" })
        .input(schema.object({ instanceId: identifier("instance") }))
        .output(eventIterator(Instance)),
};

/** Declare host permissions separately from the deployment's own resource grants. */
function access(action: string) {
    return defineProcedure({
        authentication: "host",
        permission: {
            packageId: PackageId.parse(packageDefinition.id),
            type: "instance",
            name: action,
        },
        audit: action !== "read",
    });
}
