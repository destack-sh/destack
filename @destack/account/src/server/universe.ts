import type { Call } from "@destack/sync";
import {
    accessRole,
    Contact,
    Invitation,
    principal,
    Relationship,
    Role,
    type RoleRequest,
    SIGNED_IN,
} from "@destack/access";
import { and, eq, inArray, isNull, Snapshot, Transaction } from "@destack/db";
import { RequestId } from "@destack/runtime/request";
import type { Authorization, ObjectStore } from "@destack/object/server";
import { canonicalize, type Identifier, present, schema } from "@destack/schema";
import { PublicKey } from "@destack/identity";
import { CallError } from "@destack/runtime/error";
import { Scope, Subject } from "@destack/sync";
import { Account, account } from "../object/account.ts";
import { residentCalls } from "./account.ts";
import { AccountHandle } from "../address/handle.ts";
import { organisation } from "../object/organisation.ts";
import { endpoint } from "../object/endpoint.ts";
import { identity } from "../object/identity.ts";
import { machine, ProviderCode } from "../object/machine.ts";
import { claim } from "../object/claim.ts";
import { placement } from "../object/placement.ts";
import { Residency, residency, ResidencyCode } from "../object/residency.ts";
import { space } from "../object/space.ts";
import { StackSource } from "../object/stack-source.ts";
import { Claim, type ResultsOf } from "@destack/object";

