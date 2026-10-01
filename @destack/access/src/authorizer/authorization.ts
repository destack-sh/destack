import { and, asc, eq, or, type DatabaseConnection, type Select } from "@destack/db";
import { Scope, type ObjectReference, Subject } from "@destack/sync";
import { Snapshot } from "@destack/db/log";
import { identifier } from "@destack/schema";
import { v7 } from "uuid";
import { AccessError } from "../error/index.ts";
import type { PermissionReference } from "../policy/policy.ts";
import * as principal from "../policy/principal.ts";
import { anyone, isPrincipal } from "../policy/principal.ts";
import { VerifiedIdentifier, AccessContext } from "../context/context.ts";
import { Relationship, type RelationshipRequest } from "../relationship/relationship.ts";
import { accessRelationship } from "../relationship/table.ts";
import { Capability, type Link } from "../relationship/link.ts";
import {
    Proposal,
    PROPOSAL_LIFETIME_MILLISECONDS,
    type ProposalPage,
    type ProposalRequest,
} from "../proposal/proposal.ts";
import { accessProposal } from "../proposal/table.ts";
import { Role, type RoleRequest } from "../role/role.ts";
import { accessRole, accessRolePermission } from "../role/table.ts";
import type { Access } from "./access.ts";
import type { Admission, Authorizer } from "./authorizer.ts";
import type { Decision } from "./decision.ts";
import type { GrantReader } from "./grant.ts";

/** The relationship fields that authorization reads from a request, proposal or stored row. */
type Grantable = Omit<RelationshipRequest, "subject" | "expiresAt"> & {
    readonly subject?: Subject;
    readonly expiresAt?: number | null;
};

/** The first relationships of a new object and the owners of a new scope. */
export interface Creation {
    /** The relations the object's first holders hold. */
    readonly relationships?: readonly {
        readonly relation: string;
        readonly subject: Subject;
    }[];
    /** The subject a new scope's owner role binds to, absent when the owners of the scopes containing it own it. */
    readonly owner?: Subject;
}

/** A caller's authorization under a set of policies in one database. */
export class Authorization {
    /** The policies deciding the caller's access. */
    readonly authorizer: Authorizer;
    /** The database, or transaction, holding the access rows that decide. */
    readonly database: DatabaseConnection;
    /** The database as its decisions read it, as each read finds it. */
    readonly snapshot: Snapshot;
    /** Bind the verified caller to a scope it acts in, rejecting scopes its credential excludes. */
    readonly #bind: (scope: string) => AccessContext;
    /** The caller's access resolved in each scope during this call, until renewed. */
    readonly #resolved = new Map<string, Promise<Access>>();
    /** The caller's access in the scopes below each scope, absent for a scope it does not enclose, until renewed. */
    readonly #below = new Map<string, Map<string, Access | undefined>>();

    /** Authorize one caller, bound to each scope it acts in, with its access already resolved in some scope. */
    constructor(
        authorizer: Authorizer,
        database: DatabaseConnection,
        bind: (scope: string) => AccessContext,
        resolved?: Access,
    ) {
        // bind the caller per scope, reusing the access resolved in one scope
        this.authorizer = authorizer;
        this.database = database;
        this.snapshot = Snapshot.live(database);
        this.#bind = bind;
        if (resolved !== undefined) {
            this.#resolved.set(resolved.scope, Promise.resolve(resolved));
        }
    }

    /** Authorize the same caller within a transaction, resolving afresh there. */
    within(transaction: DatabaseConnection): Authorization {
        return new Authorization(this.authorizer, transaction, this.#bind);
    }

    /** Bind the verified caller to a scope it acts in. */
    context(scope: string): AccessContext {
        return this.#bind(scope);
    }

    /** Resolve the caller's access in a scope once per call: its subject sets, roles and scope chain. */
    in(scope: string): Promise<Access> {
        // resolve each scope once, until renewed
        const known = this.#resolved.get(scope);
        if (known !== undefined) {
            return known;
        }
        const resolved = this.authorizer.resolve(this.snapshot, scope, this.context(scope));
        this.#resolved.set(scope, resolved);

        return resolved;
    }

    /** Forget every resolved scope so the next decision reads current access. */
    renew(): void {
        this.#resolved.clear();
        this.#below.clear();
    }

    /** Require the caller to hold permissions on a scope as a transaction shows its access, resolved afresh. */
    async #requireFresh(
        transaction: DatabaseConnection,
        scope: ObjectReference,
        permissions: readonly PermissionReference[],
    ): Promise<void> {
        const snapshot = Snapshot.live(transaction);
        const access = await this.authorizer.resolve(snapshot, scope.id, this.context(scope.id));
        await this.authorizer.require(snapshot, permissions, scope, access);
    }

