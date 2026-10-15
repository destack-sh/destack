import {
    type DatabaseConnection,
    Transaction,
    type Row,
    type RowImage,
    type Select,
    type Snapshot,
    Table,
} from "@destack/db";
import { ObjectReference, Scope, Subject } from "@destack/sync";
import { aligned, type Identifier, schema } from "@destack/schema";
import { v7 } from "uuid";
import { AccessError } from "../error/index.ts";
import { PermissionReference } from "../declare/policy.ts";
import * as policies from "../declare/principal.ts";
import { anyone, isPrincipal } from "../declare/principal.ts";
import { AccessContext, Caller } from "../context/context.ts";
import { HIGHEST_ASSURANCE } from "../context/elevation.ts";
import {
    Relationship,
    type RelationshipCondition,
    type RelationshipSelection,
    type RelationshipRequest,
} from "../relationship/relationship.ts";
import { accessRelationship } from "../relationship/table.ts";
import { LinkSecret, type Link } from "../relationship/link.ts";
import {
    Invitation,
    INVITATION_LIFETIME_MILLISECONDS,
    type InvitationPage,
    type InvitationRequest,
} from "../invitation/invitation.ts";
import { accessInvitation } from "../invitation/table.ts";
import { Manager } from "../manager/manager.ts";
import { Role, type RoleRequest } from "../role/role.ts";
import { accessRole, accessRolePermission } from "../role/table.ts";
import { Grant, type GrantRequest } from "../grant/grant.ts";
import { accessGrant } from "../grant/table.ts";
import type { Access } from "./access.ts";
import type { Admission, Authorizer } from "./authorizer.ts";
import type { Decision } from "./decision.ts";
import type { TupleReader, Lookup } from "./tuple.ts";

/** The relationship fields that authorization reads from a request, invitation or stored row. */
type RelationshipFields = {
    /** The related object. */
    readonly object: ObjectReference;
    /** The subject, absent while an offer awaits its recipient. */
    readonly subject?: Subject;
    /** What a request must satisfy for the relationship to apply. */
    readonly conditions?: RelationshipCondition;
} & ({ readonly relation: string } | { readonly role: string });

/** The first relationships of a new object and the owners of a new scope. */
export interface Creation {
    /** The relationships the object starts with. */
    readonly relationships?: readonly {
        readonly relation: string;
        readonly subject: Subject;
    }[];
    /** The subject of a new scope's owner role, absent if its containing scopes' owners suffice. */
    readonly owner?: Subject;
}

/** A caller's authorization under a set of policies in one database. */
export class Authorization {
    /** The policies deciding the caller's access. */
    readonly authorizer: Authorizer;
    /** The transaction that reads and writes the access rows. */
    readonly transaction: Transaction;
    /** The database as its decisions read it: the transaction's view, recording each read. */
    readonly snapshot: Snapshot;
    /** Read the verified caller at the time it acts. */
    readonly #context: () => AccessContext;
    /** The caller's access resolved in each scope during this call, until renewed. */
    readonly #resolved: Map<string, Promise<Access>>;
    /** The caller's access in the scopes below each scope until renewed, absent outside it. */
    readonly #below: Map<string, Map<string, Access | undefined>>;
    /** The applied declaration, the only writer of its rows, null for direct access changes. */
    #manager: Manager | null;

    /** Authorize one caller in a transaction, with access in some scope. */
    constructor(
        authorizer: Authorizer,
        transaction: Transaction,
        context: () => AccessContext,
        resolved?: Access,
    ) {
        // start with empty state
        this.#resolved = new Map<string, Promise<Access>>();
        this.#below = new Map<string, Map<string, Access | undefined>>();
        this.#manager = null;

        // keep the caller, reusing the access resolved in one scope
        this.authorizer = authorizer;
        this.transaction = transaction;
        this.snapshot = transaction.view();
        this.#context = context;
        if (resolved !== undefined) {
            this.#resolved.set(resolved.scope, Promise.resolve(resolved));
        }
    }

    /** Run a call with one caller's authorization in a database transaction until commit. */
    static run<Value>(
        authorizer: Authorizer,
        database: DatabaseConnection,
        context: () => AccessContext,
        call: (authorization: Authorization) => Promise<Value>,
    ): Promise<Value> {
        return Transaction.run(database, (transaction) =>
            call(new Authorization(authorizer, transaction, context)),
        );
    }

