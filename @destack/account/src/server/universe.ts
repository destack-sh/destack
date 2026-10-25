import type { Call } from "@destack/sync";
import {
    anyone,
    InstallationId,
    accessRole,
    Authorization,
    Contact,
    Invitation,
    principal,
    Relationship,
    Role,
    type RoleRequest,
    SIGNED_IN,
} from "@destack/access";
import { and, eq, inArray, isNull, Snapshot, Transaction } from "@destack/db";
import { RequestId, Scope, Subject } from "@destack/sync";
import type { ObjectStore } from "@destack/object/server";
import { canonicalize, type Identifier, present, schema } from "@destack/schema";
import { PublicKey } from "@destack/identity";
import { Account, account } from "../object/account.ts";
import { residentCalls } from "./account.ts";
import { AccountHandle } from "../address/handle.ts";
import { organization } from "../object/organization.ts";
import { endpoint } from "../object/endpoint.ts";
import { identity } from "../object/identity.ts";
import { machine, ProviderCode } from "../object/machine.ts";
import { claim } from "../object/claim.ts";
import { placement } from "../object/placement.ts";
import { RESIDENCY_RELAY, Residency, residency, ResidencyCode } from "../object/residency.ts";
import { space } from "../object/space.ts";
import { user } from "../object/user.ts";
import { StackSource } from "../object/stack-source.ts";
import { Claim, type ResultsOf } from "@destack/object";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";

/** The universe role of the operating organization's owners. */
const OPERATOR_ROLE = {
    name: "operator",
    description: "Operate the cloud machines serving Destack's residencies, and suspend people",
    permissions: [residency.permission("operate"), user.permission("suspend")],
} as const satisfies RoleRequest;

/** The universe role of every residency's servers, which route its names and serve its people. */
const SERVER_ROLE = {
    name: "server",
    description: "Route Destack's names",
    permissions: [
        placement.permission("read"),
        claim.permission("read"),
        machine.permission("read"),
    ],
} as const satisfies RoleRequest;

/** The universe role of every machine. */
const MACHINE_ROLE = {
    name: "machine",
    description:
        "Read the placements of spaces and the names they route, and start the identities of the spaces machines serve",
    permissions: [
        placement.permission("read"),
        claim.permission("read"),
        identity.permission("start"),
    ],
} as const satisfies RoleRequest;

/** The universe role of every signed-in caller. */
const SIGNED_IN_ROLE = {
    name: "signed-in",
    description:
        "Resolve Destack's names, find the residencies of accounts and the endpoints of machines, and read and extend identities",
    permissions: [
        claim.permission("resolve"),
        residency.permission("locate"),
        endpoint.permission("read"),
        identity.permission("read"),
        identity.permission("extend"),
    ],
} as const satisfies RoleRequest;

/** The component name of the platform's calls. */
const PLATFORM_COMPONENT = "platform";

/** The store the platform's definition applies through, as account's base installation. */
export type PlatformStore = Pick<
    ObjectStore,
    "database" | "principal" | "as" | "mutate" | "authorize" | "clock" | "environment"
>;

