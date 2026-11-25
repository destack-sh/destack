import { TelemetryScope } from "@destack/telemetry";
import { type Call, RequestId, Scope } from "@destack/sync";
import {
    InstallationId,
    Contact,
    principal,
    SIGNED_IN,
    accessRole,
    Authorization,
    Invitation,
    Relationship,
    Role,
    type RoleRequest,
} from "@destack/access";
import { and, eq, inArray, isNull } from "@destack/db";
import type { Snapshot } from "@destack/db/database";
import type { ObjectStore } from "@destack/object/server";
import { canonicalize, type Identifier, present, schema } from "@destack/schema";
import { PublicKey } from "@destack/identity";
import { Account, account } from "../object/account.ts";
import { AccountHandle } from "../address/handle.ts";
import { organization } from "../object/organization.ts";
import { endpoint } from "../object/endpoint.ts";
import { identity } from "../object/identity.ts";
import { machine, ProviderCode } from "../object/machine.ts";
import { claim } from "../object/claim.ts";
import { placement } from "../object/placement.ts";
import { REGION_MERCHANT, Region, region, RegionCode } from "../object/region.ts";
import { space } from "../object/space.ts";
import { user } from "../object/user.ts";
import { StackRelease } from "../object/stack-release.ts";
import { domain, Hostname, HostnamePattern } from "../object/domain.ts";
import { client } from "../object/client.ts";
import { DisplayName } from "../object/display-name.ts";
import { hostname } from "../object/hostname.ts";
import { Claim, Claimant, type ResultsOf } from "@destack/object";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";

/** The universe role of the operating organization's owners. */
const OPERATOR_ROLE = {
    name: "operator",
    description: "Operate the managed machines serving Destack's regions, and suspend people",
    permissions: [region.permission("operate"), user.permission("suspend")],
} as const satisfies RoleRequest;

/** The universe role of every machine. */
const MACHINE_ROLE = {
    name: "machine",
    description: "Read placements, names and machines, and start the identities of served spaces",
    permissions: [
        placement.permission("read"),
        claim.permission("read"),
        machine.permission("read"),
        identity.permission("start"),
    ],
} as const satisfies RoleRequest;

/** The universe role of every space, whose installations follow the names they resolve. */
const SPACE_ROLE = {
    name: "space",
    description: "Read the names spaces resolve",
    permissions: [claim.permission("read")],
} as const satisfies RoleRequest;

/** The universe role of every signed-in caller. */
const SIGNED_IN_ROLE = {
    name: "signed-in",
    description:
        "Resolve Destack's names, regions and the endpoints of machines, and read and extend identities",
    permissions: [
        claim.permission("resolve"),
        region.permission("read"),
        endpoint.permission("read"),
        identity.permission("read"),
        identity.permission("extend"),
    ],
} as const satisfies RoleRequest;

/** The component name of the platform's calls. */
const PLATFORM_COMPONENT = "platform";

/** The store the platform's definition applies through, as the universe. */
export type PlatformStore = Pick<
    ObjectStore,
    "database" | "principal" | "as" | "mutate" | "authorize" | "clock" | "environment"
>;