    /** Authorize the caller to apply a declaration that manages its rows. */
    applying(manager: Manager): Authorization {
        const applying = new Authorization(this.authorizer, this.transaction, this.#context);
        applying.#manager = manager;

        return applying;
    }

    /** Read the verified caller as it acts now. */
    context(): AccessContext {
        return this.#context();
    }

    /** Resolve the caller's access in a scope once per call: subject sets, roles, scope chain. */
    in(scope: string): Promise<Access> {
        // resolve each scope once, until renewed
        const known = this.#resolved.get(scope);
        if (known !== undefined) {
            return known;
        }
        const resolved = this.authorizer.resolve(this.snapshot, scope, this.context());
        this.#resolved.set(scope, resolved);

        return resolved;
    }

    /** Forget every resolved scope so the next decision reads current access. */
    renew(): void {
        this.#resolved.clear();
        this.#below.clear();
    }

    /** Decide whether the caller has a permission on one object and until when. */
    async check(
        permission: PermissionReference,
        target: ObjectReference,
        reader?: TupleReader,
    ): Promise<Decision> {
        const access = await this.in(this.authorizer.governingScope(target));

        return this.authorizer.check(this.snapshot, permission, target, access, reader);
    }

    /** Start reading tuples for decisions in a scope. */
    async reader(scope: string): Promise<TupleReader> {
        return this.authorizer.reader(this.snapshot, (await this.in(scope)).scopes);
    }

    /** Require the caller to have a permission on one object. */
    async require(permission: PermissionReference, target: ObjectReference): Promise<void> {
        const access = await this.in(this.authorizer.governingScope(target));
        await this.authorizer.require(this.snapshot, [permission], target, access);
    }

    /** Check a permission on a scope's current or past rows. */
    async checkRows(
        permission: PermissionReference,
        scope: string,
        rows: readonly Row[],
        reader?: TupleReader,
        below?: ReadonlyMap<string, Access>,
        lookup?: Lookup,
    ): Promise<Admission> {
        const access = await this.in(scope);

        return this.authorizer.checkRows(
            this.snapshot,
            permission,
            access,
            rows,
            reader,
            below,
            lookup,
        );
    }

    /** Resolve the caller's access in a scope's enclosed scopes, in reads shared by all. */
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

    /** Record a new object's access: a new scope's row, and its access where kept here. */
    async create(object: ObjectReference, creation: Creation): Promise<void> {
        // record a new scope below the scopes containing it, none below the universe
        const isScope = this.authorizer.policy(object).definition.scope === true;
        if (isScope) {
            const ancestors =
                object.scope === Scope.universe.id
                    ? []
                    : [object.scope, ...(await this.#parent(object.scope)).ancestors];
            await this.#record(object, ancestors);
        }

        // relate its first subjects here, or leave a scope homed elsewhere to its home's birth
        if (!isScope || (await this.authorizer.isLocal(this.transaction, object))) {
            await this.#relate(object, creation);
        }
        this.renew();
    }

    /** Record a scope at its home as it is born, below its containing scopes, with its access. */
    async receive(
        object: ObjectReference,
        ancestors: readonly string[],
        creation: Creation,
    ): Promise<void> {
        await this.#record(object, ancestors);
        await this.#relate(object, creation);
        this.renew();
    }

    /** Forget a removed object's access kept here: relationships, a scope's row and own access. */
    async remove(object: ObjectReference): Promise<void> {
        // require the object's row here: a scope object's row lives in the scope containing it
        const isScope = this.authorizer.policy(object).definition.scope === true;
        const view = this.transaction.view();
        await this.authorizer.requireLocal(
            this.transaction,
            isScope ? await Scope.object(view, object.scope) : object,
        );

        // read the relationships on the object, and a scope's own access only at its home
        const related = await this.transaction
            .read(accessRelationship)
            .findMany({ where: Relationship.objectColumns(object) });
        const isHome = isScope && (await this.authorizer.keeps(view, object.id));
        const inside = isHome
            ? await Promise.all([
                  this.transaction
                      .read(accessRelationship)
                      .findMany({ where: { scope: object.id } }),
                  this.transaction.read(accessInvitation).findMany({ where: { scope: object.id } }),
                  this.transaction.read(accessRole).findMany({ where: { scope: object.id } }),
              ])
            : undefined;
        const scope = isScope
            ? await this.transaction.read(Scope.table).findFirst({ where: { scope: object.id } })
            : undefined;

        // delete the relationships once each, then the scope's invitations and roles with their permissions
        const relationships = new Map(
            [...related, ...(inside?.[0] ?? [])].map((row) => [row.id, row]),
        );
        const roles = inside?.[2] ?? [];
        const permissions = await this.transaction.read(accessRolePermission).findMany({
            where: {
                roleId: { in: roles.map((row) => row.id) },
            },
        });
        await this.transaction.write(
            ...[...relationships.values()].map((row) => ({
                table: accessRelationship,
                before: row,
                after: null,
            })),
            ...(inside?.[1] ?? []).map((row) => ({
                table: accessInvitation,
                before: row,
                after: null,
            })),
            ...permissions.map((row) => ({
                table: accessRolePermission,
                before: row,
                after: null,
            })),
            ...roles.map((row) => ({ table: accessRole, before: row, after: null })),
        );

        // forget the scope's row last, which the containing scope's home keeps
        if (scope !== undefined) {
            await this.transaction.write({ table: Scope.table, before: scope, after: null });
        }
        this.renew();
    }

    /** Relate a local object's first subjects, and bind a scope's owner. */
    async #relate(object: ObjectReference, creation: Creation): Promise<void> {
        // write the access of a local object with no relationships yet
        await this.authorizer.requireLocal(this.transaction, object);
        const related = await this.transaction
            .read(accessRelationship)
            .findMany({ where: Relationship.objectColumns(object), limit: 1 });
        if (related.length > 0) {
            throw new AccessError("INVALID_CONTEXT", "a relationship needs a grant after creation");
        }

        // refuse owners of an object that is not a scope
        const definition = this.authorizer.policy(object).definition;
        if (definition.scope !== true && creation.owner !== undefined) {
            throw new AccessError("INVALID_CONTEXT", "only a scope can have owners");
        }
        const context = this.context();

        // relate the first subjects
        const relationships = this.authorizer.initialRelationships(object, creation, context.now);
        await this.transaction.write(
            ...relationships.map((relationship) => ({
                table: accessRelationship,
                before: null,
                after: Table.complete(accessRelationship, relationship),
            })),
        );

        // bind the owner role on a scope to its owner
        if (creation.owner !== undefined) {
            await Role.bindOwner(this.transaction, object, creation.owner, context.now);
        }
    }

    /** Record a scope's row below its ancestors, in its own partition or its parent's. */
    async #record(object: ObjectReference, ancestors: readonly string[]): Promise<void> {
        // write the row in the scope's own partition or its parent's
        const definition = this.authorizer.policy(object).definition;
        const partition =
            definition.partition === true
                ? object.id
                : (await this.#parent(object.scope)).partition;
        const row = Table.complete(Scope.table, {
            scope: object.id,
            parent: object.scope,
            partition,
            packageId: object.packageId,
            type: object.type,
            ancestors: [...ancestors],
        });
        await this.transaction.write({ table: Scope.table, before: null, after: row });
    }

    /** Read the row of a new scope's parent, refusing a scope this database lacks. */
    async #parent(scope: string): Promise<RowImage<typeof Scope.table>> {
        const [row] = await Scope.read(this.snapshot, [scope]);
        if (row === undefined) {
            throw new AccessError("NOT_FOUND", `unknown scope: ${scope}`);
        }

        return row;
    }

    /** Suspend a scope and withhold every permission in it except administration until resumed. */
    async suspend(object: ObjectReference): Promise<void> {
        await this.#mark(object, { suspendedAt: this.context().now });
    }

    /** Resume a suspended scope, as a caller with that permission. */
    async resume(object: ObjectReference): Promise<void> {
        await this.#mark(object, { suspendedAt: null });
    }

    /** Fence a scope and its subtree for a move to another machine. */
    async fence(object: ObjectReference, machine: string): Promise<void> {
        await this.#mark(object, { fencedAt: this.context().now, movedTo: machine });
    }

    /** Lift a scope's fence on the database keeping it. */
    async unfence(object: ObjectReference): Promise<void> {
        await this.#mark(object, { fencedAt: null, movedTo: null });
    }

    /** Relate a subject to an object by a relation or bound role, as the caller may grant. */
    async grant(request: RelationshipRequest): Promise<Relationship> {
        const relationship = await this.transaction.savepoint(async () => {
            await this.authorizeGrant(request);

            return this.#insert(request);
        });
        this.renew();

        return relationship;
    }

    /** Revoke one relationship of an object as the caller may, keeping its history in the log. */
    async revoke(object: ObjectReference, id: string): Promise<Relationship> {
        return this.transaction.savepoint(async () => {
            // read the object's relationship
            const key = schema.identifier("relationship").parse(id);
            const row = await this.transaction.read(accessRelationship).findFirst({
                where: {
                    ...Relationship.objectColumns(object),
                    id: key,
                },
            });
            if (!row) {
                throw new AccessError("NOT_FOUND", "relationship not found");
            }

            // require permission to revoke what it grants, and keep an owner
            const relationship = Relationship.decode(row);
            await this.authorizeRevoke(relationship, row);
            await Authorization.#requireRemainingOwner(this.transaction, row);
            await this.transaction.write({ table: accessRelationship, before: row, after: null });
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
        const linkSecret = await LinkSecret.create();
        const relationship = await this.grant({
            ...request,
            subject: anyone.reference("*", "*"),
            conditions: { linkSecret: linkSecret.digest },
        });

        return { id: relationship.id, secret: linkSecret.secret };
    }

    /** Invite a relationship that applies once accepted, asked by its subject or offered. */
    async invite(request: InvitationRequest): Promise<Invitation> {
        return this.transaction.savepoint(async () => {
            // require a local object and an inviting principal or the system
            const invited = request.relationship;
            await this.authorizer.requireLocal(this.transaction, invited.object);
            const context = this.context();
            const inviter = Caller.principal(context) ?? null;

            // validate the relationship, checking an invited contact's principal once it accepts
            const { subject, ...validated } = invited;
            this.authorizer.validate(
                policies.principal.contact.is(subject) ? validated : invited,
                context.now,
            );
            const expiresAt = request.expiresAt ?? context.now + INVITATION_LIFETIME_MILLISECONDS;
            if (!Number.isFinite(expiresAt) || expiresAt <= context.now) {
                throw new AccessError("FORBIDDEN", "invitation must expire in the future");
            }

            // let a principal ask for itself, and require grant authority to offer to anyone else
            if (inviter === null || !Subject.same(invited.subject, inviter)) {
                if (
                    invited.subject.relation !== undefined ||
                    invited.subject.id === "*" ||
                    invited.subject.scope === "*"
                ) {
                    throw new AccessError("FORBIDDEN", "invitation subject must be one principal");
                }
                await this.authorizeGrant(invited);
            }

            // store the invitation apart from the relationships that apply
            const invitation: Invitation = {
                id: schema.identifier("invitation").parse(`invitation-${v7()}`),
                relationship: {
                    object: invited.object,
                    ...Relationship.via(invited),
                    subject: invited.subject,
                    expiresAt: invited.expiresAt ?? null,
                    ...(invited.conditions === undefined ? {} : { conditions: invited.conditions }),
                },
                inviter,
                ...(request.purpose === undefined ? {} : { purpose: request.purpose }),
                status: "pending",
                createdAt: context.now,
                expiresAt,
            };
            const scope = this.authorizer.governingScope(invited.object);
            const row = Table.complete(accessInvitation, Invitation.encode(invitation, scope));
            await this.transaction.write({ table: accessInvitation, before: null, after: row });

            return invitation;
        });
    }

    /** Accept an invitation on an object so its relationship applies. */
    async accept(object: ObjectReference, id: string): Promise<Relationship> {
        return this.transaction.savepoint(async () => {
            // read the pending invitation
            const context = this.context();
            const invitation = await Invitation.read(this.transaction, object, id);
            if (invitation.expiresAt <= context.now) {
                throw new AccessError("FORBIDDEN", "invitation has lapsed");
            }

            // require a grantor for a request
            const invited = invitation.relationship;
            let subject: Subject;
            if (Invitation.asksForItself(invitation)) {
                await this.authorizeGrant(invited);
                subject = invitation.relationship.subject;
            }
            // require the addressed principal for an offer, and an inviter that still has the grant
            else {
                const accepting = Caller.requirePrincipal(context);
                if (!Invitation.addresses(invitation, accepting, context)) {
                    throw new AccessError("FORBIDDEN", "invitation is addressed to someone else");
                }
                subject = accepting;
                await this.#requireInviter(invitation, context.now);
            }

            // relate the accepting subject and settle the invitation
            const { expiresAt, ...granted } = invited;
            const relationship = await this.#insert({
                ...granted,
                subject,
                ...(expiresAt === null ? {} : { expiresAt }),
            });
            await this.#settle(id, "accepted", context.now);
            this.renew();

            return relationship;
        });
    }

    /** Withdraw an invitation on an object as its inviter, addressee, or a revoking grantor. */
    async withdraw(object: ObjectReference, id: string): Promise<Invitation> {
        return this.transaction.savepoint(async () => {
            // let the inviting principal and the addressee withdraw without grant authority
            const context = this.context();
            const invitation = await Invitation.read(this.transaction, object, id);
            const acting = Caller.principal(context);
            const isInviter =
                acting !== undefined &&
                invitation.inviter !== null &&
                Subject.same(acting, invitation.inviter);
            if (
                acting === undefined ||
                (!isInviter && !Invitation.addresses(invitation, acting, context))
            ) {
                await this.authorizeRevoke(invitation.relationship);
            }

            // keep the invitation as revoked
            await this.#settle(id, "revoked", context.now);

            return { ...invitation, status: "revoked" };
        });
    }

    /** Require an offer's inviter to still have the grant, except for system offers. */
    async #requireInviter(invitation: Invitation, now: number): Promise<void> {
        // accept system offers as made
        if (invitation.inviter === null) {
            return;
        }

        // decide the grant as the inviting principal, at its authentication when it sent the offer
        const inviter = invitation.inviter;
        const authorization = new Authorization(this.authorizer, this.transaction, () => ({
            subjects: [inviter],
            now,
            attributes: {},
            assurance: { level: HIGHEST_ASSURANCE, authenticatedAt: now },
        }));
        await authorization.authorizeGrant(invitation.relationship);
    }

