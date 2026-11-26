import { none, permission, principal, relation, through, union } from "@destack/access";
import type { Select } from "@destack/db";
import type { Snapshot } from "@destack/db/database";
import { type MethodContext, defineObject, field, recoverable } from "@destack/object";
import { ServerRuntime } from "@destack/package/runtime";
import { type Identifier, schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import type { AssertionKey } from "@destack/access/token";
import { CompactProof, PublicKey } from "@destack/identity";
import { Label } from "../address/label.ts";
import { key, Key } from "./key.ts";
import { account } from "./account.ts";
import { region } from "./region.ts";
import { Platform, PLATFORMS } from "./platform.ts";
import { AccountError } from "../error/error.ts";
import type { Tenancy } from "./space.ts";
import { argument, plural, t } from "@destack/locale";

/** The longest machine name, so `<machine>--<handle>` fits one DNS label: see `HANDLE_LENGTH`. */
const MACHINE_NAME_LENGTH = 37;

/** Whether a machine hosts new work, drains its existing work, or refuses work. */
const MACHINE_STATUSES = ["enabled", "draining", "disabled"] as const;

/** The columns only a self-hosted machine has. */
const SELF_COLUMNS = ["platform", "architecture", "model"] as const;

/** The columns only a managed machine has. */
const MANAGED_COLUMNS = ["regionId", "providerCode", "location"] as const;

/** The sizes of a managed machine, smallest first. */
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

/** The size of a managed machine. */
export const MachineSize = schema.enum(MACHINE_SIZES);
/** The size of a managed machine. */
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

/** The fields of a self-hosted machine: its operating system and hardware. */
const SelfMachine = schema.object({
    /** A machine its account enrolled, on any hardware anywhere. */
    kind: schema.literal("self"),
    /** The operating system. */
    platform: Platform,
    /** The processor architecture, such as arm64. */
    architecture: schema.string().min(1),
    /** The hardware model. */
    model: schema.string().min(1),
});

/** The fields of a managed machine: its region and where the provider runs it. */
const ManagedMachine = schema.object({
    /** A machine a provider runs, declared and priced through the platform. */
    kind: schema.literal("managed"),
    /** The jurisdiction the machine serves. */
    regionId: schema.identifier("region"),
    /** The provider running the machine. */
    providerCode: ProviderCode,
    /** The provider location, such as Cloudflare's eu jurisdiction. */
    location: schema.string().min(1),
    /** The size, null for a machine of the platform's shared pool. */
    size: MachineSize.nullable(),
});

/** A machine's kind with the fields only that kind has. */
export const MachineVariant = schema.discriminatedUnion("kind", [SelfMachine, ManagedMachine]);
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

/** The platform and architecture a self-hosted machine reports each time, so later changes show. */
export const MachinePlatform = SelfMachine.pick({ platform: true, architecture: true });
/** The platform and architecture a self-hosted machine reports. */
export type MachinePlatform = schema.Infer<typeof MachinePlatform>;

/** A runner identity spaces are placed on. */
export const machine = defineObject(
    {
        name: "machine",
        plural: "machines",
        title: t`${plural(argument("count"), { one: "Machine", other: "Machines" })}`,
        scope: account,
        key: "name",
        attachments: [key.attach({ by: "rotate" })],
        fields: {
            /** The name within the account. */
            name: field.string(MachineName),
            /** Whether its account enrolled it or a provider runs it for the platform. */
            kind: field.enum(["self", "managed"]),

            // the fields of a self-hosted machine, null for a managed one
            /** The operating system of a self-hosted machine. */
            platform: field.enum(PLATFORMS).optional(),
            /** The processor architecture of a self-hosted machine. */
            architecture: field.string(SelfMachine.shape.architecture).optional(),
            /** The hardware model of a self-hosted machine. */
            model: field.string(SelfMachine.shape.model).optional(),

            // the fields of a managed machine, null for a self-hosted one
            /** The jurisdiction a managed machine serves. */
            region: field.reference(region, { delete: "restrict" }).optional(),
            /** The provider running a managed machine. */
            providerCode: field.string(ProviderCode).optional(),
            /** The provider location of a managed machine. */
            location: field.string(ManagedMachine.shape.location).optional(),
            /** The size of a managed machine, null for one of the platform's shared pool. */
            size: field.enum(MACHINE_SIZES).optional(),
            /** The SKU rating each meter of the machine's spaces, by meter key. */
            rates: field.json(schema.record(schema.string(), schema.string())).default({}),

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
            /** When the machine last left its home, or enrolled, in UTC epoch milliseconds. */
            lastSeenAt: field.time().optional(),
        },
        constraints: {
            scopeId: { kind: "unique", on: ["scope", "id"] },
            name: { kind: "check", condition: { name: Label.condition(MACHINE_NAME_LENGTH) } },
            kind: {
                kind: "check",
                condition: {
                    OR: [
                        {
                            kind: "self",
                            platform: { isNotNull: true },
                            architecture: { isNotNull: true },
                            model: { isNotNull: true },
                            regionId: { isNull: true },
                            providerCode: { isNull: true },
                            location: { isNull: true },
                            size: { isNull: true },
                        },
                        {
                            kind: "managed",
                            regionId: { isNotNull: true },
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
        access: {
            represents: principal.machine,
            relations: {
                // the people of other accounts who operate and observe the machine
                operator: { subjects: [account.members("member")], grantedBy: "share" },
                // the accounts whose spaces and steps the machine hosts, every account for a shared pool
                tenant: { subjects: [account.policy, account.all()], grantedBy: "share" },
                self: { subjects: [principal.machine], grantedBy: null },
                // NOTE #Security: every machine verifies every machine's keys, to check its peers in moves
                peers: { subjects: [principal.machine.all()] },
            },
            permissions: {
                read: union(relation("operator"), relation("self")),
                // let the account's servers and its sellers find its machines, the sellers to meter their hours
                list: union(
                    permission("read"),
                    through("account", "serve"),
                    through("account", "sell"),
                ),
                verify: union(
                    relation("operator"),
                    relation("self"),
                    relation("peers"),
                    through("account", "serve"),
                ),
                enroll: none(),
                rotate: relation("self"),
                rename: relation("self"),
                operate: union(relation("operator"), relation("self")),
                // let the accounts it admits place their spaces and steps on it
                host: relation("tenant"),
                // let the machine record its own endpoint and tunnel
                record: relation("self"),
                relay: relation("self"),

                // let the account's members read the machine's telemetry, its managers unmasked, and the machine forward it
                observe: relation("operator"),
                analyze: relation("operator"),
                unmask: through("account", "manage"),
                serve: relation("self"),
                // let whoever replicates the account replicate its machines' chains, and its machines each other's
                replicate: union(
                    relation("self"),
                    through("account", "replicate"),
                    through("account", "serve"),
                ),
                delete: relation("self"),
                // let the machine revoke its own keys, as every key holder declares
                revoke: relation("self"),
                update: none(),
                leave: none(),
                // let the account's managers share the machine with operators and tenants
                share: through("account", "manage"),
                declare: none(),
                rate: none(),
            },
            reserved: ["rotate", "serve", "replicate"],
        },
        traits: [recoverable({ within: { days: 30 }, by: "delete" })],
    },
    (method) => ({
        list: method.list("list"),
        enroll: method.create("enroll", {
            fields: ["name", "version", "runtimes"],
            variants: { kind: { self: SELF_COLUMNS, managed: MANAGED_COLUMNS } },
            input: KeyProof,
        }),
        declare: method.create("declare", {
            visibility: "package",
            fields: ["name", "size", "rates"],
            variants: { kind: { managed: MANAGED_COLUMNS } },
            input: KeyProof.pick({ publicKey: true }).extend({
                /** The URL the managed machine answers at, its endpoint. */
                origin: schema.url(),
            }),
        }),
        /** Rate a managed machine's spaces by other SKUs, as the universe declares them. */
        rate: method.mutation({
            title: t.context("action")`Rate`,
            permission: "rate",
            visibility: "package",
            input: schema.object({
                /** The SKU rating each meter of the machine's spaces, by meter key. */
                rates: schema.record(schema.string(), schema.string()),
            }),
        }),
        /** Record a self-hosted machine's tunnel at its derived name. */
        relay: method.mutation({
            title: t.context("action")`Relay`,
            permission: "relay",
            input: EndpointRelay,
        }),
        enable: method.mutation({ title: t.context("action")`Enable`, permission: "operate" }),
        drain: method.mutation({ title: t.context("action")`Drain`, permission: "operate" }),
        disable: method.mutation({ title: t.context("action")`Disable`, permission: "operate" }),
        /** Record the machine leaving its home, as the home sees its last live link close. */
        leave: method.mutation({ title: t.context("action")`Leave`, permission: "leave" }),
        /** Record what the machine runs now, replacing the last report. */
        report: method.mutation({
            title: t.context("action")`Report`,
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
                /** A self-hosted machine's platform and architecture, null for a managed one. */
                host: MachinePlatform.nullable(),
            }),
        }),
        rename: method.mutation({
            title: t.context("action")`Rename`,
            permission: "rename",
            input: schema.object({
                /** The new name. */
                name: MachineName,
            }),
        }),
    }),
).implement({
    methods: {
        leave: (call) => call.update({ lastSeenAt: call.now }),
        report: (call) => {
            // refuse a host for a managed machine, and require one of a self-hosted machine
            const { input } = call;
            const isSelf = call.target.kind === "self";
            if (input.host !== null && !isSelf) {
                throw AccountError.of(
                    "HOST_REPORT_INVALID",
                    "host report is invalid: a managed machine reports no host",
                );
            } else if (input.host === null && isSelf) {
                throw AccountError.of(
                    "HOST_REQUIRED",
                    "host is required: a self-hosted machine reports its host",
                );
            }

            return call.update({
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
    | "regionId"
    | "providerCode"
    | "location"
    | "size"
> &
    MachineVariant;

/** The checks and reads of machines that methods and the token endpoint route by. */
export const Machine = {
    /** Read the tenancy a machine runs an account's spaces at: shared, self or dedicated. */
    tenancy(
        row: Pick<Select<typeof machine.table>, "kind" | "scope">,
        accountId: Identifier<"account">,
    ): Tenancy {
        // another account's machine
        if (row.scope !== accountId) {
            return "shared";
        }
        // the account's self-hosted machine
        else if (row.kind === "self") {
            return "self";
        }
        // the account's managed machine
        else {
            return "dedicated";
        }
    },

    /** Read a machine record as its kind's variant. */
    of(row: Select<typeof machine.table>): Machine {
        // parse the columns of the row's kind apart from the common ones
        const {
            kind,
            platform,
            architecture,
            model,
            regionId,
            providerCode,
            location,
            size,
            ...shared
        } = row;
        const variant = MachineVariant.parse(
            kind === "self"
                ? { kind, platform, architecture, model }
                : { kind, regionId, providerCode, location, size },
        );

        return { ...shared, ...variant };
    },

    /** Take a preferred machine name, or its first free numbered variant in the account. */
    async uniqueName(
        context: Pick<MethodContext, "read">,
        accountId: string,
        preferred: string,
    ): Promise<string> {
        // read the names of the account's live machines
        const owner = account.identifier(accountId);
        const rows = await context.read(machine).findMany({ where: { scope: owner } });
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
