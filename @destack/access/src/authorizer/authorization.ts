import {
    and,
    asc,
    eq,
    or,
    type DatabaseConnection,
    type Row,
    type Select,
    Snapshot,
} from "@destack/db";
import { ObjectReference, Scope, Subject } from "@destack/sync";
import { aligned, schema } from "@destack/schema";
import { v7 } from "uuid";
import { AccessError } from "../error/index.ts";
import { PermissionReference } from "../declare/policy.ts";
import * as policies from "../declare/principal.ts";
import { anyone, isPrincipal } from "../declare/principal.ts";
import { AccessContext, Caller } from "../context/context.ts";
import { HIGHEST_ASSURANCE } from "../context/elevation.ts";
import {
    readSelected,
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
import { define, own, replace, Role, type RoleRequest } from "../role/role.ts";
import { accessRole } from "../role/table.ts";
import type { Access } from "./access.ts";
import type { Admission, Authorizer } from "./authorizer.ts";
import type { Decision } from "./decision.ts";
import type { GrantReader, Lookup } from "./grant.ts";

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
    /** The database, or transaction, with the access rows that decide. */
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

    /** Decide whether the caller has a permission on one object and until when. */
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

    /** Require the caller to have a permission on one object. */
    async require(permission: PermissionReference, target: ObjectReference): Promise<void> {
        const access = await this.in(this.authorizer.governingScope(target));
        await this.authorizer.require(this.snapshot, [permission], target, access);
    }

    /** Check a permission on current or past rows of a scope and return the rows permitted and until when. */
    async checkRows(
        permission: PermissionReference,
        scope: string,
        rows: readonly Row[],
        reader?: GrantReader,
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

    /** Resolve the caller's access in the scopes a scope encloses, in reads shared by all of them. */
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

    /** Record a new object's access: a scope's containing scopes, its first relationships and a scope's owner role. */
    async create(object: ObjectReference, creation: Creation): Promise<void> {
        // write the access of an object this database keeps
        await this.authorizer.requireLocal(this.database, object);
        const [existing] = await this.database
            .select({ id: accessRelationship.id })
            .from(accessRelationship)
            .where(Relationship.on(object))
            .limit(1);
        if (existing) {
            throw new AccessError("INVALID_CONTEXT", "only a new object relates without a grant");
        }

        // refuse owners of an object that contains no others
        const definition = this.authorizer.policy(object).definition;
        const isScope = definition.scope === true;
        if (!isScope && creation.owner !== undefined) {
            throw new AccessError("INVALID_CONTEXT", "only a new scope has owners");
        }
        const scope = this.authorizer.governingScope(object);
        const context = this.context(scope);

        // record a scope below the scopes containing it
        if (isScope) {
            // read the containing scopes, none below the universe
            let ancestors: string[] = [];
            if (object.scope !== Scope.universe.id) {
                const [parent] = await this.database
                    .select({ ancestors: Scope.table.ancestors })
                    .from(Scope.table)
                    .where(eq(Scope.table.scope, object.scope));
                if (parent === undefined) {
                    throw new AccessError("NOT_FOUND", `unknown scope: ${object.scope}`);
                }
                ancestors = [object.scope, ...parent.ancestors];
            }

            // record the scope with its chain
            await this.database.insert(Scope.table).values({
                scope: object.id,
                parent: object.scope,
                packageId: object.packageId,
                type: object.type,
                ancestors,
            });
        }

        // relate the first holders
        const relationships = this.authorizer.initialRelationships(object, creation, context.now);
        if (relationships.length > 0) {
            await this.database.insert(accessRelationship).values(relationships);
        }

        // define a scope's owner role and bind it to its owner
        if (creation.owner !== undefined) {
            await own(this.database, object, creation.owner, context.now);
        }
        this.renew();
    }

    /** Suspend a scope and withhold every permission in it except administration until resumed. */
    async suspend(object: ObjectReference): Promise<void> {
        await this.#suspension(object, this.context(object.id).now);
    }

    /** Resume a suspended scope as a caller with the permission to. */
    async resume(object: ObjectReference): Promise<void> {
        await this.#suspension(object, null);
    }

    /** Relate a subject to an object through a relation or a bound role, under its manager if any, as the caller may grant. */
    async grant(request: RelationshipRequest, manager?: Manager): Promise<Relationship> {
        const relationship = await this.database.transaction(async (transaction) => {
            const authorization = this.within(transaction);
            await authorization.authorizeGrant(request);

            return authorization.#insert(request, manager);
        });
        this.renew();

        return relationship;
    }

    /** Revoke one relationship of an object as the caller may, keeping its history in the log. */
    async revoke(object: ObjectReference, id: string): Promise<Relationship> {
        return this.database.transaction(async (transaction) => {
            // load the object's relationship inside the authorization transaction
            const authorization = this.within(transaction);
            const key = schema.identifier("relationship").parse(id);
            const [row] = await transaction
                .select()
                .from(accessRelationship)
                .where(and(Relationship.on(object), eq(accessRelationship.id, key)));
            if (!row) {
                throw new AccessError("NOT_FOUND", "relationship not found");
            }

            // require permission to revoke what it grants, and keep an owner
            const relationship = Relationship.decode(row);
            await authorization.authorizeRevoke(relationship, row);
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
        const linkSecret = await LinkSecret.create();
        const relationship = await this.grant({
            ...request,
            subject: anyone.reference("*", "*"),
            conditions: { linkSecret: linkSecret.digest },
        });

        return { id: relationship.id, secret: linkSecret.secret };
    }

    /** Invite a relationship that applies once accepted, asked for by its subject or offered by a grantor. */
    async invite(request: InvitationRequest): Promise<Invitation> {
        return this.database.transaction(async (transaction) => {
            // require an object this database keeps, invited by a principal or the system
            const authorization = this.within(transaction);
            const invited = request.relationship;
            await this.authorizer.requireLocal(transaction, invited.object);
            const context = this.context(this.authorizer.governingScope(invited.object));
            const inviter = Caller.principal(context) ?? null;

            // validate the relationship, checking an invited contact's principal once it accepts
            const { subject, ...validated } = invited;
            this.authorizer.validate(
                policies.principal.contact.is(subject) ? validated : invited,
                context.now,
            );
            const expiresAt = request.expiresAt ?? context.now + INVITATION_LIFETIME_MILLISECONDS;
            if (!Number.isFinite(expiresAt) || expiresAt <= context.now) {
                throw new AccessError("FORBIDDEN", "invitation must lapse in the future");
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
                await authorization.authorizeGrant(invited);
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
            await transaction.insert(accessInvitation).values(Invitation.encode(invitation, scope));

            return invitation;
        });
    }

    /** Accept an invitation on an object so its relationship applies. */
    async accept(object: ObjectReference, id: string): Promise<Relationship> {
        return this.database.transaction(async (transaction) => {
            // load the pending invitation inside the authorization transaction
            const authorization = this.within(transaction);
            const context = this.context(this.authorizer.governingScope(object));
            const invitation = await Invitation.read(transaction, object, id);
            if (invitation.expiresAt <= context.now) {
                throw new AccessError("FORBIDDEN", "invitation has lapsed");
            }

            // require a grantor for a request
            const invited = invitation.relationship;
            let subject: Subject;
            if (Invitation.asksForItself(invitation)) {
                await authorization.authorizeGrant(invited);
                subject = invitation.relationship.subject;
            }
            // require the addressed principal for an offer, which its inviting principal still may grant
            else {
                const accepting = Caller.requirePrincipal(context);
                if (!Invitation.addresses(invitation, accepting, context)) {
                    throw new AccessError("FORBIDDEN", "invitation is addressed to someone else");
                }
                subject = accepting;
                await this.#requireInviter(transaction, invitation, context.now);
            }

            // relate the accepting subject and settle the invitation
            const { expiresAt, ...granted } = invited;
            const relationship = await authorization.#insert({
                ...granted,
                subject,
                ...(expiresAt === null ? {} : { expiresAt }),
            });
            await Authorization.#settle(transaction, id, "accepted", context.now);
            this.renew();

            return relationship;
        });
    }

    /** Withdraw an invitation on an object: its inviting principal or addressee, or a grantor revoking it. */
    async withdraw(object: ObjectReference, id: string): Promise<Invitation> {
        return this.database.transaction(async (transaction) => {
            // let the inviting principal and the addressee withdraw without grant authority
            const authorization = this.within(transaction);
            const context = this.context(this.authorizer.governingScope(object));
            const invitation = await Invitation.read(transaction, object, id);
            const acting = Caller.principal(context);
            const isInviter =
                acting !== undefined &&
                invitation.inviter !== null &&
                Subject.same(acting, invitation.inviter);
            if (
                acting === undefined ||
                (!isInviter && !Invitation.addresses(invitation, acting, context))
            ) {
                await authorization.authorizeRevoke(invitation.relationship);
            }

            // keep the invitation as revoked
            await Authorization.#settle(transaction, id, "revoked", context.now);

            return { ...invitation, status: "revoked" };
        });
    }

    /** Require an offer's inviting principal to still hold the grant, the system's offers standing as made. */
    async #requireInviter(
        transaction: DatabaseConnection,
        invitation: Invitation,
        now: number,
    ): Promise<void> {
        // let the system's offers stand as made
        if (invitation.inviter === null) {
            return;
        }

        // decide the grant as the inviting principal at the authentication it met to send the offer
        const inviter = invitation.inviter;
        const authorization = new Authorization(this.authorizer, transaction, () => ({
            subjects: [inviter],
            now,
            attributes: {},
            assurance: { level: HIGHEST_ASSURANCE, authenticatedAt: now },
        }));
        await authorization.authorizeGrant(invitation.relationship);
    }

    /** Settle a pending invitation as accepted or revoked, refusing one a concurrent write settled first. */
    static async #settle(
        transaction: DatabaseConnection,
        id: string,
        status: "accepted" | "revoked",
        now: number,
    ): Promise<void> {
        const settled = await transaction
            .update(accessInvitation)
            .set({ status, updatedAt: now })
            .where(
                and(
                    eq(accessInvitation.id, Invitation.id(id)),
                    eq(accessInvitation.status, "pending"),
                ),
            )
            .returning({ id: accessInvitation.id });
        if (settled.length === 0) {
            throw new AccessError("CONFLICT", "invitation is no longer pending");
        }
    }

    /** List a page of the invitations pending on an object, ordered by identifier, as its grantors may. */
    async invitations(
        request: {
            readonly object: ObjectReference;
            readonly relation?: string;
            readonly role?: string;
        },
        page: InvitationPage,
    ): Promise<Invitation[]> {
        return this.database.transaction(async (transaction) => {
            // require the grant permission before reading the object's invitations after the cursor
            const context = this.context(this.authorizer.governingScope(request.object));
            await this.within(transaction).#requireGrant(request.object, request.relation);
            const rows = await transaction
                .select()
                .from(accessInvitation)
                .where(
                    and(
                        Invitation.on(request.object),
                        request.relation === undefined
                            ? undefined
                            : eq(accessInvitation.relation, request.relation),
                        request.role === undefined
                            ? undefined
                            : eq(
                                  accessInvitation.roleId,
                                  schema.identifier("role").parse(request.role),
                              ),
                        Invitation.pending(page, context.now),
                    ),
                )
                .orderBy(asc(accessInvitation.id))
                .limit(page.limit);

            return rows.map(Invitation.decode);
        });
    }

    /** Define a role in a scope, granting only permissions the caller has there. */
    async createRole(scope: ObjectReference, request: RoleRequest): Promise<Role> {
        return this.database.transaction(async (transaction) =>
            this.#defineRole(transaction, scope, request, undefined),
        );
    }

    /** Change a role's name, purpose or permissions at a revision within the caller's permissions. */
    async updateRole(
        scope: ObjectReference,
        id: string,
        changes: Partial<RoleRequest> & { readonly revision: number },
    ): Promise<Role> {
        return this.database.transaction(async (transaction) => {
            // require an editable role at the revision, permission to change roles, and every permission granted
            const context = this.context(scope.id);
            const record = await Role.read(transaction, scope.id, id);
            if (record.revision !== changes.revision) {
                throw new AccessError("CONFLICT", "role revision has changed");
            }
            await this.authorizeRole(
                transaction,
                scope,
                [policies.role.permission("update"), ...(changes.permissions ?? [])],
                record,
            );

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
                await replace(transaction, record.id, scope.id, changes.permissions);
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
            const record = await Role.read(transaction, scope.id, id);
            if (record.revision !== revision) {
                throw new AccessError("CONFLICT", "role revision has changed");
            }
            await this.authorizeRole(
                transaction,
                scope,
                [policies.role.permission("delete")],
                record,
            );

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
                                policies.role.definition.packageId,
                            ),
                            eq(accessRelationship.subjectType, policies.role.name),
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

    /** Keep a role by name in a scope with exactly the requested purpose and permissions, under its manager if any, writing only a difference, and return it. */
    async keepRole(scope: ObjectReference, request: RoleRequest, manager?: Manager): Promise<Role> {
        return this.database.transaction(async (transaction) => {
            // read the role holding the name, defining it when none does
            const now = this.context(scope.id).now;
            const [kept] = await transaction
                .select()
                .from(accessRole)
                .where(and(eq(accessRole.scope, scope.id), eq(accessRole.name, request.name)));
            if (kept === undefined) {
                return this.#defineRole(transaction, scope, request, manager);
            }

            // keep only a role under the same manager, leaving one granting exactly the request as it is
            await this.authorizeRole(
                transaction,
                scope,
                [policies.role.permission("update"), ...request.permissions],
                kept,
            );
            if (!isManagedBy(kept, manager)) {
                throw new AccessError("CONFLICT", "role name is already in use");
            }
            const granted = await Role.permissions(transaction, kept.id);
            const isGranted = isSamePermissions(granted, request.permissions);
            if (kept.description === request.description && isGranted) {
                return Role.describe(kept, granted);
            }

            // describe it anew at its next revision, replacing differing permissions
            const [updated] = await transaction
                .update(accessRole)
                .set({
                    description: request.description,
                    updatedAt: now,
                    revision: kept.revision + 1,
                })
                .where(and(eq(accessRole.id, kept.id), eq(accessRole.revision, kept.revision)))
                .returning();
            if (updated === undefined) {
                throw new AccessError("CONFLICT", "role revision has changed");
            }
            if (!isGranted) {
                await replace(transaction, kept.id, scope.id, request.permissions);
            }
            this.renew();

            return Role.describe(updated, request.permissions);
        });
    }

    /** Define a role under a free name and its manager if any, as the caller may. */
    async #defineRole(
        transaction: DatabaseConnection,
        scope: ObjectReference,
        request: RoleRequest,
        manager: Manager | undefined,
    ): Promise<Role> {
        // require permission to define roles and every permission the role grants
        await this.authorizeRole(transaction, scope, [
            policies.role.permission("create"),
            ...request.permissions,
        ]);

        // define the role under a name free in the scope
        const now = this.context(scope.id).now;
        const record = await define(transaction, scope.id, request, now, manager);
        if (record === undefined) {
            throw new AccessError("CONFLICT", "role name is already in use");
        }
        this.renew();

        return Role.describe(record, request.permissions);
    }

    /** Bind a role on an object to one subject alone as the caller may, revoking its other bindings and writing nothing when bound so already. */
    async assign(object: ObjectReference, role: string, subject: Subject): Promise<Relationship> {
        const relationship = await this.database.transaction(async (transaction) => {
            // read the role's bindings
            const authorization = this.within(transaction);
            const request = { object, role, subject };
            const id = schema.identifier("role").parse(role);
            const rows = await transaction
                .select()
                .from(accessRelationship)
                .where(eq(accessRelationship.roleId, id));
            const bound = rows.map((row) => Relationship.decode(row));

            // leave a role bound to the subject alone as it is
            const [only] = bound;
            if (bound.length === 1 && only !== undefined && isBinding(only, request)) {
                return only;
            }

            // revoke its other bindings and bind it to the subject, as the caller may
            await authorization.authorizeGrant(request);
            for (const [index, row] of rows.entries()) {
                await authorization.authorizeRevoke(aligned(bound, index), row);
            }
            await transaction.delete(accessRelationship).where(eq(accessRelationship.roleId, id));

            return authorization.#insert(request);
        });
        this.renew();

        return relationship;
    }

    /** Keep exactly the wanted relationships among those a selection covers as the caller may, leaving the ones still wanted and granting new ones under the selection's manager if any. */
    async keepRelationships(
        selection: RelationshipSelection,
        wanted: readonly RelationshipRequest[],
    ): Promise<void> {
        await this.database.transaction(async (transaction) => {
            // read the relationships the selection covers
            const authorization = this.within(transaction);
            const related = (await readSelected(transaction, selection)).map((row) => ({
                row,
                relationship: Relationship.decode(row),
            }));

            // revoke the relationships no longer wanted
            const missing = new Map(wanted.map((request) => [identityOf(request), request]));
            const stale = related.filter(
                ({ relationship }) => !missing.delete(identityOf(relationship)),
            );
            for (const { row, relationship } of stale) {
                await authorization.authorizeRevoke(relationship, row);
                await transaction
                    .delete(accessRelationship)
                    .where(eq(accessRelationship.id, row.id));
            }

            // grant the wanted relationships missing
            const manager = "manager" in selection ? selection.manager : undefined;
            for (const request of missing.values()) {
                await authorization.authorizeGrant(request);
                await authorization.#insert(request, manager);
            }
        });
        this.renew();
    }

    /** Require permissions to change roles in a scope this database keeps as a transaction shows the caller's access afresh, refusing a role a declaration manages. */
    protected async authorizeRole(
        transaction: DatabaseConnection,
        scope: ObjectReference,
        permissions: readonly PermissionReference[],
        record?: Select<typeof accessRole>,
    ): Promise<void> {
        // write only the roles of a scope this database keeps, refusing a managed role until it is detached
        await this.authorizer.requireLocal(transaction, scope);
        if (record !== undefined) {
            requireUnmanaged(record);
        }

        // require every permission in the scope
        const snapshot = Snapshot.live(transaction);
        const access = await this.authorizer.resolve(snapshot, scope.id, this.context(scope.id));
        await this.authorizer.require(snapshot, permissions, scope, access);
    }

    /** Require the grant permission and every permission a granted role grants, or only the lender for a delegation. */
    protected async authorizeGrant(request: RelationshipFields): Promise<void> {
        // write only the access of an object this database keeps
        await this.authorizer.requireLocal(this.database, request.object);

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
        const role = "role" in request ? request.role : undefined;
        const required: PermissionReference[] =
            onBehalfOf === undefined || role === undefined
                ? [this.#grantPermission(request.object, relationOf(request))]
                : [];

        // prevent escalation: bind or lend only scope chain roles with permissions the caller has
        const grants = role === undefined ? undefined : access.grants.get(role);
        if (role !== undefined && grants === undefined) {
            throw new AccessError("NOT_FOUND", "role not found");
        }
        required.push(...(grants?.permissions ?? []));
        await this.authorizer.require(this.snapshot, required, request.object, access);

        // require ownership to bind a role granting everything
        if (
            grants?.isUniversal === true &&
            !(await this.authorizer.isOwner(this.snapshot, request.object, access))
        ) {
            throw new AccessError("FORBIDDEN", "only owners may bind a role granting everything");
        }
    }

    /** Require an object this database keeps and the permission granting what a relationship grants, unless the caller lent it, refusing a relationship a declaration manages. */
    protected async authorizeRevoke(
        request: RelationshipFields,
        record?: Select<typeof accessRelationship>,
    ): Promise<void> {
        // write only the access of an object this database keeps, refusing a managed relationship until it is detached
        await this.authorizer.requireLocal(this.database, request.object);
        if (record !== undefined) {
            requireUnmanaged(record);
        }

        // let a lender manage its own delegations
        if (!this.#lends(request)) {
            await this.#requireGrant(request.object, relationOf(request));
        }
    }

    /** Require the permission granting a relation, or binding roles, on an object. */
    async #requireGrant(object: ObjectReference, relation: string | undefined): Promise<void> {
        const scope = this.authorizer.governingScope(object);
        const access = await this.authorizer.resolve(this.snapshot, scope, this.context(scope));
        await this.authorizer.require(
            this.snapshot,
            [this.#grantPermission(object, relation)],
            object,
            access,
        );
    }

    /** Insert a valid relationship under a new identifier and its manager if any, and refuse a duplicate. */
    async #insert(request: RelationshipRequest, manager?: Manager): Promise<Relationship> {
        // build the relationship
        const scope = this.authorizer.governingScope(request.object);
        const context = this.context(scope);
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
        const [inserted] = await this.database
            .insert(accessRelationship)
            .values({
                ...Relationship.encode(relationship, scope),
                ...Manager.values(manager ?? null),
            })
            .onConflictDoNothing()
            .returning({ id: accessRelationship.id });
        if (!inserted) {
            throw new AccessError("CONFLICT", "relationship already exists");
        }

        return relationship;
    }

    /** Determine whether the caller is the principal a delegation lends authority from. */
    #lends(request: RelationshipFields): boolean {
        const onBehalfOf = request.conditions?.onBehalfOf;
        const caller = Caller.principal(
            this.context(this.authorizer.governingScope(request.object)),
        );

        return onBehalfOf !== undefined && caller !== undefined && Subject.same(caller, onBehalfOf);
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

    /** Set when a scope this database keeps was suspended, or clear it. */
    async #suspension(object: ObjectReference, suspendedAt: number | null): Promise<void> {
        // require a scope this database keeps
        await this.authorizer.requireLocal(this.database, object);
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
    if (owners.length === 1 && aligned(owners, 0).id === row.id) {
        throw new AccessError("CONFLICT", "the last owner cannot be removed");
    }
}

/** Report whether a relationship binds a role on an object to a subject, unconditionally and without expiry. */
function isBinding(
    relationship: Relationship,
    request: { readonly object: ObjectReference; readonly role: string; readonly subject: Subject },
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

/** Report whether a role is under a manager, or under none when absent. */
function isManagedBy(record: Select<typeof accessRole>, manager: Manager | undefined): boolean {
    const columns = Manager.values(manager ?? null);

    return (
        record.managerInstallationId === columns.managerInstallationId &&
        record.managerPackageId === columns.managerPackageId &&
        record.managerName === columns.managerName
    );
}

/** Name a relationship by what it grants: its object, relation or role, subject, expiry and conditions. */
function identityOf(relationship: RelationshipRequest | Relationship): string {
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
function isSamePermissions(
    left: readonly PermissionReference[],
    right: readonly PermissionReference[],
): boolean {
    // compare the sets of their keys
    const lefts = new Set(left.map((permission) => PermissionReference.key(permission)));
    const rights = new Set(right.map((permission) => PermissionReference.key(permission)));

    return lefts.size === rights.size && [...lefts].every((key) => rights.has(key));
}

/** Read the relation a relationship relates through, absent for a role binding. */
function relationOf(request: RelationshipFields): string | undefined {
    return "relation" in request ? request.relation : undefined;
}