    /** Settle a pending invitation as accepted or revoked, refusing one settled first. */
    async #settle(id: string, status: "accepted" | "revoked", now: number): Promise<void> {
        // refuse an invitation no longer pending
        const row = await this.transaction
            .read(accessInvitation)
            .findFirst({ where: { id: Invitation.id(id) } });
        if (row === undefined || row.status !== "pending") {
            throw new AccessError("CONFLICT", "invitation is no longer pending");
        }

        // keep it settled
        await this.transaction.write({
            table: accessInvitation,
            before: row,
            after: { ...row, status, updatedAt: now },
        });
    }

    /** List a page of an object's pending invitations by identifier, as its grantors may. */
    async invitations(
        request: {
            readonly object: ObjectReference;
            readonly relation?: string;
            readonly role?: string;
        },
        page: InvitationPage,
    ): Promise<Invitation[]> {
        // require the grant permission
        const context = this.context();
        await this.#requireGrant(request.object, request.relation);

        // read the object's invitations after the cursor
        const role =
            request.role === undefined ? undefined : schema.identifier("role").parse(request.role);
        const rows = await this.transaction.read(accessInvitation).findMany({
            where: {
                ...Relationship.objectColumns(request.object),
                ...(request.relation === undefined ? {} : { relation: request.relation }),
                ...(role === undefined ? {} : { roleId: role }),
                ...Invitation.pending(page, context.now),
            },
            orderBy: { id: "asc" },
            limit: page.limit + 1,
        });

        return rows.map((row) => Invitation.decode(row));
    }

    /** Define a role in a scope, granting only permissions the caller has there. */
    async createRole(scope: ObjectReference, request: RoleRequest): Promise<Role> {
        return this.transaction.savepoint(() => this.#defineRole(scope, request));
    }

    /** Change a role's name, purpose or permissions at a revision, within the caller's rights. */
    async updateRole(
        scope: ObjectReference,
        id: string,
        changes: Partial<RoleRequest> & { readonly revision: number },
    ): Promise<Role> {
        return this.transaction.savepoint(async () => {
            // require an editable role at the revision, permission to change roles, and every permission granted
            const context = this.context();
            const record = await Role.read(this.transaction, scope.id, id);
            if (record.revision !== changes.revision) {
                throw new AccessError("CONFLICT", "role revision has changed");
            }
            await this.authorizeRole(
                scope,
                [policies.role.permission("update"), ...(changes.permissions ?? [])],
                record,
            );

            // apply the changes at the next revision
            const updated = {
                ...record,
                ...(changes.name === undefined ? {} : { name: changes.name }),
                ...(changes.description === undefined ? {} : { description: changes.description }),
                revision: record.revision + 1,
                updatedAt: context.now,
            };
            await this.transaction.write({ table: accessRole, before: record, after: updated });

            // replace the permissions when given
            if (changes.permissions !== undefined) {
                await Role.replace(this.transaction, record.id, scope.id, changes.permissions);
            }
            this.renew();

            return Role.describe(
                updated,
                changes.permissions ?? (await Role.permissions(this.transaction, record.id)),
            );
        });
    }

    /** Delete an unbound role at a revision, managed roles only through their declaration. */
    async deleteRole(scope: ObjectReference, id: string, revision: number): Promise<Role> {
        return this.transaction.savepoint(async () => {
            // require an editable role at the revision and permission to delete roles
            const record = await Role.read(this.transaction, scope.id, id);
            if (record.revision !== revision) {
                throw new AccessError("CONFLICT", "role revision has changed");
            }
            await this.authorizeRole(scope, [policies.role.permission("delete")], record);

            // refuse deleting a role that is still bound or included
            const bindings = await this.transaction.read(accessRelationship).findMany({
                where: {
                    OR: [
                        { roleId: record.id },
                        {
                            subjectPackageId: policies.role.definition.packageId,
                            subjectType: policies.role.name,
                            subjectId: record.id,
                        },
                    ],
                },
                limit: 1,
            });
            if (bindings.length > 0) {
                throw new AccessError("CONFLICT", "role is still bound or included");
            }

            // delete the role with its permissions
            const permissions = await Role.permissions(this.transaction, record.id);
            await Role.replace(this.transaction, record.id, scope.id, []);
            await this.transaction.write({ table: accessRole, before: record, after: null });
            this.renew();

            return Role.describe(record, permissions);
        });
    }

    /** Grant a delegate some of the caller's authority, in the caller's own scope. */
    async createGrant(scope: ObjectReference, request: GrantRequest): Promise<Grant> {
        // write only the grants of a local scope
        await this.authorizer.requireLocal(this.transaction, scope);

        // grant only a principal's own authority, in its own scope
        const context = this.context();
        const delegator = Caller.principal(context);
        if (delegator === undefined || Caller.isDelegated(context) || delegator.id !== scope.id) {
            throw new AccessError("FORBIDDEN", "a principal can grant only its own authority");
        }

        // grant to one other principal, until a later time
        const { delegate } = request;
        if (
            !isPrincipal(delegate) ||
            delegate.id === "*" ||
            delegate.scope === "*" ||
            Subject.same(delegate, delegator)
        ) {
            throw new AccessError("FORBIDDEN", "a grant needs one other principal as delegate");
        } else if (request.expiresAt !== undefined && request.expiresAt <= context.now) {
            throw new AccessError("CONFLICT", "a grant must expire after its creation");
        }

        // write the grant
        const grant = Table.complete(accessGrant, {
            id: schema.identifier("grant").parse(`grant-${v7()}`),
            createdAt: context.now,
            scope: scope.id,
            delegator,
            delegatorKey: Subject.key(delegator),
            delegate,
            delegateKey: Subject.key(delegate),
            delegateScope: delegate.scope,
            permissions: [...request.permissions],
            expiresAt: request.expiresAt ?? null,
        });
        await this.transaction.write({ table: accessGrant, before: null, after: grant });

        return grant;
    }

    /** Revoke a grant, as its delegator. */
    async revokeGrant(scope: ObjectReference, id: Identifier<"grant">): Promise<Grant> {
        // write only the grants of a local scope
        await this.authorizer.requireLocal(this.transaction, scope);

        // require the grant in the scope and its delegator acting as itself
        const context = this.context();
        const delegator = Caller.principal(context);
        const grant = await Grant.read(this.snapshot, id);
        if (grant === undefined || grant.scope !== scope.id) {
            throw new AccessError("NOT_FOUND", `no grant ${id}`);
        } else if (
            delegator === undefined ||
            Caller.isDelegated(context) ||
            !Subject.same(delegator, grant.delegator)
        ) {
            throw new AccessError("FORBIDDEN", "only the delegator can revoke a grant");
        }

        // delete it
        await this.transaction.write({ table: accessGrant, before: grant, after: null });

        return grant;
    }

    /** Keep a role by name in a scope with exactly the requested purpose and permissions. */
    async keepRole(scope: ObjectReference, request: RoleRequest): Promise<Role> {
        return this.transaction.savepoint(async () => {
            // read the role with the name, and define it when none exists
            const now = this.context().now;
            const kept = await this.transaction.read(accessRole).findFirst({
                where: {
                    scope: scope.id,
                    name: request.name,
                },
            });
            if (kept === undefined) {
                return this.#defineRole(scope, request);
            }

            // accept only a role under the same manager, and leave a matching role as it is
            await this.authorizeRole(
                scope,
                [policies.role.permission("update"), ...request.permissions],
                kept,
            );
            if (!Authorization.#isManagedBy(kept, this.#manager)) {
                throw new AccessError("CONFLICT", "role name is already in use");
            }
            const granted = await Role.permissions(this.transaction, kept.id);
            const isGranted = Authorization.#isSamePermissions(granted, request.permissions);
            if (kept.description === request.description && isGranted) {
                return Role.describe(kept, granted);
            }

            // describe it anew at its next revision, replacing differing permissions
            const updated = {
                ...kept,
                description: request.description,
                updatedAt: now,
                revision: kept.revision + 1,
            };
            await this.transaction.write({ table: accessRole, before: kept, after: updated });
            if (!isGranted) {
                await Role.replace(this.transaction, kept.id, scope.id, request.permissions);
            }
            this.renew();

            return Role.describe(updated, request.permissions);
        });
    }

    /** Define a role under a free name and the declaration applied if any, as the caller may. */
    async #defineRole(scope: ObjectReference, request: RoleRequest): Promise<Role> {
        // require permission to define roles and every permission the role grants
        await this.authorizeRole(scope, [
            policies.role.permission("create"),
            ...request.permissions,
        ]);

        // define the role under a name free in the scope
        const now = this.context().now;
        const id = await Role.define(this.transaction, scope.id, request, now, this.#manager);
        if (id === undefined) {
            throw new AccessError("CONFLICT", "role name is already in use");
        }
        const record = await Role.read(this.transaction, scope.id, id);
        this.renew();

        return Role.describe(record, request.permissions);
    }

    /** Bind a role on an object to only one subject, as the caller may. */
    async assign(object: ObjectReference, role: string, subject: Subject): Promise<Relationship> {
        const relationship = await this.transaction.savepoint(async () => {
            // read the role's bindings
            const request = { object, role, subject };
            const id = schema.identifier("role").parse(role);
            const rows = await this.transaction
                .read(accessRelationship)
                .findMany({ where: { roleId: id } });
            const bound = rows.map((row) => Relationship.decode(row));

            // leave a role bound only to the subject as it is
            const [only] = bound;
            if (
                bound.length === 1 &&
                only !== undefined &&
                Authorization.#isBinding(only, request)
            ) {
                return only;
            }

            // revoke its other bindings and bind it to the subject, as the caller may
            await this.authorizeGrant(request);
            for (const [index, row] of rows.entries()) {
                await this.authorizeRevoke(aligned(bound, index), row);
            }
            for (const row of rows) {
                await this.transaction.write({
                    table: accessRelationship,
                    before: row,
                    after: null,
                });
            }

            return this.#insert(request);
        });
        this.renew();

        return relationship;
    }

    /** Keep exactly the wanted relationships among a selection's, as the caller may. */
    async keepRelationships(
        selection: RelationshipSelection,
        wanted: readonly RelationshipRequest[],
    ): Promise<void> {
        await this.transaction.savepoint(async () => {
            // read the relationships the selection covers
            const related = (
                await Relationship.readSelected(this.transaction, selection, this.#manager)
            ).map((row) => ({
                row,
                relationship: Relationship.decode(row),
            }));

            // revoke the relationships no longer wanted
            const missing = new Map(
                wanted.map((request) => [Authorization.#identity(request), request]),
            );
            const stale = related.filter(
                ({ relationship }) => !missing.delete(Authorization.#identity(relationship)),
            );
            for (const { row, relationship } of stale) {
                await this.authorizeRevoke(relationship, row);
                await this.transaction.write({
                    table: accessRelationship,
                    before: row,
                    after: null,
                });
            }

            // grant the wanted relationships missing
            for (const request of missing.values()) {
                await this.authorizeGrant(request);
                await this.#insert(request);
            }
        });
        this.renew();
    }

    /** Keep one role by name in a scope granting exactly some permissions, bound to one subject. */
    async keepBinding(
        scope: ObjectReference,
        binding: Omit<RoleRequest, "permissions"> & {
            /** The permissions the role grants, none to drop the role and its binding. */
            readonly permissions: RoleRequest["permissions"];
            /** The one subject the role is bound to. */
            readonly subject: Subject;
            /** The object in the scope the role is bound on, the scope itself when absent. */
            readonly object?: ObjectReference;
            /** What a request must meet for the binding to apply. */
            readonly conditions?: RelationshipCondition;
        },
    ): Promise<void> {
        // drop the binding and the role when it grants nothing
        const { subject, object, conditions, ...request } = binding;
        if (request.permissions.length === 0) {
            await this.keepRelationships({ scope: scope.id }, []);
            const kept = await this.transaction.read(accessRole).findFirst({
                where: {
                    scope: scope.id,
                    name: request.name,
                },
            });
            if (kept !== undefined) {
                await this.deleteRole(scope, kept.id, kept.revision);
            }

            return;
        }

        // keep the role with exactly the permissions, bound only to the subject
        const role = await this.keepRole(scope, request);
        await this.keepRelationships({ scope: scope.id }, [
            {
                object: object ?? scope,
                role: role.id,
                subject,
                ...(conditions === undefined ? {} : { conditions }),
            },
        ]);
    }

    /** Require permissions to change roles in a local scope, from fresh caller access. */
    protected async authorizeRole(
        scope: ObjectReference,
        permissions: readonly PermissionReference[],
        record?: Select<typeof accessRole>,
    ): Promise<void> {
        // write only the roles of a local scope, and refuse a managed role outside its declaration until detached
        await this.authorizer.requireLocal(this.transaction, scope);
        if (record !== undefined) {
            Authorization.#requireChangeable(record, this.#manager);
        }

        // require every permission in the scope
        const access = await this.authorizer.resolve(this.snapshot, scope.id, this.context());
        await this.authorizer.require(this.snapshot, permissions, scope, access);
    }

    /** Require the grant permission and every permission of a bound role. */
    protected async authorizeGrant(request: RelationshipFields): Promise<void> {
        // write only the access of a local object
        await this.authorizer.requireLocal(this.transaction, request.object);

        // require the relation's grant permission to grant it
        const scope = this.authorizer.governingScope(request.object);
        const access = await this.authorizer.resolve(this.snapshot, scope, this.context());
        const role = "role" in request ? request.role : undefined;
        const required: PermissionReference[] = [
            this.#grantPermission(request.object, Authorization.#relation(request)),
        ];

        // prevent escalation: bind only scope chain roles with permissions the caller has
        const grants = role === undefined ? undefined : access.grants.get(role);
        if (role !== undefined && grants === undefined) {
            throw new AccessError("NOT_FOUND", "role not found");
        }
        required.push(...(grants?.permissions ?? []));
        await this.authorizer.require(this.snapshot, required, request.object, access);

        // require ownership to bind a role granting everything
        if (
            grants?.isOwner === true &&
            !(await this.authorizer.isOwner(this.snapshot, request.object, access))
        ) {
            throw new AccessError("FORBIDDEN", "only owners may bind a role granting everything");
        }
    }

    /** Require a local object and the grant permission of a relationship. */
    protected async authorizeRevoke(
        request: RelationshipFields,
        record?: Select<typeof accessRelationship>,
    ): Promise<void> {
        // write only the access of a local object, and refuse a managed relationship outside its declaration until detached
        await this.authorizer.requireLocal(this.transaction, request.object);
        if (record !== undefined) {
            Authorization.#requireChangeable(record, this.#manager);
        }

        // require the relation's grant permission
        await this.#requireGrant(request.object, Authorization.#relation(request));
    }

    /** Require the permission granting a relation, or binding roles, on an object. */
    async #requireGrant(object: ObjectReference, relation: string | undefined): Promise<void> {
        const scope = this.authorizer.governingScope(object);
        const access = await this.authorizer.resolve(this.snapshot, scope, this.context());
        await this.authorizer.require(
            this.snapshot,
            [this.#grantPermission(object, relation)],
            object,
            access,
        );
    }

    /** Insert a valid relationship under a new identifier and declaration, refusing duplicates. */
    async #insert(request: RelationshipRequest): Promise<Relationship> {
        // build the relationship
        const scope = this.authorizer.governingScope(request.object);
        const context = this.context();
        this.authorizer.validate(request, context.now);
        const relationship: Relationship = {
            id: schema.identifier("relationship").parse(`relationship-${v7()}`),
            object: request.object,
            ...Relationship.via(request),
            subject: request.subject,
            createdAt: context.now,
            expiresAt: request.expiresAt ?? null,
            ...(request.conditions === undefined ? {} : { conditions: request.conditions }),
        };

        // insert it unless the same relationship exists
        const row = {
            ...Relationship.encode(relationship, scope),
            ...Manager.values(this.#manager),
        };
        const same = await this.transaction
            .read(accessRelationship)
            .findMany({ where: Relationship.equal(row), limit: 1 });
        if (same.length > 0) {
            throw new AccessError("CONFLICT", "relationship already exists");
        }
        await this.transaction.write({
            table: accessRelationship,
            before: null,
            after: Table.complete(accessRelationship, row),
        });

        return relationship;
    }

    /** Resolve the permission granting a relation, or binding roles, on an object. */
    #grantPermission(object: ObjectReference, relation: string | undefined): PermissionReference {
        // read the relation's grant, or the type's for a role binding
        const policy = this.authorizer.policy(object);
        const grant =
            relation === undefined
                ? policy.definition.grantedBy
                : this.authorizer.relation(policy, relation).grantedBy;
        if (grant === undefined) {
            throw new AccessError("FORBIDDEN", "relationship cannot be granted");
        }

        return policy.permission(grant);
    }

    /** Write some columns of a local scope's record. */
    async #mark(
        object: ObjectReference,
        changes: Partial<Select<typeof Scope.table>>,
    ): Promise<void> {
        // require a local scope
        await this.authorizer.requireLocal(this.transaction, object);
        if (this.authorizer.policy(object).definition.scope !== true) {
            throw new AccessError("INVALID_CONTEXT", `${object.type} is not a scope`);
        }

        // mark the scope's record
        const row = await this.transaction
            .read(Scope.table)
            .findFirst({ where: { scope: object.id } });
        if (row === undefined) {
            throw new AccessError("NOT_FOUND", `unknown scope: ${object.id}`);
        }
        await this.transaction.write({
            table: Scope.table,
            before: row,
            after: { ...row, ...changes },
        });
        this.renew();
    }

    /** Refuse changing a declaration's managed row until detached. */
    static #requireChangeable(
        record: {
            readonly managerInstallationId: string | null;
            readonly managerPackageId: string | null;
            readonly detachedAt: number | null;
        },
        manager: Manager | null,
    ): void {
        const isManaged = record.managerInstallationId !== null && record.detachedAt === null;
        const isDeclaring =
            manager !== null &&
            record.managerInstallationId === manager.installationId &&
            record.managerPackageId === manager.packageId;
        if (isManaged && !isDeclaring) {
            throw new AccessError("CONFLICT", "record is managed by its source declaration");
        }
    }

    /** Refuse removing the last owner binding on an object. */
    static async #requireRemainingOwner(
        transaction: Transaction,
        row: Select<typeof accessRelationship>,
    ): Promise<void> {
        // skip relations and other roles' bindings
        if (row.roleId !== Role.OWNER) {
            return;
        }

        // read the object's bindings of the owner role
        const owners = await transaction.read(accessRelationship).findMany({
            where: {
                objectScope: row.objectScope,
                packageId: row.packageId,
                type: row.type,
                objectId: row.objectId,
                roleId: Role.OWNER,
            },
        });

        // refuse removing the only such binding
        if (owners.length === 1 && aligned(owners, 0).id === row.id) {
            throw new AccessError("CONFLICT", "the last owner cannot be removed");
        }
    }

    /** Report whether a relationship binds a role to a subject without condition or expiry. */
    static #isBinding(
        relationship: Relationship,
        request: {
            readonly object: ObjectReference;
            readonly role: string;
            readonly subject: Subject;
        },
    ): boolean {
        return (
            "role" in relationship &&
            relationship.role === request.role &&
            Subject.same(relationship.object, request.object) &&
            Subject.same(relationship.subject, request.subject) &&
            relationship.expiresAt === null &&
            relationship.conditions === undefined
        );
    }

    /** Report whether a role is under a manager, or under none for null. */
    static #isManagedBy(record: Select<typeof accessRole>, manager: Manager | null): boolean {
        const columns = Manager.values(manager);

        return (
            record.managerInstallationId === columns.managerInstallationId &&
            record.managerPackageId === columns.managerPackageId &&
            record.managerName === columns.managerName
        );
    }

    /** Identify a relationship by its object, relation or role, subject, expiry and conditions. */
    static #identity(relationship: RelationshipRequest | Relationship): string {
        return JSON.stringify([
            ObjectReference.key(relationship.object),
            Relationship.viaColumns(relationship),
            Subject.key(relationship.subject),
            relationship.expiresAt ?? null,
            Object.entries(relationship.conditions ?? {}).toSorted(([left], [right]) =>
                left.localeCompare(right),
            ),
        ]);
    }

    /** Report whether two lists of permissions grant the same set. */
    static #isSamePermissions(
        left: readonly PermissionReference[],
        right: readonly PermissionReference[],
    ): boolean {
        // compare the sets of their keys
        const lefts = new Set(left.map((permission) => PermissionReference.key(permission)));
        const rights = new Set(right.map((permission) => PermissionReference.key(permission)));

        return lefts.size === rights.size && [...lefts].every((key) => rights.has(key));
    }

    /** Read the relation a relationship relates through, absent for a role binding. */
    static #relation(request: RelationshipFields): string | undefined {
        return "relation" in request ? request.relation : undefined;
    }
}
