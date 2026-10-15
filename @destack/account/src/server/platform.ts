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
import { eq, inArray, Snapshot, Transaction } from "@destack/db";
import { RequestId } from "@destack/runtime/request";
import type { Authorization, ObjectStore } from "@destack/object/server";
import { canonicalize, defineSchema, type Identifier, present, schema } from "@destack/schema";
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
import { user } from "../object/user.ts";
import { SpaceStack } from "../object/space-stack.ts";
import { Claim, type ResultsOf } from "@destack/object";

/** The universe role of the platform organisation's owners. */
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
        "Read the homes of the people machines serve, the placements of spaces and the names they route, and start the identities of the spaces machines serve",
    permissions: [
        user.permission("home"),
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
const PLATFORM_ROLE = {
    name: "platform",
    description:
        "Place accounts and organisations in the residencies keeping their data, seat organisations' owners, enroll the keys of the machines it keeps and keep its homes at their declared stacks",
    permissions: [
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

/** The server the platform keeps its records through, as the universe. */
type PlatformStore = Pick<
    ObjectStore,
    "database" | "principal" | "self" | "mutate" | "authorize" | "clock"
>;

/** The platform's own organisation as the universe declares it, with its cloud machines. */
export const PlatformDeclaration = defineSchema(
    schema.object({
        /** The organisation's display name, such as Destack. */
        name: schema.string().min(1),
        /** The jurisdiction the organisation keeps its data in. */
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
        /** The stacks spaces are born with: residency homes, personal homes and the rest. */
        stacks: schema.object({ residency: SpaceStack, person: SpaceStack, space: SpaceStack }),
    }),
);
/** The platform's own organisation as the universe declares it. */
export type PlatformDeclaration = schema.Infer<typeof PlatformDeclaration>;

/** The platform's own organisation, whose owners operate the universe. */
export class Platform {
    /** The store the platform keeps its records through. */
    readonly #objects: PlatformStore;
    /** The declared organisation and its machines. */
    readonly #definition: PlatformDeclaration;
    /** The time the records are kept at, in UTC epoch milliseconds. */
    readonly #now: number;

    /** Keep a declaration through a store at a time. */
    private constructor(objects: PlatformStore, definition: PlatformDeclaration, now: number) {
        this.#objects = objects;
        this.#definition = definition;
        this.#now = now;
    }

    /** Find the organisation whose owners have the operator role, absent before the first start. */
    static async find(
        reader: Pick<Snapshot, "read">,
    ): Promise<Identifier<"organisation"> | undefined> {
        // read the role's bindings, absent before the first start
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
    }

    /** Read the handle of the platform's account in a residency. */
    static handle(
        definition: Pick<PlatformDeclaration, "handle" | "residencyId">,
        code: ResidencyCode,
    ): AccountHandle {
        return AccountHandle.parse(
            code === definition.residencyId ? definition.handle : `${definition.handle}-${code}`,
        );
    }

    /** Keep the platform organisation as declared, and return it. */
    static keep(
        objects: PlatformStore,
        definition: PlatformDeclaration,
        now: number,
    ): Promise<Identifier<"organisation">> {
        return new Platform(objects, definition, now).#keep();
    }

    /** Keep the universe's owner, the machines, the organisation and everything it declares. */
    async #keep(): Promise<Identifier<"organisation">> {
        // own the universe as the universe, with the platform role its own code needs
        const { database, principal: universe } = this.#objects;
        await Transaction.run(database, async (transaction) => {
            await Role.bindOwner(transaction, Scope.universe, universe, this.#now);
            await Role.keepInherent(
                transaction,
                Scope.universe,
                PLATFORM_ROLE,
                universe,
                this.#now,
            );
        });

        // keep the cloud machines serving the residencies
        await this.#keepMachines();

        // find or create the organisation
        const organisationId =
            (await Platform.find(Snapshot.live(database))) ?? (await this.#createOrganisation());

        // make its owners the universe's operators
        const owners = {
            ...organisation.reference(Scope.universe.id, organisationId),
            relation: "owner",
        };
        await this.#authorize(Scope.universe.id, async (authorization) => {
            // bind the operator role to the owners
            const operator = await authorization.keepRole(Scope.universe, OPERATOR_ROLE);
            await authorization.assign(Scope.universe, operator.id, owners);

            // bind the roles of the residencies' servers, of every machine and of every signed-in caller
            await Platform.#keepServers(authorization, SERVER_ROLE);
            await Platform.#keepWildcards(authorization);
            await Platform.#keepBound(authorization, MACHINE_ROLE, [
                { ...Scope.universe, relation: "machine" },
            ]);
            await Platform.#keepBound(authorization, SIGNED_IN_ROLE, [
                { ...Scope.universe, relation: "signed-in" },
            ]);
        });

        // claim the reserved handles, and keep the publishers, residency accounts, residents and invitation
        await this.#keepReserved(organisationId);
        await this.#keepPublishers(organisationId);
        await this.#keepResidencyHomes(organisationId);
        await this.#mutate(
            await residentCalls(Snapshot.live(this.#objects.database), organisationId),
        );
        await this.#keepInvitation(organisationId);
        await this.#authorize(organisationId, (authorization) =>
            this.#keepPublishingMachines(authorization, organisationId),
        );

        return organisationId;
    }

    /** Create the missing declared cloud machines in the universe. */
    async #keepMachines(): Promise<void> {
        // skip an empty declaration
        if (this.#definition.machines.length === 0) {
            return;
        }

        // read the declared machines that exist, refusing one kept at another origin than declared
        const snapshot = Snapshot.live(this.#objects.database);
        const ids = this.#definition.machines.map((entry) => entry.id);
        const existing = await snapshot
            .read(machine.table)
            .findMany({ where: { id: { in: ids } } });
        const published = await snapshot
            .read(endpoint.table)
            .findMany({ where: { id: { in: ids } } });
        for (const row of existing) {
            const declared = present(
                this.#definition.machines.find((entry) => entry.id === row.id),
                `the declaration of ${row.id}`,
            );
            const origin = published.find((entry) => entry.id === row.id)?.url;
            if (origin !== declared.origin) {
                throw new CallError("CONFLICT", {
                    message: `cloud machine ${row.id} is kept at ${origin ?? "no origin"}, not its declared ${declared.origin}`,
                });
            }
        }
        const missing = this.#definition.machines.filter(
            (entry) => !existing.some((row) => row.id === entry.id),
        );

        // keep the missing ones in the universe, publishing their declared origins
        await this.#mutate(
            missing.flatMap((entry) => [
                machine.call("keep", {
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

    /** Run calls as the universe in one platform request. */
    #mutate<const Calls extends readonly Call[]>(calls: Calls): Promise<ResultsOf<Calls>> {
        return this.#objects.mutate(
            this.#objects.self,
            { id: RequestId.create(this.#now), calls },
            { component: PLATFORM_COMPONENT },
        );
    }

    /** Create the platform organisation and its own account as the universe. */
    async #createOrganisation(): Promise<Identifier<"organisation">> {
        // create the organisation as the universe
        const [row] = await this.#mutate([
            organisation.call("create", {
                ...organisation.in(Scope.universe.id),
                name: this.#definition.name,
                handle: this.#definition.handle,
                residencyId: this.#definition.residencyId,
                stack: this.#definition.stacks.residency,
            }),
        ]);

        return present(row, "the platform organisation").id;
    }

    /** Run a step in one transaction, authorized as the universe in a scope. */
    #authorize<Value>(
        scope: string,
        step: (authorization: Authorization) => Promise<Value>,
    ): Promise<Value> {
        const now = this.#now;

        return Transaction.run(this.#objects.database, async (transaction) =>
            step(await this.#objects.authorize(transaction, scope, this.#objects.self, { now })),
        );
    }

    /** Bind a role to the machines and the installations serving each residency once. */
    static async #keepServers(authorization: Authorization, request: RoleRequest): Promise<void> {
        const residencies = await authorization.transaction.read(residency.table).findMany();
        await Platform.#keepBound(
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
    static async #keepBound(
        authorization: Authorization,
        request: RoleRequest,
        subjects: readonly Subject[],
    ): Promise<void> {
        // read the subjects the role is bound to
        const role = await authorization.keepRole(Scope.universe, request);
        const bindings = await Relationship.readOn(
            authorization.transaction.view(),
            Scope.universe,
            {
                role: role.id,
            },
        );
        const bound = bindings.map((row) => Relationship.subjectOf(row));

        // bind it to the others
        for (const subject of subjects) {
            if (!bound.some((existing) => Subject.same(existing, subject))) {
                await authorization.grant({ object: Scope.universe, role: role.id, subject });
            }
        }
    }

    /** Relate every machine and every signed-in caller to the universe once, through wildcards. */
    static async #keepWildcards(authorization: Authorization): Promise<void> {
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
                await authorization.grant({ object: Scope.universe, relation, subject });
            }
        }
    }

    /** Claim the reserved handles as the organisation's names in the account handle index. */
    async #keepReserved(organisationId: Identifier<"organisation">): Promise<void> {
        // claim each handle for the organisation, which lives in the universe
        const owner = Scope.universe.id;
        const claims = this.#definition.reserved.map((handle) => ({
            ...Claim.key(account, "handle", [handle]),
            packageId: account.typeReference.packageId,
            objectId: organisationId,
            owner,
        }));

        // describe the organisation as the claimant
        const claimant = {
            indexes: [Claim.index(account, "handle")],
            objectId: organisationId,
            owner,
            claims,
            state: "active" as const,
        };

        // replace the organisation's names, refusing a handle another object claims
        const [conflict] = await Claim.replace(this.#objects.database, claimant);
        if (conflict !== undefined) {
            throw new CallError("CONFLICT", {
                message: `reserved handle ${conflict.key} is claimed outside the platform organisation ${organisationId}`,
            });
        }
    }

    /** Keep an account of the organisation in its residency for each handle it publishes under. */
    async #keepPublishers(organisationId: Identifier<"organisation">): Promise<void> {
        // read the publishers that exist
        const existing = await this.#objects.database
            .select({ handle: account.table.handle })
            .from(account.table)
            .where(inArray(account.table.handle, [...this.#definition.publishers]));
        const missing = this.#definition.publishers.filter(
            (handle) => !existing.some((row) => row.handle === handle),
        );
        if (missing.length === 0) {
            return;
        }

        // create the missing ones as the organisation's shared accounts
        await this.#mutate(
            missing.map((handle) =>
                account.call("create", {
                    ...account.in(organisationId),
                    handle,
                    name: handle,
                    kind: "shared",
                    residencyId: this.#definition.residencyId,
                }),
            ),
        );
    }

    /** Keep the platform's account and home in each residency as the residency's home. */
    async #keepResidencyHomes(organisationId: Identifier<"organisation">): Promise<void> {
        for (const entry of await Residency.list(this.#objects.database)) {
            // read the platform's account in the residency
            const handle = Platform.handle(this.#definition, entry.id);
            const [existing] = await this.#objects.database
                .select({ id: account.table.id })
                .from(account.table)
                .where(eq(account.table.handle, handle));

            // create the account when missing
            const [created] =
                existing === undefined
                    ? await this.#mutate([
                          account.call("create", {
                              ...account.in(organisationId),
                              handle,
                              name: handle,
                              kind: "shared",
                              residencyId: entry.id,
                              stack: this.#definition.stacks.residency,
                          }),
                      ])
                    : [existing];
            const accountId = present(created, `the platform's account in ${entry.id}`).id;

            // keep its home at the declared stack
            await this.#keepStack(Account.home(accountId), this.#definition.stacks.residency);

            // make its home the residency's once
            const kept = await Residency.require(Snapshot.live(this.#objects.database), entry.id);
            const home = kept.home ?? Account.home(accountId);
            if (kept.home === null) {
                await this.#mutate([
                    residency.call("update", {
                        ...residency.target({ scope: Scope.universe.id, id: entry.id }),
                        home,
                    }),
                ]);
            }

            // serve the residency with every installation of its home
            await this.#authorize(Scope.universe.id, (authorization) =>
                Platform.#keepInstallations(authorization, entry.id, home),
            );
        }
    }

    /** Move a home to a stack once it keeps another. */
    async #keepStack(spaceId: Identifier<"space">, stack: SpaceStack): Promise<void> {
        // read the stack the home keeps
        const [home] = await this.#objects.database
            .select({ scope: space.table.scope, stack: space.table.stack })
            .from(space.table)
            .where(eq(space.table.id, spaceId));
        const kept = present(home, `the home ${spaceId}`);

        // move it to the declared one
        if (canonicalize(kept.stack) !== canonicalize(stack)) {
            await this.#mutate([
                space.call("upgrade", {
                    ...space.target({ scope: kept.scope, id: spaceId }),
                    stack,
                }),
            ]);
        }
    }

    /** Relate a residency's home and its installations to the residency as its servers. */
    static async #keepInstallations(
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
                await authorization.grant({ object, relation, subject });
            }
        }
    }

    /** Invite the operator as an owner unless an invitation to them was accepted or still waits. */
    async #keepInvitation(organisationId: Identifier<"organisation">): Promise<void> {
        // read the invitations to the operator's email on the organisation
        const object = organisation.reference(Scope.universe.id, organisationId);
        const contact = Contact.subject(Contact.email(this.#definition.operator));
        const invitations = await Invitation.readOn(
            Snapshot.live(this.#objects.database),
            object,
            contact,
        );

        // keep an accepted or pending invitation
        const isInvited = invitations.some(
            (row) =>
                row.status === "accepted" ||
                (row.status === "pending" && row.expiresAt > this.#now),
        );
        if (isInvited) {
            return;
        }

        // invite the operator as the universe
        await this.#authorize(organisationId, (authorization) =>
            authorization.invite({ relationship: { object, relation: "owner", subject: contact } }),
        );
    }

    /** Relate the platform residency's cloud machines as its publishing administrators. */
    async #keepPublishingMachines(
        authorization: Authorization,
        organisationId: Identifier<"organisation">,
    ): Promise<void> {
        // read the administrators the residency's machines are
        const object = organisation.reference(Scope.universe.id, organisationId);
        const subject = {
            ...residency.reference(Scope.universe.id, this.#definition.residencyId),
            relation: "machine",
        };
        const related = await Relationship.readOn(authorization.transaction.view(), object, {
            relation: "admin",
            subject,
        });

        // relate them once
        if (related.length === 0) {
            await authorization.grant({ object, relation: "admin", subject });
        }
    }
}
