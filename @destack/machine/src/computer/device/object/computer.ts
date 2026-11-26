import { contained, principal } from "@destack/access";
import { AccountHandle, SpaceAddress, UniverseOrigin } from "@destack/account/address";
import { PlacementEndpoint } from "@destack/account/object";
import { MachineName } from "@destack/account/object";
import { defineObject, device, field } from "@destack/object";
import { schema } from "@destack/schema";
import { TUNNEL_STATUSES } from "@destack/tunnel";
import { argument, plural, t } from "@destack/locale";

/** Whether the machine's relay tunnel runs: none without a relay, else the tunnel's status. */
export const COMPUTER_TUNNELS = ["none", ...TUNNEL_STATUSES] as const;

/** An enrollment the universe verified. */
export const MachineEnrollment = schema.object({
    /** Whether its account enrolled the machine or a provider runs it for the platform. */
    kind: schema.enum(["self", "managed"]),
    /** The account the machine belongs to. */
    accountId: schema.identifier("account"),
    /** The universe the machine enrolled at, whose issuer signs its credentials. */
    universe: UniverseOrigin.schema,
    /** Whether the enrollment is valid, or was revoked. */
    status: schema.enum(["enrolled", "revoked"]),
});
/** An enrollment the universe verified. */
export type MachineEnrollment = schema.Infer<typeof MachineEnrollment>;

/** This computer as its machine process runs it, stored in its device by the machine's id. */
export const computer = defineObject(
    {
        name: "computer",
        plural: "computers",
        title: t`${plural(argument("count"), { one: "Computer", other: "Computers" })}`,
        scope: device,
        key: "machineId",
        fields: {
            /** The machine the computer runs. */
            machine: field.reference(principal.machine).immutable(),
            /** The enrollment, absent before the machine enrolled. */
            enrollment: field.json(MachineEnrollment).optional(),

            // process
            /** The machine version running. */
            version: field.string(schema.string().min(1)),
            /** The machine process's operating system identifier. */
            pid: field.integer(),
            /** The machine process's start, in UTC epoch milliseconds. */
            startedAt: field.time(),
            /** Whether the machine process's relay tunnel runs: none, open, or dialing. */
            tunnel: field.enum(COMPUTER_TUNNELS).default("none"),
        },
        // let the device do the computer's own work, as its owner acts as the device
        access: {
            permissions: {
                read: contained(principal.device),
                enroll: contained(principal.device),
                record: contained(principal.device),
                stop: contained(principal.device),
            },
        },
    },
    (method) => ({
        create: method.create("record", { visibility: "package" }),
        start: method.update("record", {
            visibility: "package",
            fields: ["version", "pid", "startedAt", "tunnel", "enrollment"],
        }),
        link: method.update("record", { visibility: "package", fields: ["tunnel"] }),
        enroll: method.mutation({
            title: t.context("action")`Enroll`,
            permission: "enroll",
            input: schema.object({
                /** The signed-in person approving the enrollment. */
                login: schema.identifier("login"),
                /** The account the machine enrolls under. */
                accountId: schema.identifier("account"),
                /** The machine's name within the account, the computer's name when absent. */
                name: MachineName.exactOptional(),
            }),
            prepared: MachineEnrollment,
        }),
        revoke: method.mutation({
            title: t.context("action")`Revoke`,
            permission: "enroll",
            prepared: MachineEnrollment,
        }),

        /** Locate a space by its identifier or address in this machine's directory replica. */
        locate: method.query({
            title: t.context("action")`Locate`,
            permission: "read",
            input: schema.object({
                /** The space's identifier or address. */
                space: schema.union([schema.identifier("space"), SpaceAddress]),
            }),
            output: PlacementEndpoint,
        }),
        /** Find the account claiming a handle in this machine's replica of the directory. */
        resolve: method.query({
            title: t.context("action")`Resolve`,
            permission: "read",
            input: schema.object({
                /** The account's handle. */
                handle: AccountHandle,
            }),
            output: schema.object({
                /** The account claiming the handle. */
                id: schema.identifier("account"),
            }),
        }),
        /** Stop the machine process once the answer is sent. */
        stop: method.mutation({ title: t.context("action")`Stop`, permission: "stop" }),
    }),
);
