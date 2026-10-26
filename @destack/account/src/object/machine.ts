import { none, permission, principal, relation, through, union } from "@destack/access";
import { type Select, type Snapshot } from "@destack/db";
import { type MethodContext, defineObject, field } from "@destack/object";
import { ServerRuntime } from "@destack/package/runtime";
import { schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import type { AssertionKey } from "@destack/access/token";
import { CompactProof, PublicKey } from "@destack/identity";
import { Label } from "../address/label.ts";
import { key, Key } from "./key.ts";
import { account } from "./account.ts";
import { residency, ResidencyCode } from "./residency.ts";
import { Platform, PLATFORMS } from "./platform.ts";
import { AccountError } from "../error/error.ts";
import type { Tenancy } from "./placement.ts";

/** The longest machine name, so `<machine>--<handle>` fits one DNS label: see `HANDLE_LENGTH`. */
const MACHINE_NAME_LENGTH = 37;

/** How often a serving machine reports its contact, in milliseconds. */
export const MACHINE_REPORT_MILLISECONDS = 30_000;

/** How many reports a machine may miss before it counts as offline. */
const MACHINE_MISSED_REPORTS = 3;

/** Whether a machine hosts new work, drains its existing work, or refuses work. */
const MACHINE_STATUSES = ["enabled", "draining", "disabled"] as const;

/** The columns only a personal machine has. */
const PERSONAL_COLUMNS = ["platform", "architecture", "model"] as const;

/** The columns only a cloud machine has. */
const CLOUD_COLUMNS = ["residencyId", "providerCode", "location"] as const;

/** The sizes of an account's dedicated cloud machines, smallest first. */
export const MACHINE_SIZES = ["small", "medium", "large", "xlarge"] as const;

/** A machine's name within its account: a DNS label with single hyphens inside, other than `xn`. */
export const MachineName = Label.schema(MACHINE_NAME_LENGTH);

/** A machine's key and the proof that the machine has its private half. */
const KeyProof = schema.object({
    /** The public key. */
    publicKey: PublicKey,
    /** A proof of the machine by the key's private half. */
    proof: CompactProof,
});

/** A provider's code, a lowercase DNS label such as cloudflare or aws. */
export const ProviderCode = schema.string().regex(/^[a-z][a-z0-9-]{1,30}$/u);
/** A provider's code. */
export type ProviderCode = schema.Infer<typeof ProviderCode>;

/** The size of an account's dedicated cloud machine. */
export const MachineSize = schema.enum(MACHINE_SIZES);
/** The size of an account's dedicated cloud machine. */
export type MachineSize = schema.Infer<typeof MachineSize>;

/** The relay serving a machine's tunnel. */
export const EndpointRelay = schema.object({
    /** The relay's origin, where other relays forward the machine's requests. */
    url: schema.url(),
    /** When the tunnel's token lapses, in UTC epoch milliseconds. */
    lapsesAt: schema.number().int(),
});
/** The relay serving a machine's tunnel. */
export type EndpointRelay = schema.Infer<typeof EndpointRelay>;

/** The fields of a person's computer: its operating system and hardware. */
const PersonalMachine = schema.object({
    /** A person's computer. */
    kind: schema.literal("personal"),
    /** The operating system. */
    platform: Platform,
    /** The processor architecture, such as arm64. */
    architecture: schema.string().min(1),
    /** The hardware model. */
    model: schema.string().min(1),
});

/** The fields of a provider's cloud machine: its residency and where the provider runs it. */
const CloudMachine = schema.object({
    /** A provider's cloud machine. */
    kind: schema.literal("cloud"),
    /** The jurisdiction the machine serves. */
    residencyId: ResidencyCode,
    /** The provider running the machine. */
    providerCode: ProviderCode,
    /** The provider location, such as Cloudflare's eu jurisdiction. */
    location: schema.string().min(1),
    /** The size of an account's dedicated machine, null for a residency's shared one. */
    size: MachineSize.nullable(),
});

/** A machine's kind with the fields only that kind has. */
export const MachineVariant = schema.discriminatedUnion("kind", [PersonalMachine, CloudMachine]);
/** A machine's kind with the fields only that kind has. */
export type MachineVariant = schema.Infer<typeof MachineVariant>;

/** The Destack update state a machine's device reports, as its distribution records it. */
export const MachineUpdate = schema.object({
    /** The installed release's version. */
    installed: schema.string().min(1),
    /** The version of the release staged for activation, null while none newer is. */
    staged: schema.string().min(1).nullable(),
    /** Whether the installed release is current, a newer one is staged, or the restart runs. */
    status: schema.enum(["current", "staged", "activating"]),
});
/** The Destack update state a machine's device reports. */
export type MachineUpdate = schema.Infer<typeof MachineUpdate>;

/** The platform and architecture a personal machine reports each time, so later changes show. */
export const MachinePlatform = PersonalMachine.pick({ platform: true, architecture: true });
/** The platform and architecture a personal machine reports. */
export type MachinePlatform = schema.Infer<typeof MachinePlatform>;

/** A runner identity spaces are placed on. */
export const machine = defineObject(
    {
        name: "machine",
        plural: "machines",
        scope: [account, "universe"],
        represents: principal.machine,
        attachments: [key.attach({ by: "rotate" })],
        fields: {
            /** The name within the account. */
            name: field.string(MachineName),
            /** Whether the machine is a person's computer or a provider's cloud machine. */
            kind: field.enum(["personal", "cloud"]),

            // the fields of a personal machine, null for a cloud machine
            /** The operating system of a personal machine. */
            platform: field.enum(PLATFORMS).optional(),
            /** The processor architecture of a personal machine. */
            architecture: field.string(PersonalMachine.shape.architecture).optional(),
            /** The hardware model of a personal machine. */
            model: field.string(PersonalMachine.shape.model).optional(),

            // the fields of a cloud machine, null for a personal machine
            /** The jurisdiction a cloud machine serves. */
            residency: field.reference(residency, { delete: "restrict" }).optional(),
            /** The provider running a cloud machine. */
            providerCode: field.string(ProviderCode).optional(),
            /** The provider location of a cloud machine. */
            location: field.string(CloudMachine.shape.location).optional(),
            /** The size of an account's dedicated cloud machine. */
            size: field.enum(MACHINE_SIZES).optional(),

            // the machine's status and contact
            /** Whether the machine hosts new work, drains existing work, or neither. */
            status: field.enum(MACHINE_STATUSES).default("enabled"),
            /** The Destack version the machine last reported. */
            version: field.string().optional(),
            /** The runtimes the machine can execute. */
            runtimes: field.json(schema.array(ServerRuntime)).default([]),
            /** The operating system version the machine last reported. */
            osVersion: field.string().optional(),
            /** When the machine's running process started, in UTC epoch milliseconds. */
            startedAt: field.time().optional(),
            /** The Destack update state the machine last reported, absent for one updating none. */
            update: field.json(MachineUpdate).optional(),
            /** The machine's last proven contact, in UTC epoch milliseconds. */
            lastSeenAt: field.time().optional(),
            /** The time the account withdrew the machine and its keys. */
            revokedAt: field.time().optional(),
        },
        relations: {
            tenant: { subjects: [account.members("member")], grantedBy: "share" },
            self: { subjects: [principal.machine], grantedBy: null },
        },
        permissions: {
            read: union(relation("tenant"), relation("self"), permission("relay")),
            list: union(permission("read"), through("account", "serve")),
            verify: union(relation("tenant"), relation("self"), through("account", "serve")),
            enroll: none(),
            rotate: relation("self"),
            rename: relation("self"),
            operate: union(relation("tenant"), relation("self")),
            publish: relation("self"),
            // let the account's residency's relay publish the tunnel it accepted
            relay: through("account", "relaying"),

            // let the account's members read the machine's telemetry, its managers unmasked, and the machine forward it
            observe: relation("tenant"),
            analyze: relation("tenant"),
            unmask: through("account", "manage"),
            serve: relation("self"),
            // let whoever replicates the account replicate its machines' chains, and its machines each other's
            replicate: union(
                relation("self"),
                through("account", "replicate"),
                through("account", "serve"),
            ),
            revoke: relation("self"),
            update: none(),
            // let the account's managers share the machine with another account's members as tenants
            share: through("account", "manage"),
            declare: none(),
        },
        reserved: ["rotate", "publish", "serve", "relay", "replicate"],
        constraints: {
            scopeId: { kind: "unique", on: ["scope", "id"] },
            scopeName: {
                kind: "index",
                on: ["scope", "name"],
                unique: true,
                where: { revokedAt: { isNull: true } },
            },
            name: { kind: "check", condition: { name: Label.condition(MACHINE_NAME_LENGTH) } },
            kind: {
                kind: "check",
                condition: {
                    OR: [
                        {
                            kind: "personal",
                            platform: { isNotNull: true },
                            architecture: { isNotNull: true },
                            model: { isNotNull: true },
                            residencyId: { isNull: true },
                            providerCode: { isNull: true },
                            location: { isNull: true },
                            size: { isNull: true },
                        },
                        {
                            kind: "cloud",
                            residencyId: { isNotNull: true },
                            providerCode: { isNotNull: true },
                            location: { length: { gt: 0 } },
                            platform: { isNull: true },
                            architecture: { isNull: true },
                            model: { isNull: true },
                        },
                    ],
                },
            },
        },
    },
    (method) => ({
        list: method.list("list"),
        enroll: method.create("enroll", {
            fields: ["name", "version", "runtimes"],
            variants: { kind: { personal: PERSONAL_COLUMNS, cloud: CLOUD_COLUMNS } },
            input: KeyProof,
        }),
        declare: method.create("declare", {
            visibility: "package",
            fields: ["name", "size"],
            variants: { kind: { cloud: CLOUD_COLUMNS } },
            input: KeyProof.pick({ publicKey: true }),
        }),
        revoke: method.mutation({ permission: "revoke" }),
        /** Publish where a cloud machine answers, replacing what it published before. */
        publish: method.mutation({
            permission: "publish",
            input: schema.object({
                /** The URL the machine's packages answer at. */
                url: schema.url(),
            }),
        }),
        /** Publish the tunnel a relay accepted from a personal machine, at its derived name. */
        relay: method.mutation({ permission: "relay", input: EndpointRelay }),
        enable: method.mutation({ permission: "operate" }),
        drain: method.mutation({ permission: "operate" }),
        disable: method.mutation({ permission: "operate" }),
        /** Record the machine's proven contact and what it runs now, replacing the last report. */
        report: method.mutation({
            permission: "rotate",
            input: schema.object({
                /** The Destack version the machine runs. */
                version: schema.string().min(1).nullable(),
                /** The runtimes the machine can execute. */
                runtimes: schema.array(ServerRuntime),
                /** The operating system version, null where none is known. */
                osVersion: schema.string().min(1).nullable(),
                /** When the running process started, in epoch milliseconds, null when unknown. */
                startedAt: schema.number().int().nonnegative().nullable(),
                /** The Destack update state, null for a machine updating none. */
                update: MachineUpdate.nullable(),
                /** A personal machine's platform and architecture, null for a cloud machine. */
                host: MachinePlatform.nullable(),
            }),
        }),
        rename: method.mutation({
            permission: "rename",
            input: schema.object({
                /** The new name. */
                name: MachineName,
            }),
        }),
    }),
).implement({
    methods: {
        report: (call) => {
            // refuse a host for a cloud machine, which has no platform of its own
            const { input } = call;
            const isPersonal = call.target.kind === "personal";
            if (input.host !== null && !isPersonal) {
                throw AccountError.of("CLOUD_HOST_REPORTED", "a cloud machine reports no host");
            } else if (input.host === null && isPersonal) {
                throw AccountError.of("HOST_REQUIRED", "a personal machine reports its host");
            }

            return call.update({
                lastSeenAt: call.now,
                version: input.version,
                runtimes: input.runtimes,
                osVersion: input.osVersion,
                startedAt: input.startedAt,
                update: input.update,
                ...(input.host === null ? {} : input.host),
            });
        },
    },
});
/** A persisted machine record with the fields of its kind. */
export type Machine = Omit<
    Select<typeof machine.table>,
    | "kind"
    | "platform"
    | "architecture"
    | "model"
    | "residencyId"
    | "providerCode"
    | "location"
    | "size"
> &
    MachineVariant;

/** The checks and reads of machines that methods and the token endpoint route by. */
export const Machine = {
    /** Read the tenancy a machine runs its spaces at: shared, dedicated or local. */
    tenancy(row: Pick<Select<typeof machine.table>, "kind" | "scope">): Tenancy {
        return row.kind === "personal"
            ? "local"
            : row.scope === Scope.universe.id
              ? "shared"
              : "dedicated";
    },

    /** Read a machine record as its kind's variant. */
    of(row: Select<typeof machine.table>): Machine {
        // parse the columns of the row's kind apart from the shared ones
        const {
            kind,
            platform,
            architecture,
            model,
            residencyId,
            providerCode,
            location,
            size,
            ...shared
        } = row;
        const variant = MachineVariant.parse(
            kind === "personal"
                ? { kind, platform, architecture, model }
                : { kind, residencyId, providerCode, location, size },
        );

        return { ...shared, ...variant };
    },

    /** Report whether a machine proved contact recently enough to count as online at a time. */
    isOnline(lastSeenAt: number | null, now: number): boolean {
        const window = MACHINE_REPORT_MILLISECONDS * MACHINE_MISSED_REPORTS;

        return lastSeenAt !== null && now - lastSeenAt < window;
    },

    /** Take a preferred machine name, or its first free numbered variant in the account. */
    async uniqueName(
        context: Pick<MethodContext, "read">,
        accountId: string,
        preferred: string,
    ): Promise<string> {
        // read the names of the account's standing machines
        const rows = await context
            .read(machine)
            .findMany({ where: { scope: accountId, revokedAt: { isNull: true } } });
        const taken = new Set(rows.map((row) => row.name));

        // number the name past the taken ones within a DNS label
        let name = preferred;
        for (let number = 2; taken.has(name); number++) {
            const suffix = `-${number}`;
            name = `${preferred.slice(0, MACHINE_NAME_LENGTH - suffix.length)}${suffix}`;
        }

        return name;
    },

    /** Read the machine key an assertion identifies by its thumbprint, while it authenticates. */
    async assertionKey(
        snapshot: Snapshot,
        thumbprint: string,
        now: number,
    ): Promise<AssertionKey | undefined> {
        // read the machine's key while it authenticates
        const found = await snapshot.read(key.table).findFirst({
            where: {
                AND: [
                    {
                        thumbprint,
                        parentPackageId: machine.typeReference.packageId,
                        parentType: machine.name,
                    },
                    Key.authenticates(now),
                ],
            },
        });
        if (found === undefined) {
            return undefined;
        }

        return {
            id: found.thumbprint,
            subject: principal.machine.reference(Scope.universe.id, found.parentId),
            publicKey: found.publicKey,
        };
    },
};