    /** Decide whether the caller holds a permission on one object and until when. */
    async check(
        permission: PermissionReference,
        target: ObjectReference,
        reader?: GrantReader,
    ): Promise<Decision> {
        const access = await this.in(this.authorizer.governingScope(target));

        return this.authorizer.check(this.snapshot, permission, target, access, reader);
    }

    /** Start reading grants for decisions in a scope. */
    async reader(scope: string): Promise<GrantReader> {
        return this.authorizer.reader(this.snapshot, (await this.in(scope)).scopes);
    }

    /** Require the caller to hold a permission on one object. */
    async require(permission: PermissionReference, target: ObjectReference): Promise<void> {
        const access = await this.in(this.authorizer.governingScope(target));
        await this.authorizer.require(this.snapshot, [permission], target, access);
    }

    /** Check a permission on current or past rows of a scope and return the rows held and until when. */
    async checkRows(
        permission: PermissionReference,
        scope: string,
        rows: readonly Readonly<Record<string, unknown>>[],
        reader?: GrantReader,
        below?: ReadonlyMap<string, Access>,
    ): Promise<Admission> {
        const access = await this.in(scope);

        return this.authorizer.checkRows(this.snapshot, permission, access, rows, reader, below);
    }

    /**
     * Resolve the caller's access in scopes another scope encloses, in reads shared by all of them.
     *
     * A scope it does not enclose is left out, and a credential pinned to the enclosing scope is refused below it.
     */
    async descend(scope: string, below: readonly string[]): Promise<Map<string, Access>> {
        // resolve the scopes not resolved before together
        const known = this.#below.get(scope) ?? new Map<string, Access | undefined>();
        this.#below.set(scope, known);
        const missing = [...new Set(below)].filter((enclosed) => !known.has(enclosed));
        if (missing.length > 0) {
            const resolved = await (await this.in(scope)).descend(this.snapshot, missing);
            for (const enclosed of missing) {
                known.set(enclosed, resolved.get(enclosed));
            }

            // refuse a credential pinned to another scope in each enclosed scope
            for (const enclosed of resolved.keys()) {
                this.context(enclosed);
            }
        }

        // keep the scopes the scope encloses
        const enclosing = new Map<string, Access>();
        for (const enclosed of below) {
            const access = known.get(enclosed);
            if (access !== undefined) {
                enclosing.set(enclosed, access);
            }
        }

        return enclosing;
    }

    /**
     * Record a new object's access: a scope's place among the scopes containing it, its first relationships, and a scope's owner role.
     *
     * The object's own create permission is its caller's to require; the first holders relate without a grant, since none can precede them.
     */
    async create(object: ObjectReference, creation: Creation): Promise<void> {
        // write the access of an object this database holds
        await this.authorizer.requireHeld(this.database, object);
        const [existing] = await this.database
            .select({ id: accessRelationship.id })
            .from(accessRelationship)
            .where(Relationship.on(object))
            .limit(1);
        if (existing) {
            throw new AccessError("INVALID_CONTEXT", "only a new object relates without a grant");
        }

        // refuse owners of an object that holds no others
        const definition = this.authorizer.policy(object).definition;
        const isScope = definition.scope === true;
        if (!isScope && creation.owner !== undefined) {
            throw new AccessError("INVALID_CONTEXT", "only a new scope has owners");
        }
        const scope = this.authorizer.governingScope(object);
        const context = this.context(scope);

        // record a scope below the scopes containing it
        if (isScope) {
            const [parent] =
                object.scope === Scope.universe.id
                    ? []
                    : await this.database
                          .select({ ancestors: Scope.table.ancestors })
                          .from(Scope.table)
                          .where(eq(Scope.table.scope, object.scope));
            if (object.scope !== Scope.universe.id && parent === undefined) {
                throw new AccessError("NOT_FOUND", `unknown scope: ${object.scope}`);
            }
            await this.database.insert(Scope.table).values({
                scope: object.id,
                parent: object.scope,
                packageId: object.packageId,
                type: object.type,
                ancestors:
                    object.scope === Scope.universe.id ? [] : [object.scope, ...parent!.ancestors],
            });
        }

        // relate the first holders
        const relationships = this.authorizer.initialRelationships(object, creation, context.now);
        if (relationships.length > 0) {
            await this.database.insert(accessRelationship).values(relationships);
        }

        // define a scope's owner role and bind it to its owner
        if (creation.owner !== undefined) {
            await Role.own(this.database, object, creation.owner, context.now);
        }
        this.renew();
    }

