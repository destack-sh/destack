import { contained, principal } from "@destack/access";
import { AccountHandle, SpaceAddress } from "@destack/account/address";
import { PlacementEndpoint } from "@destack/account/object";
import { MachineName, ResidencyCode } from "@destack/account/object";
import { Expression } from "@destack/db";
import { defineObject, device, field } from "@destack/object";
import { schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { TUNNEL_STATUSES } from "@destack/tunnel";

/** Whether the machine's relay tunnel runs: none without a relay, else the tunnel's status. */
export const COMPUTER_TUNNELS = ["none", ...TUNNEL_STATUSES] as const;

/** The fields of every enrollment: its universe and its validity. */
const ENROLLMENT_FIELDS = {
    /** The origin of the universe the machine enrolled at, whose issuer signs its credentials. */
    universe: schema.url(),
    /** Whether the enrollment is valid, or was revoked. */
    status: schema.enum(["enrolled", "revoked"]),
};

/** An enrollment the universe verified. */
const machineEnrollment = schema.discriminatedUnion("kind", [
    schema.object({
        /** A person's machine, enrolled by them. */
        kind: schema.literal("personal"),
        /** The account the machine enrolled under. */
        accountId: schema.identifier("account"),
        ...ENROLLMENT_FIELDS,
    }),
    schema.object({
        /** A cloud machine of the universe, enrolled by its declaration. */
        kind: schema.literal("cloud"),
        /** The residency the machine serves. */
        residencyId: ResidencyCode,
        ...ENROLLMENT_FIELDS,
    }),
]);

/** An enrollment the universe verified, and the scope it places its machine in. */
export const MachineEnrollment = Object.assign(machineEnrollment, {
    /** Read the scope an enrollment places a machine in: its account, or the universe. */
    scope(enrollment: MachineEnrollment): string {
        return enrollment.kind === "personal" ? enrollment.accountId : Scope.universe.id;
    },
});
/** An enrollment the universe verified. */
export type MachineEnrollment = schema.Infer<typeof machineEnrollment>;

/** This computer as its machine process runs it, stored in its device by the machine's id. */
export const computer = defineObject(
    {
        name: "computer",
        plural: "computers",
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
        convert: {
            // read the universe's origin that enrollments before 2026.11.0 stored as their issuer
            "2026.11.0": {
                enrollment: Expression.case(
                    Expression.scalar(
                        Expression.path(Expression.column("enrollment"), "kind"),
                        "text",
                    ),
                    [
                        {
                            when: "personal",
                            then: Expression.object({
                                kind: Expression.literal("personal"),
                                accountId: Expression.path(
                                    Expression.column("enrollment"),
                                    "accountId",
                                ),
                                universe: Expression.path(
                                    Expression.column("enrollment"),
                                    "issuer",
                                ),
                                status: Expression.path(Expression.column("enrollment"), "status"),
                            }),
                        },
                        {
                            when: "cloud",
                            then: Expression.object({
                                kind: Expression.literal("cloud"),
                                residencyId: Expression.path(
                                    Expression.column("enrollment"),
                                    "residencyId",
                                ),
                                universe: Expression.path(
                                    Expression.column("enrollment"),
                                    "issuer",
                                ),
                                status: Expression.path(Expression.column("enrollment"), "status"),
                            }),
                        },
                    ],
                    Expression.column("enrollment"),
                ),
            },
        },
        permissions: {
            read: contained(principal.device),
            enroll: contained(principal.device),
            record: contained(principal.device),
            stop: contained(principal.device),
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
        revoke: method.mutation({ permission: "enroll", prepared: MachineEnrollment }),

        /** Locate a space by its identifier or address in this machine's directory replica. */
        locate: method.query({
            permission: "read",
            input: schema.object({
                /** The space's identifier or address. */
                space: schema.union([schema.identifier("space"), SpaceAddress]),
            }),
            output: PlacementEndpoint,
        }),
        /** Find the account claiming a handle in this machine's replica of the directory. */
        resolve: method.query({
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
        stop: method.mutation({ permission: "stop" }),
    }),
);