/** The organization operating the universe as it declares it, with its managed machines. */
export const PlatformDefinition = schema.object({
    /** The organization's display name, such as Destack. */
    name: schema.string().min(1),
    /** The jurisdiction the organization stores its data in. */
    regionId: RegionCode,
    /** The email of the operator, invited as the organization's owner until they accept. */
    operator: schema.email(),
    /** The reserved handles, which the organization claims without an account. */
    reserved: schema.array(AccountHandle),
    /** The handles the organization transfers, each to the account it renames to it. */
    transfers: schema.record(AccountHandle, schema.identifier("account")),
    /** The handles the organization publishes under, each an account in its region. */
    publishers: schema.array(AccountHandle),
    /** The clients publishing the platform's releases, such as its release workflow. */
    clients: schema.array(
        schema.object({
            /** The client's name. */
            name: DisplayName,
            /** The OIDC issuer vouching for the client. */
            issuer: schema.url(),
            /** The client's subject at its issuer. */
            subject: schema.string().min(1),
        }),
    ),
    /** The handle of the organization's account in the platform's region. */
    handle: AccountHandle,
    /** The domains the platform's organization holds, which the universe declares verified. */
    domains: schema.array(Hostname),
    /** The stack the platform's home runs in each region, by region. */
    homes: schema.record(RegionCode, StackRelease),
    /** The stack each kind of space people create is born with. */
    stacks: schema.object({
        /** Every account's home. */
        home: StackRelease,
        /** Any other space. */
        space: StackRelease,
    }),
    /** The hostnames the platform's account in each region serves its home's routes on. */
    hostnames: schema.record(RegionCode, schema.array(HostnamePattern)),
    /** The managed machines serving the regions. */
    machines: schema.array(
        schema.object({
            /** The machine's identifier. */
            id: schema.identifier("machine"),
            /** The machine's name, such as eu-central. */
            name: schema.string().min(1),
            /** The jurisdiction it serves. */
            regionId: RegionCode,
            /** The provider running it. */
            providerCode: ProviderCode,
            /** The provider location, such as Cloudflare's eu jurisdiction. */
            location: schema.string().min(1),
            /** The machine's public key. */
            key: PublicKey,
            /** The origin the machine answers at, published as its endpoint. */
            origin: schema.url(),
            /** The SKU rating each meter of the machine's spaces, by meter key. */
            rates: schema.record(schema.string(), schema.string()),
        }),
    ),
});
/** The organization operating the universe as it declares it. */
export type PlatformDefinition = schema.Infer<typeof PlatformDefinition>;

/** A step applying a piece of the platform's definition, again without effect once applied. */
export interface PlatformStep {
    /** The step's name, unique among the steps. */
    readonly name: string;
    /** Apply the step at a time. */
    readonly apply: (now: number) => Promise<void>;
}

/** The universe's operator role. */
export const UniverseRole = {
    /** Find the platform's organization, the operator role's holder, absent before setup. */
    async platform(
        reader: Pick<Snapshot, "read">,
    ): Promise<Identifier<"organization"> | undefined> {
        // read the operator role's bindings, absent before the first start
        const operator = await reader
            .read(accessRole)
            .findFirst({ where: { scope: Scope.universe.id, name: OPERATOR_ROLE.name } });
        if (operator === undefined) {
            return undefined;
        }
        const bindings = await Relationship.read(reader, Scope.universe, {
            role: operator.id,
        });

        // take the organization whose owners it binds
        const owners = bindings.find(
            (row) =>
                row.subjectPackageId === organization.typeReference.packageId &&
                row.subjectType === organization.typeReference.type &&
                row.subjectRelation === "owner",
        );

        return owners === undefined
            ? undefined
            : schema.identifier("organization").parse(owners.subjectId);
    },
};

/** The organization operating the universe, applied with the stacks of its homes. */
export class Platform {
    /** The universe's store the platform is applied through, as the universe. */
    readonly store: PlatformStore;
    /** The operating organization, its homes and its machines as declared. */
    readonly definition: PlatformDefinition;

    /** Create the platform from its definition, applied through a store. */
    constructor(store: PlatformStore, definition: PlatformDefinition) {
        this.store = store;
        this.definition = definition;
    }

    /** Read the handle of the platform's account in a region. */
    static handle(
        definition: Pick<PlatformDefinition, "handle" | "regionId">,
        code: RegionCode,
    ): AccountHandle {
        const { handle, regionId } = definition;

        return AccountHandle.parse(code === regionId ? handle : `${handle}-${code}`);
    }

    /** List the steps applying the definition, in order. */
    steps(): readonly PlatformStep[] {
        return [
            { name: "organization", apply: (now) => this.upsertOrganization(now) },
            { name: "roles", apply: (now) => this.bindRoles(now) },
            { name: "names", apply: (now) => this.reserveNames(now) },
            { name: "domains", apply: () => this.declareDomains() },
            { name: "homes", apply: (now) => this.serveRegions(now) },
            { name: "machines", apply: (now) => this.declareMachines(now) },
            { name: "invitation", apply: (now) => this.inviteOperator(now) },
        ];
    }

