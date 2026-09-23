import { identifier, schema } from "@destack/schema";
import { defineProcedure, eventIterator } from "@destack/service";
import { instance } from "./instance.ts";
import { checkout } from "./checkout.ts";
import { preview } from "./preview.ts";
import { login } from "./login.ts";
import { credential } from "./credential.ts";
import { deployment } from "./deployment.ts";
import { host } from "./host.ts";
import { PackageId } from "@destack/package";
import definition from "../../destack.json" with { type: "json" };

/** The persistent identity of this local host. */
export const Host = schema.object({
    /** Enrolled device, absent on hosts without interactive device registration. */
    deviceId: identifier("device").nullable(),
    /** Host identifier retained across daemon restarts. */
    hostId: identifier("host"),
    /** Display name chosen on this device. */
    name: schema
        .string()
        .min(1)
        .max(128)
        .regex(/^\S(?:[\s\S]*\S)?$/),
    /** Host identity creation time in UTC milliseconds. */
    createdAt: schema.number().int(),
});

/** State reported by a running daemon. */
export const Status = schema.object({
    /** Installed daemon version. */
    version: schema.string(),
    /** Operating system process identifier. */
    pid: schema.number().int().positive(),
    /** Time when the process started. */
    started: schema.string(),
    /** Persistent local host identity. */
    host: Host,
});

/** State reported by a running daemon. */
export type Status = schema.Infer<typeof Status>;

/** Common authentication failures for every host operation. */
const authenticated = defineProcedure({
    authentication: "host",
    permission: null,
    audit: true,
}).errors({ UNAUTHORIZED: { status: 401 } });

/** Host administration procedures shared by the CLI and desktop. */
export const daemonService = {
    /** Issue restricted credentials for approved local clients. */
    credential,
    /** Retain independently selectable cloud identities. */
    login,
    /** Supervise prepared workload deployments. */
    instance,
    /** Accept durable execution instructions from the space authority. */
    deployment,
    /** Register editable repository working directories. */
    checkout,
    /** Start and stop preview sessions. */
    preview,
    status: procedure("host", "read").route({ method: "GET", path: "/status" }).output(Status),
    watch: procedure("host", "read")
        .route({ method: "GET", path: "/status/watch" })
        .output(eventIterator(Status)),
    stop: authenticated.route({ method: "POST", path: "/stop" }).output(schema.object({})),
    host: {
        ...host,
        rename: procedure("host", "rename")
            .route({ method: "PATCH", path: "/host" })
            .input(schema.object({ name: Host.shape.name }))
            .output(Host),
    },
};

/** Declare operations available to explicitly authorized applications. */
function procedure(type: string, name: string) {
    return defineProcedure({
        authentication: "identity",
        permission: { packageId: PackageId.parse(definition.id), type, name },
        audit: name !== "read",
    });
}