    /** Suspend a scope and withhold every permission in it except administration until resumed. */
    async suspend(object: ObjectReference): Promise<void> {
        await this.#suspension(object, this.context(object.id).now);
    }

    /** Resume a suspended scope; its caller requires the permission to. */
    async resume(object: ObjectReference): Promise<void> {
        await this.#suspension(object, null);
    }

    /** Relate a subject to an object through a relation or a bound role, as the caller may grant. */
    async grant(request: RelationshipRequest): Promise<Relationship> {
        const relationship = await this.database.transaction(async (transaction) => {
            const authorization = this.within(transaction);
            await authorization.authorizeGrant(request);

            return authorization.#insert(request);
        });
        this.renew();

        return relationship;
    }

    /** Revoke one relationship of an object, as the caller may; the log keeps its history. */
    async revoke(object: ObjectReference, id: string): Promise<Relationship> {
        return this.database.transaction(async (transaction) => {
            // load the object's relationship inside the authorization transaction
            const authorization = this.within(transaction);
            const key = identifier("relationship").parse(id);
            const [row] = await transaction
                .select()
                .from(accessRelationship)
                .where(and(Relationship.on(object), eq(accessRelationship.id, key)));
            if (!row) {
                throw new AccessError("NOT_FOUND", "relationship not found");
            }

            // require permission to revoke what it grants, and keep an owner
            await this.authorizer.requireHeld(transaction, object);
            requireUnmanaged(row);
            const relationship = Relationship.decode(row);
            await authorization.authorizeRevoke(relationship);
            await requireRemainingOwner(transaction, row);
            await transaction.delete(accessRelationship).where(eq(accessRelationship.id, key));
            this.renew();

            return relationship;
        });
    }

    /** Relate the holder of a new secret to an object and return the secret once. */
    async link(request: {
        readonly object: ObjectReference;
        readonly relation: string;
        readonly expiresAt?: number;
    }): Promise<Link> {
        const capability = await Capability.create();
        const relationship = await this.grant({
            ...request,
            subject: anyone.reference("*", "*"),
            conditions: { capability: capability.digest },
        });

        return { id: relationship.id, secret: capability.secret };
    }

