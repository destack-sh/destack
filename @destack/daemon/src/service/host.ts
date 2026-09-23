import { identifier, schema } from "@destack/schema";
import { PackageId } from "@destack/package";
import { Creation, defineProcedure } from "@destack/service/procedure";
import { eventIterator } from "@destack/service";
import definition from "../../destack.json" with { type: "json" };

/** Enrollment verified by the configured universe. */
export const HostEnrollment = schema.object({
    /** Account authority issuing host credentials. */
    issuer: schema.httpUrl(),
    /** Account administering this host. */
    accountId: identifier("account"),
    /** Associated interactive device, absent on headless hosts. */
    deviceId: identifier("device").nullable(),
    /** Enrollment validity independently of network reachability. */
    status: schema.enum(["enrolled", "reauthentication", "revoked"]),
    /** Last successful issuer verification, in UTC milliseconds. */
    verifiedAt: schema.number().int(),
});

/** Host execution support and current availability. */
export const HostExecution = schema.object({
    /** Whether the host accepts additional executions. */
    status: schema.enum(["enabled", "draining", "disabled"]),
    /** Supported workload runtime adapters. */
    runtimes: schema.array(schema.enum(["bun", "workerd"])),
    /** Whether required local sandbox enforcement is available. */
    sandbox: schema.enum(["available", "unavailable"]),
    /** Usable logical CPU count. */
    cpu: schema.number().int().positive(),
    /** Physical memory capacity, in bytes. */
    memoryBytes: schema.number().int().positive(),
});

/** Enroll this host and control its execution availability. */
export const host = {
    enrollment: {
        get: access("read")
            .route({ method: "GET", path: "/host/enrollment" })
            .output(HostEnrollment.nullable()),
        /** Exchange a short-lived host enrollment credential issued by AccountService. */
        enroll: access("enroll")
            .route({ method: "POST", path: "/host/enrollment" })
            .input(Creation.extend({ issuer: schema.httpUrl(), token: schema.string().min(1) }))
            .output(HostEnrollment),
        /** Remove retained host credentials and stop executions requiring them. */
        revoke: access("enroll")
            .route({ method: "DELETE", path: "/host/enrollment" })
            .input(Creation)
            .output(schema.object({ revocation: schema.enum(["confirmed", "unconfirmed"]) })),
    },
    execution: {
        get: access("read").route({ method: "GET", path: "/host/execution" }).output(HostExecution),
        watch: access("read")
            .route({ method: "GET", path: "/host/execution/watch" })
            .output(eventIterator(HostExecution)),
        drain: access("update")
            .route({ method: "POST", path: "/host/execution/drain" })
            .input(Creation.extend({ gracePeriodMs: schema.number().int().nonnegative() }))
            .output(HostExecution),
        resume: access("update")
            .route({ method: "POST", path: "/host/execution/resume" })
            .input(Creation)
            .output(HostExecution),
    },
};

/** Require private host authority for enrollment and execution administration. */
function access(name: "read" | "enroll" | "update") {
    return defineProcedure({
        authentication: "host",
        permission: { packageId: PackageId.parse(definition.id), type: "host", name },
        audit: name !== "read",
    });
}