/** The organization operating the universe as it declares it, with its cloud machines. */
export const PlatformDefinition = schema.object({
    /** The organization's display name, such as Destack. */
    name: schema.string().min(1),
    /** The jurisdiction the organization stores its data in. */
    residencyId: ResidencyCode,
    /** The email of the operator, invited as the organization's owner until they accept. */
    operator: schema.email(),
    /** The reserved handles, which the organization claims without an account. */
    reserved: schema.array(AccountHandle),
    /** The handles the organization publishes under, each an account in its residency. */
    publishers: schema.array(AccountHandle),
    /** The handle of the organization's account in the platform's residency. */
    handle: AccountHandle,
    /** The cloud machines serving the residencies. */
    machines: schema.array(
        schema.object({
            /** The machine's identifier. */
            id: schema.identifier("machine"),
            /** The machine's name, such as eu-central. */
            name: schema.string().min(1),
            /** The jurisdiction it serves. */
            residencyId: ResidencyCode,
            /** The provider running it. */
            providerCode: ProviderCode,
            /** The provider location, such as Cloudflare's eu jurisdiction. */
            location: schema.string().min(1),
            /** The machine's public key. */
            key: PublicKey,
            /** The origin the machine answers at, published as its endpoint. */
            origin: schema.url(),
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
    /** Find the organization whose owners have the operator role, absent before the first start. */
    async operating(
        reader: Pick<Snapshot, "read">,
    ): Promise<Identifier<"organization"> | undefined> {
        // read the operator role's bindings, absent before the first start
        const operator = await reader
            .read(accessRole)
            .findFirst({ where: { scope: Scope.universe.id, name: OPERATOR_ROLE.name } });
        if (operator === undefined) {
            return undefined;
        }
        const bindings = await Relationship.readOn(reader, Scope.universe, {
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

/** The organization operating the universe, applied with the stack of its homes. */
export class Platform {
    /** The universe's store the platform is applied through, as account's base installation. */
    readonly store: PlatformStore;
    /** The operating organization and its machines as declared. */
    readonly definition: PlatformDefinition;
    /** The stack each residency's home runs. */
    readonly stack: StackSource;

    /** Create the platform from its definition and its homes' stack, applied through a store. */
    constructor(store: PlatformStore, definition: PlatformDefinition, stack: StackSource) {
        this.store = store;
        this.definition = definition;
        this.stack = stack;
    }

    /** Read the handle of the platform's account in a residency. */
    static handle(
        definition: Pick<PlatformDefinition, "handle" | "residencyId">,
        code: ResidencyCode,
    ): AccountHandle {
        const { handle, residencyId } = definition;

        return AccountHandle.parse(code === residencyId ? handle : `${handle}-${code}`);
    }

    /** List the steps applying the definition, in order. */
    steps(): readonly PlatformStep[] {
        return [
            { name: "machines", apply: () => this.declareMachines() },
            { name: "organization", apply: (now) => this.upsertOrganization(now) },
            { name: "roles", apply: (now) => this.bindRoles(now) },
            { name: "names", apply: () => this.reserveNames() },
            { name: "homes", apply: (now) => this.serveResidencies(now) },
            { name: "invitation", apply: (now) => this.inviteOperator(now) },
        ];
    }

    /** Apply every step, as a start without durable steps does, answering the organization. */
    async apply(now: number): Promise<Identifier<"organization">> {
        for (const step of this.steps()) {
            await step.apply(now);
        }

        return this.#organization();
    }

    /** Create the missing declared cloud machines in the universe, publishing their origins. */
    async declareMachines(): Promise<void> {
        // skip an empty definition
        if (this.definition.machines.length === 0) {
            return;
        }

        // read the declared machines that exist, refusing one stored at another origin than declared
        const snapshot = Snapshot.live(this.store.database);
        const ids = this.definition.machines.map((entry) => entry.id);
        const existing = await snapshot
            .read(machine.table)
            .findMany({ where: { id: { in: ids } } });
        const published = await snapshot
            .read(endpoint.table)
            .findMany({ where: { id: { in: ids } } });
        for (const row of existing) {
            const declared = present(
                this.definition.machines.find((entry) => entry.id === row.id),
                `the definition of ${row.id}`,
            );
            const origin = published.find((entry) => entry.id === row.id)?.url;
            if (origin !== declared.origin) {
                throw AccountError.of(
                    "ORIGIN_MISMATCH",
                    `cloud machine ${row.id} is published at ${origin ?? "no origin"}, not its declared ${declared.origin}`,
                );
            }
        }
        const missing = this.definition.machines.filter(
            (entry) => !existing.some((row) => row.id === entry.id),
        );

        // store the missing ones in the universe, publishing their declared origins
        await this.#mutate(
            missing.flatMap((entry) => [
                machine.call("declare", {
                    ...machine.target({ scope: Scope.universe.id, id: entry.id }),
                    name: entry.name,
                    kind: "cloud" as const,
                    residencyId: entry.residencyId,
                    providerCode: entry.providerCode,
                    location: entry.location,
                    publicKey: entry.key,
                }),
                endpoint.call("create", {
                    ...endpoint.target({ scope: Scope.universe.id, id: entry.id }),
                    parentId: entry.id,
                    url: entry.origin,
                    relay: null,
                }),
            ]),
        );
    }

    /** Create the operating organization with its account once, as account's base installation. */
    async upsertOrganization(now: number): Promise<void> {
        // take the organization with its handle, created once
        const organizationId =
            (await this.#claimedOrganization()) ?? (await this.#createOrganization());

        // make its owners the universe's operators
        await this.#authorize(Scope.universe.id, now, async (authorization) => {
            const operator = await Role.upsert(Scope.universe, OPERATOR_ROLE, authorization);
            await Role.assign(
                Scope.universe,
                operator.id,
                [
                    {
                        ...organization.reference(Scope.universe.id, organizationId),
                        relation: "owner",
                    },
                ],
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
                "HANDLE_TAKEN",
                `handle ${handle} already belongs to ${claimed.scope}`,
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
                residencyId: this.definition.residencyId,
                stack: this.stack,
            }),
        ]);

        return present(created, "the operating organization").id;
    }

    /** Bind the universe's roles to residency servers, machines and signed-in callers. */
    async bindRoles(now: number): Promise<void> {
        // bind the roles of the residencies' servers, of every machine and of every signed-in caller
        await this.#authorize(Scope.universe.id, now, async (authorization) => {
            // bind the servers' role to each residency's machines and space
            const residencies = await authorization.transaction.read(residency.table).findMany();
            const servers = residencies.flatMap((entry) =>
                ["machine", "space"].map((relation) => ({
                    ...residency.reference(Scope.universe.id, entry.id),
                    relation,
                })),
            );
            await Platform.#bind(authorization, SERVER_ROLE, servers);

            // relate every machine and every signed-in caller to the universe through wildcards
            const wildcards = [
                { relation: "machine", subject: principal.machine.reference("*", "*") },
                ...SIGNED_IN.map((subject) => ({ relation: "signed-in", subject })),
            ];
            for (const { relation, subject } of wildcards) {
                await Relationship.touch(
                    { object: Scope.universe, relation, subject },
                    authorization,
                );
            }

            // bind the machines' and signed-in callers' roles to them
            await Platform.#bind(authorization, MACHINE_ROLE, [
                { ...Scope.universe, relation: "machine" },
            ]);
            await Platform.#bind(authorization, SIGNED_IN_ROLE, [
                { ...Scope.universe, relation: "signed-in" },
            ]);
        });

        // relate the platform's residency's cloud machines as the organization's publishing administrators
        const organizationId = await this.#organization();
        const object = organization.reference(Scope.universe.id, organizationId);
        const subject = {
            ...residency.reference(Scope.universe.id, this.definition.residencyId),
            relation: "machine",
        };
        await this.#authorize(organizationId, now, (authorization) =>
            Relationship.touch({ object, relation: "admin", subject }, authorization),
        );
    }

    /** Claim the reserved handles for the organization, and create its publisher accounts. */
    async reserveNames(): Promise<void> {
        // claim each reserved handle for the organization, which lives in the universe
        const organizationId = await this.#organization();
        const owner = Scope.universe.id;
        const claims = this.definition.reserved.map((handle) => ({
            ...Claim.key(account, "handle", [handle]),
            packageId: account.typeReference.packageId,
            objectId: organizationId,
            owner,
        }));
        const claimant = {
            indexes: [Claim.index(account, "handle")],
            objectId: organizationId,
            owner,
            claims,
            state: "active" as const,
        };

        // replace the organization's names, refusing a handle another object claims
        const [conflict] = await Claim.replace(
            this.store.database,
            claimant,
            this.store.environment,
        );
        if (conflict !== undefined) {
            throw AccountError.of(
                "HANDLE_RESERVED",
                `reserved handle ${conflict.key} is claimed outside the operating organization ${organizationId}`,
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
                        residencyId: this.definition.residencyId,
                    }),
                ),
            );
        }
    }

    /** Serve each residency from the platform's account's home there, with its declared stack. */
    async serveResidencies(now: number): Promise<void> {
        const organizationId = await this.#organization();
        for (const entry of await Residency.list(this.store.database)) {
            // read the platform's account in the residency
            const handle = Platform.handle(this.definition, entry.id);
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
                              residencyId: entry.id,
                              stack: this.stack,
                          }),
                      ])
                    : [existing];
            const accountId = present(created, `the platform's account in ${entry.id}`).id;

            // move its home to the declared stack
            await this.#upgradeHome(Account.home(accountId));

            // make its home the residency's space once
            const stored = await Residency.require(Snapshot.live(this.store.database), entry.id);
            const spaceId = stored.space ?? Account.home(accountId);
            if (stored.space === null) {
                await this.#mutate([
                    residency.call("update", {
                        ...residency.target({ scope: Scope.universe.id, id: entry.id }),
                        space: spaceId,
                    }),
                ]);
            }

            // serve the residency with its space, and relay its machines' tunnels through the relay its space's stack declares
            const served = residency.reference(Scope.universe.id, entry.id);
            const subject = principal.space.reference(Scope.universe.id, spaceId);
            const residencySpace = space.reference(accountId, spaceId);
            const relayId = await InstallationId.declared(
                residencySpace,
                RESIDENCY_RELAY,
                this.store.environment.crypto,
            );
            const relay = principal.installation.reference(spaceId, relayId);
            await this.#authorize(Scope.universe.id, now, async (authorization) => {
                await Relationship.touch(
                    { object: served, relation: "space", subject },
                    authorization,
                );
                await Relationship.touch(
                    { object: served, relation: "relay", subject: relay },
                    authorization,
                );
            });

            // let everyone find the residency's space, which serves its public registry
            await this.#authorize(accountId, now, async (authorization) => {
                await Relationship.touch(
                    {
                        object: residencySpace,
                        relation: "guest",
                        subject: anyone.reference("*", "*"),
                    },
                    authorization,
                );
            });
        }

        // relate the organization to the servers of its accounts' residencies
        await this.#mutate(await residentCalls(Snapshot.live(this.store.database), organizationId));
    }

    /** Invite the operator as an owner unless an invitation to them was accepted or still waits. */
    async inviteOperator(now: number): Promise<void> {
        // read the invitations to the operator's email on the organization
        const organizationId = await this.#organization();
        const object = organization.reference(Scope.universe.id, organizationId);
        const contact = Contact.subject(Contact.email(this.definition.operator));
        const invitations = await Invitation.readOn(
            Snapshot.live(this.store.database),
            object,
            contact,
        );

        // stop at an accepted or pending invitation
        const isInvited = invitations.some(
            (row) => row.status === "accepted" || (row.status === "pending" && row.expiresAt > now),
        );
        if (isInvited) {
            return;
        }

        // invite the operator as account's base installation
        await this.#authorize(organizationId, now, (authorization) =>
            Invitation.create(
                { relationship: { object, relation: "owner", subject: contact } },
                authorization,
            ),
        );
    }

    /** Read the operating organization, refusing a universe without one. */
    async #organization(): Promise<Identifier<"organization">> {
        return present(
            await UniverseRole.operating(Snapshot.live(this.store.database)),
            "the operating organization",
        );
    }

    /** Run calls as account's base installation in one request. */
    #mutate<const Calls extends readonly Call[]>(calls: Calls): Promise<ResultsOf<Calls>> {
        return this.store.mutate(
            this.store.as(account),
            { id: RequestId.create(this.store.environment), calls },
            { component: PLATFORM_COMPONENT },
        );
    }

    /** Run work in one transaction, authorized as account's base installation in a scope. */
    #authorize<Value>(
        scope: string,
        now: number,
        work: (authorization: Authorization) => Promise<Value>,
    ): Promise<Value> {
        return Transaction.run(this.store.database, async (transaction) =>
            work(
                await this.store.authorize(transaction, scope, this.store.as(account), {
                    now,
                }),
            ),
        );
    }

    /** Move a home to the declared stack once it has another. */
    async #upgradeHome(spaceId: Identifier<"space">): Promise<void> {
        // read the home's stack
        const [home] = await this.store.database
            .select({ scope: space.table.scope, stack: space.table.stack })
            .from(space.table)
            .where(eq(space.table.id, spaceId));
        const existing = present(home, `the home ${spaceId}`);

        // move it to the declared one
        if (canonicalize(existing.stack) !== canonicalize(this.stack)) {
            await this.#mutate([
                space.call("upgrade", {
                    ...space.target({ scope: existing.scope, id: spaceId }),
                    stack: this.stack,
                }),
            ]);
        }
    }

    /** Bind a universe role to exactly some subjects, revoking the bindings no longer declared. */
    static async #bind(
        authorization: Authorization,
        request: RoleRequest,
        subjects: readonly Subject[],
    ): Promise<void> {
        const role = await Role.upsert(Scope.universe, request, authorization);
        await Role.assign(Scope.universe, role.id, subjects, authorization);
    }
}