    /**
     * Propose a relationship that applies only once accepted.
     *
     * A principal asks for a relationship with itself as subject; a grantor offers one to a principal or to whoever proves a recipient identifier.
     */
    async propose(request: ProposalRequest): Promise<Proposal> {
        return this.database.transaction(async (transaction) => {
            // require an authenticated proposer, an object this database holds, and exactly one subject or recipient
            const authorization = this.within(transaction);
            const proposed = request.relationship;
            await this.authorizer.requireHeld(transaction, proposed.object);
            const context = this.context(this.authorizer.governingScope(proposed.object));
            const proposer = AccessContext.requirePrincipal(context);
            if (
                (proposed.subject === undefined) === (request.recipient === undefined) ||
                (request.recipient !== undefined &&
                    !VerifiedIdentifier.safeParse(request.recipient).success)
            ) {
                throw new AccessError(
                    "FORBIDDEN",
                    "proposal needs exactly one subject or recipient",
                );
            }
            this.authorizer.validate(proposed, context.now);
            const expiresAt = request.expiresAt ?? context.now + PROPOSAL_LIFETIME_MILLISECONDS;
            if (!Number.isFinite(expiresAt) || expiresAt <= context.now) {
                throw new AccessError("FORBIDDEN", "proposal must lapse in the future");
            }

            // let a principal ask for itself, and require grant authority to offer to anyone else
            if (proposed.subject === undefined || !Subject.same(proposed.subject, proposer)) {
                if (
                    proposed.subject !== undefined &&
                    (proposed.subject.relation !== undefined ||
                        proposed.subject.id === "*" ||
                        proposed.subject.scope === "*")
                ) {
                    throw new AccessError("FORBIDDEN", "proposal subject must be one principal");
                }
                await authorization.authorizeGrant(proposed);
            }

            // store the proposal apart from the relationships that apply
            const proposal: Proposal = {
                id: identifier("proposal").parse(`proposal-${v7()}`),
                relationship: {
                    object: proposed.object,
                    ...(proposed.relation === undefined ? {} : { relation: proposed.relation }),
                    ...(proposed.role === undefined ? {} : { role: proposed.role }),
                    ...(proposed.subject === undefined ? {} : { subject: proposed.subject }),
                    expiresAt: proposed.expiresAt ?? null,
                    ...(proposed.conditions === undefined
                        ? {}
                        : { conditions: proposed.conditions }),
                },
                ...(request.recipient === undefined ? {} : { recipient: request.recipient }),
                proposer,
                ...(request.purpose === undefined ? {} : { purpose: request.purpose }),
                createdAt: context.now,
                expiresAt,
            };
            await transaction
                .insert(accessProposal)
                .values(Proposal.encode(proposal, this.authorizer.governingScope(proposed.object)));

            return proposal;
        });
    }