/** The universe role of the operating organisation's owners. */
const OPERATOR_ROLE = {
    name: "operator",
    description: "Operate the cloud machines serving Destack's residencies",
    permissions: [residency.permission("operate")],
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

/** The universe role of the universe itself. */
const UNIVERSE_ROLE = {
    name: "universe",
    description:
        "Operate residencies, place accounts and organisations in them, seat organisations' owners, enroll its machines' keys and apply its homes' declared stacks",
    permissions: [
        residency.permission("operate"),
        account.permission("place"),
        organisation.permission("place"),
        organisation.permission("own"),
        machine.permission("rotate"),
        machine.permission("publish"),
        identity.permission("start"),
        identity.permission("extend"),
        space.permission("upgrade"),
    ],
} as const satisfies RoleRequest;

/** The component name of the platform's calls. */
const PLATFORM_COMPONENT = "platform";

/** The store the universe applies the platform's definition through, as itself. */
export type PlatformStore = Pick<
    ObjectStore,
    "database" | "principal" | "serving" | "mutate" | "authorize" | "clock"
>;

/** The organisation operating the universe as it declares it, with its cloud machines. */
export const PlatformDefinition = schema.object({
    /** The organisation's display name, such as Destack. */
    name: schema.string().min(1),
    /** The jurisdiction the organisation stores its data in. */
    residencyId: ResidencyCode,
    /** The email of the operator, invited as the organisation's owner until they accept. */
    operator: schema.email(),
    /** The reserved handles, which the organisation claims without an account. */
    reserved: schema.array(AccountHandle),
    /** The handles the organisation publishes under, each an account in its residency. */
    publishers: schema.array(AccountHandle),
    /** The handle of the organisation's own account in the platform's residency. */
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
/** The organisation operating the universe as it declares it. */
export type PlatformDefinition = schema.Infer<typeof PlatformDefinition>;

/** A step applying part of the platform's definition, again without effect once applied. */
export interface PlatformStep {
    /** The step's name, unique among the steps. */
    readonly name: string;
    /** Apply the step through the universe's store at a time. */
    readonly apply: (objects: PlatformStore, now: number) => Promise<void>;
}

/** The universe's inherent role. */
export const UniverseRole = {
    /** Own the universe as the universe with the role its own code needs: the one trust root. */
    async trust(objects: Pick<ObjectStore, "database" | "principal">, now: number): Promise<void> {
        const { database, principal: universe } = objects;
        await Transaction.run(database, async (transaction) => {
            await Role.bindOwner(transaction, Scope.universe, universe, now);
            await Role.upsertInherent(transaction, Scope.universe, UNIVERSE_ROLE, universe, now);
        });
    },

    /** Find the organisation whose owners hold the operator role, absent before the first start. */
    async operating(
        reader: Pick<Snapshot, "read">,
    ): Promise<Identifier<"organisation"> | undefined> {
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

        // take the organisation whose owners it binds
        const owners = bindings.find(
            (row) =>
                row.subjectPackageId === organisation.typeReference.packageId &&
                row.subjectType === organisation.typeReference.type &&
                row.subjectRelation === "owner",
        );

        return owners === undefined
            ? undefined
            : schema.identifier("organisation").parse(owners.subjectId);
    },
};

/** The organisation operating the universe, applied with the stack of its homes. */
export class Platform {
    /** The operating organisation and its machines as declared. */
    readonly definition: PlatformDefinition;
    /** The stack each residency's home runs. */
    readonly stack: StackSource;

    /** Create the platform from its definition and its homes' stack. */
    constructor(definition: PlatformDefinition, stack: StackSource) {
        this.definition = definition;
        this.stack = stack;
    }

    /** Read the handle of the platform's account in a residency. */
    static handle(
        definition: Pick<PlatformDefinition, "handle" | "residencyId">,
        code: ResidencyCode,
    ): AccountHandle {
        return AccountHandle.parse(
            code === definition.residencyId ? definition.handle : `${definition.handle}-${code}`,
        );
    }

    /** List the steps applying the definition, in order. */
    steps(): readonly PlatformStep[] {
        return [
            { name: "machines", apply: (store, now) => this.declareMachines(store, now) },
            { name: "organisation", apply: (store, now) => this.upsertOrganisation(store, now) },
            { name: "roles", apply: (store, now) => this.bindRoles(store, now) },
            { name: "names", apply: (store, now) => this.reserveNames(store, now) },
            { name: "homes", apply: (store, now) => this.serveResidencies(store, now) },
            { name: "invitation", apply: (store, now) => this.inviteOperator(store, now) },
        ];
    }

    /** Apply every step, as a start without durable steps does, answering the organisation. */
    async apply(store: PlatformStore, now: number): Promise<Identifier<"organisation">> {
        for (const step of this.steps()) {
            await step.apply(store, now);
        }

        return requireOrganisation(store);
    }

    /** Create the missing declared cloud machines in the universe, publishing their origins. */
    async declareMachines(store: PlatformStore, now: number): Promise<void> {
        // skip an empty definition
        if (this.definition.machines.length === 0) {
            return;
        }

        // read the declared machines that exist, refusing one stored at another origin than declared
        const snapshot = Snapshot.live(store.database);
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
                throw new CallError("CONFLICT", {
                    message: `cloud machine ${row.id} is published at ${origin ?? "no origin"}, not its declared ${declared.origin}`,
                });
            }
        }
        const missing = this.definition.machines.filter(
            (entry) => !existing.some((row) => row.id === entry.id),
        );

        // store the missing ones in the universe, publishing their declared origins
        await mutate(
            store,
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
            now,
        );
    }

    /** Create the operating organisation with its own account once, as the universe. */
    async upsertOrganisation(store: PlatformStore, now: number): Promise<void> {
        // take the organisation its handle names, created once
        const organisationId =
            (await this.#claimedOrganisation(store)) ??
            (await this.#createOrganisation(store, now));

        // make its owners the universe's operators
        await authorize(store, Scope.universe.id, now, async (authorization) => {
            const operator = await Role.upsert(Scope.universe, OPERATOR_ROLE, authorization);
            await Role.assign(
                Scope.universe,
                operator.id,
                { ...organisation.reference(Scope.universe.id, organisationId), relation: "owner" },
                authorization,
            );
        });
    }

    /** Find the organisation owning the operating account's handle, refusing any other owner. */
    async #claimedOrganisation(
        store: PlatformStore,
    ): Promise<Identifier<"organisation"> | undefined> {
        // read the live shared account with the handle
        const { handle } = this.definition;
        const [claimed] = await store.database
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

        // refuse a handle another kind of scope holds
        const owner = schema.identifier("organisation").safeParse(claimed.scope);
        if (!owner.success) {
            throw new CallError("CONFLICT", {
                message: `handle ${handle} belongs to ${claimed.scope}`,
            });
        }

        return owner.data;
    }

    /** Create the operating organisation as the universe, claiming its handle on its own. */
    async #createOrganisation(
        store: PlatformStore,
        now: number,
    ): Promise<Identifier<"organisation">> {
        const [created] = await mutate(
            store,
            [
                organisation.call("create", {
                    ...organisation.in(Scope.universe.id),
                    name: this.definition.name,
                    handle: this.definition.handle,
                    residencyId: this.definition.residencyId,
                    stack: this.stack,
                }),
            ],
            now,
        );

        return present(created, "the operating organisation").id;
    }

    /** Bind the universe's roles to residency servers, machines and signed-in callers. */
    async bindRoles(store: PlatformStore, now: number): Promise<void> {
        // bind the roles of the residencies' servers, of every machine and of every signed-in caller
        await authorize(store, Scope.universe.id, now, async (authorization) => {
            // bind each role once
            await bindServers(authorization, SERVER_ROLE);
            await relateWildcards(authorization);
            await bindRole(authorization, MACHINE_ROLE, [
                { ...Scope.universe, relation: "machine" },
            ]);
            await bindRole(authorization, SIGNED_IN_ROLE, [
                { ...Scope.universe, relation: "signed-in" },
            ]);
        });

        // relate the platform's residency's cloud machines as the organisation's publishing administrators
        const organisationId = await requireOrganisation(store);
        await authorize(store, organisationId, now, (authorization) =>
            relatePublishingMachines(authorization, this.definition, organisationId),
        );
    }

    /** Claim the reserved handles for the organisation, and create its publisher accounts. */
    async reserveNames(store: PlatformStore, now: number): Promise<void> {
        // claim each reserved handle for the organisation, which lives in the universe
        const organisationId = await requireOrganisation(store);
        const owner = Scope.universe.id;
        const claims = this.definition.reserved.map((handle) => ({
            ...Claim.key(account, "handle", [handle]),
            packageId: account.typeReference.packageId,
            objectId: organisationId,
            owner,
        }));
        const claimant = {
            indexes: [Claim.index(account, "handle")],
            objectId: organisationId,
            owner,
            claims,
            state: "active" as const,
        };

        // replace the organisation's names, refusing a handle another object claims
        const [conflict] = await Claim.replace(store.database, claimant);
        if (conflict !== undefined) {
            throw new CallError("CONFLICT", {
                message: `reserved handle ${conflict.key} is claimed outside the operating organisation ${organisationId}`,
            });
        }

        // create the missing publishers as the organisation's shared accounts
        const existing = await store.database
            .select({ handle: account.table.handle })
            .from(account.table)
            .where(inArray(account.table.handle, [...this.definition.publishers]));
        const missing = this.definition.publishers.filter(
            (handle) => !existing.some((row) => row.handle === handle),
        );
        if (missing.length > 0) {
            await mutate(
                store,
                missing.map((handle) =>
                    account.call("create", {
                        ...account.in(organisationId),
                        handle,
                        name: handle,
                        kind: "shared",
                        residencyId: this.definition.residencyId,
                    }),
                ),
                now,
            );
        }
    }

    /** Serve each residency from the platform's account's home there, with its declared stack. */
    async serveResidencies(store: PlatformStore, now: number): Promise<void> {
        const organisationId = await requireOrganisation(store);
        for (const entry of await Residency.list(store.database)) {
            // read the platform's account in the residency
            const handle = Platform.handle(this.definition, entry.id);
            const [existing] = await store.database
                .select({ id: account.table.id })
                .from(account.table)
                .where(eq(account.table.handle, handle));

            // create the account when missing
            const [created] =
                existing === undefined
                    ? await mutate(
                          store,
                          [
                              account.call("create", {
                                  ...account.in(organisationId),
                                  handle,
                                  name: handle,
                                  kind: "shared",
                                  residencyId: entry.id,
                                  stack: this.stack,
                              }),
                          ],
                          now,
                      )
                    : [existing];
            const accountId = present(created, `the platform's account in ${entry.id}`).id;

            // move its home to the declared stack
            await upgradeHome(store, Account.home(accountId), this.stack, now);

            // make its home the residency's once
            const stored = await Residency.require(Snapshot.live(store.database), entry.id);
            const home = stored.home ?? Account.home(accountId);
            if (stored.home === null) {
                await mutate(
                    store,
                    [
                        residency.call("update", {
                            ...residency.target({ scope: Scope.universe.id, id: entry.id }),
                            home,
                        }),
                    ],
                    now,
                );
            }

            // serve the residency with every installation of its home
            await authorize(store, Scope.universe.id, now, (authorization) =>
                relateInstallations(authorization, entry.id, home),
            );
        }

        // relate the organisation to the servers of its accounts' residencies
        await mutate(
            store,
            await residentCalls(Snapshot.live(store.database), organisationId),
            now,
        );
    }

    /** Invite the operator as an owner unless an invitation to them was accepted or still waits. */
    async inviteOperator(store: PlatformStore, now: number): Promise<void> {
        // read the invitations to the operator's email on the organisation
        const organisationId = await requireOrganisation(store);
        const object = organisation.reference(Scope.universe.id, organisationId);
        const contact = Contact.subject(Contact.email(this.definition.operator));
        const invitations = await Invitation.readOn(Snapshot.live(store.database), object, contact);

        // keep an accepted or pending invitation
        const isInvited = invitations.some(
            (row) => row.status === "accepted" || (row.status === "pending" && row.expiresAt > now),
        );
        if (isInvited) {
            return;
        }

        // invite the operator as the universe
        await authorize(store, organisationId, now, (authorization) =>
            Invitation.create(
                { relationship: { object, relation: "owner", subject: contact } },
                authorization,
            ),
        );
    }
}

/** Read the operating organisation, refusing a universe without one. */
async function requireOrganisation(
    objects: Pick<ObjectStore, "database">,
): Promise<Identifier<"organisation">> {
    return present(
        await UniverseRole.operating(Snapshot.live(objects.database)),
        "the operating organisation",
    );
}

/** Run calls as the universe in one request. */
function mutate<const Calls extends readonly Call[]>(
    objects: PlatformStore,
    calls: Calls,
    now: number,
): Promise<ResultsOf<Calls>> {
    return objects.mutate(
        objects.serving(account),
        { id: RequestId.create(now), calls },
        { component: PLATFORM_COMPONENT },
    );
}

/** Run work in one transaction, authorized as the universe in a scope. */
function authorize<Value>(
    objects: PlatformStore,
    scope: string,
    now: number,
    work: (authorization: Authorization) => Promise<Value>,
): Promise<Value> {
    return Transaction.run(objects.database, async (transaction) =>
        work(await objects.authorize(transaction, scope, objects.serving(account), { now })),
    );
}

/** Bind a role to the machines and the installations serving each residency once. */
async function bindServers(authorization: Authorization, request: RoleRequest): Promise<void> {
    const residencies = await authorization.transaction.read(residency.table).findMany();
    await bindRole(
        authorization,
        request,
        residencies.flatMap((entry) =>
            ["machine", "installation"].map((relation) => ({
                ...residency.reference(Scope.universe.id, entry.id),
                relation,
            })),
        ),
    );
}

/** Bind a universe role to some subjects once. */
async function bindRole(
    authorization: Authorization,
    request: RoleRequest,
    subjects: readonly Subject[],
): Promise<void> {
    // read the subjects the role is bound to
    const role = await Role.upsert(Scope.universe, request, authorization);
    const bindings = await Relationship.readOn(authorization.transaction.view(), Scope.universe, {
        role: role.id,
    });
    const bound = bindings.map((row) => Relationship.subjectOf(row));

    // bind it to the others
    for (const subject of subjects) {
        if (!bound.some((existing) => Subject.same(existing, subject))) {
            await Relationship.grant(
                { object: Scope.universe, role: role.id, subject },
                authorization,
            );
        }
    }
}

/** Relate every machine and every signed-in caller to the universe once, through wildcards. */
async function relateWildcards(authorization: Authorization): Promise<void> {
    // list the wildcards of every machine and every signed-in caller
    const wanted = [
        { relation: "machine", subject: principal.machine.reference("*", "*") },
        ...SIGNED_IN.map((subject) => ({ relation: "signed-in", subject })),
    ];

    // relate each missing one
    for (const { relation, subject } of wanted) {
        const related = await Relationship.readOn(
            authorization.transaction.view(),
            Scope.universe,
            { relation, subject },
        );
        if (related.length === 0) {
            await Relationship.grant({ object: Scope.universe, relation, subject }, authorization);
        }
    }
}

/** Make the platform's residency's cloud machines the organisation's administrators. */
async function relatePublishingMachines(
    authorization: Authorization,
    definition: PlatformDefinition,
    organisationId: Identifier<"organisation">,
): Promise<void> {
    // read the administrators the residency's machines are
    const object = organisation.reference(Scope.universe.id, organisationId);
    const subject = {
        ...residency.reference(Scope.universe.id, definition.residencyId),
        relation: "machine",
    };
    const related = await Relationship.readOn(authorization.transaction.view(), object, {
        relation: "admin",
        subject,
    });

    // relate them once
    if (related.length === 0) {
        await Relationship.grant({ object, relation: "admin", subject }, authorization);
    }
}

/** Move a home to a stack once it has another. */
async function upgradeHome(
    objects: PlatformStore,
    spaceId: Identifier<"space">,
    stack: StackSource,
    now: number,
): Promise<void> {
    // read the home's stack
    const [home] = await objects.database
        .select({ scope: space.table.scope, stack: space.table.stack })
        .from(space.table)
        .where(eq(space.table.id, spaceId));
    const existing = present(home, `the home ${spaceId}`);

    // move it to the declared one
    if (canonicalize(existing.stack) !== canonicalize(stack)) {
        await mutate(
            objects,
            [
                space.call("upgrade", {
                    ...space.target({ scope: existing.scope, id: spaceId }),
                    stack,
                }),
            ],
            now,
        );
    }
}

/** Relate a residency's home and its installations to the residency as its servers. */
async function relateInstallations(
    authorization: Authorization,
    code: ResidencyCode,
    home: Identifier<"space">,
): Promise<void> {
    // relate the home itself and its installations through their wildcard, each once
    const object = residency.reference(Scope.universe.id, code);
    const servers = [
        { relation: "home", subject: principal.space.reference(Scope.universe.id, home) },
        { relation: "installation", subject: principal.installation.reference(home, "*") },
    ];
    for (const { relation, subject } of servers) {
        const related = await Relationship.readOn(authorization.transaction.view(), object, {
            relation,
            subject,
        });
        if (related.length === 0) {
            await Relationship.grant({ object, relation, subject }, authorization);
        }
    }
}
