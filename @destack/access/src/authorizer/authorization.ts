import {
    TABLE,
    type DatabaseConnection,
    Transaction,
    type Row,
    type RowImage,
    type Select,
    type Snapshot,
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
    /** The transaction reading and writing the access rows that decide. */
    readonly transaction: Transaction;
    /** The database as its decisions read it: the transaction's view, recording each read. */
    readonly snapshot: Snapshot;
    /** Read the verified caller at the time it acts, as its decisions see it. */
    readonly #context: () => AccessContext;
    /** The caller's access resolved in each scope during this call, until renewed. */
    readonly #resolved = new Map<string, Promise<Access>>();
    /** The caller's access in the scopes below each scope, absent for a scope it does not enclose, until renewed. */
    readonly #below = new Map<string, Map<string, Access | undefined>>();
    /** The declaration this authorization applies, whose rows it alone changes, null for a caller changing access itself. */
    #manager: Manager | null = null;

    /** Authorize one caller, read when it acts, within a transaction, with its access already resolved in some scope. */
    constructor(
        authorizer: Authorizer,
        transaction: Transaction,
        context: () => AccessContext,
        resolved?: Access,
    ) {
        // keep the caller, reusing the access resolved in one scope
        this.authorizer = authorizer;
        this.transaction = transaction;
        this.snapshot = transaction.view();
        this.#context = context;
        if (resolved !== undefined) {
            this.#resolved.set(resolved.scope, Promise.resolve(resolved));
        }
    }

    /** Run a call of one caller's authorization in a transaction on a database to its commit. */
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

    /** Authorize the same caller applying a declaration: its writes are managed by it, and it changes the rows the declaration manages. */
    applying(manager: Manager): Authorization {
        const applying = new Authorization(this.authorizer, this.transaction, this.#context);
        applying.#manager = manager;

        return applying;
    }

    /** Read the verified caller as it acts now. */
    context(): AccessContext {
        return this.#context();
    }

    /** Resolve the caller's access in a scope once per call: its subject sets, roles and scope chain. */
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
        // read the containing scopes of a new scope, none below the universe
        const isScope = this.authorizer.policy(object).definition.scope === true;
        const ancestors =
            isScope && object.scope !== Scope.universe.id
                ? [object.scope, ...(await this.#parent(object.scope)).ancestors]
                : [];

        // record its access below them
        await this.receive(object, ancestors, creation);
    }

    /** Record the access of an object created at its home, below the containing scopes its home lists. */
    async receive(
        object: ObjectReference,
        ancestors: readonly string[],
        creation: Creation,
    ): Promise<void> {
        // write the access of an object this database keeps, relating none yet
        await this.authorizer.requireLocal(this.transaction, object);
        const related = await this.transaction.rows(
            accessRelationship,
            Relationship.objectColumns(object),
            { limit: 1 },
        );
        if (related.length > 0) {
            throw new AccessError("INVALID_CONTEXT", "only a new object relates without a grant");
        }

        // refuse owners of an object that contains no others
        const definition = this.authorizer.policy(object).definition;
        const isScope = definition.scope === true;
        if (!isScope && creation.owner !== undefined) {
            throw new AccessError("INVALID_CONTEXT", "only a new scope has owners");
        }
        const context = this.context();

        // record a scope below the scopes containing it, in its own partition or its parent's
        if (isScope) {
            const partition =
                definition.partition === true
                    ? object.id
                    : (await this.#parent(object.scope)).partition;
            const row = Scope.table[TABLE].complete({
                scope: object.id,
                parent: object.scope,
                partition,
                packageId: object.packageId,
                type: object.type,
                ancestors: [...ancestors],
            });
            await this.transaction.write(Scope.table, null, row);
        }

        // relate the first holders
        const relationships = this.authorizer.initialRelationships(object, creation, context.now);
        for (const relationship of relationships) {
            const row = accessRelationship[TABLE].complete(relationship);
            await this.transaction.write(accessRelationship, null, row);
        }

        // define a scope's owner role and bind it to its owner
        if (creation.owner !== undefined) {
            await own(this.transaction, object, creation.owner, context.now);
        }
        this.renew();
    }

    /** Read the row of a scope a new scope lives in, refusing a scope this database does not record. */
    async #parent(scope: string): Promise<RowImage<typeof Scope.table>> {
        const [row] = await Scope.read(this.snapshot, [scope]);
        if (row === undefined) {
            throw new AccessError("NOT_FOUND", `unknown scope: ${scope}`);
        }

        return row;
    }

    /** Suspend a scope and withhold every permission in it except administration until resumed. */
    async suspend(object: ObjectReference): Promise<void> {
        await this.#suspension(object, this.context().now);
    }

    /** Resume a suspended scope as a caller with the permission to. */
    async resume(object: ObjectReference): Promise<void> {
        await this.#suspension(object, null);
    }

    /** Relate a subject to an object through a relation or a bound role, as the caller may grant. */
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
            const [row] = await this.transaction.rows(accessRelationship, {
                ...Relationship.objectColumns(object),
                id: key,
            });
            if (!row) {
                throw new AccessError("NOT_FOUND", "relationship not found");
            }

            // require permission to revoke what it grants, and keep an owner
            const relationship = Relationship.decode(row);
            await this.authorizeRevoke(relationship, row);
            await requireRemainingOwner(this.transaction, row);
            await this.transaction.write(accessRelationship, row, null);
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
        return this.transaction.savepoint(async () => {
            // require an object this database keeps, invited by a principal or the system
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
            const row = accessInvitation[TABLE].complete(Invitation.encode(invitation, scope));
            await this.transaction.write(accessInvitation, null, row);

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
            // require the addressed principal for an offer, which its inviting principal still may grant
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

    /** Withdraw an invitation on an object: its inviting principal or addressee, or a grantor revoking it. */
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

    /** Require an offer's inviting principal to still hold the grant, the system's offers standing as made. */
    async #requireInviter(invitation: Invitation, now: number): Promise<void> {
        // let the system's offers stand as made
        if (invitation.inviter === null) {
            return;
        }

        // decide the grant as the inviting principal at the authentication it met to send the offer
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
        const row = await this.transaction.row(accessInvitation, { id: Invitation.id(id) });
        if (row === null || row.status !== "pending") {
            throw new AccessError("CONFLICT", "invitation is no longer pending");
        }

        // keep it settled
        await this.transaction.write(accessInvitation, row, { ...row, status, updatedAt: now });
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
        // require the grant permission
        const context = this.context();
        await this.#requireGrant(request.object, request.relation);

        // read the object's invitations after the cursor
        const role =
            request.role === undefined ? undefined : schema.identifier("role").parse(request.role);
        const rows = await this.transaction.rows(
            accessInvitation,
            {
                ...Relationship.objectColumns(request.object),
                ...(request.relation === undefined ? {} : { relation: request.relation }),
                ...(role === undefined ? {} : { roleId: role }),
                ...Invitation.pending(page, context.now),
            },
            { orderBy: { id: "asc" }, limit: page.limit },
        );

        return rows.map(Invitation.decode);
    }

    /** Define a role in a scope, granting only permissions the caller has there. */
    async createRole(scope: ObjectReference, request: RoleRequest): Promise<Role> {
        return this.transaction.savepoint(() => this.#defineRole(scope, request));
    }

    /** Change a role's name, purpose or permissions at a revision within the caller's permissions. */
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
            await this.transaction.write(accessRole, record, updated);

            // replace the permissions when given
            if (changes.permissions !== undefined) {
                await replace(this.transaction, record.id, scope.id, changes.permissions);
            }
            this.renew();

            return Role.describe(
                updated,
                changes.permissions ?? (await Role.permissions(this.transaction, record.id)),
            );
        });
    }

    /** Delete an unbound role at a revision, editable or deleted through the declaration managing it. */
    async deleteRole(scope: ObjectReference, id: string, revision: number): Promise<Role> {
        return this.transaction.savepoint(async () => {
            // require an editable role at the revision and permission to delete roles
            const record = await Role.read(this.transaction, scope.id, id);
            if (record.revision !== revision) {
                throw new AccessError("CONFLICT", "role revision has changed");
            }
            await this.authorizeRole(scope, [policies.role.permission("delete")], record);

            // refuse deleting a role that is still bound or included
            const bindings = await this.transaction.rows(
                accessRelationship,
                {
                    OR: [
                        { roleId: record.id },
                        {
                            subjectPackageId: policies.role.definition.packageId,
                            subjectType: policies.role.name,
                            subjectId: record.id,
                        },
                    ],
                },
                { limit: 1 },
            );
            if (bindings.length > 0) {
                throw new AccessError("CONFLICT", "role is still bound or included");
            }

            // delete the role with its permissions
            const permissions = await Role.permissions(this.transaction, record.id);
            await replace(this.transaction, record.id, scope.id, []);
            await this.transaction.write(accessRole, record, null);
            this.renew();

            return Role.describe(record, permissions);
        });
    }

    /** Keep a role by name in a scope with exactly the requested purpose and permissions, under the declaration applied if any, writing only a difference, and return it. */
    async keepRole(scope: ObjectReference, request: RoleRequest): Promise<Role> {
        return this.transaction.savepoint(async () => {
            // read the role holding the name, defining it when none does
            const now = this.context().now;
            const [kept] = await this.transaction.rows(accessRole, {
                scope: scope.id,
                name: request.name,
            });
            if (kept === undefined) {
                return this.#defineRole(scope, request);
            }

            // keep only a role under the same manager, leaving one granting exactly the request as it is
            await this.authorizeRole(
                scope,
                [policies.role.permission("update"), ...request.permissions],
                kept,
            );
            if (!isManagedBy(kept, this.#manager)) {
                throw new AccessError("CONFLICT", "role name is already in use");
            }
            const granted = await Role.permissions(this.transaction, kept.id);
            const isGranted = isSamePermissions(granted, request.permissions);
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
            await this.transaction.write(accessRole, kept, updated);
            if (!isGranted) {
                await replace(this.transaction, kept.id, scope.id, request.permissions);
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
        const id = await define(this.transaction, scope.id, request, now, this.#manager);
        if (id === undefined) {
            throw new AccessError("CONFLICT", "role name is already in use");
        }
        const record = await Role.read(this.transaction, scope.id, id);
        this.renew();

        return Role.describe(record, request.permissions);
    }

    /** Bind a role on an object to one subject alone as the caller may, revoking its other bindings and writing nothing when bound so already. */
    async assign(object: ObjectReference, role: string, subject: Subject): Promise<Relationship> {
        const relationship = await this.transaction.savepoint(async () => {
            // read the role's bindings
            const request = { object, role, subject };
            const id = schema.identifier("role").parse(role);
            const rows = await this.transaction.rows(accessRelationship, { roleId: id });
            const bound = rows.map((row) => Relationship.decode(row));

            // leave a role bound to the subject alone as it is
            const [only] = bound;
            if (bound.length === 1 && only !== undefined && isBinding(only, request)) {
                return only;
            }

            // revoke its other bindings and bind it to the subject, as the caller may
            await this.authorizeGrant(request);
            for (const [index, row] of rows.entries()) {
                await this.authorizeRevoke(aligned(bound, index), row);
            }
            for (const row of rows) {
                await this.transaction.write(accessRelationship, row, null);
            }

            return this.#insert(request);
        });
        this.renew();

        return relationship;
    }

    /** Keep exactly the wanted relationships among those a selection covers as the caller may, leaving the ones still wanted and granting new ones under the declaration applied if any. */
    async keepRelationships(
        selection: RelationshipSelection,
        wanted: readonly RelationshipRequest[],
    ): Promise<void> {
        await this.transaction.savepoint(async () => {
            // read the relationships the selection covers
            const related = (await readSelected(this.transaction, selection, this.#manager)).map(
                (row) => ({
                    row,
                    relationship: Relationship.decode(row),
                }),
            );

            // revoke the relationships no longer wanted
            const missing = new Map(wanted.map((request) => [identityOf(request), request]));
            const stale = related.filter(
                ({ relationship }) => !missing.delete(identityOf(relationship)),
            );
            for (const { row, relationship } of stale) {
                await this.authorizeRevoke(relationship, row);
                await this.transaction.write(accessRelationship, row, null);
            }

            // grant the wanted relationships missing
            for (const request of missing.values()) {
                await this.authorizeGrant(request);
                await this.#insert(request);
            }
        });
        this.renew();
    }

    /** Keep one role by name in a scope granting exactly some permissions, bound to one subject on the scope or an object in it under the binding's conditions, as the declaration applied, dropping both when it grants none. */
    async keepGrant(
        scope: ObjectReference,
        grant: Omit<RoleRequest, "permissions"> & {
            /** The permissions the role grants, none to drop the grant. */
            readonly permissions: RoleRequest["permissions"];
            /** The one subject the role is bound to. */
            readonly subject: Subject;
            /** The object in the scope the role is bound on, the scope itself when absent. */
            readonly object?: ObjectReference;
            /** What a request must satisfy for the binding to apply, such as the principal a delegate acts for. */
            readonly conditions?: RelationshipCondition;
        },
    ): Promise<void> {
        // drop the binding and the role of a grant of nothing
        const { subject, object, conditions, ...request } = grant;
        if (request.permissions.length === 0) {
            await this.keepRelationships({ scope: scope.id }, []);
            const [kept] = await this.transaction.rows(accessRole, {
                scope: scope.id,
                name: request.name,
            });
            if (kept !== undefined) {
                await this.deleteRole(scope, kept.id, kept.revision);
            }

            return;
        }

        // keep the role with exactly the permissions, bound to the subject alone
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

    /** Require permissions to change roles in a scope this database keeps as the transaction shows the caller's access afresh, refusing a managed role outside its declaration. */
    protected async authorizeRole(
        scope: ObjectReference,
        permissions: readonly PermissionReference[],
        record?: Select<typeof accessRole>,
    ): Promise<void> {
        // write only the roles of a scope this database keeps, refusing a managed role outside its declaration until it is detached
        await this.authorizer.requireLocal(this.transaction, scope);
        if (record !== undefined) {
            requireChangeable(record, this.#manager);
        }

        // require every permission in the scope
        const access = await this.authorizer.resolve(this.snapshot, scope.id, this.context());
        await this.authorizer.require(this.snapshot, permissions, scope, access);
    }

    /** Require the grant permission and every permission a granted role grants, or only the lender for a delegation. */
    protected async authorizeGrant(request: RelationshipFields): Promise<void> {
        // write only the access of an object this database keeps
        await this.authorizer.requireLocal(this.transaction, request.object);

        // lend only the caller's own authority, and only to one principal
        const scope = this.authorizer.governingScope(request.object);
        const context = this.context();
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

    /** Require an object this database keeps and the permission granting what a relationship grants, unless the caller lent it, refusing a managed relationship outside its declaration. */
    protected async authorizeRevoke(
        request: RelationshipFields,
        record?: Select<typeof accessRelationship>,
    ): Promise<void> {
        // write only the access of an object this database keeps, refusing a managed relationship outside its declaration until it is detached
        await this.authorizer.requireLocal(this.transaction, request.object);
        if (record !== undefined) {
            requireChangeable(record, this.#manager);
        }

        // let a lender manage its own delegations
        if (!this.#lends(request)) {
            await this.#requireGrant(request.object, relationOf(request));
        }
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

    /** Insert a valid relationship under a new identifier and the declaration applied if any, and refuse a duplicate. */
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
        const same = await this.transaction.rows(accessRelationship, Relationship.equal(row), {
            limit: 1,
        });
        if (same.length > 0) {
            throw new AccessError("CONFLICT", "relationship already exists");
        }
        await this.transaction.write(
            accessRelationship,
            null,
            accessRelationship[TABLE].complete(row),
        );

        return relationship;
    }

    /** Determine whether the caller is the principal a delegation lends authority from. */
    #lends(request: RelationshipFields): boolean {
        const onBehalfOf = request.conditions?.onBehalfOf;
        const caller = Caller.principal(this.context());

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
        await this.authorizer.requireLocal(this.transaction, object);
        if (this.authorizer.policy(object).definition.scope !== true) {
            throw new AccessError("INVALID_CONTEXT", `${object.type} is not a scope`);
        }

        // mark the scope's record
        const row = await this.transaction.row(Scope.table, { scope: object.id });
        if (row === null) {
            throw new AccessError("NOT_FOUND", `unknown scope: ${object.id}`);
        }
        await this.transaction.write(Scope.table, row, { ...row, suspendedAt });
        this.renew();
    }
}

/** Refuse changing a row that a declaration manages until it is detached, unless the change applies that declaration's installation and package. */
function requireChangeable(
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

/** Refuse removing the last binding of a role that grants everything on an object. */
async function requireRemainingOwner(
    transaction: Transaction,
    row: Select<typeof accessRelationship>,
): Promise<void> {
    // skip relations
    if (row.roleId === null) {
        return;
    }

    // read the object's role bindings and the roles among them granting everything
    const bindings = await transaction.rows(accessRelationship, {
        objectScope: row.objectScope,
        packageId: row.packageId,
        type: row.type,
        objectId: row.objectId,
        roleId: { isNotNull: true },
    });
    const roles = [...new Set(bindings.flatMap((binding) => binding.roleId ?? []))];
    const universal = await transaction.rows(accessRole, { id: { in: roles }, isUniversal: true });
    const owning = new Set(universal.map((role) => role.id));
    const owners = bindings.filter(
        (binding) => binding.roleId !== null && owning.has(binding.roleId),
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

/** Report whether a role is under a manager, or under none for null. */
function isManagedBy(record: Select<typeof accessRole>, manager: Manager | null): boolean {
    const columns = Manager.values(manager);

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