    /**
     * Accept a proposal on an object so its relationship applies.
     *
     * A grantor accepts a principal's request; the offered principal, or whoever proves the recipient identifier, accepts an offer and becomes its subject.
     */
    async accept(object: ObjectReference, id: string): Promise<Relationship> {
        return this.database.transaction(async (transaction) => {
            // load the current proposal inside the authorization transaction
            const authorization = this.within(transaction);
            const context = this.context(this.authorizer.governingScope(object));
            const proposal = await Proposal.read(transaction, object, id);
            if (proposal.expiresAt <= context.now) {
                throw new AccessError("FORBIDDEN", "proposal has lapsed");
            }

            // require a grantor for a request
            const proposed = proposal.relationship;
            let subject: Subject;
            if (Proposal.asksForItself(proposal)) {
                await authorization.authorizeGrant(proposed);
                subject = proposed.subject!;
            }
            // require the addressed principal for an offer
            else {
                const accepting = AccessContext.requirePrincipal(context);
                if (!Proposal.addresses(proposal, accepting, context)) {
                    throw new AccessError("FORBIDDEN", "proposal is addressed to someone else");
                }
                subject = accepting;
                const proposer = new Authorization(this.authorizer, transaction, () => ({
                    subjects: [proposal.proposer],
                    now: context.now,
                    attributes: {},
                }));
                await proposer.authorizeGrant(proposed);
            }

            // relate the accepting subject and retire the proposal
            const { expiresAt, ...granted } = proposed;
            const relationship = await authorization.#insert({
                ...granted,
                subject,
                ...(expiresAt === null ? {} : { expiresAt }),
            });
            await transaction.delete(accessProposal).where(eq(accessProposal.id, Proposal.id(id)));
            this.renew();

            return relationship;
        });
    }

    /** Decline a proposal on an object: its proposer withdrawing, its addressee refusing, or a grantor rejecting it. */
    async decline(object: ObjectReference, id: string): Promise<Proposal> {
        return this.database.transaction(async (transaction) => {
            // let the proposer and the addressee decline without grant authority
            const authorization = this.within(transaction);
            const context = this.context(this.authorizer.governingScope(object));
            const proposal = await Proposal.read(transaction, object, id);
            const acting = AccessContext.principal(context);
            if (
                acting === undefined ||
                (!Subject.same(acting, proposal.proposer) &&
                    !Proposal.addresses(proposal, acting, context))
            ) {
                await authorization.authorizeRevoke(proposal.relationship);
            }
            await transaction.delete(accessProposal).where(eq(accessProposal.id, Proposal.id(id)));

            return proposal;
        });
    }

    /** List a page of the proposals pending on an object, ordered by identifier, as its grantors may. */
    async proposals(
        request: Pick<RelationshipRequest, "object" | "relation" | "role">,
        page: ProposalPage,
    ): Promise<Proposal[]> {
        return this.database.transaction(async (transaction) => {
            // require the grant permission, then read the object's proposals after the cursor
            const context = this.context(this.authorizer.governingScope(request.object));
            await this.within(transaction).authorizeRevoke(request);
            const rows = await transaction
                .select()
                .from(accessProposal)
                .where(
                    and(
                        Proposal.on(request.object),
                        request.relation === undefined
                            ? undefined
                            : eq(accessProposal.relation, request.relation),
                        request.role === undefined
                            ? undefined
                            : eq(accessProposal.roleId, identifier("role").parse(request.role)),
                        Proposal.pending(page, context.now),
                    ),
                )
                .orderBy(asc(accessProposal.id))
                .limit(page.limit);

            return rows.map(Proposal.decode);
        });
    }

    /** List a page of the proposals the caller made, may lend authority for, or receives in a scope. */
    async addressed(scope: string, page: ProposalPage): Promise<Proposal[]> {
        // match the caller as proposer, lender, subject, or through a verified identifier
        const context = this.context(scope);
        const own = Subject.key(AccessContext.requirePrincipal(context));
        const rows = await this.database
            .select()
            .from(accessProposal)
            .where(
                and(
                    or(
                        eq(accessProposal.proposerKey, own),
                        eq(accessProposal.lender, own),
                        ...[own, ...AccessContext.identifiers(context)].map((addressee) =>
                            eq(accessProposal.addressee, addressee),
                        ),
                    ),
                    Proposal.pending(page, context.now),
                ),
            )
            .orderBy(asc(accessProposal.id))
            .limit(page.limit);

        return rows.map(Proposal.decode);
    }

    /** Define a role in a scope, granting only permissions the caller holds there. */
    async createRole(scope: ObjectReference, request: RoleRequest): Promise<Role> {
        return this.database.transaction(async (transaction) => {
            // require permission to define roles and every permission the role grants
            await this.authorizer.requireHeld(transaction, scope);
            const context = this.context(scope.id);
            await this.#requireFresh(transaction, scope, [
                principal.role.permission("create"),
                ...request.permissions,
            ]);

            // insert the role under a name free in the scope, then its permissions
            const [record] = await transaction
                .insert(accessRole)
                .values({
                    id: identifier("role").parse(`role-${v7()}`),
                    createdAt: context.now,
                    updatedAt: context.now,
                    scope: scope.id,
                    name: request.name,
                    description: request.description,
                })
                .onConflictDoNothing()
                .returning();
            if (!record) {
                throw new AccessError("CONFLICT", "role name is already in use");
            }
            await Role.permit(transaction, record.id, scope.id, request.permissions);
            this.renew();

            return Role.describe(record, request.permissions);
        });
    }

    /** Change a role's name, purpose or permissions at a revision within the caller's permissions. */
    async updateRole(
        scope: ObjectReference,
        id: string,
        changes: Partial<RoleRequest> & { readonly revision: number },
    ): Promise<Role> {
        return this.database.transaction(async (transaction) => {
            // require an editable role at the revision, permission to change roles, and every permission granted
            await this.authorizer.requireHeld(transaction, scope);
            const context = this.context(scope.id);
            const record = await Role.read(transaction, scope.id, id);
            requireUnmanaged(record);
            if (record.revision !== changes.revision) {
                throw new AccessError("CONFLICT", "role revision has changed");
            }
            await this.#requireFresh(transaction, scope, [
                principal.role.permission("update"),
                ...(changes.permissions ?? []),
            ]);

            // apply the changes and replace the permissions when given
            const [updated] = await transaction
                .update(accessRole)
                .set({
                    ...(changes.name === undefined ? {} : { name: changes.name }),
                    ...(changes.description === undefined
                        ? {}
                        : { description: changes.description }),
                    revision: record.revision + 1,
                    updatedAt: context.now,
                })
                .where(and(eq(accessRole.id, record.id), eq(accessRole.revision, record.revision)))
                .returning();
            if (!updated) {
                throw new AccessError("CONFLICT", "role revision has changed");
            }
            if (changes.permissions !== undefined) {
                await transaction
                    .delete(accessRolePermission)
                    .where(eq(accessRolePermission.roleId, record.id));
                await Role.permit(transaction, record.id, scope.id, changes.permissions);
            }
            this.renew();

            return Role.describe(
                updated,
                changes.permissions ?? (await Role.permissions(transaction, record.id)),
            );
        });
    }

    /** Delete an unbound, editable role at a revision. */
    async deleteRole(scope: ObjectReference, id: string, revision: number): Promise<Role> {
        return this.database.transaction(async (transaction) => {
            // require an editable role at the revision and permission to delete roles
            await this.authorizer.requireHeld(transaction, scope);
            const record = await Role.read(transaction, scope.id, id);
            requireUnmanaged(record);
            if (record.revision !== revision) {
                throw new AccessError("CONFLICT", "role revision has changed");
            }
            await this.#requireFresh(transaction, scope, [principal.role.permission("delete")]);

            // refuse deleting a role that is still bound or included
            const [binding] = await transaction
                .select({ id: accessRelationship.id })
                .from(accessRelationship)
                .where(
                    or(
                        eq(accessRelationship.roleId, record.id),
                        and(
                            eq(
                                accessRelationship.subjectPackageId,
                                principal.role.definition.packageId,
                            ),
                            eq(accessRelationship.subjectType, principal.role.name),
                            eq(accessRelationship.subjectId, record.id),
                        ),
                    ),
                )
                .limit(1);
            if (binding) {
                throw new AccessError("CONFLICT", "role is still bound or included");
            }
            const permissions = await Role.permissions(transaction, record.id);
            await transaction.delete(accessRole).where(eq(accessRole.id, record.id));
            this.renew();

            return Role.describe(record, permissions);
        });
    }

    /** Require the grant permission and, for a role, every permission the role grants; a delegation needs only its lender. */
    protected async authorizeGrant(request: Grantable): Promise<void> {
        // write only the access of an object this database holds
        await this.authorizer.requireHeld(this.database, request.object);

        // lend only the caller's own authority, and only to one principal
        const scope = this.authorizer.governingScope(request.object);
        const context = this.context(scope);
        const onBehalfOf = request.conditions?.onBehalfOf;
        if (onBehalfOf !== undefined) {
            if (!this.#lends(request)) {
                throw new AccessError("FORBIDDEN", "only a principal may lend its own authority");
            }
            const delegate = request.subject;
            if (
                delegate === undefined ||
                !isPrincipal(delegate) ||
                delegate.id === "*" ||
                delegate.scope === "*"
            ) {
                throw new AccessError("FORBIDDEN", "a delegate must be one principal");
            }
        }

        // require the relation's grant permission to grant or lend it
        const access = await this.authorizer.resolve(this.snapshot, scope, context);
        const required: PermissionReference[] =
            onBehalfOf === undefined || request.role === undefined
                ? [this.#grantPermission(request)]
                : [];

        // prevent escalation: bind or lend only scope chain roles with permissions the caller holds
        const grants = request.role === undefined ? undefined : access.grants.get(request.role);
        if (request.role !== undefined && grants === undefined) {
            throw new AccessError("NOT_FOUND", "role not found");
        }
        required.push(...(grants?.permissions ?? []));
        await this.authorizer.require(this.snapshot, required, request.object, access);

        // require ownership to bind a role granting everything
        if (
            grants?.isUniversal &&
            !(await this.authorizer.owns(this.snapshot, request.object, access))
        ) {
            throw new AccessError("FORBIDDEN", "only owners may bind a role granting everything");
        }
    }

    /** Require the permission granting what a relationship grants, unless the caller lent it. */
    protected async authorizeRevoke(request: Grantable): Promise<void> {
        // let a lender manage its own delegations
        if (this.#lends(request)) {
            return;
        }
        const scope = this.authorizer.governingScope(request.object);
        const access = await this.authorizer.resolve(this.snapshot, scope, this.context(scope));
        await this.authorizer.require(
            this.snapshot,
            [this.#grantPermission(request)],
            request.object,
            access,
        );
    }

    /** Insert a valid relationship under a new identifier and refuse a duplicate. */
    async #insert(request: RelationshipRequest): Promise<Relationship> {
        // build the relationship
        const scope = this.authorizer.governingScope(request.object);
        const context = this.context(scope);
        this.authorizer.validate(request, context.now);
        const relationship: Relationship = {
            id: identifier("relationship").parse(`relationship-${v7()}`),
            object: request.object,
            ...(request.relation === undefined ? {} : { relation: request.relation }),
            ...(request.role === undefined ? {} : { role: request.role }),
            subject: request.subject,
            createdAt: context.now,
            expiresAt: request.expiresAt ?? null,
            ...(request.conditions === undefined ? {} : { conditions: request.conditions }),
        };

        // insert it unless the same relationship exists
        const [inserted] = await this.database
            .insert(accessRelationship)
            .values(Relationship.encode(relationship, scope))
            .onConflictDoNothing()
            .returning({ id: accessRelationship.id });
        if (!inserted) {
            throw new AccessError("CONFLICT", "relationship already exists");
        }

        return relationship;
    }

    /** Determine whether the caller is the principal a delegation lends authority from. */
    #lends(request: Grantable): boolean {
        const onBehalfOf = request.conditions?.onBehalfOf;
        const caller = AccessContext.principal(
            this.context(this.authorizer.governingScope(request.object)),
        );

        return onBehalfOf !== undefined && caller !== undefined && Subject.same(caller, onBehalfOf);
    }

    /** Resolve the permission granting a relation, or binding roles, on the object. */
    #grantPermission(request: Grantable): PermissionReference {
        // read the relation's grant, or the type's for a role binding
        const policy = this.authorizer.policy(request.object);
        const grant =
            request.relation === undefined
                ? policy.definition.grantedBy
                : this.authorizer.relation(policy, request.relation).grantedBy;
        if (grant === undefined) {
            throw new AccessError("FORBIDDEN", "relationship cannot be granted");
        }

        return policy.permission(grant);
    }

    /** Set when a scope this database holds was suspended, or clear it. */
    async #suspension(object: ObjectReference, suspendedAt: number | null): Promise<void> {
        // require a scope this database holds
        await this.authorizer.requireHeld(this.database, object);
        if (this.authorizer.policy(object).definition.scope !== true) {
            throw new AccessError("INVALID_CONTEXT", `${object.type} is not a scope`);
        }

        // mark the scope's record
        const [marked] = await this.database
            .update(Scope.table)
            .set({ suspendedAt })
            .where(eq(Scope.table.scope, object.id))
            .returning({ scope: Scope.table.scope });
        if (marked === undefined) {
            throw new AccessError("NOT_FOUND", `unknown scope: ${object.id}`);
        }
        this.renew();
    }
}

/** Refuse changing a row that a declaration manages until it is detached. */
function requireUnmanaged(record: {
    readonly managerInstallationId: string | null;
    readonly detachedAt: number | null;
}): void {
    if (record.managerInstallationId !== null && record.detachedAt === null) {
        throw new AccessError("CONFLICT", "record is managed by its source declaration");
    }
}

/** Refuse removing the last binding of a role that grants everything on an object. */
async function requireRemainingOwner(
    database: DatabaseConnection,
    row: Select<typeof accessRelationship>,
): Promise<void> {
    // skip relations
    if (row.roleId === null) {
        return;
    }

    // read the object's bindings of roles granting everything
    const owners = await database
        .select({ id: accessRelationship.id })
        .from(accessRelationship)
        .innerJoin(accessRole, eq(accessRole.id, accessRelationship.roleId))
        .where(
            and(Relationship.on(Relationship.decode(row).object), eq(accessRole.isUniversal, true)),
        );

    // refuse removing the only such binding
    if (owners.length === 1 && owners[0]!.id === row.id) {
        throw new AccessError("CONFLICT", "the last owner cannot be removed");
    }
}