    /** Apply every step, as a start without durable steps does, answering the organization. */
    apply(now: number): Promise<Identifier<"organization">> {
        // apply each step in its own span below the apply's, as a workflow spans its steps
        const { span } = TelemetryScope.of(
            import.meta.destack.package,
            this.store.environment.telemetry,
        );

        return span("destack.platform.apply", {}, async () => {
            for (const step of this.steps()) {
                const attributes = { "destack.platform.step": step.name };
                await span("destack.platform.step", attributes, () => step.apply(now), {
                    duration: { attributes },
                });
            }

            return this.#organization();
        });
    }

    /** Declare, rate and pool the managed machines in the platform's accounts. */
    async declareMachines(now: number): Promise<void> {
        // skip an empty definition
        if (this.definition.machines.length === 0) {
            return;
        }

        // read the declared machines that exist, refusing one stored at another origin than declared
        const snapshot = this.store.database.snapshot();
        const ids = this.definition.machines.map((entry) => entry.id);
        const existing = await snapshot
            .read(machine.table)
            .findMany({ where: { id: { in: ids } } });
        const published = await snapshot
            .read(endpoint.table)
            .findMany({ where: { parentId: { in: ids } } });
        for (const row of existing) {
            const declared = present(
                this.definition.machines.find((entry) => entry.id === row.id),
                `the definition of ${row.id}`,
            );
            const origin = published.find((entry) => entry.parentId === row.id)?.url;
            if (origin !== declared.origin) {
                throw AccountError.of(
                    "ORIGIN_BLOCKED",
                    `cannot publish managed machine '${row.id}': it is published at '${origin ?? "no origin"}', not its declared '${declared.origin}'`,
                );
            }
        }

        // declare, rate and pool each region's machines in the platform's account there
        for (const [code, entries] of Map.groupBy(
            this.definition.machines,
            (each) => each.regionId,
        )) {
            // declare the missing ones at their origins, and rate the ones whose rates changed
            const scope = await this.#account(code);
            const calls = entries.flatMap((entry) => {
                const target = machine.target({ scope, id: entry.id });
                const row = existing.find((stored) => stored.id === entry.id);

                // declare a missing machine
                if (row === undefined) {
                    return [
                        machine.call("declare", {
                            ...target,
                            name: entry.name,
                            kind: "managed" as const,
                            regionId: entry.regionId,
                            providerCode: entry.providerCode,
                            location: entry.location,
                            rates: entry.rates,
                            publicKey: entry.key,
                            origin: entry.origin,
                        }),
                    ];
                }
                // rate a machine whose declared rates changed
                else if (!Platform.#isRated(row.rates, entry.rates)) {
                    return [machine.call("rate", { ...target, rates: entry.rates })];
                }
                // leave an unchanged machine
                else {
                    return [];
                }
            });
            if (calls.length > 0) {
                await this.#mutate(calls);
            }

            // admit every account as a tenant of the region's shared pool
            await this.#authorize(scope, now, async (authorization) => {
                for (const entry of entries) {
                    await Relationship.touch(
                        {
                            object: machine.reference(scope, entry.id),
                            relation: "tenant",
                            subject: account.reference("*", "*"),
                        },
                        authorization,
                    );
                }
            });
        }
    }

    /** Read whether a machine's stored rates are its declared ones, meter by meter. */
    static #isRated(
        stored: Readonly<Record<string, string>>,
        declared: Readonly<Record<string, string>>,
    ): boolean {
        const entries = Object.entries(stored);

        return (
            entries.length === Object.keys(declared).length &&
            entries.every(([meter, sku]) => declared[meter] === sku)
        );
    }

    /** Read the platform's account in a region, created by the homes step. */
    async #account(code: RegionCode): Promise<Identifier<"account">> {
        const handle = Platform.handle(this.definition, code);
        const found = await Account.byHandle(this.store.database.snapshot(), handle);

        return present(found, `the platform's account in ${code}`).id;
    }

    /** Create the operating organization with its account once, as the universe. */
    async upsertOrganization(now: number): Promise<void> {
        // take the organization with its handle, created once
        const organizationId =
            (await this.#claimedOrganization()) ?? (await this.#createOrganization());

        // make its owners the universe's operators
        await this.#authorize(Scope.universe.id, now, async (authorization) => {
            const owners = {
                ...organization.reference(Scope.universe.id, organizationId),
                relation: "owner",
            };
            await Role.declare(
                Scope.universe,
                { ...OPERATOR_ROLE, subjects: [owners] },
                authorization,
            );
        });
    }

    /** Find the organization with the operating account's handle, refusing any other owner. */
    async #claimedOrganization(): Promise<Identifier<"organization"> | undefined> {
        // read the live shared account with the handle
        const { handle } = this.definition;
        const [claimed] = await this.store.database
            .select({ scope: account.table.scope })
            .from(account.table)
            .where(
                and(
                    eq(account.table.handle, handle),
                    eq(account.table.kind, "shared"),
                    isNull(account.table.deletedAt),
                ),
            );
        if (claimed === undefined) {
            return undefined;
        }

        // refuse a handle another kind of scope has
        const owner = schema.identifier("organization").safeParse(claimed.scope);
        if (!owner.success) {
            throw AccountError.of(
                "HANDLE_EXISTS",
                `handle '${handle}' already exists: it belongs to '${claimed.scope}'`,
            );
        }

        return owner.data;
    }

    /** Create the operating organization as the store, claiming its handle by itself. */
    async #createOrganization(): Promise<Identifier<"organization">> {
        const [created] = await this.#mutate([
            organization.call("create", {
                ...organization.in(Scope.universe.id),
                name: this.definition.name,
                handle: this.definition.handle,
                regionId: this.definition.regionId,
                stack: this.#home(this.definition.regionId),
            }),
        ]);

        return present(created, "the operating organization").id;
    }

    /** Bind the universe's roles to machines, spaces and signed-in callers. */
    async bindRoles(now: number): Promise<void> {
        // bind the roles of every machine, every space and every signed-in caller
        await this.#authorize(Scope.universe.id, now, async (authorization) => {
            // relate every machine, space and signed-in caller to the universe through wildcards
            const wildcards = [
                { relation: "machine", subject: principal.machine.reference("*", "*") },
                { relation: "space", subject: principal.space.reference("*", "*") },
                ...SIGNED_IN.map((subject) => ({ relation: "signed-in", subject })),
            ];
            for (const { relation, subject } of wildcards) {
                await Relationship.touch(
                    { object: Scope.universe, relation, subject },
                    authorization,
                );
            }

            // bind the machines', spaces' and signed-in callers' roles to them
            const machines = { ...Scope.universe, relation: "machine" };
            await Role.declare(
                Scope.universe,
                { ...SPACE_ROLE, subjects: [{ ...Scope.universe, relation: "space" }] },
                authorization,
            );
            const signedIn = { ...Scope.universe, relation: "signed-in" };
            await Role.declare(
                Scope.universe,
                { ...MACHINE_ROLE, subjects: [machines] },
                authorization,
            );
            await Role.declare(
                Scope.universe,
                { ...SIGNED_IN_ROLE, subjects: [signedIn] },
                authorization,
            );
        });
    }

    /** Claim the reserved handles, transfer the transferred ones, and create the publishers. */
    async reserveNames(now: number): Promise<void> {
        // claim each reserved handle the organization keeps, which lives in the universe
        const organizationId = await this.#organization();
        const owner = Scope.universe.id;
        const { reserved, transfers } = this.definition;
        const kept = reserved.filter((handle) => !Object.hasOwn(transfers, handle));
        const indexes = Object.keys(account.indexes);
        const claims = kept.flatMap((handle) =>
            indexes.map((index) => ({
                ...Claim.key(account, index, [handle]),
                packageId: account.typeReference.packageId,
                objectId: organizationId,
                owner,
            })),
        );
        const claimant = {
            indexes: indexes.map((index) => Claim.index(account, index)),
            objectId: organizationId,
            owner,
            claims,
            state: "active" as const,
            // hand transferred handles on rather than keeping them as former names
            formerIndexes: [],
            isRemoved: false,
        };

        // read the accounts the handles transfer to, refusing a missing one
        const { database, environment } = this.store;
        const recipients = await database
            .select()
            .from(account.table)
            .where(inArray(account.table.id, Object.values(transfers)));
        const targets = Object.entries(transfers).map(([handle, accountId]) => {
            const recipient = recipients.find((row) => row.id === accountId);
            if (recipient === undefined) {
                throw AccountError.of(
                    "ACCOUNT_NOT_FOUND",
                    `account not found: handle '${handle}' transfers to an account that does not exist`,
                );
            }

            return { handle, recipient };
        });
        const renaming = targets.filter(({ handle, recipient }) => recipient.handle !== handle);

        // hand the organization's transferred names to their recipients in one transaction
        await database.transaction(async (transaction) => {
            // replace the organization's names, refusing a handle another object claims
            const [conflict] = await Claim.replace(transaction, claimant, environment);
            if (conflict !== undefined) {
                throw AccountError.of(
                    "HANDLE_BLOCKED",
                    `cannot reserve handle '${conflict.key}': it is claimed outside the operating organization`,
                );
            }

            // give each recipient its new names beside its old ones, which its rename drops
            const snapshot = transaction.snapshot();
            for (const { handle, recipient } of renaming) {
                const object = { objectId: recipient.id, owner: Claim.owning(account, recipient) };
                const before = await Claimant.of(account, object, recipient, snapshot);
                const after = await Claimant.of(
                    account,
                    object,
                    { ...recipient, handle },
                    snapshot,
                );
                const keyed = [...after.claims, ...before.claims].map(
                    (entry) => [`${entry.index} ${entry.key}`, entry] as const,
                );
                const merged = [...new Map(keyed).values()];
                const [taken] = await Claim.replace(
                    transaction,
                    { ...after, claims: merged },
                    environment,
                );
                if (taken !== undefined) {
                    throw AccountError.of(
                        "HANDLE_BLOCKED",
                        `cannot transfer handle '${handle}': another object than account '${recipient.id}' claims it`,
                    );
                }
            }
        });

        // rename each recipient to its handle, which drops its old names
        if (renaming.length > 0) {
            await this.#mutate(
                renaming.map(({ handle, recipient }) =>
                    account.call("update", { ...account.target(recipient), handle }),
                ),
            );
        }

        // create the missing publishers as the organization's shared accounts
        const existing = await this.store.database
            .select({ handle: account.table.handle })
            .from(account.table)
            .where(inArray(account.table.handle, [...this.definition.publishers]));
        const missing = this.definition.publishers.filter(
            (handle) => !existing.some((row) => row.handle === handle),
        );
        if (missing.length > 0) {
            await this.#mutate(
                missing.map((handle) =>
                    account.call("create", {
                        ...account.in(organizationId),
                        handle,
                        name: handle,
                        kind: "shared",
                        regionId: this.definition.regionId,
                    }),
                ),
            );
        }

        // trust the platform's publishing workflows in the organization's account
        const publishing = await this.store.database
            .select({ id: account.table.id, handle: account.table.handle })
            .from(account.table)
            .where(
                inArray(account.table.handle, [
                    this.definition.handle,
                    ...this.definition.publishers,
                ]),
            );
        const own = publishing.find((row) => row.handle === this.definition.handle);
        if (own === undefined) {
            throw AccountError.of(
                "ACCOUNT_NOT_FOUND",
                `account @'${this.definition.handle}' not found: the organization has no such account`,
            );
        }
        if (this.definition.clients.length > 0) {
            await this.#mutate(
                this.definition.clients.map((declared) =>
                    client.call("declare", { ...client.in(own.id), ...declared }),
                ),
            );
        }

        // revoke the federated clients the declaration leaves out, with their publishing
        const stored = await this.store.database
            .select({
                id: client.table.id,
                scope: client.table.scope,
                issuer: client.table.issuer,
                subject: client.table.subject,
            })
            .from(client.table)
            .where(
                and(
                    eq(client.table.scope, own.id),
                    eq(client.table.kind, "federated"),
                    isNull(client.table.revokedAt),
                ),
            );
        const isListed = (row: {
            readonly issuer: string | null;
            readonly subject: string | null;
        }) =>
            this.definition.clients.some(
                (declared) => declared.issuer === row.issuer && declared.subject === row.subject,
            );
        const unlisted = stored.filter((row) => !isListed(row));
        if (unlisted.length > 0) {
            await this.#mutate(unlisted.map((row) => client.call("revoke", client.target(row))));
        }

        // make each publishing account's client publishers exactly the declared clients
        const trusted = stored.filter((row) => isListed(row));
        const declared = trusted.map((row) =>
            principal.client.reference(Scope.universe.id, row.id),
        );
        await this.#authorize(organizationId, now, async (authorization) => {
            for (const row of publishing) {
                // revoke every client publisher the declaration leaves out, keeping the people managers added
                const object = account.reference(organizationId, row.id);
                const related = await Relationship.read(authorization.snapshot, object, {
                    relation: "publisher",
                });
                for (const relationship of related) {
                    const isPerson = relationship.subjectType === principal.user.name;
                    const isDeclared = declared.some(
                        (subject) =>
                            relationship.subjectType === subject.type &&
                            relationship.subjectId === subject.id &&
                            relationship.subjectRelation === null,
                    );
                    if (!isPerson && !isDeclared) {
                        await Relationship.revoke(object, relationship.id, authorization);
                    }
                }

                // relate each declared one
                for (const subject of declared) {
                    await Relationship.touch(
                        { object, relation: "publisher", subject },
                        authorization,
                    );
                }
            }
        });
    }

    /** Declare the organization's domains verified, keeping the ones it holds already. */
    async declareDomains(): Promise<void> {
        const organizationId = await this.#organization();
        const { domains } = this.definition;
        if (domains.length > 0) {
            await this.#mutate(
                domains.map((name) =>
                    domain.call("declare", { ...domain.in(organizationId), name }),
                ),
            );
        }
    }

    /** Serve each region from the platform's account's home there, with its declared stack. */
    async serveRegions(now: number): Promise<void> {
        // create or update the platform's account in each region
        const organizationId = await this.#organization();
        for (const entry of await Region.list(this.store.database)) {
            // read the platform's account in the region
            const handle = Platform.handle(this.definition, entry.code);
            const [existing] = await this.store.database
                .select({ id: account.table.id })
                .from(account.table)
                .where(eq(account.table.handle, handle));

            // create the account when missing
            const [created] =
                existing === undefined
                    ? await this.#mutate([
                          account.call("create", {
                              ...account.in(organizationId),
                              handle,
                              name: handle,
                              kind: "shared",
                              regionId: entry.id,
                              stack: this.#home(entry.code),
                          }),
                      ])
                    : [existing];
            const accountId = present(created, `the platform's account in ${entry.code}`).id;

            // give the account the hostnames its home's routes serve on, and move its home to its stack
            await this.#addHostnames(accountId, this.definition.hostnames[entry.code] ?? []);
            await this.#upgradeHome(Account.home(accountId), this.#home(entry.code));
        }

        // sell to every region's accounts through the finance installation in the platform's home
        const home = Account.home(await this.#account(this.definition.regionId));
        const owner = await Scope.object(this.store.database.snapshot(), home);
        const merchantId = await InstallationId.of(
            owner,
            REGION_MERCHANT,
            this.store.environment.crypto,
        );
        const merchant = principal.installation.reference(home, merchantId);
        const regions = await Region.list(this.store.database);
        await this.#authorize(Scope.universe.id, now, async (authorization) => {
            for (const entry of regions) {
                await Relationship.touch(
                    {
                        object: region.reference(Scope.universe.id, entry.id),
                        relation: "merchant",
                        subject: merchant,
                    },
                    authorization,
                );
            }
        });
    }

    /** Read whether the operator accepted an invitation to own the organization, or one waits. */
    async isOperatorInvited(now: number): Promise<boolean> {
        return await this.#isInvited(await this.#organization(), now);
    }

    /** Invite the operator as an owner unless an invitation to them was accepted or still waits. */
    async inviteOperator(now: number): Promise<void> {
        // stop at an accepted or pending invitation
        const organizationId = await this.#organization();
        if (await this.#isInvited(organizationId, now)) {
            return;
        }

        // invite the operator as the universe
        const relationship = this.#ownership(organizationId);
        await this.#authorize(organizationId, now, (authorization) =>
            Invitation.create({ relationship }, authorization),
        );
    }

    /** Read whether the operator accepted an invitation to own an organization, or one waits. */
    async #isInvited(organizationId: Identifier<"organization">, now: number): Promise<boolean> {
        const { object, subject } = this.#ownership(organizationId);
        const invitations = await Invitation.readOn(
            this.store.database.snapshot(),
            object,
            subject,
        );

        return invitations.some(
            (row) => row.status === "accepted" || (row.status === "pending" && row.expiresAt > now),
        );
    }

    /** Build the operator's ownership of an organization, which their invitation offers. */
    #ownership(organizationId: Identifier<"organization">) {
        return {
            object: organization.reference(Scope.universe.id, organizationId),
            relation: "owner",
            subject: Contact.subject(Contact.email(this.definition.operator)),
        } as const;
    }

    /** Read the operating organization, refusing a universe without one. */
    async #organization(): Promise<Identifier<"organization">> {
        return present(
            await UniverseRole.platform(this.store.database.snapshot()),
            "the operating organization",
        );
    }

    /** Run calls as the universe in one request. */
    #mutate<const Calls extends readonly Call[]>(calls: Calls): Promise<ResultsOf<Calls>> {
        return this.store.mutate(
            this.store.as(account),
            { id: RequestId.create(this.store.environment), calls },
            { component: PLATFORM_COMPONENT },
        );
    }

    /** Run work in one transaction, authorized as the universe in a scope. */
    #authorize<Value>(
        scope: string,
        now: number,
        work: (authorization: Authorization) => Promise<Value>,
    ): Promise<Value> {
        return this.store.database.commit(async (transaction) =>
            work(
                await this.store.authorize(transaction, scope, this.store.as(account), {
                    now,
                }),
            ),
        );
    }

    /** Move a home to the declared stack once it has another. */
    async #upgradeHome(spaceId: Identifier<"space">, stack: StackRelease): Promise<void> {
        // read the home's stack
        const [home] = await this.store.database
            .select({ scope: space.table.scope, stack: space.table.stack })
            .from(space.table)
            .where(eq(space.table.id, spaceId));
        const existing = present(home, `the home ${spaceId}`);

        // move it to the declared one
        if (canonicalize(existing.stack) !== canonicalize(stack)) {
            await this.#mutate([
                space.call("upgrade", {
                    ...space.target({ scope: existing.scope, id: spaceId }),
                    stack,
                }),
            ]);
        }
    }

    /** Read the stack the platform's home runs in a region, refusing one the definition lacks. */
    #home(code: RegionCode): StackRelease {
        return present(this.definition.homes[code], `the stack of the home of ${code}`);
    }

    /** Give an account the declared hostnames it lacks. */
    async #addHostnames(accountId: Identifier<"account">, names: readonly string[]): Promise<void> {
        // read the hostnames the account has
        const owned = await this.store.database
            .select({ name: hostname.table.name })
            .from(hostname.table)
            .where(eq(hostname.table.scope, accountId));
        const has = new Set(owned.map((row) => row.name));

        // create the missing ones in one mutation
        const missing = names.filter((name) => !has.has(name));
        if (missing.length > 0) {
            await this.#mutate(
                missing.map((name) => hostname.call("create", { ...hostname.in(accountId), name })),
            );
        }
    }
}
